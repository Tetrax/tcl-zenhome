//! État des lieux de la box (lecture seule) : analyseurs purs des sorties adb, puis `inventaire` qui lit tout,
//! relit ce qui revient vide (lien instable) et écrit la sauvegarde brute et `resume.txt`, comme le kit.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use super::appareils::SOCLE;
use super::connexion::{EtatConnexion, Politique, assurer_connexion, nettoyer_modele};
use super::transport::{Commande, Dumpsys, Espace, ErreurTransport, OptionPm, Transport, est_format, paquet_valide};

/// Lectures d'une même commande avant d'abandonner (le kit relit 3 fois la liste des paquets).
const ESSAIS_LECTURE: u32 = 3;
const DEBUT_DATA_LIBELLES: &str = "Broadcast completed: result=1, data=\"";

// ---------------------------------------------------------------- analyseurs purs

/// Noms de paquets d'une sortie `pm list packages` (avec ou sans `-f` : `package:/chemin.apk=nom`). Pas de validation ici.
pub fn noms_paquets(texte: &str) -> Vec<String> {
    texte
        .lines()
        .filter_map(|l| l.trim_end().strip_prefix("package:"))
        .map(|r| r.rsplit_once('=').map_or(r, |(_, n)| n).trim().to_string())
        .filter(|n| !n.is_empty())
        .collect()
}

/// Éléments de `Libellé: ["a","b",...]` (entre le premier `[` et le premier `]` suivant).
fn liste_json(ligne: &str) -> Vec<String> {
    let Some((_, reste)) = ligne.split_once('[') else { return Vec::new() };
    let contenu = reste.split(']').next().unwrap_or_default().replace('"', "");
    if contenu.is_empty() { Vec::new() } else { contenu.split(',').map(str::to_string).collect() }
}

/// Octets (appli + données + cache) par paquet, depuis les tableaux alignés de `dumpsys diskstats`.
/// Un tableau plus court compte pour 0 (comme awk) ; en cas de doublon, la première occurrence gagne.
pub fn tailles(texte: &str) -> HashMap<String, u64> {
    let (mut noms, mut app, mut donnees, mut cache) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for l in texte.lines() {
        if l.contains("Package Names:") {
            noms = liste_json(l);
        } else if l.contains("App Data Sizes:") {
            donnees = liste_json(l);
        } else if l.contains("App Sizes:") {
            app = liste_json(l);
        } else if l.contains("Cache Sizes:") {
            cache = liste_json(l);
        }
    }
    let n = |v: &[String], i: usize| v.get(i).and_then(|s| s.trim().parse::<u64>().ok()).unwrap_or(0);
    let mut res = HashMap::new();
    for (i, nom) in noms.into_iter().enumerate() {
        let total = n(&app, i).saturating_add(n(&donnees, i)).saturating_add(n(&cache, i));
        res.entry(nom).or_insert(total);
    }
    res
}

/// « 1,2 Go », « 19 Mo » ou « moins de 1 Mo » (fonction `hum` du kit).
pub fn taille_lisible(octets: u64) -> String {
    if octets >= 1 << 30 {
        format!("{:.1} Go", octets as f64 / 1_073_741_824.0).replacen('.', ",", 1)
    } else if octets >= 1 << 20 {
        format!("{} Mo", (octets as f64 / 1_048_576.0 + 0.5) as u64)
    } else {
        "moins de 1 Mo".to_string()
    }
}

/// `In-memory <mot> stats` quelque part dans la ligne.
fn entete_stats(l: &str) -> bool {
    l.match_indices("In-memory ").any(|(i, m)| {
        let reste = &l[i + m.len()..];
        let mot = reste.chars().take_while(char::is_ascii_lowercase).count();
        mot > 0 && reste[mot..].starts_with(" stats")
    })
}

/// Dernière utilisation « JJ/MM/AAAA » par paquet, depuis la section « In-memory yearly stats » de `dumpsys usagestats`.
/// Dates antérieures à 1971 ou mal formées ignorées.
pub fn utilisations(texte: &str) -> HashMap<String, String> {
    let mut meilleur: HashMap<String, String> = HashMap::new();
    let mut dedans = false;
    for l in texte.lines() {
        if l.contains("In-memory yearly stats") {
            dedans = true;
            continue;
        }
        if dedans && entete_stats(l) {
            dedans = false;
        }
        if !dedans {
            continue;
        }
        let (Some(i), Some(j)) = (l.rfind("package="), l.rfind("lastTime=\"")) else { continue };
        let paquet = l[i + "package=".len()..].split([' ', '\t']).next().unwrap_or_default();
        let d: String = l[j + "lastTime=\"".len()..].chars().take(10).collect();
        let b = d.as_bytes();
        let forme = b.len() == 10
            && b.iter().enumerate().all(|(k, c)| if k == 4 || k == 7 { *c == b'-' } else { c.is_ascii_digit() });
        if !forme || d.as_str() <= "1971" || paquet.is_empty() {
            continue;
        }
        let m = meilleur.entry(paquet.to_string()).or_default();
        if d > *m {
            *m = d;
        }
    }
    meilleur.into_iter().map(|(p, d)| (p, format!("{}/{}/{}", &d[8..10], &d[5..7], &d[0..4]))).collect()
}

/// Paquets/libellés de la réponse du broadcast `labels` : exactement 2 champs séparés par une tabulation, paquet valide.
/// `None` si la réponse n'est pas un `Broadcast completed: result=1, data="..."` (libellés indisponibles).
/// Les libellés sont BRUTS : les passer par `nettoyer_libelle` avant tout affichage.
pub fn libelles_broadcast(sortie: &str) -> Option<Vec<(String, String)>> {
    let (_, data) = sortie.split_once(DEBUT_DATA_LIBELLES)?;
    let data = data.rfind('"').map_or(data, |i| &data[..i]);
    Some(
        data.lines()
            .filter_map(|l| l.split_once('\t'))
            .filter(|(p, reste)| !reste.contains('\t') && paquet_valide(p))
            .map(|(p, lib)| (p.to_string(), lib.to_string()))
            .collect(),
    )
}

/// Libellé renettoyé (revue sécurité) : sans contrôles C0/C1, formats/bidi, séparateurs de ligne/paragraphe ni guillemets,
/// blancs de bords retirés, 60 caractères au plus ; le nom du paquet si rien ne reste.
pub fn nettoyer_libelle(brut: &str, paquet: &str) -> String {
    let propre: String =
        brut.chars().filter(|&c| !(c.is_control() || est_format(c) || c == '\u{2028}' || c == '\u{2029}' || c == '"')).collect();
    let coupe: String = propre.trim().chars().take(60).collect();
    let coupe = coupe.trim_end();
    if coupe.is_empty() { paquet.to_string() } else { coupe.to_string() }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EtatWifi {
    /// Vérifié, aucun réseau créé par setupwraith.
    Libre,
    /// Un réseau a été créé par setupwraith : ne JAMAIS le désactiver.
    LieSetupwraith,
    /// Sortie vide ou inattendue : on ne conclut pas, setupwraith reste actif.
    NonVerifiable,
}

/// Logique de `controler` de verifier-tv.sh sur la sortie de `dumpsys wifi`.
pub fn etat_wifi(texte: &str) -> EtatWifi {
    let minuscules: Vec<String> = texte.lines().map(str::to_lowercase).collect();
    let a_un = |l: &str, motifs: &[&str]| motifs.iter().any(|m| l.contains(m));
    if !minuscules.iter().any(|l| a_un(l, &["configured networks", "cuid=", "cname=", "creatorname"])) {
        EtatWifi::NonVerifiable
    } else if minuscules.iter().any(|l| a_un(l, &["creatorname", "cname"]) && l.contains("setupwraith")) {
        EtatWifi::LieSetupwraith
    } else {
        EtatWifi::Libre
    }
}

/// Paquet de l'accueil préféré (Android 9 sans rôle HOME), lu dans `dumpsys package preferred-xml` : le dernier
/// `<item name="...">` avant la première ligne `category.HOME`.
pub fn accueil_prefere(xml: &str) -> Option<String> {
    let mut nom = None;
    for l in xml.lines() {
        if l.contains("<item name=") {
            nom = l.split('"').nth(1);
        }
        if l.contains("category.HOME") {
            let p: String = nom?.split('/').next()?.chars().filter(|c| c.is_ascii_alphanumeric() || "._-".contains(*c)).collect();
            return if p.is_empty() { None } else { Some(p) };
        }
    }
    None
}

/// Paquet au premier plan, lu dans `dumpsys activity activities` (`mResumedActivity` Android 9, `topResumedActivity` 10+).
pub fn premier_plan(texte: &str) -> Option<String> {
    texte
        .lines()
        .filter(|l| l.contains("mResumedActivity") || l.contains("topResumedActivity"))
        .filter_map(|l| l.find("ActivityRecord{").map(|i| &l[i..]))
        .find_map(|l| l.split_whitespace().find_map(|t| t.split_once('/').map(|(p, _)| p.to_string())))
}

/// Propriétés de `getprop` : lignes `[clé]: [valeur]`.
pub fn proprietes(texte: &str) -> HashMap<String, String> {
    texte
        .lines()
        .filter_map(|l| {
            let (k, v) = l.strip_prefix('[')?.split_once("]: [")?;
            Some((k.to_string(), v.strip_suffix(']')?.to_string()))
        })
        .collect()
}

/// `settings get global zenhome_garder` : liste séparée par des virgules, « null » et entrées invalides ignorés.
pub fn liste_garder(sortie: &str) -> Vec<String> {
    sortie.trim().split(',').filter(|e| paquet_valide(e)).map(str::to_string).collect()
}

// ---------------------------------------------------------------- inventaire

#[derive(Debug, Clone, PartialEq)]
pub struct AppUtilisateur {
    pub paquet: String,
    pub octets: Option<u64>,
    /// « JJ/MM/AAAA ».
    pub derniere_utilisation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Inventaire {
    pub modele: String,
    /// `ro.product.name`.
    pub produit: String,
    pub android: String,
    pub tous: Vec<String>,
    pub desactives: Vec<String>,
    pub utilisateur: Vec<String>,
    pub systeme: Vec<String>,
    /// Applis de l'utilisateur hors paquets protégés, dans l'ordre de `pm list packages -3`.
    pub applis: Vec<AppUtilisateur>,
    /// Dernière ligne de `cmd package resolve-activity` (accueil actuel, informatif).
    pub accueil_actuel: String,
    pub accueil_prefere: Option<String>,
    pub premier_plan: Option<String>,
    pub wifi: EtatWifi,
    pub garder: Vec<String>,
    /// Lectures facultatives qui ont échoué ou sont restées vides (l'inventaire continue).
    pub avertissements: Vec<String>,
}

#[derive(Debug)]
pub enum ErreurInventaire {
    /// La box ne répond plus ou attend une autorisation : l'appelant affiche la consigne et relance.
    AttenteUtilisateur { non_autorise: bool },
    /// Lecture indispensable restée vide ou en échec après relectures.
    Illisible(String),
    Transport(ErreurTransport),
    Ecriture(String),
}

impl fmt::Display for ErreurInventaire {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AttenteUtilisateur { non_autorise: true } => write!(f, "autorisation à accepter sur la TV"),
            Self::AttenteUtilisateur { .. } => write!(f, "la box ne répond plus"),
            Self::Illisible(m) => write!(f, "lecture impossible : {m}"),
            Self::Transport(e) => write!(f, "{e}"),
            Self::Ecriture(m) => write!(f, "écriture de la sauvegarde impossible : {m}"),
        }
    }
}

impl std::error::Error for ErreurInventaire {}

/// Lit une commande en s'assurant d'abord du lien ; vide (si `requis`), coupure ou échec : lien neuf puis relecture
/// (3 essais). Facultatif : un échec final donne un texte vide et un avertissement.
fn lire(t: &dyn Transport, cmd: &Commande, requis: bool, avert: &mut Vec<String>) -> Result<String, ErreurInventaire> {
    let pol = Politique::default();
    let mut dernier = String::new();
    for _ in 0..ESSAIS_LECTURE {
        if let EtatConnexion::AttenteUtilisateur { non_autorise } = assurer_connexion(t, &pol) {
            return Err(ErreurInventaire::AttenteUtilisateur { non_autorise });
        }
        match t.shell(cmd) {
            Ok(s) if !requis || !s.stdout.trim().is_empty() => return Ok(s.stdout),
            Ok(_) => dernier = "réponse vide".into(),
            Err(ErreurTransport::NonAutorise) => return Err(ErreurInventaire::AttenteUtilisateur { non_autorise: true }),
            Err(e @ (ErreurTransport::Refusee(_) | ErreurTransport::Lancement(_))) => return Err(ErreurInventaire::Transport(e)),
            Err(e) => dernier = e.to_string(),
        }
        t.deconnecter();
    }
    if requis {
        Err(ErreurInventaire::Illisible(format!("{cmd} : {dernier}")))
    } else {
        avert.push(format!("{cmd} : {dernier}"));
        Ok(String::new())
    }
}

/// Liste de paquets telle que le kit l'écrit : sans ligne vide finale, un seul saut de ligne.
fn avec_fin_de_ligne(t: &str) -> String {
    format!("{}\n", t.trim_end_matches('\n'))
}

/// Contenu de `resume.txt` (même format que le kit).
fn construire_resume(horodatage: &str, inv: &Inventaire) -> String {
    let accueil: String = inv.accueil_actuel.chars().filter(|c| c.is_ascii_alphanumeric() || "._/$ -".contains(*c)).collect();
    let mut s = format!(
        "État des lieux de la box, le {horodatage}\nModèle : {} ({}), Android {}\nApplis installées : {}, dont désactivées : {}\n\
         Accueil actuel : {accueil}\n\nApplis installées par vous (paquet : taille, dernière utilisation) :\n",
        nettoyer_modele(&inv.modele),
        nettoyer_modele(&inv.produit),
        nettoyer_modele(&inv.android),
        inv.tous.len(),
        inv.desactives.len(),
    );
    for a in &inv.applis {
        let taille = a.octets.map_or_else(|| "inconnue".to_string(), taille_lisible);
        let date = a.derniere_utilisation.as_deref().unwrap_or("inconnue");
        s.push_str(&format!("  {} : {taille}, {date}\n", a.paquet));
    }
    s
}

#[cfg(unix)]
fn creer_dossier_prive(d: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    // Le dossier lui-même doit être neuf : un dossier ou un lien déjà là n'est jamais réutilisé.
    if let Some(parent) = d.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(parent)?;
    }
    std::fs::DirBuilder::new().mode(0o700).create(d)?;
    std::fs::set_permissions(d, std::fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn creer_dossier_prive(d: &Path) -> std::io::Result<()> {
    if let Some(parent) = d.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir(d)
}

#[cfg(unix)]
fn ecrire_prive(f: &Path, contenu: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let mut fichier = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(f)?;
    fichier.write_all(contenu.as_bytes())?;
    std::fs::set_permissions(f, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn ecrire_prive(f: &Path, contenu: &str) -> std::io::Result<()> {
    use std::io::Write;
    std::fs::OpenOptions::new().write(true).create_new(true).open(f)?.write_all(contenu.as_bytes())
}

/// Lit l'état de la box (rien n'est modifié), écrit la sauvegarde brute et `resume.txt` dans `dossier`
/// (neuf : créé en 700, jamais réutilisé ; fichiers en 600, jamais écrasés) et renvoie l'inventaire. `horodatage` : « JJ/MM/AAAA HH:MM », fourni par l'appelant.
///
/// Fichiers comme le kit : paquets-{tous,desactives,utilisateur,systeme}, reglages-{global,secure,system}, proprietes,
/// diskstats, accueil, resume. `dumpsys usagestats` (historique détaillé) et `dumpsys wifi` (SSID) ne sont PAS conservés.
pub fn inventaire(t: &dyn Transport, dossier: &Path, horodatage: &str) -> Result<Inventaire, ErreurInventaire> {
    let mut avert = Vec::new();
    let pm = |o: &[OptionPm]| Commande::pm_list(o);
    let tous = lire(t, &pm(&[OptionPm::Chemins]), true, &mut avert)?;
    if !tous.contains("package:") {
        return Err(ErreurInventaire::Illisible("liste des paquets sans « package: »".into()));
    }
    let desactives = lire(t, &pm(&[OptionPm::Desactives]), false, &mut avert)?;
    let utilisateur = lire(t, &pm(&[OptionPm::Tiers]), false, &mut avert)?;
    let systeme = lire(t, &pm(&[OptionPm::Systeme]), false, &mut avert)?;
    let global = lire(t, &Commande::settings_list(Espace::Global), true, &mut avert)?;
    let secure = lire(t, &Commande::settings_list(Espace::Secure), true, &mut avert)?;
    let system = lire(t, &Commande::settings_list(Espace::System), false, &mut avert)?;
    let props_brut = lire(t, &Commande::getprop_tout(), true, &mut avert)?;
    let diskstats = lire(t, &Commande::dumpsys(Dumpsys::Diskstats), false, &mut avert)?;
    let usagestats = lire(t, &Commande::dumpsys(Dumpsys::Usagestats), false, &mut avert)?;
    let accueil = lire(t, &Commande::resoudre_accueil(), false, &mut avert)?;
    let xml = lire(t, &Commande::dumpsys(Dumpsys::PreferredXml), false, &mut avert)?;
    let activites = lire(t, &Commande::dumpsys(Dumpsys::Activities), false, &mut avert)?;
    let wifi = lire(t, &Commande::dumpsys(Dumpsys::Wifi), false, &mut avert)?;
    let garder = lire(t, &Commande::settings_get(Espace::Global, "zenhome_garder").map_err(ErreurInventaire::Transport)?, false, &mut avert)?;

    let (tai, der) = (tailles(&diskstats), utilisations(&usagestats));
    let utilisateur_noms = noms_paquets(&utilisateur);
    let applis = utilisateur_noms
        .iter()
        .filter(|p| paquet_valide(p) && !SOCLE.contains(&p.as_str()))
        .map(|p| AppUtilisateur { paquet: p.clone(), octets: tai.get(p).copied(), derniere_utilisation: der.get(p).cloned() })
        .collect();
    let props = proprietes(&props_brut);
    let prop = |k: &str| props.get(k).cloned().unwrap_or_default();
    let inv = Inventaire {
        modele: prop("ro.product.model"),
        produit: prop("ro.product.name"),
        android: prop("ro.build.version.release"),
        tous: noms_paquets(&tous),
        desactives: noms_paquets(&desactives),
        utilisateur: utilisateur_noms,
        systeme: noms_paquets(&systeme),
        applis,
        accueil_actuel: accueil.lines().last().unwrap_or_default().to_string(),
        accueil_prefere: accueil_prefere(&xml),
        premier_plan: premier_plan(&activites),
        wifi: etat_wifi(&wifi),
        garder: liste_garder(&garder),
        avertissements: avert,
    };

    let fichiers = [
        ("paquets-tous.txt", tous),
        ("paquets-desactives.txt", avec_fin_de_ligne(&desactives)),
        ("paquets-utilisateur.txt", avec_fin_de_ligne(&utilisateur)),
        ("paquets-systeme.txt", systeme),
        ("reglages-global.txt", global),
        ("reglages-secure.txt", secure),
        ("reglages-system.txt", system),
        ("proprietes.txt", props_brut),
        ("diskstats.txt", diskstats),
        ("accueil.txt", accueil),
        ("resume.txt", construire_resume(horodatage, &inv)),
    ];
    let ecrire = || -> std::io::Result<()> {
        creer_dossier_prive(dossier)?;
        fichiers.iter().try_for_each(|(nom, contenu)| ecrire_prive(&dossier.join(nom), contenu))
    };
    ecrire().map_err(|e| ErreurInventaire::Ecriture(e.to_string()))?;
    Ok(inv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::fake_box::{FakeBox, WifiFake};
    use std::sync::atomic::{AtomicU32, Ordering};

    const TOUS: &str = include_str!("../../tests/donnees/paquets-tous.txt");
    const SYSTEME: &str = include_str!("../../tests/donnees/paquets-systeme.txt");
    const UTILISATEUR: &str = include_str!("../../tests/donnees/paquets-utilisateur.txt");
    const DESACTIVES: &str = include_str!("../../tests/donnees/paquets-desactives.txt");
    const DISKSTATS: &str = include_str!("../../tests/donnees/diskstats.txt");
    const USAGESTATS: &str = include_str!("../../tests/donnees/usagestats.txt");
    const PROPRIETES: &str = include_str!("../../tests/donnees/proprietes.txt");
    const ACCUEIL: &str = include_str!("../../tests/donnees/accueil.txt");
    const RESUME: &str = include_str!("../../tests/donnees/resume.txt");
    const LABELS: &str = include_str!("../../tests/donnees/labels.txt");

    /// Dossier de test supprimé à la fin.
    struct Temp(std::path::PathBuf);
    impl Temp {
        fn nouveau() -> Self {
            static N: AtomicU32 = AtomicU32::new(0);
            let d = std::env::temp_dir().join(format!("zenhome-test-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
            Self(d.join("sauvegarde"))
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
        }
    }

    #[test]
    fn pm_list_sur_donnees_reelles() {
        let tous = noms_paquets(TOUS);
        assert_eq!(tous.len(), 90);
        assert_eq!(tous.iter().filter(|n| n.as_str() == "com.google.android.apps.mediashell").count(), 1, "chemin avec « = » conservé");
        assert!(tous.iter().all(|n| !n.contains('/') && !n.contains('=')));
        assert_eq!(noms_paquets(UTILISATEUR), ["com.spotify.tv.android", "com.exemple.lecteur", "com.exemple.musique", "fr.tf1.mytf1"]);
        assert_eq!(noms_paquets(SYSTEME).len(), 86);
        assert!(noms_paquets(DESACTIVES).is_empty());
        assert_eq!(noms_paquets("package:a.b\r\nbruit\npackage:\npackage:/x.apk=c.d uid:1\n"), ["a.b", "c.d uid:1"]);
    }

    #[test]
    fn tailles_sur_diskstats_reel() {
        let t = tailles(DISKSTATS);
        assert_eq!(t.len(), 91, "diskstats connaît un paquet de plus que pm list");
        assert_eq!(taille_lisible(t["com.spotify.tv.android"]), "19 Mo");
        assert_eq!(taille_lisible(t["fr.tf1.mytf1"]), "143 Mo");
        assert_eq!(taille_lisible(t["com.exemple.musique"]), "595 Mo");
        assert!(!t.contains_key("App Size"));
    }

    #[test]
    fn tailles_cas_limites() {
        let t = tailles("Package Names: [\"a.b\",\"c.d\"]\nApp Sizes: [1,2]\nApp Data Sizes: [10]\nCache Sizes: [x,100]\n");
        assert_eq!((t["a.b"], t["c.d"]), (11, 102));
        assert!(tailles("Package Names: []\nApp Sizes: []\n").is_empty());
        assert!(tailles("").is_empty());
        assert_eq!(taille_lisible(0), "moins de 1 Mo");
        assert_eq!(taille_lisible(1_048_575), "moins de 1 Mo");
        assert_eq!(taille_lisible(1_048_576), "1 Mo");
        assert_eq!(taille_lisible(1_572_864), "2 Mo");
        assert_eq!(taille_lisible(1_073_741_823), "1024 Mo");
        assert_eq!(taille_lisible(1_073_741_824), "1,0 Go");
        assert_eq!(taille_lisible(2_684_354_560), "2,5 Go");
    }

    #[test]
    fn utilisations_section_annuelle_seulement() {
        let u = utilisations(USAGESTATS);
        assert_eq!(u["com.spotify.tv.android"], "27/10/2026", "la plus récente de la section annuelle, pas du jour");
        assert_eq!(u["fr.tf1.mytf1"], "14/10/2026");
        assert_eq!(u["com.exemple.lecteur"], "28/10/2026", "la section mensuelle est ignorée");
        assert!(!u.contains_key("com.exemple.ancien") && !u.contains_key("com.exemple.illisible"));
        assert!(utilisations("package=a.b lastTime=\"2026-01-01 00:00:00\"").is_empty(), "hors section annuelle");
    }

    #[test]
    fn proprietes_et_modele() {
        let p = proprietes(PROPRIETES);
        assert_eq!(p["ro.product.name"], "oneday");
        assert_eq!(p["ro.product.model"], "MIBOX4");
        assert_eq!(p["ro.build.version.release"], "9");
    }

    #[test]
    fn libelles_du_broadcast() {
        let l = libelles_broadcast(LABELS).unwrap();
        let paquets: Vec<&str> = l.iter().map(|(p, _)| p.as_str()).collect();
        // 2 tabulations, paquet invalide et ligne sans tabulation écartés
        assert_eq!(paquets, ["com.spotify.tv.android", "com.exemple.lecteur", "tv.zenhome", "fr.tf1.mytf1"]);
        let net = |p: &str| nettoyer_libelle(&l.iter().find(|(x, _)| x == p).unwrap().1, p);
        assert_eq!(net("com.exemple.lecteur"), "Lecteur TV", "bidi retiré");
        assert_eq!(net("fr.tf1.mytf1"), "MyTF1", "guillemets et blancs retirés");
        assert_eq!(libelles_broadcast("Broadcast completed: result=0"), None);
        assert_eq!(libelles_broadcast("Broadcast completed: result=1, data=\"a.b\tX\n\""), Some(vec![("a.b".into(), "X".into())]));
        assert_eq!(libelles_broadcast("Broadcast completed: result=1, data=\"\""), Some(vec![]));
    }

    #[test]
    fn nettoyage_des_libelles() {
        assert_eq!(nettoyer_libelle("Ma\u{7}Box\u{85}\u{202e}TV", "a.b"), "MaBoxTV");
        assert_eq!(nettoyer_libelle("\u{2028}\u{2029}\u{200b}\u{feff} \"\" ", "a.b"), "a.b", "rien ne reste : nom du paquet");
        assert_eq!(nettoyer_libelle("   ", "a.b"), "a.b");
        assert_eq!(nettoyer_libelle("", "a.b"), "a.b");
        assert_eq!(nettoyer_libelle(&"é".repeat(70), "a.b").chars().count(), 60);
        // coupure à 60 puis blancs finaux retirés
        assert_eq!(nettoyer_libelle(&format!("{} {}", "a".repeat(59), "b"), "x.y"), "a".repeat(59));
        assert_eq!(nettoyer_libelle("Café Müller 😀", "a.b"), "Café Müller 😀");
    }

    #[test]
    fn etat_wifi_sur_trois_cas() {
        assert_eq!(etat_wifi(include_str!("../../tests/donnees/wifi-libre.txt")), EtatWifi::Libre);
        assert_eq!(etat_wifi(include_str!("../../tests/donnees/wifi-setupwraith.txt")), EtatWifi::LieSetupwraith);
        assert_eq!(etat_wifi(include_str!("../../tests/donnees/wifi-vide.txt")), EtatWifi::NonVerifiable);
        assert_eq!(etat_wifi(""), EtatWifi::NonVerifiable);
        // setupwraith ailleurs que dans le créateur n'est pas un lien
        assert_eq!(etat_wifi("Configured networks\nSSID: \"setupwraith\"\n"), EtatWifi::Libre);
        assert_eq!(etat_wifi("Configured networks\nCreatorName=COM.GOOGLE.SETUPWRAITH\n"), EtatWifi::LieSetupwraith);
    }

    #[test]
    fn accueil_prefere_depuis_preferred_xml() {
        assert_eq!(accueil_prefere(include_str!("../../tests/donnees/preferred-xml-zenhome.txt")).as_deref(), Some("tv.zenhome"));
        assert_eq!(accueil_prefere(include_str!("../../tests/donnees/preferred-xml-aucun.txt")), None);
        assert_eq!(accueil_prefere(""), None);
        assert_eq!(accueil_prefere("<cat name=\"android.intent.category.HOME\" />"), None, "pas d'item avant");
    }

    #[test]
    fn premier_plan_depuis_dumpsys_activity() {
        assert_eq!(premier_plan(include_str!("../../tests/donnees/activity-zenhome.txt")).as_deref(), Some("tv.zenhome"));
        assert_eq!(
            premier_plan(include_str!("../../tests/donnees/activity-origine.txt")).as_deref(),
            Some("com.google.android.tvlauncher")
        );
        assert_eq!(premier_plan("  mResumedActivity: null\n"), None);
        assert_eq!(
            premier_plan("  topResumedActivity=ActivityRecord{1 u0 com.x.y/.Z t4}\n").as_deref(),
            Some("com.x.y"),
            "Android 10+"
        );
        // la dernière activité en pause n'est pas le premier plan
        assert_eq!(premier_plan("  mLastPausedActivity: ActivityRecord{1 u0 a.b/.C t1}\n"), None);
    }

    #[test]
    fn garder_ignore_null_et_invalides() {
        assert_eq!(liste_garder("null\n"), Vec::<String>::new());
        assert_eq!(liste_garder("a.b,c.d;rm,e.f\n"), ["a.b", "e.f"]);
        assert!(liste_garder("").is_empty());
    }

    #[test]
    fn resume_identique_a_celui_du_kit() {
        let tai = tailles(DISKSTATS);
        let der = utilisations(USAGESTATS);
        let applis = noms_paquets(UTILISATEUR)
            .into_iter()
            .filter(|p| paquet_valide(p) && !SOCLE.contains(&p.as_str()))
            .map(|p| AppUtilisateur { octets: tai.get(&p).copied(), derniere_utilisation: der.get(&p).cloned(), paquet: p })
            .collect();
        let p = proprietes(PROPRIETES);
        let inv = Inventaire {
            modele: p["ro.product.model"].clone(),
            produit: p["ro.product.name"].clone(),
            android: p["ro.build.version.release"].clone(),
            tous: noms_paquets(TOUS),
            desactives: noms_paquets(DESACTIVES),
            utilisateur: vec![],
            systeme: vec![],
            applis,
            accueil_actuel: ACCUEIL.lines().last().unwrap().to_string(),
            accueil_prefere: None,
            premier_plan: None,
            wifi: EtatWifi::Libre,
            garder: vec![],
            avertissements: vec![],
        };
        assert_eq!(construire_resume("28/10/2026 22:06", &inv), RESUME);
    }

    /// Les données de test vont dans un dépôt PUBLIC : seules des valeurs factices sont admises.
    #[test]
    fn donnees_de_test_anonymisees() {
        let dossier = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/donnees");
        let mut vus = 0;
        for entree in std::fs::read_dir(dossier).unwrap() {
            let chemin = entree.unwrap().path();
            let nom = chemin.file_name().unwrap().to_string_lossy().into_owned();
            let texte = std::fs::read_to_string(&chemin).unwrap();
            vus += 1;
            for (n, l) in texte.lines().enumerate() {
                let ou = format!("{nom}:{}", n + 1);
                // adresses MAC et Bluetooth : uniquement 02:00:00:00:00:xx (localement administrées, factices)
                for m in l.as_bytes().windows(17) {
                    let mac = m.iter().enumerate().all(|(k, c)| if k % 3 == 2 { *c == b':' } else { c.is_ascii_hexdigit() });
                    assert!(!mac || m.to_ascii_lowercase().starts_with(b"02:00:00:00:00:"), "MAC réelle ? {ou}");
                }
                for (cle, factice) in [
                    ("ro.serialno]", "[TESTSERIAL0001]"),
                    ("ro.boot.serialno]", "[TESTSERIAL0001]"),
                    ("android_id=", "0123456789abcdef"),
                    ("device_name=", "BOX-TEST"),
                    ("mitv_one_id=", "00000000000000000000000000000000000000"),
                    ("mitv_iteration_id=", "00000000000000000000000000000000"),
                ] {
                    if let Some((_, v)) = l.split_once(cle) {
                        assert!(v.trim().ends_with(factice) || v.trim() == factice, "identifiant réel ? {ou}");
                    }
                }
                if l.contains("SSID") || l.contains("ssid") {
                    assert!(l.contains("RESEAU-TEST") || l.contains("\"setupwraith\""), "SSID réel ? {ou}");
                }
            }
        }
        assert!(vus >= 20, "dossier de données incomplet : {vus} fichiers");
    }

    fn box_connectee_apres_install() -> FakeBox {
        let f = FakeBox::mibox4();
        f.modifier(|e| {
            e.global.insert("zenhome_garder".into(), "com.google.android.katniss,x;y".into());
        });
        f
    }

    #[test]
    fn inventaire_complet_sur_fakebox() {
        let t = Temp::nouveau();
        let f = box_connectee_apres_install();
        let inv = inventaire(&f, &t.0, "28/10/2026 22:06").unwrap();
        assert_eq!((inv.produit.as_str(), inv.modele.as_str(), inv.android.as_str()), ("oneday", "MIBOX4", "9"));
        assert_eq!(inv.tous.len(), 24 + 5 + 4);
        assert!(inv.desactives.is_empty() && inv.avertissements.is_empty());
        assert_eq!(inv.utilisateur.len(), 4);
        assert_eq!(inv.garder, ["com.google.android.katniss"]);
        let spotify = inv.applis.iter().find(|a| a.paquet == "com.spotify.tv.android").unwrap();
        assert_eq!(spotify.octets.map(taille_lisible).as_deref(), Some("19 Mo"));
        assert_eq!(spotify.derniere_utilisation.as_deref(), Some("27/10/2026"));
        assert_eq!(inv.accueil_actuel, "com.google.android.tvlauncher/.MainActivity");
        assert_eq!(inv.accueil_prefere, None);
        assert_eq!(inv.premier_plan.as_deref(), Some("com.google.android.tvlauncher"));
        assert_eq!(inv.wifi, EtatWifi::Libre);
        // la lecture ne modifie rien : aucune commande d'écriture
        let journal = f.journal().join("\n");
        for ecriture in ["settings put", "settings delete", "disable-user", "pm enable", "set-home-activity", "install", "keyevent"] {
            assert!(!journal.contains(ecriture), "{ecriture}");
        }
        let resume = std::fs::read_to_string(t.0.join("resume.txt")).unwrap();
        assert!(resume.starts_with("État des lieux de la box, le 28/10/2026 22:06\nModèle : MIBOX4 (oneday), Android 9\nApplis installées : 33, dont désactivées : 0\n"));
        assert!(resume.contains("  com.spotify.tv.android : 19 Mo, 27/10/2026\n"));
    }

    #[test]
    fn inventaire_wifi_accueil_et_zenhome() {
        let t = Temp::nouveau();
        let f = FakeBox::mibox4();
        f.wifi(WifiFake::LieSetupwraith);
        f.connecter().unwrap();
        f.install(Path::new("ZenHome.apk")).unwrap();
        f.shell(&Commande::set_home_activity("tv.zenhome/.Home").unwrap()).unwrap();
        f.shell(&Commande::pm_disable("com.google.android.tvlauncher").unwrap()).unwrap();
        f.shell(&Commande::touche_accueil()).unwrap();
        let inv = inventaire(&f, &t.0, "h").unwrap();
        assert_eq!(inv.wifi, EtatWifi::LieSetupwraith);
        assert_eq!(inv.accueil_prefere.as_deref(), Some("tv.zenhome"));
        assert_eq!(inv.premier_plan.as_deref(), Some("tv.zenhome"));
        assert_eq!(inv.desactives, ["com.google.android.tvlauncher"]);
        assert!(inv.applis.iter().all(|a| a.paquet != "tv.zenhome"), "paquet protégé jamais proposé");
        // Zen Home absent du premier plan
        let g = FakeBox::mibox4();
        g.connecter().unwrap();
        g.install(Path::new("ZenHome.apk")).unwrap();
        g.zen_home_visible(false);
        g.modifier(|e| e.premier_plan = None);
        assert_eq!(inventaire(&g, &Temp::nouveau().0, "h").unwrap().premier_plan, None);
    }

    #[cfg(unix)]
    #[test]
    fn sauvegarde_refuse_un_dossier_existant_et_les_liens() {
        let t = Temp::nouveau();
        let cible = t.0.parent().unwrap().join("cible.txt");
        std::fs::create_dir_all(&t.0).unwrap();
        std::fs::write(&cible, "intact").unwrap();
        std::os::unix::fs::symlink(&cible, t.0.join("resume.txt")).unwrap();
        let r = inventaire(&FakeBox::mibox4(), &t.0, "h");
        assert!(matches!(r, Err(ErreurInventaire::Ecriture(_))), "{r:?}");
        assert_eq!(std::fs::read_to_string(&cible).unwrap(), "intact", "le lien n'est pas suivi");
    }

    #[test]
    fn resume_filtre_modele_produit_et_version() {
        let inv = Inventaire {
            modele: "MIBOX4\u{202e}\n\u{1b}[31m; evil".into(),
            produit: "one\r\nday\u{85}".into(),
            android: "9\u{200b}\nInjecté".into(),
            tous: vec![],
            desactives: vec![],
            utilisateur: vec![],
            systeme: vec![],
            applis: vec![],
            accueil_actuel: String::new(),
            accueil_prefere: None,
            premier_plan: None,
            wifi: EtatWifi::Libre,
            garder: vec![],
            avertissements: vec![],
        };
        let r = construire_resume("h", &inv);
        assert!(!r.chars().any(|c| c.is_control() && c != '\n') && !r.contains('\u{202e}') && !r.contains('\u{200b}'), "{r:?}");
        assert_eq!(r.lines().nth(1), Some("Modèle : MIBOX431m evil (oneday), Android 9Inject"), "{r:?}");
    }

    #[cfg(unix)]
    #[test]
    fn sauvegarde_en_700_600_sans_usagestats_ni_wifi() {
        use std::os::unix::fs::PermissionsExt;
        let t = Temp::nouveau();
        inventaire(&FakeBox::mibox4(), &t.0, "h").unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&t.0), 0o700);
        let mut noms: Vec<String> = std::fs::read_dir(&t.0).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        noms.sort();
        assert_eq!(
            noms,
            [
                "accueil.txt", "diskstats.txt", "paquets-desactives.txt", "paquets-systeme.txt", "paquets-tous.txt",
                "paquets-utilisateur.txt", "proprietes.txt", "reglages-global.txt", "reglages-secure.txt", "reglages-system.txt",
                "resume.txt"
            ]
        );
        for n in &noms {
            assert_eq!(mode(&t.0.join(n)), 0o600, "{n}");
        }
        assert_eq!(std::fs::read_to_string(t.0.join("paquets-desactives.txt")).unwrap(), "\n");
        assert!(std::fs::read_to_string(t.0.join("paquets-tous.txt")).unwrap().starts_with("package:/"));
    }

    #[test]
    fn lecture_vide_relue() {
        let t = Temp::nouveau();
        let f = FakeBox::mibox4();
        f.lectures_vides(1);
        let inv = inventaire(&f, &t.0, "h").unwrap();
        assert_eq!(inv.tous.len(), 33);
        let n = f.journal().iter().filter(|l| l.as_str() == "shell: pm list packages -f").count();
        assert_eq!(n, 2, "relue une fois après la réponse vide");
        // la relecture repart d'un lien neuf
        assert!(f.journal().iter().filter(|l| l.as_str() == "disconnect").count() >= 2);
    }

    #[test]
    fn lecture_toujours_vide_est_une_erreur_et_n_ecrit_rien() {
        let t = Temp::nouveau();
        let f = FakeBox::mibox4();
        f.lectures_vides(50);
        let e = inventaire(&f, &t.0, "h").unwrap_err();
        assert!(matches!(e, ErreurInventaire::Illisible(_)), "{e}");
        assert!(!t.0.exists());
    }

    #[test]
    fn coupures_puis_reprise() {
        let t = Temp::nouveau();
        let f = FakeBox::mibox4();
        f.couper(2);
        let inv = inventaire(&f, &t.0, "h").unwrap();
        assert_eq!(inv.tous.len(), 33);
        assert!(inv.avertissements.is_empty());
        // lien « offline » : plusieurs reconnexions nécessaires
        let g = FakeBox::mibox4();
        g.perdre_lien(3);
        assert!(inventaire(&g, &Temp::nouveau().0, "h").is_ok());
    }

    #[test]
    fn autorisation_en_attente_et_box_qui_ne_repond_plus() {
        let f = FakeBox::mibox4();
        f.autorisation_en_attente();
        let e = inventaire(&f, &Temp::nouveau().0, "h").unwrap_err();
        assert!(matches!(e, ErreurInventaire::AttenteUtilisateur { non_autorise: true }), "{e}");
        let g = FakeBox::mibox4();
        g.injoignable(true);
        let e = inventaire(&g, &Temp::nouveau().0, "h").unwrap_err();
        assert!(matches!(e, ErreurInventaire::AttenteUtilisateur { non_autorise: false }), "{e}");
    }

    #[test]
    fn lectures_facultatives_en_echec_donnent_des_avertissements() {
        // wifi toujours vide : non vérifiable (setupwraith resterait actif), sans bloquer l'inventaire
        let f = FakeBox::mibox4();
        f.wifi(WifiFake::Vide);
        let inv = inventaire(&f, &Temp::nouveau().0, "h").unwrap();
        assert_eq!(inv.wifi, EtatWifi::NonVerifiable);
    }
}
