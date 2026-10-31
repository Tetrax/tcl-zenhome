//! Contrôle et réparation de la box : équivalent de `tv/verifier-tv.sh [--fix]`.
//! Mêmes contrôles (OK / KO / AVERT), même ordre sûr de réparation, mêmes garde-fous.

use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use super::appareils::{Appareil, ModeAccueil};
use super::connexion::Politique;
use super::debloat::Entree;
use super::installation::{DREAM, KEYSVC_FULL, KEYSVC_SHORT, PKG, activer_accessibilite, definir_accueil, installer_apk};
use super::inventaire::{EtatWifi, accueil_prefere, etat_wifi, liste_garder, noms_paquets};
use super::session::{Arret, Observateur, Reprise, Session};
use super::transport::{Commande, Dumpsys, Espace, OptionPm, Transport};

pub const WRAITH: &str = "com.google.android.tungsten.setupwraith";
pub const SPOTIFY: &str = "com.spotify.tv.android";
const DELAI_ACCUEIL: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Statut {
    Ok,
    Ko,
    Avert,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ligne {
    pub nom: String,
    pub statut: Statut,
    pub detail: String,
}

#[derive(Debug, Clone)]
struct Etats {
    inst: Statut,
    role: Statut,
    lx: Statut,
    sw: Statut,
    a11y: Statut,
    dream: Statut,
    usage: Statut,
    spot: Statut,
    wifi: EtatWifi,
    debloat_ko: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Controle {
    pub lignes: Vec<Ligne>,
    pub ko: usize,
    /// Liste des paquets illisible : état des paquets inconnu (le kit sort alors en code 1).
    pub inconnu: bool,
    etats: Etats,
}

impl Controle {
    pub fn ok(&self) -> bool {
        self.ko == 0 && !self.inconnu
    }

    /// Le tableau du kit : `ETAT CONTRÔLE DÉTAIL`, une ligne par contrôle, puis `KO : n`.
    pub fn tableau(&self) -> String {
        let mut t = format!("\n{:<7} {:<58} {}\n", "ETAT", "CONTRÔLE", "DÉTAIL");
        for l in &self.lignes {
            let e = match l.statut {
                Statut::Ok => "OK",
                Statut::Ko => "KO",
                Statut::Avert => "AVERT",
            };
            let _ = writeln!(t, "{:<7} {:<58} {}", e, l.nom, l.detail);
        }
        let _ = write!(t, "\nKO : {}\n", self.ko);
        t
    }
}

struct Tableau(Vec<Ligne>);

impl Tableau {
    /// `enreg` du kit : un AVERT sans détail devient « non vérifié ».
    fn ajouter(&mut self, nom: impl Into<String>, statut: Statut, detail: &str) -> Statut {
        let detail = if statut == Statut::Avert && detail.is_empty() { "non vérifié" } else { detail };
        self.0.push(Ligne { nom: nom.into(), statut, detail: detail.to_string() });
        statut
    }
}

fn controler(s: &mut Session, app: &Appareil, liste: &[Entree]) -> Result<Controle, Arret> {
    let mut tab = Tableau(Vec::new());
    // Wi-Fi : dans les cas « lié » et « non vérifiable », setupwraith n'est JAMAIS désactivé.
    let wifi = etat_wifi(&s.lire(&Commande::dumpsys(Dumpsys::Wifi))?);
    let tous_brut = s.lire_non_vide(&Commande::pm_list(&[]))?.unwrap_or_default();
    let desactives = noms_paquets(&s.lire(&Commande::pm_list(&[OptionPm::Desactives]))?);
    let tous = noms_paquets(&tous_brut);
    let inconnu = !tous_brut.contains("package:");
    if inconnu {
        s.avertir("liste des paquets illisible : état des paquets inconnu.");
    }
    // 1 désactivé ou absent (OK), actif (KO), inconnu (AVERT).
    let etat_pkg = |p: &str| {
        if inconnu {
            Statut::Avert
        } else if desactives.iter().any(|d| d == p) || !tous.iter().any(|t| t == p) {
            Statut::Ok
        } else {
            Statut::Ko
        }
    };

    let inst = if inconnu {
        tab.ajouter("Zen Home installé", Statut::Avert, "")
    } else if tous.iter().any(|p| p == PKG) {
        let dump = s.lire(&Commande::dumpsys_paquet(PKG)?)?;
        let ver = dump.lines().find(|l| l.contains("versionName=")).and_then(|l| l.split('=').nth(1)).filter(|v| !v.is_empty());
        tab.ajouter("Zen Home installé", Statut::Ok, &format!("version {}", ver.unwrap_or("?")))
    } else {
        tab.ajouter("Zen Home installé", Statut::Ko, "absent")
    };

    let holder = detenteur_accueil(s, app)?;
    let role = if holder.as_deref() == Some(PKG) {
        tab.ajouter(format!("Rôle HOME = {PKG}"), Statut::Ok, "")
    } else {
        tab.ajouter(format!("Rôle HOME = {PKG}"), Statut::Ko, &format!("actuel : {}", holder.as_deref().unwrap_or("aucun")))
    };

    let (mut lx, mut sw) = (Statut::Ok, Statut::Ok);
    let launcher = app.launcher();
    for p in [launcher, WRAITH] {
        let (mut etat, mut det) = (etat_pkg(p), String::new());
        if p == WRAITH && etat == Statut::Ko && wifi != EtatWifi::Libre {
            etat = Statut::Avert;
            det = format!("gardé actif : Wi-Fi {}", if wifi == EtatWifi::LieSetupwraith { "lié à setupwraith" } else { "non vérifié" });
        }
        let r = tab.ajouter(format!("Désactivé : {p}"), etat, &det);
        if p == launcher { lx = r } else { sw = r }
    }

    // Choix « garder » du kit : s'applique à TOUT paquet de la liste.
    let garder = liste_garder(&s.lire(&Commande::settings_get(Espace::Global, "zenhome_garder")?)?);
    let mut debloat_ko = Vec::new();
    for e in liste {
        let p = e.paquet.as_str();
        let (mut etat, mut det) = (etat_pkg(p), "");
        if etat == Statut::Ko && garder.iter().any(|g| g == p) {
            (etat, det) = (Statut::Avert, "gardé actif (votre choix)");
        }
        if tab.ajouter(format!("Désactivé : {p}"), etat, det) == Statut::Ko {
            debloat_ko.push(p.to_string());
        }
    }

    let a11y = s.lire(&Commande::settings_get(Espace::Secure, "enabled_accessibility_services")?)?;
    let a11y = format!(":{}:", a11y.trim());
    let a11y = if a11y.contains(&format!(":{KEYSVC_FULL}:")) || a11y.contains(&format!(":{KEYSVC_SHORT}:")) {
        tab.ajouter("Accessibilité : KeyService", Statut::Ok, "")
    } else {
        tab.ajouter("Accessibilité : KeyService", Statut::Ko, "absent de enabled_accessibility_services")
    };

    let veille = s.lire(&Commande::settings_get(Espace::Secure, "screensaver_components")?)?;
    let veille = veille.trim();
    let dream = if veille == DREAM || veille == "tv.zenhome/.ZenDream" {
        tab.ajouter("Écran de veille = ZenDream", Statut::Ok, "")
    } else {
        tab.ajouter("Écran de veille = ZenDream", Statut::Ko, &format!("actuel : {}", if veille.is_empty() { "aucun" } else { veille }))
    };

    let usage = if s.lire(&Commande::appops_get(PKG, "GET_USAGE_STATS")?)?.contains("GET_USAGE_STATS: allow") {
        tab.ajouter("Stats d'usage (Applis ouvertes)", Statut::Ok, "")
    } else {
        tab.ajouter("Stats d'usage (Applis ouvertes)", Statut::Ko, "appops GET_USAGE_STATS non accordé")
    };

    let spot = if !inconnu && !tous.iter().any(|p| p == SPOTIFY) {
        tab.ajouter("Spotify : arrière-plan restreint", Statut::Ok, "Spotify absent")
    } else if s.lire(&Commande::appops_get(SPOTIFY, "RUN_ANY_IN_BACKGROUND")?)?.contains("RUN_ANY_IN_BACKGROUND: ignore") {
        tab.ajouter("Spotify : arrière-plan restreint", Statut::Ok, "")
    } else {
        tab.ajouter("Spotify : arrière-plan restreint", Statut::Ko, "pas en « ignore »")
    };

    match wifi {
        EtatWifi::Libre => tab.ajouter("Wi-Fi non lié à setupwraith", Statut::Ok, ""),
        EtatWifi::LieSetupwraith => {
            tab.ajouter("Wi-Fi non lié à setupwraith", Statut::Avert, "réseau créé par setupwraith : ne PAS le désactiver")
        }
        EtatWifi::NonVerifiable => tab.ajouter(
            "Wi-Fi non lié à setupwraith",
            Statut::Avert,
            "non vérifié (dumpsys wifi vide ou format inattendu) : setupwraith gardé actif",
        ),
    };

    let ko = tab.0.iter().filter(|l| l.statut == Statut::Ko).count();
    Ok(Controle { lignes: tab.0, ko, inconnu, etats: Etats { inst, role, lx, sw, a11y, dream, usage, spot, wifi, debloat_ko } })
}

/// Une commande qui échoue n'arrête pas la réparation (mieux vaut finir le retour arrière que s'arrêter au milieu) :
/// l'échec devient un avertissement, le nouveau contrôle le montrera. Une pause, elle, remonte.
fn tolerer(s: &mut Session, r: Result<(), Arret>) -> Result<bool, Arret> {
    match r {
        Ok(()) => Ok(true),
        Err(Arret::Echec(e)) => {
            s.avertir(e.to_string());
            Ok(false)
        }
        Err(pause) => Err(pause),
    }
}

/// Paquet qui détient l'accueil : rôle HOME (Android 10+) ou accueil préféré (Android 9 : l'accueil d'origine passe
/// devant tant qu'il est actif).
fn detenteur_accueil(s: &mut Session, app: &Appareil) -> Result<Option<String>, Arret> {
    Ok(match app.mode_accueil {
        ModeAccueil::RoleHome => s.lire(&Commande::role_home_detenteur())?.lines().map(str::trim).find(|l| !l.is_empty()).map(String::from),
        ModeAccueil::AccueilPrefere => accueil_prefere(&s.lire(&Commande::dumpsys(Dumpsys::PreferredXml))?),
    })
}

/// Zen Home est-il au premier plan ? (une ligne `mResumedActivity` ou `topResumedActivity` qui contient `tv.zenhome/`)
fn au_premier_plan(s: &mut Session) -> Result<bool, Arret> {
    let t = s.lire(&Commande::dumpsys(Dumpsys::Activities))?;
    Ok(t.lines().any(|l| (l.contains("topResumedActivity") || l.contains("mResumedActivity")) && l.contains(&format!("{PKG}/"))))
}

/// Mémorise un paquet que ce passage va désactiver (`pm enable` est idempotent : le noter trop tôt est sans risque).
fn noter_desactive(s: &mut Session, paquet: &str) {
    if !s.reprise.desactives.iter().any(|d| d == paquet) {
        s.reprise.desactives.push(paquet.to_string());
        s.signaler_reprise();
    }
}

/// Réparation dans l'ordre sûr du kit. Renvoie vrai si un retour arrière a eu lieu.
fn corriger(s: &mut Session, app: &Appareil, c: &Controle, apk: Option<&Path>) -> Result<bool, Arret> {
    let e = &c.etats;
    // 1. Zen Home d'abord : installation, accueil, services
    let mut ok_home = e.inst == Statut::Ok;
    if e.inst == Statut::Ko {
        match apk {
            Some(a) => {
                let r = installer_apk(s, a);
                ok_home = tolerer(s, r)?;
            }
            None => s.avertir("APK absent : Zen Home ne peut pas être réinstallé."),
        }
    }
    if ok_home {
        if e.role != Statut::Ok {
            let r = definir_accueil(s, app);
            tolerer(s, r)?;
        }
        if e.a11y != Statut::Ok {
            let r = activer_accessibilite(s);
            tolerer(s, r)?;
        }
        if e.dream != Statut::Ok {
            s.essayer(&Commande::settings_put(Espace::Secure, "screensaver_components", DREAM)?)?;
        }
        if e.usage != Statut::Ok {
            s.essayer(&Commande::appops_set(PKG, "GET_USAGE_STATS", "allow")?)?;
        }
    }
    // 2. Debloat
    for p in &e.debloat_ko {
        if app.est_protege(p) {
            s.avertir(format!("{p} : protégé, jamais désactivé."));
            continue;
        }
        s.essayer(&Commande::pm_disable(p)?)?;
    }
    if e.spot != Statut::Ok {
        s.essayer(&Commande::appops_set(SPOTIFY, "RUN_ANY_IN_BACKGROUND", "ignore")?)?;
    }
    // 3. L'accueil d'origine : seulement si Zen Home est installé ET accueil préféré
    if e.lx == Statut::Ko {
        if ok_home && detenteur_accueil(s, app)?.as_deref() == Some(PKG) {
            // Inscrit AVANT l'envoi : une coupure après l'exécution sur la box ne doit pas perdre le retour arrière.
            noter_desactive(s, app.launcher());
            s.essayer(&Commande::pm_disable(app.launcher())?)?;
        } else {
            s.avertir("l'accueil d'origine est laissé actif : Zen Home n'est pas installé ou n'est pas l'accueil préféré.");
        }
    }
    // 4. setupwraith : seulement si le Wi-Fi est VÉRIFIÉ non lié ; échec fermé sinon
    if e.sw == Statut::Ko && e.wifi == EtatWifi::Libre {
        noter_desactive(s, WRAITH);
        s.essayer(&Commande::pm_disable(WRAITH)?)?;
    }
    // 5. Contrôle de l'accueil, retour arrière si nécessaire
    s.essayer(&Commande::touche_accueil())?;
    s.attendre(DELAI_ACCUEIL);
    if au_premier_plan(s)? {
        return Ok(false);
    }
    s.avertir(format!("{PKG} n'est pas au premier plan après Accueil."));
    // L'accueil d'origine est rendu s'il est désactivé, quelle qu'en soit l'origine (ce passage, une reprise, un autre outil) ;
    // setupwraith seulement s'il a été désactivé par ce passage.
    let eteints = noms_paquets(&s.lire(&Commande::pm_list(&[OptionPm::Desactives]))?);
    let a_rendre: Vec<&str> = [app.launcher(), WRAITH]
        .into_iter()
        .filter(|p| s.reprise.desactives.iter().any(|d| d == p) || (*p == app.launcher() && eteints.iter().any(|d| d == p)))
        .collect();
    if a_rendre.is_empty() {
        return Ok(false);
    }
    s.avertir("retour arrière : réactivation de l'accueil d'origine et de setupwraith désactivés par ce passage.");
    for p in a_rendre {
        s.essayer(&Commande::pm_enable(p)?)?;
        // Écart volontaire au kit : le rôle HOME est rendu à l'accueil d'origine, sinon Zen Home le garderait.
        if p == app.launcher() && app.mode_accueil == ModeAccueil::RoleHome {
            s.essayer(&Commande::role_home_ajouter(p)?)?;
        }
    }
    s.essayer(&Commande::touche_accueil())?;
    s.reprise.desactives.clear();
    Ok(true)
}

#[derive(Debug, Clone)]
pub struct RapportVerification {
    /// Premier contrôle (avant réparation).
    pub avant: Controle,
    /// Contrôle final (seulement si une réparation a eu lieu).
    pub apres: Option<Controle>,
    pub retour_arriere: bool,
    pub avertissements: Vec<String>,
    /// Code de sortie 0 du kit : aucun KO et état des paquets lisible (dans le dernier contrôle).
    pub ok: bool,
}

impl RapportVerification {
    /// Le dernier tableau (l'état final), comme le kit l'affiche.
    pub fn final_(&self) -> &Controle {
        self.apres.as_ref().unwrap_or(&self.avant)
    }
}

/// `verifier-tv.sh` (`reparer` = `--fix`). `apk` : APK à réinstaller si Zen Home est absent.
pub fn verifier(
    t: &dyn Transport,
    app: &Appareil,
    pol: &Politique,
    obs: &mut dyn Observateur,
    reparer: bool,
    apk: Option<&Path>,
    depuis: &Reprise,
) -> Result<RapportVerification, Arret> {
    let liste = app.liste();
    // Pas de saut d'étape ici (le contrôle est en lecture seule) ; seuls les paquets déjà désactivés par un passage
    // interrompu sont conservés, pour que le retour arrière reste possible.
    let mut s = Session::nouvelle(t, pol, obs, "verification", 3, &Reprise { etape: 0, desactives: depuis.desactives.clone(), serie: depuis.serie.clone() });
    s.etape(0, "Contrôle");
    let avant = controler(&mut s, app, &liste)?;
    let (mut apres, mut retour_arriere) = (None, false);
    if reparer && avant.ko > 0 {
        s.etape(1, "Réparation");
        retour_arriere = corriger(&mut s, app, &avant, apk)?;
        s.etape(2, "Nouveau contrôle");
        apres = Some(controler(&mut s, app, &liste)?);
    }
    let ok = apres.as_ref().unwrap_or(&avant).ok();
    Ok(RapportVerification { avant, apres, retour_arriere, avertissements: s.avertissements, ok })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::appareils::{MIBOX4, TCL_G08};
    use crate::moteur::fake_box::{FakeBox, WifiFake};
    use crate::moteur::session::Silencieux;

    const LAUNCHERX: &str = "com.google.android.tvlauncher";

    fn apk() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
    }

    fn connectee() -> FakeBox {
        let f = FakeBox::mibox4();
        f.connecter().unwrap();
        f
    }

    fn lancer(f: &FakeBox, reparer: bool) -> RapportVerification {
        verifier(f, &MIBOX4, &Politique::default(), &mut Silencieux, reparer, Some(&apk()), &Reprise::default()).unwrap()
    }

    fn ligne<'a>(c: &'a Controle, nom: &str) -> &'a Ligne {
        c.lignes.iter().find(|l| l.nom == nom).unwrap_or_else(|| panic!("ligne {nom}"))
    }

    #[test]
    fn controle_d_une_box_fraiche_sans_reparer() {
        let f = connectee();
        let r = lancer(&f, false);
        assert!(r.apres.is_none() && !r.ok);
        let c = &r.avant;
        assert_eq!(ligne(c, "Zen Home installé").statut, Statut::Ko);
        assert_eq!(ligne(c, "Zen Home installé").detail, "absent");
        // Spotify est installé sur la box de test et n'est pas restreint
        assert_eq!(ligne(c, "Spotify : arrière-plan restreint").statut, Statut::Ko);
        assert_eq!(ligne(c, "Wi-Fi non lié à setupwraith").statut, Statut::Ok);
        // 2 + 24 paquets actifs + installé + rôle + a11y + veille + usage + spotify
        assert_eq!(c.ko, 1 + 1 + 2 + 24 + 1 + 1 + 1 + 1);
        assert!(c.tableau().contains("KO : 32"));
        assert!(f.journal().iter().all(|l| !l.contains("disable") && !l.starts_with("install")), "lecture seule");
    }

    #[test]
    fn reparation_complete_puis_tout_est_ok() {
        let f = connectee();
        let r = lancer(&f, true);
        assert!(r.ok, "{}", r.final_().tableau());
        assert!(!r.retour_arriere);
        let e = f.instantane();
        assert!(e.paquets[LAUNCHERX].desactive && e.paquets[WRAITH].desactive);
        assert!(MIBOX4.liste().iter().all(|d| e.paquets[&d.paquet].desactive));
        assert_eq!(e.premier_plan.as_deref(), Some("tv.zenhome/.Home"));
        // seconde passe : rien à réparer, aucune commande de modification
        let avant = f.journal().len();
        let r2 = lancer(&f, true);
        assert!(r2.ok && r2.apres.is_none());
        assert!(f.journal()[avant..].iter().all(|l| l.starts_with("shell: dumpsys") || l.starts_with("shell: pm list") || l.starts_with("shell: settings get") || l.starts_with("shell: cmd appops get")));
    }

    #[test]
    fn zenhome_garder_s_applique_a_tout_paquet_de_la_liste() {
        let f = connectee();
        f.modifier(|e| {
            e.global.insert("zenhome_garder".into(), "tv.alphonso.alphonso_eula,com.google.android.katniss,pas un nom".into());
        });
        let r = lancer(&f, true);
        assert!(r.ok, "AVERT n'est pas KO : {}", r.final_().tableau());
        let c = r.final_();
        for p in ["tv.alphonso.alphonso_eula", "com.google.android.katniss"] {
            let l = ligne(c, &format!("Désactivé : {p}"));
            assert_eq!((l.statut, l.detail.as_str()), (Statut::Avert, "gardé actif (votre choix)"));
            assert!(!f.instantane().paquets[p].desactive);
        }
    }

    #[test]
    fn wifi_lie_a_setupwraith_ne_desactive_jamais_setupwraith() {
        for wifi in [WifiFake::LieSetupwraith, WifiFake::Vide] {
            let f = connectee();
            f.wifi(wifi);
            let r = lancer(&f, true);
            assert!(r.ok);
            assert!(!f.instantane().paquets[WRAITH].desactive);
            assert!(f.journal().iter().all(|l| !l.contains(WRAITH)));
            let l = ligne(r.final_(), &format!("Désactivé : {WRAITH}"));
            assert_eq!(l.statut, Statut::Avert);
        }
    }

    #[test]
    fn zen_home_pas_au_premier_plan_retour_arriere() {
        let f = connectee();
        f.zen_home_visible(false);
        let r = lancer(&f, true);
        assert!(r.retour_arriere);
        let e = f.instantane();
        assert!(!e.paquets[LAUNCHERX].desactive && !e.paquets[WRAITH].desactive);
        assert_eq!(e.premier_plan.as_deref(), Some("com.google.android.tvlauncher/.MainActivity"));
        assert!(r.avertissements.iter().any(|a| a.contains("premier plan")));
        // le retour arrière n'est pas un succès : l'accueil d'origine est à nouveau KO
        assert!(!r.ok);
    }

    /// Transport qui exécute `pm disable-user` sur la box puis coupe le lien pour de bon : la réponse se perd.
    struct CoupeApresDisable<'a>(&'a FakeBox, &'a str);
    impl Transport for CoupeApresDisable<'_> {
        fn shell(&self, cmd: &Commande) -> Result<crate::moteur::transport::Sortie, crate::moteur::transport::ErreurTransport> {
            let r = self.0.shell(cmd);
            if cmd.afficher().starts_with("pm disable-user") && cmd.afficher().ends_with(self.1) {
                self.0.perdre_lien(99);
                return Err(crate::moteur::transport::ErreurTransport::Coupure("error: device offline".into()));
            }
            r
        }
        fn shell_stdin(&self, cmd: &Commande) -> Result<crate::moteur::transport::Sortie, crate::moteur::transport::ErreurTransport> {
            self.0.shell_stdin(cmd)
        }
        fn install(&self, apk: &Path) -> Result<crate::moteur::transport::Sortie, crate::moteur::transport::ErreurTransport> {
            self.0.install(apk)
        }
        fn uninstall(&self, p: &str) -> Result<crate::moteur::transport::Sortie, crate::moteur::transport::ErreurTransport> {
            self.0.uninstall(p)
        }
        fn etat(&self) -> Result<crate::moteur::transport::EtatLien, crate::moteur::transport::ErreurTransport> {
            self.0.etat()
        }
        fn connecter(&self) -> Result<(), crate::moteur::transport::ErreurTransport> {
            self.0.connecter()
        }
        fn deconnecter(&self) {
            self.0.deconnecter()
        }
        fn attendre(&self, d: Duration) {
            self.0.attendre(d)
        }
    }

    #[test]
    fn coupure_juste_apres_pm_disable_puis_reprise_rend_l_accueil() {
        let f = connectee();
        f.zen_home_visible(false);
        let r = verifier(&CoupeApresDisable(&f, LAUNCHERX), &MIBOX4, &Politique::default(), &mut Silencieux, true, Some(&apk()), &Reprise::default());
        let Err(Arret::EnPause(pause)) = r else { panic!("attendu : pause, obtenu {:?}", r.map(|_| ())) };
        assert!(f.instantane().paquets[LAUNCHERX].desactive, "la box a exécuté la désactivation, seule la réponse s'est perdue");
        f.perdre_lien(0);
        let r = verifier(&f, &MIBOX4, &Politique::default(), &mut Silencieux, true, Some(&apk()), &pause.reprise).unwrap();
        assert!(r.retour_arriere);
        assert!(!f.instantane().paquets[LAUNCHERX].desactive, "l'accueil d'origine est réactivé malgré la coupure");
    }

    #[test]
    fn accueil_d_origine_deja_desactive_est_reactive_si_zen_home_n_est_pas_au_premier_plan() {
        let f = connectee();
        f.modifier(|e| e.paquets.get_mut(LAUNCHERX).unwrap().desactive = true);
        f.zen_home_visible(false);
        let r = lancer(&f, true);
        assert!(r.retour_arriere);
        let e = f.instantane();
        assert!(!e.paquets[LAUNCHERX].desactive, "désactivé avant le passage, réactivé quand même");
        assert!(!e.paquets[WRAITH].desactive);
    }

    #[test]
    fn jamais_de_desactivation_de_l_accueil_sans_zen_home() {
        let f = connectee();
        // pas d'APK : Zen Home reste absent, donc ni accueil d'origine ni setupwraith... sauf setupwraith (Wi-Fi libre)
        let r = verifier(&f, &MIBOX4, &Politique::default(), &mut Silencieux, true, None, &Reprise::default()).unwrap();
        let e = f.instantane();
        assert!(!e.paquets[LAUNCHERX].desactive);
        assert!(!r.ok);
        assert!(r.avertissements.iter().any(|a| a.contains("APK absent")));
        assert!(r.avertissements.iter().any(|a| a.contains("laissé actif")));
    }

    #[test]
    fn liste_illisible_donne_avert() {
        let f = connectee();
        // 1 lecture vide pour le Wi-Fi, puis les 3 lectures de la liste des paquets
        f.lectures_vides(4);
        let r = lancer(&f, false);
        assert!(r.avant.inconnu && !r.ok);
        let l = ligne(&r.avant, "Zen Home installé");
        assert_eq!((l.statut, l.detail.as_str()), (Statut::Avert, "non vérifié"));
        assert_eq!(ligne(&r.avant, &format!("Désactivé : {LAUNCHERX}")).statut, Statut::Avert);
    }

    const LAUNCHERX_TCL: &str = "com.google.android.apps.tv.launcherx";

    fn tcl() -> FakeBox {
        let f = FakeBox::tcl_g08();
        f.connecter().unwrap();
        f
    }

    fn lancer_tcl(f: &FakeBox, reparer: bool, apk: Option<&Path>) -> RapportVerification {
        verifier(f, &TCL_G08, &Politique::default(), &mut Silencieux, reparer, apk, &Reprise::default()).unwrap()
    }

    #[test]
    fn tcl_reparation_complete_par_le_role_home() {
        let f = tcl();
        let r = lancer_tcl(&f, true, Some(&apk()));
        assert!(r.ok, "{}", r.final_().tableau());
        let e = f.instantane();
        assert_eq!(e.role_home.as_deref(), Some(PKG));
        assert!(e.paquets[LAUNCHERX_TCL].desactive && e.paquets[WRAITH].desactive);
        assert!(TCL_G08.liste().iter().all(|d| e.paquets[&d.paquet].desactive));
        assert_eq!(e.premier_plan.as_deref(), Some("tv.zenhome/.Home"));
        assert_eq!(ligne(r.final_(), "Rôle HOME = tv.zenhome").statut, Statut::Ok);
        let j = f.journal();
        let place = |m: &str| j.iter().position(|l| l == m).unwrap_or_else(|| panic!("{m}"));
        assert!(place("shell: cmd role add-role-holder android.app.role.HOME tv.zenhome") < place(&format!("shell: pm disable-user --user 0 {LAUNCHERX_TCL}")));
        assert!(j.iter().all(|l| !l.contains("preferred-xml")), "le détenteur est lu par le rôle");
        // seconde passe : rien à réparer
        let avant = f.journal().len();
        assert!(lancer_tcl(&f, true, Some(&apk())).apres.is_none());
        assert!(f.journal()[avant..].iter().all(|l| !l.contains("disable") && !l.contains("add-role")));
    }

    #[test]
    fn tcl_accueil_d_origine_jamais_desactive_sans_zen_home() {
        let f = tcl();
        let r = lancer_tcl(&f, true, None);
        assert!(!r.ok && !f.instantane().paquets[LAUNCHERX_TCL].desactive);
        assert_eq!(f.instantane().role_home.as_deref(), Some(LAUNCHERX_TCL));
    }

    #[test]
    fn tcl_retour_arriere_rend_le_role_home_a_l_accueil_d_origine() {
        let f = tcl();
        f.zen_home_visible(false);
        let r = lancer_tcl(&f, true, Some(&apk()));
        assert!(r.retour_arriere);
        let e = f.instantane();
        assert!(!e.paquets[LAUNCHERX_TCL].desactive && !e.paquets[WRAITH].desactive);
        assert_eq!(e.role_home.as_deref(), Some(LAUNCHERX_TCL));
        assert_eq!(e.premier_plan.as_deref(), Some(TCL_G08.accueil));
    }
}
