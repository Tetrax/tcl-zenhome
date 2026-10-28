//! Installation de Zen Home : équivalent de `tv/deployer.sh --sans-build` pour la Mi Box 4 (même ordre de commandes).
//! La connexion (`connecter_box`) est faite avant par l'appelant.

use std::fmt;
use std::path::PathBuf;

use super::connexion::Politique;
use super::session::{Arret, ErreurAction, Observateur, Reprise, Session};
use super::transport::{ArgConfig, Commande, Espace, ErreurTransport};

pub const PKG: &str = "tv.zenhome";
pub const KEYSVC_FULL: &str = "tv.zenhome/tv.zenhome.KeyService";
pub const KEYSVC_SHORT: &str = "tv.zenhome/.KeyService";
pub const DREAM: &str = "tv.zenhome/tv.zenhome.ZenDream";
pub const ACCUEIL_ZEN: &str = "tv.zenhome/.Home";
const SERVICES_ACCESSIBILITE: &str = "enabled_accessibility_services";

pub struct Parametres {
    pub apk: PathBuf,
    pub lat: Option<String>,
    pub lon: Option<String>,
    pub ville: Option<String>,
    /// Jeton TMDB : envoyé uniquement par `shell_stdin`, jamais journalisé ni affiché.
    pub jeton: Option<String>,
}

impl fmt::Debug for Parametres {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Parametres")
            .field("apk", &self.apk)
            .field("lat", &self.lat.as_ref().map(|_| "***"))
            .field("lon", &self.lon.as_ref().map(|_| "***"))
            .field("ville", &self.ville.as_ref().map(|_| "***"))
            .field("jeton", &self.jeton.as_ref().map(|_| "***"))
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RapportInstallation {
    pub avertissements: Vec<String>,
}

/// Valeur de `enabled_accessibility_services` sans les marqueurs « null » et `""` (valeur écrite par le kit pour « vide »).
fn services_actuels(brut: &str) -> String {
    let v = brut.trim();
    if v == "null" || v == "\"\"" { String::new() } else { v.to_string() }
}

/// `activer_accessibilite` de lib.sh : ajoute KeyService sans écraser les autres services. S'il y est déjà, il est
/// retiré puis remis (une réinstallation de l'APK le délie sans le retirer de la liste), y compris quand il est seul.
pub(super) fn activer_accessibilite(s: &mut Session) -> Result<(), Arret> {
    let cur = services_actuels(&s.lire(&Commande::settings_get(Espace::Secure, SERVICES_ACCESSIBILITE)?)?);
    let entoure = format!(":{cur}:");
    if entoure.contains(&format!(":{KEYSVC_FULL}:")) || entoure.contains(&format!(":{KEYSVC_SHORT}:")) {
        let reste = cur.split(':').filter(|e| *e != KEYSVC_FULL && *e != KEYSVC_SHORT).collect::<Vec<_>>().join(":");
        s.ecrire(&Commande::settings_put(Espace::Secure, SERVICES_ACCESSIBILITE, if reste.is_empty() { "\"\"" } else { &reste })?)?;
        let avec = if reste.is_empty() { KEYSVC_FULL.to_string() } else { format!("{reste}:{KEYSVC_FULL}") };
        s.ecrire(&Commande::settings_put(Espace::Secure, SERVICES_ACCESSIBILITE, &avec)?)?;
    } else {
        let avec = if cur.is_empty() { KEYSVC_FULL.to_string() } else { format!("{cur}:{KEYSVC_FULL}") };
        s.ecrire(&Commande::settings_put(Espace::Secure, SERVICES_ACCESSIBILITE, &avec)?)?;
    }
    s.ecrire(&Commande::settings_put(Espace::Secure, "accessibility_enabled", "1")?)?;
    Ok(())
}

/// `definir_accueil` pour un profil sans rôle HOME (Android 9).
pub(super) fn definir_accueil(s: &mut Session) -> Result<(), Arret> {
    s.ecrire(&Commande::set_home_activity(ACCUEIL_ZEN)?)?;
    Ok(())
}

/// `installer_apk` : `adb install -r`, jamais de désinstallation.
pub(super) fn installer_apk(s: &mut Session, apk: &std::path::Path) -> Result<(), Arret> {
    if !apk.is_file() {
        return Err(ErreurAction::ApkAbsent(apk.display().to_string()).into());
    }
    s.verifier_identite()?;
    match s.op("installation de l'APK", |t| t.install(apk)) {
        Ok(_) => Ok(()),
        Err(Arret::Echec(ErreurAction::Transport(ErreurTransport::Commande { stderr, .. }))) => Err(if stderr.contains("INSTALL_FAILED_UPDATE_INCOMPATIBLE") {
            ErreurAction::SignatureIncompatible
        } else {
            ErreurAction::Installation(stderr)
        }
        .into()),
        Err(e) => Err(e),
    }
}

/// `envoyer_config` : par l'entrée standard du shell adb ; la sortie n'est jamais réaffichée (elle pourrait contenir le jeton).
fn envoyer_config(s: &mut Session, cmd: &Commande) -> Result<(), Arret> {
    let confirme = match s.op("configuration", |t| t.shell_stdin(cmd)) {
        Ok(sortie) => sortie.stdout.contains("Broadcast completed"),
        Err(Arret::Echec(ErreurAction::Transport(ErreurTransport::Commande { .. }))) => false,
        Err(e) => return Err(e),
    };
    if !confirme {
        s.avertir("broadcast de configuration sans confirmation (sortie non affichée).");
    }
    Ok(())
}

/// `accorder_permissions` : READ_MEDIA_IMAGES si SDK >= 33, READ_EXTERNAL_STORAGE si SDK <= 32 (rien si SDK illisible).
fn accorder_permissions(s: &mut Session) -> Result<(), Arret> {
    let brut = s.lire(&Commande::getprop("ro.build.version.sdk")?)?;
    let sdk = brut.trim();
    let sdk = if !sdk.is_empty() && sdk.chars().all(|c| c.is_ascii_digit()) { sdk.parse::<u32>().ok() } else { None };
    if sdk.is_some_and(|v| v >= 33) {
        s.essayer(&Commande::pm_grant(PKG, "android.permission.READ_MEDIA_IMAGES")?)?;
    }
    // Stats d'usage (écran « Applis ouvertes ») : opération appops, pas une permission runtime.
    s.essayer(&Commande::appops_set(PKG, "GET_USAGE_STATS", "allow")?)?;
    if sdk.is_some_and(|v| v <= 32) {
        s.essayer(&Commande::pm_grant(PKG, "android.permission.READ_EXTERNAL_STORAGE")?)?;
    }
    Ok(())
}

pub fn installer(
    t: &dyn super::transport::Transport,
    pol: &Politique,
    obs: &mut dyn Observateur,
    p: &Parametres,
    depuis: &Reprise,
) -> Result<RapportInstallation, Arret> {
    // Tout est validé avant le premier envoi : une valeur refusée n'a rien modifié.
    let geo = match (&p.lat, &p.lon) {
        (Some(lat), Some(lon)) => {
            let mut args = vec![ArgConfig::Lat(lat.clone()), ArgConfig::Lon(lon.clone())];
            if let Some(v) = p.ville.as_ref().filter(|v| !v.is_empty()) {
                args.push(ArgConfig::Ville(v.clone()));
            }
            Some(Commande::broadcast_config(&args)?)
        }
        _ => None,
    };
    let jeton = match &p.jeton {
        Some(j) => {
            let nu: String = j.chars().filter(|c| !c.is_whitespace()).collect();
            Some(Commande::broadcast_config(&[ArgConfig::Jeton(nu)])?)
        }
        None => None,
    };

    let mut s = Session::nouvelle(t, pol, obs, "installation", 6, depuis);
    if s.etape(0, "Installation de Zen Home") {
        installer_apk(&mut s, &p.apk)?;
    }
    if s.etape(1, "Configuration") {
        match &geo {
            Some(c) => envoyer_config(&mut s, c)?,
            None => s.avertir("LAT/LON absents : météo non configurée."),
        }
        match &jeton {
            Some(c) => {
                s.verifier_identite()?;
                envoyer_config(&mut s, c)?
            }
            None => s.avertir("jeton TMDB absent : films désactivés."),
        }
    }
    if s.etape(2, "Permissions") {
        accorder_permissions(&mut s)?;
    }
    if s.etape(3, "Accessibilité") {
        activer_accessibilite(&mut s)?;
    }
    if s.etape(4, "Écran de veille et accueil") {
        s.ecrire(&Commande::settings_put(Espace::Secure, "screensaver_components", DREAM)?)?;
        definir_accueil(&mut s)?;
    }
    if s.etape(5, "Retour à l'accueil") {
        s.ecrire(&Commande::touche_accueil())?;
    }
    Ok(RapportInstallation { avertissements: s.avertissements })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::fake_box::FakeBox;
    use crate::moteur::session::{Evenement, Silencieux};
    use crate::moteur::transport::Transport;

    fn apk() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
    }

    fn params() -> Parametres {
        Parametres { apk: apk(), lat: Some("48.86".into()), lon: Some("2.35".into()), ville: Some("Paris".into()), jeton: None }
    }

    fn connectee() -> FakeBox {
        let f = FakeBox::mibox4();
        f.connecter().unwrap();
        f
    }

    #[test]
    fn installation_a_neuf() {
        let f = connectee();
        let r = installer(&f, &Politique::default(), &mut Silencieux, &params(), &Reprise::default()).unwrap();
        let e = f.instantane();
        assert!(e.paquets.contains_key(PKG));
        assert_eq!(e.accueil_prefere.as_deref(), Some(ACCUEIL_ZEN));
        assert_eq!(e.secure["enabled_accessibility_services"], KEYSVC_FULL);
        assert_eq!(e.secure["accessibility_enabled"], "1");
        assert_eq!(e.secure["screensaver_components"], DREAM);
        assert_eq!(e.config["city"], "Paris");
        assert_eq!(e.appops[&(PKG.to_string(), "GET_USAGE_STATS".to_string())], "allow");
        assert_eq!(r.avertissements, ["jeton TMDB absent : films désactivés."]);
    }

    #[test]
    fn jeton_jamais_dans_le_journal_ni_dans_debug() {
        let f = connectee();
        let mut p = params();
        p.jeton = Some("SECRET.jeton-123".into());
        installer(&f, &Politique::default(), &mut Silencieux, &p, &Reprise::default()).unwrap();
        let journal = f.journal().join("\n");
        assert!(!journal.contains("SECRET"), "{journal}");
        assert!(journal.contains("stdin: am broadcast -a tv.zenhome.CONFIG -n tv.zenhome/.ConfigReceiver --es tmdb_token ***"));
        assert!(!format!("{p:?}").contains("SECRET"));
        assert_eq!(f.instantane().config["tmdb_token"], "***");
        // jamais par shell : aucune ligne « shell: » ne parle du jeton
        assert!(f.journal().iter().all(|l| !(l.starts_with("shell:") && l.contains("tmdb_token"))));
    }

    #[test]
    fn valeurs_invalides_refusees_avant_tout_envoi() {
        let f = connectee();
        let mut p = params();
        p.lat = Some("48.8; reboot".into());
        let r = installer(&f, &Politique::default(), &mut Silencieux, &p, &Reprise::default());
        assert!(matches!(r, Err(Arret::Echec(ErreurAction::Transport(ErreurTransport::Refusee(_))))));
        assert!(f.journal().iter().all(|l| l == "connect"), "{:?}", f.journal());
        p.lat = Some("48.86".into());
        p.jeton = Some("a b'c".into());
        assert!(installer(&f, &Politique::default(), &mut Silencieux, &p, &Reprise::default()).is_err());
        assert_eq!(f.journal(), ["connect"]);
    }

    #[test]
    fn apk_absent_et_signature_incompatible() {
        let f = connectee();
        let mut p = params();
        p.apk = PathBuf::from("/inexistant/ZenHome.apk");
        assert!(matches!(
            installer(&f, &Politique::default(), &mut Silencieux, &p, &Reprise::default()),
            Err(Arret::Echec(ErreurAction::ApkAbsent(_)))
        ));
        f.install(&apk()).unwrap();
        f.signature_incompatible(true);
        assert_eq!(
            installer(&f, &Politique::default(), &mut Silencieux, &params(), &Reprise::default()),
            Err(Arret::Echec(ErreurAction::SignatureIncompatible))
        );
        assert!(!f.journal().iter().any(|l| l.starts_with("uninstall")), "rien n'est désinstallé automatiquement");
    }

    #[test]
    fn keyservice_seul_est_relance() {
        let f = connectee();
        f.modifier(|e| {
            e.secure.insert("enabled_accessibility_services".into(), KEYSVC_FULL.into());
        });
        installer(&f, &Politique::default(), &mut Silencieux, &params(), &Reprise::default()).unwrap();
        let put: Vec<String> =
            f.journal().into_iter().filter(|l| l.contains("enabled_accessibility_services") && l.contains("put")).collect();
        assert_eq!(
            put,
            [
                "shell: settings put secure enabled_accessibility_services '\"\"'".to_string(),
                format!("shell: settings put secure enabled_accessibility_services '{KEYSVC_FULL}'"),
            ]
        );
        assert_eq!(f.instantane().secure["enabled_accessibility_services"], KEYSVC_FULL);
        // la valeur `""` laissée par une coupure à cet endroit est comprise comme « vide »
        assert_eq!(services_actuels("\"\""), "");
        assert_eq!(services_actuels("null\n"), "");
    }

    #[test]
    fn coupure_reprend_l_operation_en_cours_seulement() {
        let f = connectee();
        // 1 commande passe (le getprop SDK ou la configuration), la suivante est coupée une fois
        f.couper_apres(3, 1);
        installer(&f, &Politique::default(), &mut Silencieux, &params(), &Reprise::default()).unwrap();
        let journal = f.journal();
        assert_eq!(journal.iter().filter(|l| l.starts_with("install:")).count(), 1, "pas de réinstallation");
        assert_eq!(f.instantane().secure["enabled_accessibility_services"], KEYSVC_FULL);
    }

    /// Coupe le lien pour de bon au début de l'étape `numero` (la box est à réveiller).
    struct Coupeur<'a>(&'a FakeBox, usize);
    impl Observateur for Coupeur<'_> {
        fn notifier(&mut self, e: Evenement) {
            if matches!(e, Evenement::Etape { numero, .. } if numero == self.1) {
                self.0.perdre_lien(99);
            }
        }
    }

    #[test]
    fn pause_puis_reprise_a_l_etape_interrompue() {
        let f = connectee();
        let r = installer(&f, &Politique::default(), &mut Coupeur(&f, 3), &params(), &Reprise::default());
        let Err(Arret::EnPause(pause)) = r else { panic!("attendu : pause, obtenu {r:?}") };
        assert_eq!((pause.action.as_str(), pause.etape.as_str(), pause.reprise.etape), ("installation", "Permissions", 2));
        assert_eq!(f.journal().iter().filter(|l| l.starts_with("install:")).count(), 1);
        f.perdre_lien(0);
        installer(&f, &Politique::default(), &mut Silencieux, &params(), &pause.reprise).unwrap();
        assert_eq!(f.journal().iter().filter(|l| l.starts_with("install:")).count(), 1, "l'étape 0 n'est pas rejouée");
        assert_eq!(f.instantane().accueil_prefere.as_deref(), Some(ACCUEIL_ZEN));
    }

    /// Remplace la box (autre numéro de série) dès que le lien décroche, ou au début de l'étape `.1`.
    struct Echange<'a>(&'a FakeBox, Option<usize>);
    impl Observateur for Echange<'_> {
        fn notifier(&mut self, e: Evenement) {
            let declenche = match (&e, self.1) {
                (Evenement::Etape { numero, .. }, Some(n)) => *numero == n,
                (Evenement::ConnexionPerdue { .. }, None) => true,
                _ => false,
            };
            if declenche {
                self.0.modifier(|b| b.serie = "AUTRE-BOX".into());
            }
        }
    }

    #[test]
    fn serie_changee_apres_une_reconnexion_arrete_l_action() {
        let f = connectee();
        f.couper_apres(2, 1);
        let mut p = params();
        p.jeton = Some("SECRET.jeton-123".into());
        let e = installer(&f, &Politique::default(), &mut Echange(&f, None), &p, &Reprise::default()).unwrap_err();
        assert!(e.to_string().contains("plus la même box"), "{e}");
        assert!(f.journal().iter().all(|l| !l.contains("tmdb_token")), "{:?}", f.journal());
    }

    #[test]
    fn serie_changee_avant_le_jeton_ou_illisible_rien_n_est_envoye() {
        let f = connectee();
        let mut p = params();
        p.jeton = Some("SECRET.jeton-123".into());
        // la box est remplacée au début de l'étape « Configuration » : la position part encore, pas le jeton
        let e = installer(&f, &Politique::default(), &mut Echange(&f, Some(2)), &p, &Reprise::default()).unwrap_err();
        assert!(e.to_string().contains("plus la même box"), "{e}");
        assert!(f.journal().iter().all(|l| !l.contains("tmdb_token")), "{:?}", f.journal());
        // numéro de série illisible (réponses vides) : rien n'est envoyé non plus, APK compris
        let g = connectee();
        g.serie_illisible(true);
        let e = installer(&g, &Politique::default(), &mut Silencieux, &p, &Reprise::default()).unwrap_err();
        assert!(e.to_string().contains("plus la même box"), "{e}");
        assert!(g.journal().iter().all(|l| !l.starts_with("install:") && !l.contains("tmdb_token")), "{:?}", g.journal());
    }

    #[test]
    fn permissions_selon_le_sdk() {
        for (sdk, media, ext) in [("33", true, false), ("32", false, true), ("28", false, true), ("", false, false)] {
            let f = connectee();
            f.modifier(|e| e.sdk = sdk.into());
            installer(&f, &Politique::default(), &mut Silencieux, &params(), &Reprise::default()).unwrap();
            let j = f.journal().join("\n");
            assert_eq!(j.contains("READ_MEDIA_IMAGES"), media, "sdk {sdk}");
            assert_eq!(j.contains("READ_EXTERNAL_STORAGE"), ext, "sdk {sdk}");
        }
    }
}
