//! Sauvegarde d'origine d'une box : la plus ancienne lecture de cette box faite AVANT l'installation de Zen Home
//! (son `paquets-tous.txt` ne contient pas `tv.zenhome`). Simple recherche dans les sauvegardes déjà écrites par
//! `inventaire` (`AAAAMMJJ-HHMMSS-<série>[-n]`), rien n'est copié.
//!
//! Les fichiers sont des entrées NON fiables (fichiers locaux modifiables) : tailles bornées, pas de lien symbolique,
//! noms de paquets et composants validés. Au moindre doute, la restauration retombe sur ses valeurs par défaut.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::installation::PKG;
use super::transport::{composant_valide, paquet_valide};

const TAILLE_MAX: u64 = 4 * 1024 * 1024;
const DOSSIERS_MAX: usize = 5000;
const DESACTIVES_MAX: usize = 5000;
/// `AAAAMMJJ-HHMMSS-` : le nom d'un dossier de sauvegarde commence par ces 16 caractères.
const PREFIXE: usize = 16;

/// Valeur d'un réglage dans la sauvegarde.
#[derive(Debug, Clone, PartialEq)]
pub enum Reglage {
    /// Fichier illisible ou valeur refusée : on garde la valeur par défaut de la restauration.
    Inconnu,
    /// Réglage absent (ou « null ») : il est supprimé à la restauration.
    Absent,
    Valeur(String),
}

#[derive(Debug, Clone)]
pub struct Origine {
    /// `JJ/MM/AAAA HH:MM`, tirée du nom du dossier.
    pub date: String,
    /// Paquets déjà désactivés avant Zen Home : ils le restent.
    pub desactives: Vec<String>,
    pub veille: Reglage,
    /// `window_animation_scale`, `transition_animation_scale`, `animator_duration_scale`.
    pub animations: [Reglage; 3],
    /// Accueil d'origine (`paquet/.Activite`), seulement s'il est valide.
    pub accueil: Option<String>,
}

/// Contenu d'un fichier ordinaire (ni lien, ni tube) de taille bornée, en UTF-8.
fn lire(f: &Path) -> Option<String> {
    let m = std::fs::symlink_metadata(f).ok()?;
    if !m.is_file() || m.len() > TAILLE_MAX {
        return None;
    }
    let mut s = String::new();
    std::fs::File::open(f).ok()?.take(TAILLE_MAX).read_to_string(&mut s).ok()?;
    Some(s)
}

fn nom_valide(n: &str) -> bool {
    let b = n.as_bytes();
    b.len() > PREFIXE
        && b[8] == b'-'
        && b[15] == b'-'
        && b[..15].iter().enumerate().all(|(i, c)| i == 8 || c.is_ascii_digit())
}

/// Série d'un nom de dossier : sans le suffixe `+n` (2 à 99) ajouté quand deux lectures tombent dans la même seconde.
/// `+` n'existe pas dans une série (voir `nom_serie`). Les anciens dossiers `-n` ne sont pas reconnus : ils comptent
/// comme une autre série (acceptable, ils ne sont rattachés à aucune box).
fn serie_de(nom: &str) -> &str {
    let reste = &nom[PREFIXE..];
    match reste.rsplit_once('+') {
        Some((s, n)) if !s.is_empty() && (1..=2).contains(&n.len()) && n.bytes().all(|c| c.is_ascii_digit()) => s,
        _ => reste,
    }
}

/// Dossiers de sauvegarde (vrais dossiers, nom conforme), du plus ancien au plus récent.
fn dossiers(sauvegardes: &Path) -> Vec<(String, PathBuf)> {
    let Ok(lecture) = std::fs::read_dir(sauvegardes) else { return Vec::new() };
    let mut v: Vec<_> = lecture
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| Some((e.file_name().into_string().ok()?, e.path())))
        .filter(|(n, _)| nom_valide(n))
        .collect();
    v.sort();
    v.truncate(DOSSIERS_MAX); // après le tri : on garde les plus anciens, ceux qui comptent pour une origine
    v
}

fn est_origine(dossier: &Path) -> bool {
    lire(&dossier.join("paquets-tous.txt")).is_some_and(|t| t.contains("package:") && !t.contains(PKG))
}

/// Origine la plus ancienne de cette série (`serie` déjà réduite aux caractères du nom de dossier).
pub fn origine(sauvegardes: &Path, serie: &str) -> Option<Origine> {
    if serie.is_empty() {
        return None;
    }
    let (nom, dossier) = dossiers(sauvegardes)
        .into_iter()
        .filter(|(n, _)| &n[PREFIXE..] == serie || serie_de(n) == serie)
        .find(|(_, d)| est_origine(d))?;
    charger(&nom, &dossier)
}

/// Origine la plus récente toutes séries confondues (une par série : la plus ancienne de celle-ci).
pub fn derniere_origine(sauvegardes: &Path) -> Option<Origine> {
    let mut vues = HashSet::new();
    let mut derniere = None;
    for (nom, dossier) in dossiers(sauvegardes) {
        if !vues.contains(serie_de(&nom)) && est_origine(&dossier) {
            vues.insert(serie_de(&nom).to_string());
            derniere = Some((nom, dossier));
        }
    }
    derniere.and_then(|(n, d)| charger(&n, &d))
}

fn charger(nom: &str, d: &Path) -> Option<Origine> {
    let desactives = paquets_desactives(&lire(&d.join("paquets-desactives.txt"))?)?;
    let global = lire(&d.join("reglages-global.txt"));
    let secure = lire(&d.join("reglages-secure.txt"));
    let valide = |r: Reglage, ok: fn(&str) -> bool| match r {
        Reglage::Valeur(v) if !ok(&v) => Reglage::Inconnu,
        r => r,
    };
    let animation = |cle| valide(reglage(global.as_deref(), cle), echelle_valide);
    let accueil = lire(&d.join("accueil.txt"))
        .and_then(|t| t.lines().map(str::trim).rfind(|l| !l.is_empty()).map(String::from))
        .filter(|c| composant_valide(c) && c.split('/').next().is_some_and(|p| p != PKG && !desactives.iter().any(|x| x == p)));
    Some(Origine {
        date: format!("{}/{}/{} {}:{}", &nom[6..8], &nom[4..6], &nom[..4], &nom[9..11], &nom[11..13]),
        veille: valide(reglage(secure.as_deref(), "screensaver_components"), veille_valide),
        animations: [animation("window_animation_scale"), animation("transition_animation_scale"), animation("animator_duration_scale")],
        accueil,
        desactives,
    })
}

/// Sortie de `pm list packages -d` : une ligne `package:nom` par paquet, tout autre contenu invalide le fichier.
fn paquets_desactives(texte: &str) -> Option<Vec<String>> {
    let mut v = Vec::new();
    for l in texte.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let p = l.strip_prefix("package:")?;
        if !paquet_valide(p) || v.len() >= DESACTIVES_MAX {
            return None;
        }
        v.push(p.to_string());
    }
    Some(v)
}

/// Valeur de `cle` dans une sortie `settings list` (`cle=valeur`). Un fichier sans aucune ligne `k=v` est illisible.
fn reglage(texte: Option<&str>, cle: &str) -> Reglage {
    let Some(t) = texte.filter(|t| t.lines().any(|l| l.contains('='))) else { return Reglage::Inconnu };
    match t.lines().find_map(|l| l.strip_prefix(cle)?.strip_prefix('=')).map(str::trim) {
        None | Some("null") => Reglage::Absent,
        Some(v) => Reglage::Valeur(v.to_string()),
    }
}

/// Un à quatre composants séparés par des virgules, jamais celui de Zen Home (qui va être désinstallé).
fn veille_valide(v: &str) -> bool {
    let l: Vec<&str> = v.split(',').collect();
    l.len() <= 4 && l.iter().all(|c| composant_valide(c)) && !v.contains(PKG)
}

/// Échelle d'animation : nombre décimal positif court (`1.0`, `0.5`, `0`).
fn echelle_valide(v: &str) -> bool {
    v.len() <= 8
        && v.bytes().all(|c| c.is_ascii_digit() || c == b'.')
        && v.bytes().filter(|c| *c == b'.').count() <= 1
        && v.bytes().next().is_some_and(|c| c.is_ascii_digit())
        && v.bytes().last().is_some_and(|c| c.is_ascii_digit())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    pub fn dossier_temp(nom: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!("zenhome-origine-{nom}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    pub const TOUS: &str = "package:/system/priv-app/Launcher/Launcher.apk=com.google.android.tvlauncher\n";

    /// Écrit une sauvegarde `nom` avec ces fichiers.
    pub fn sauvegarde(racine: &Path, nom: &str, fichiers: &[(&str, &str)]) -> PathBuf {
        let d = racine.join(nom);
        std::fs::create_dir_all(&d).unwrap();
        for (f, c) in fichiers {
            std::fs::write(d.join(f), c).unwrap();
        }
        d
    }

    #[test]
    fn la_plus_ancienne_sans_zenhome_de_la_bonne_serie() {
        let r = dossier_temp("ancienne");
        let avec_zen = format!("{TOUS}package:/data/app/tv.zenhome-1/base.apk=tv.zenhome\n");
        sauvegarde(&r, "20261001-100000-ABC", &[("paquets-tous.txt", &avec_zen), ("paquets-desactives.txt", "")]);
        sauvegarde(&r, "20261002-100000-ABC", &[("paquets-desactives.txt", "")]); // paquets-tous absent
        sauvegarde(&r, "20261003-090000-AUTRE", &[("paquets-tous.txt", TOUS), ("paquets-desactives.txt", "")]);
        sauvegarde(&r, "20261003-110000-ABC", &[("paquets-tous.txt", TOUS), ("paquets-desactives.txt", "package:com.a.b\n")]);
        sauvegarde(&r, "20261004-110000-ABC", &[("paquets-tous.txt", TOUS), ("paquets-desactives.txt", "")]);
        sauvegarde(&r, "20261004-110000-ABC+2", &[("paquets-tous.txt", TOUS), ("paquets-desactives.txt", "")]);

        let o = origine(&r, "ABC").expect("origine");
        assert_eq!(o.date, "03/10/2026 11:00");
        assert_eq!(o.desactives, ["com.a.b"]);
        assert_eq!(origine(&r, "AUTRE").unwrap().date, "03/10/2026 09:00");
        assert!(origine(&r, "ZZZ").is_none());
        assert!(origine(&r, "").is_none());
        assert!(origine(&r, "AB").is_none(), "pas de correspondance partielle");
        // toutes séries confondues : la plus récente des origines (une par série)
        assert_eq!(derniere_origine(&r).unwrap().date, "03/10/2026 11:00");
    }

    #[test]
    fn le_suffixe_n_appartient_a_la_meme_serie() {
        let r = dossier_temp("suffixe");
        sauvegarde(&r, "20261004-110000-ABC+2", &[("paquets-tous.txt", TOUS), ("paquets-desactives.txt", "")]);
        assert!(origine(&r, "ABC").is_some());
        assert!(origine(&r, "ABC-2").is_none(), "le suffixe est `+n` : `-2` fait partie d'une autre série");
        sauvegarde(&r, "20261005-110000-ABC-2", &[("paquets-tous.txt", TOUS), ("paquets-desactives.txt", "")]);
        assert_eq!(origine(&r, "ABC-2").unwrap().date, "05/10/2026 11:00");
        assert!(origine(&dossier_temp("vide"), "ABC").is_none());
        assert!(derniere_origine(&r.join("absent")).is_none());
    }

    #[test]
    fn reglages_et_accueil_de_l_origine() {
        let r = dossier_temp("reglages");
        sauvegarde(
            &r,
            "20261029-083553-S1",
            &[
                ("paquets-tous.txt", TOUS),
                ("paquets-desactives.txt", "\n"),
                ("accueil.txt", "priority=2 preferredOrder=0\ncom.mitv.tvhome/.MainActivity\n"),
                ("reglages-secure.txt", "a=1\nscreensaver_components=com.google.android.backdrop/.Backdrop\n"),
                ("reglages-global.txt", "window_animation_scale=0.5\ntransition_animation_scale=null\nzzz=2\n"),
            ],
        );
        let o = origine(&r, "S1").unwrap();
        assert_eq!(o.accueil.as_deref(), Some("com.mitv.tvhome/.MainActivity"));
        assert_eq!(o.veille, Reglage::Valeur("com.google.android.backdrop/.Backdrop".into()));
        assert_eq!(o.animations, [Reglage::Valeur("0.5".into()), Reglage::Absent, Reglage::Absent]);
    }

    #[test]
    fn fichiers_pieges_donnent_des_valeurs_inconnues_ou_pas_d_origine() {
        let r = dossier_temp("pieges");
        let enorme = "a=1\n".repeat(2 * 1024 * 1024);
        sauvegarde(
            &r,
            "20261029-083553-S1",
            &[
                ("paquets-tous.txt", TOUS),
                ("paquets-desactives.txt", ""),
                ("accueil.txt", "x\ncom.exemple/.A; reboot\n"),
                ("reglages-secure.txt", "screensaver_components=a.b/.C,tv.zenhome/.ZenDream\n"),
                ("reglages-global.txt", &enorme),
            ],
        );
        let o = origine(&r, "S1").unwrap();
        assert_eq!(o.accueil, None);
        assert_eq!(o.veille, Reglage::Inconnu);
        assert_eq!(o.animations, [Reglage::Inconnu, Reglage::Inconnu, Reglage::Inconnu], "fichier trop gros");

        for (nom, desactives) in [("20261030-000000-S2", "package:pas un paquet\n"), ("20261030-000000-S3", "n'importe quoi\n"), ("20261030-000000-S4", "package:a.b$c\n")] {
            sauvegarde(&r, nom, &[("paquets-tous.txt", TOUS), ("paquets-desactives.txt", desactives)]);
            assert!(origine(&r, &nom[PREFIXE..]).is_none(), "{nom}");
        }
        let gros = format!("{TOUS}{}", "x".repeat(5 * 1024 * 1024));
        sauvegarde(&r, "20261030-000000-S5", &[("paquets-tous.txt", &gros), ("paquets-desactives.txt", "")]);
        assert!(origine(&r, "S5").is_none());
        assert!(echelle_valide("1.0") && echelle_valide("0") && !echelle_valide("1e9") && !echelle_valide("-1") && !echelle_valide("1.") && !echelle_valide("1.2.3"));
    }

    #[cfg(unix)]
    #[test]
    fn liens_symboliques_ignores() {
        let r = dossier_temp("liens");
        let ailleurs = sauvegarde(&dossier_temp("ailleurs"), "20261029-083553-S1", &[("paquets-tous.txt", TOUS), ("paquets-desactives.txt", "")]);
        std::os::unix::fs::symlink(&ailleurs, r.join("20261029-083553-S1")).unwrap();
        assert!(origine(&r, "S1").is_none(), "dossier lien");
        let d = sauvegarde(&r, "20261030-083553-S1", &[("paquets-desactives.txt", "")]);
        std::os::unix::fs::symlink(ailleurs.join("paquets-tous.txt"), d.join("paquets-tous.txt")).unwrap();
        assert!(origine(&r, "S1").is_none(), "fichier lien");
    }
}
