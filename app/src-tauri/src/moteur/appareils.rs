//! Table des appareils pris en charge : identification stricte, accueil et veille d'origine, liste d'allègement,
//! paquets protégés. Référence : `charger_profil` de tv/lib.sh (l'`id` est le PROFIL du kit).

use super::classification::{Classification, Lecture};
use super::connexion::nettoyer_modele;
use super::debloat::{self, Entree};
use super::transport::{composant_valide, paquet_valide};

/// Id de l'appareil générique (Android TV non vérifiée), construit par `Appareil::generique`.
pub const ID_GENERIQUE: &str = "generique";
/// Nom de l'appareil générique tant que l'utilisateur n'en a pas donné un (même texte que `commandes::memoire`).
pub const NOM_DEFAUT_GENERIQUE: &str = "TV Android";

/// Paquets que Zen Home ne désactive ni ne désinstalle jamais, sur tous les appareils.
pub const SOCLE: [&str; 13] = [
    "tv.zenhome",
    "com.google.android.tungsten.setupwraith",
    "com.android.tv.settings",
    "com.google.android.gms",
    "com.google.android.gsf",
    "com.android.vending",
    "com.google.android.packageinstaller",
    "com.android.packageinstaller",
    "com.android.systemui",
    "com.android.providers.settings",
    "com.android.providers.media",
    "com.google.android.inputmethod.latin",
    "com.android.inputmethod.latin",
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModeAccueil {
    /// Rôle HOME géré par `cmd role` (Android 10+, `ROLE_HOME=1` du kit).
    RoleHome,
    /// Accueil préféré (`set-home-activity`) seulement (Android 9).
    AccueilPrefere,
}

/// Fiche figée d'un appareil vérifié.
#[derive(Debug)]
pub struct Profil {
    /// PROFIL du kit.
    pub id: &'static str,
    /// Modèle affiché.
    pub nom: &'static str,
    /// Nom de la box tant que l'utilisateur n'en a pas donné un.
    pub nom_defaut: &'static str,
    /// « box » ou « TV », pour les textes.
    pub mot: &'static str,
    /// `ro.product.brand` attendu.
    pub marque: &'static str,
    /// `ro.product.name` accepté (liste exacte ; `PRODUIT` du kit en premier).
    pub produits: &'static [&'static str],
    /// `ro.product.device` attendu, s'il est connu.
    pub device: Option<&'static str>,
    pub sdk_min: u32,
    /// Dernier SDK testé : au-delà, avertissement seulement.
    pub sdk_teste: u32,
    /// Découverte mDNS seulement (jamais une preuve d'identité) : préfixes du champ `md` et du nom d'instance.
    pub prefixes_md: &'static [&'static str],
    pub prefixes_instance: &'static [&'static str],
    pub mode_accueil: ModeAccueil,
    /// `ACCUEIL` du kit.
    pub accueil: &'static str,
    /// `VEILLE` du kit.
    pub veille: &'static str,
    /// Paquets protégés propres à l'appareil (en plus du socle et de l'accueil d'origine).
    pub proteges: &'static [&'static str],
    /// Contenu de tv/debloat-<id>.txt.
    liste: &'static str,
}

pub static MIBOX4: Profil = Profil {
    id: "mibox4",
    nom: "Xiaomi Mi Box 4",
    nom_defaut: "Mi Box",
    mot: "box",
    marque: "Xiaomi",
    produits: &["oneday"],
    device: Some("oneday"),
    sdk_min: 28,
    sdk_teste: 28,
    prefixes_md: &["MIBOX"],
    prefixes_instance: &["MIBOX"],
    mode_accueil: ModeAccueil::AccueilPrefere,
    accueil: "com.google.android.tvlauncher/.MainActivity",
    veille: "com.google.android.backdrop/.Backdrop",
    proteges: &[],
    liste: include_str!("../../../../tv/debloat-mibox4.txt"),
};

pub static TCL_G08: Profil = Profil {
    id: "tcl-g08",
    nom: "TV TCL G08 (Google TV)",
    nom_defaut: "TV TCL",
    mot: "TV",
    marque: "TCL",
    produits: &["G08_4K_GB"],
    device: Some("G08"),
    sdk_min: 34,
    sdk_teste: 34,
    prefixes_md: &["Smart TV Pro"],
    prefixes_instance: &["Smart-TV-Pro"],
    mode_accueil: ModeAccueil::RoleHome,
    accueil: "com.google.android.apps.tv.launcherx/.home.HomeActivity",
    veille: "com.google.android.apps.tv.dreamx/.service.Backdrop",
    proteges: &["com.tcl.tv", "com.tcl.micmanager", "com.tcl.framework.custom", "com.tcl.suspension"],
    liste: include_str!("../../../../tv/debloat-tcl-g08.txt"),
};

pub static TOUS: [&Profil; 2] = [&MIBOX4, &TCL_G08];

pub fn par_id(id: &str) -> Option<&'static Profil> {
    TOUS.iter().copied().find(|a| a.id == id)
}

/// Ce que la box répond (valeurs déjà débarrassées des blancs de bord).
pub struct Proprietes<'a> {
    /// `ro.product.name`
    pub nom: &'a str,
    /// `ro.product.brand`
    pub marque: &'a str,
    /// `ro.product.device`
    pub device: &'a str,
    /// `ro.build.version.sdk`
    pub sdk: &'a str,
}

/// Appareil non reconnu : `resume` est nettoyé, affichable.
#[derive(Debug, Clone, PartialEq)]
pub struct Inconnu {
    pub resume: String,
}

impl Profil {
    /// Liste d'allègement annotée de l'appareil.
    pub fn liste(&self) -> Vec<Entree> {
        debloat::analyser(self.liste).unwrap_or_else(|_| panic!("tv/debloat-{}.txt est vide ou illisible", self.id))
    }

    /// Paquet de l'accueil d'origine.
    pub fn launcher(&self) -> &'static str {
        self.accueil.split('/').next().unwrap_or_default()
    }

    /// Indice pour la découverte mDNS (`zh_reseau::chercher_box`).
    pub fn indice(&self) -> zh_reseau::IndiceAppareil<'static> {
        zh_reseau::IndiceAppareil { id: self.id, libelle: self.nom_defaut, prefixes_md: self.prefixes_md, prefixes_instance: self.prefixes_instance }
    }

    fn reconnait(&self, p: &Proprietes, sdk: Option<u32>) -> bool {
        self.marque == p.marque
            && self.produits.contains(&p.nom)
            && self.device.is_none_or(|d| d == p.device)
            && sdk.is_some_and(|s| s >= self.sdk_min)
    }
}

/// Appareil connecté, construit à la connexion (aujourd'hui copié d'un profil).
#[derive(Debug, Clone)]
pub struct Appareil {
    pub id: String,
    pub nom: String,
    pub nom_defaut: String,
    pub mot: String,
    pub sdk_teste: u32,
    pub mode_accueil: ModeAccueil,
    pub accueil: String,
    /// Écran de veille d'origine ; `None` : aucun connu, le réglage est supprimé à la restauration.
    pub veille: Option<String>,
    pub proteges: Vec<String>,
    liste: &'static str,
}

/// Réglages `secure` bruts lus sur un appareil inconnu (sorties de `settings get`, non assainies).
pub struct ReglagesLus<'a> {
    /// `screensaver_components`
    pub veille: &'a str,
    /// `default_input_method`
    pub clavier: &'a str,
    /// `enabled_accessibility_services`
    pub accessibilite: &'a str,
}

/// Liste d'allègement prudente de l'appareil générique.
pub const LISTE_GENERIQUE: &str = include_str!("../../../../tv/debloat-generique.txt");

impl Appareil {
    pub fn depuis(p: &'static Profil) -> Self {
        Self {
            id: p.id.into(),
            nom: p.nom.into(),
            nom_defaut: p.nom_defaut.into(),
            mot: p.mot.into(),
            sdk_teste: p.sdk_teste,
            mode_accueil: p.mode_accueil,
            accueil: p.accueil.into(),
            veille: Some(p.veille.into()),
            proteges: p.proteges.iter().map(|&s| s.into()).collect(),
            liste: p.liste,
        }
    }

    /// Appareil générique (Android TV non vérifiée) ; `None` si la classification n'est pas « compatible probable »
    /// ou si le SDK est illisible. Pur : tout vient des sorties déjà lues.
    /// Accueil d'origine : le composant de la dernière ligne de la résolution HOME si c'est bien le paquet d'origine ; sinon
    /// (rôle HOME tenu par un paquet que la résolution ne montre pas) `paquet/.MainActivity`. En mode RoleHome, la
    /// restauration rend le rôle au PAQUET (`fixer_accueil`) et ne vérifie que le paquet résolu : un composant de repli
    /// erroné est donc sans effet (voir `restaurer`).
    /// Veille : le réglage courant s'il est un composant valide qui n'est pas de Zen Home, sinon aucune.
    pub fn generique(l: &Lecture, c: &Classification, r: &ReglagesLus) -> Option<Self> {
        let Classification::CompatibleProbable { mode_accueil, accueil_origine } = c else { return None };
        let sdk = l.proprietes.sdk.parse::<u32>().ok()?;
        let accueil = l
            .resolution
            .lines()
            .map(str::trim)
            .rfind(|x| !x.is_empty())
            .filter(|x| composant_valide(x) && x.split('/').next() == Some(accueil_origine))
            .map_or_else(|| format!("{accueil_origine}/.MainActivity"), String::from);
        let veille = Some(r.veille.trim()).filter(|v| composant_valide(v) && !v.starts_with("tv.zenhome/")).map(String::from);
        let mut proteges: Vec<String> = Vec::new();
        let paquets = r.clavier.trim().split('/').take(1).chain(r.accessibilite.trim().split(':').filter_map(|e| e.split('/').next()));
        for p in paquets {
            if paquet_valide(p) && p != "tv.zenhome" && !proteges.iter().any(|x| x == p) {
                proteges.push(p.into());
            }
        }
        Some(Self {
            id: ID_GENERIQUE.into(),
            nom: "TV Android (non vérifiée)".into(),
            nom_defaut: NOM_DEFAUT_GENERIQUE.into(),
            mot: "TV".into(),
            sdk_teste: sdk,
            mode_accueil: *mode_accueil,
            accueil,
            veille,
            proteges,
            liste: LISTE_GENERIQUE,
        })
    }

    /// Appareil non vérifié (construit à la connexion depuis ce que l'appareil a répondu).
    pub fn est_generique(&self) -> bool {
        self.id == ID_GENERIQUE
    }

    /// Liste d'allègement annotée de l'appareil.
    pub fn liste(&self) -> Vec<Entree> {
        debloat::analyser(self.liste).unwrap_or_else(|_| panic!("tv/debloat-{}.txt est vide ou illisible", self.id))
    }

    /// Paquet de l'accueil d'origine.
    pub fn launcher(&self) -> &str {
        self.accueil.split('/').next().unwrap_or_default()
    }

    pub fn est_protege(&self, paquet: &str) -> bool {
        SOCLE.contains(&paquet) || paquet == self.launcher() || self.proteges.iter().any(|p| p == paquet)
    }

    pub fn plus_recent_que_teste(&self, sdk: u32) -> bool {
        sdk > self.sdk_teste
    }
}

/// Identification stricte : toutes les conditions de la fiche doivent être remplies.
pub fn identifier(p: &Proprietes) -> Result<&'static Profil, Inconnu> {
    let sdk = p.sdk.parse::<u32>().ok();
    TOUS.iter()
        .copied()
        .find(|a| a.reconnait(p, sdk))
        .ok_or_else(|| Inconnu { resume: [p.nom, p.marque, p.device, p.sdk].map(nettoyer_modele).join(" / ") })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::classification::classer;
    use std::path::Path;
    use std::process::Command;

    fn bonnes(a: &Profil) -> Proprietes<'static> {
        Proprietes { nom: a.produits[0], marque: a.marque, device: a.device.unwrap_or("x"), sdk: "40" }
    }

    #[test]
    fn identification_stricte() {
        for a in TOUS {
            let sdk_min = a.sdk_min.to_string();
            let sdk_bas = (a.sdk_min - 1).to_string();
            assert_eq!(identifier(&Proprietes { sdk: &sdk_min, ..bonnes(a) }).map(|r| r.id), Ok(a.id));
            assert_eq!(identifier(&bonnes(a)).map(|r| r.id), Ok(a.id), "SDK au-delà du dernier testé : reconnu");
            let app = Appareil::depuis(a);
            assert!(app.plus_recent_que_teste(40) && !app.plus_recent_que_teste(a.sdk_teste));
            for (nom, p) in [
                ("nom", Proprietes { nom: "autre", ..bonnes(a) }),
                ("nom vide", Proprietes { nom: "", ..bonnes(a) }),
                ("marque", Proprietes { marque: "Autre", ..bonnes(a) }),
                ("sdk trop bas", Proprietes { sdk: &sdk_bas, ..bonnes(a) }),
                ("sdk illisible", Proprietes { sdk: "", ..bonnes(a) }),
            ] {
                assert!(identifier(&p).is_err(), "{} : {nom}", a.id);
            }
            if a.device.is_some() {
                assert!(identifier(&Proprietes { device: "autre", ..bonnes(a) }).is_err(), "{} : device", a.id);
            }
        }
    }

    #[test]
    fn inconnu_a_un_resume_nettoye() {
        let p = Proprietes { nom: "G08\u{7}; rm -rf /", marque: "TCL", device: "d", sdk: "34" };
        assert_eq!(identifier(&p).unwrap_err(), Inconnu { resume: "G08 rm -rf  / TCL / d / 34".into() });
    }

    #[test]
    fn role_home_demande_android_10_au_moins() {
        for a in TOUS {
            assert!(a.mode_accueil != ModeAccueil::RoleHome || a.sdk_min >= 29, "{}", a.id);
            assert!(a.sdk_min <= a.sdk_teste, "{}", a.id);
        }
    }

    #[test]
    fn fiches_coherentes() {
        for (i, a) in TOUS.iter().enumerate() {
            assert!(TOUS[..i].iter().all(|b| b.id != a.id && b.produits.iter().all(|p| !a.produits.contains(p))), "{} en double", a.id);
            assert_eq!(par_id(a.id).map(|b| b.id), Some(a.id));
            assert!(composant_valide(a.accueil) && composant_valide(a.veille), "{}", a.id);
            assert!(SOCLE.iter().chain(a.proteges).all(|p| paquet_valide(p)));
            let app = Appareil::depuis(a);
            for e in a.liste() {
                assert!(paquet_valide(&e.paquet), "{}", e.paquet);
                assert!(!app.est_protege(&e.paquet), "{} : {} est protégé", a.id, e.paquet);
            }
            assert!(app.est_protege("tv.zenhome") && app.est_protege(a.launcher()) && !app.est_protege("com.exemple.lecteur"));
        }
        assert!(par_id("inconnu").is_none());
    }

    const LAUNCHERX: &str = "com.google.android.apps.tv.launcherx";
    const COMPOSANT_X: &str = "com.google.android.apps.tv.launcherx/.home.HomeActivity";

    /// Chromecast Google TV : rôle HOME, composant donné par la résolution.
    fn generique_avec(sdk: &str, role: &str, resolution: &str, r: &ReglagesLus) -> Option<Appareil> {
        let systeme = std::collections::BTreeSet::from([LAUNCHERX.to_string(), "com.google.android.tvlauncher".to_string()]);
        let l = Lecture {
            proprietes: Proprietes { nom: "sabrina", marque: "google", device: "sabrina", sdk },
            fabricant: "Google",
            release: "14",
            caracteristiques: "tv",
            features: "feature:android.software.leanback\n",
            systeme: &systeme,
            resolution,
            role,
        };
        Appareil::generique(&l, &classer(&l), r)
    }

    const AUCUN: ReglagesLus = ReglagesLus { veille: "null", clavier: "null", accessibilite: "null" };

    #[test]
    fn generique_chromecast_role_home() {
        let resolution = format!("priority=0 preferredOrder=0\n{COMPOSANT_X}\n");
        let a = generique_avec("34", "com.google.android.apps.tv.launcherx\n", &resolution, &AUCUN).unwrap();
        assert_eq!((a.id.as_str(), a.nom_defaut.as_str(), a.mot.as_str()), ("generique", "TV Android", "TV"));
        assert_eq!((a.mode_accueil, a.accueil.as_str(), a.launcher()), (ModeAccueil::RoleHome, COMPOSANT_X, LAUNCHERX));
        assert!(composant_valide(&a.accueil));
        assert_eq!(a.sdk_teste, 34);
        assert!(!a.plus_recent_que_teste(34));
        assert_eq!(a.veille, None);
    }

    #[test]
    fn generique_role_tenu_par_un_paquet_absent_de_la_resolution_prend_le_composant_de_repli() {
        let a = generique_avec("34", "com.google.android.apps.tv.launcherx\n", "com.google.android.tvlauncher/.MainActivity\n", &AUCUN).unwrap();
        assert_eq!(a.accueil, format!("{LAUNCHERX}/.MainActivity"));
        assert!(composant_valide(&a.accueil));
    }

    #[test]
    fn generique_box_android_9_accueil_prefere() {
        let a = generique_avec("28", "", "priority=0\ncom.google.android.tvlauncher/.MainActivity\n", &AUCUN).unwrap();
        assert_eq!((a.mode_accueil, a.accueil.as_str()), (ModeAccueil::AccueilPrefere, "com.google.android.tvlauncher/.MainActivity"));
        assert_eq!(a.sdk_teste, 28);
    }

    #[test]
    fn generique_refuse_ce_qui_n_est_pas_compatible_probable() {
        let l = Lecture {
            proprietes: Proprietes { nom: "oneday", marque: "Xiaomi", device: "oneday", sdk: "28" },
            fabricant: "",
            release: "",
            caracteristiques: "",
            features: "",
            systeme: &Default::default(),
            resolution: "",
            role: "",
        };
        assert!(Appareil::generique(&l, &classer(&l), &AUCUN).is_none());
    }

    #[test]
    fn generique_veille_d_origine() {
        let veille = |v: &str| generique_avec("34", "", "com.google.android.tvlauncher/.MainActivity\n", &ReglagesLus { veille: v, ..AUCUN }).map(|a| a.veille);
        let v = |s: &str| Some(Some(s.to_string()));
        assert_eq!(veille(""), Some(None));
        assert_eq!(veille("null"), Some(None));
        assert_eq!(veille("tv.zenhome/tv.zenhome.ZenDream"), Some(None));
        assert_eq!(veille("a.b/.C;reboot"), Some(None));
        assert_eq!(veille("com.google.android.apps.tv.dreamx/.service.Backdrop\n"), v("com.google.android.apps.tv.dreamx/.service.Backdrop"));
    }

    #[test]
    fn generique_protege_clavier_et_accessibilite() {
        let r = ReglagesLus {
            veille: "null",
            clavier: "com.google.android.tv.remote.service/.Ime\n",
            accessibilite: "com.google.android.marvin.talkback/.TalkBackService:tv.zenhome/tv.zenhome.KeyService:pas valide/.X:$(reboot)/.Y:com.autre/.S",
        };
        let a = generique_avec("34", "", "com.google.android.tvlauncher/.MainActivity\n", &r).unwrap();
        assert_eq!(a.proteges, ["com.google.android.tv.remote.service", "com.google.android.marvin.talkback", "com.autre"]);
        assert!(a.est_protege("com.autre") && a.est_protege(a.launcher()) && a.est_protege("tv.zenhome") && !a.est_protege("com.exemple.lecteur"));
        let a = generique_avec("34", "", "com.google.android.tvlauncher/.MainActivity\n", &ReglagesLus { clavier: "rm -rf/x", ..AUCUN }).unwrap();
        assert!(a.proteges.is_empty());
    }

    #[test]
    fn generique_liste_prudente() {
        let a = generique_avec("34", "", "com.google.android.tvlauncher/.MainActivity\n", &AUCUN).unwrap();
        let liste = a.liste();
        assert!(!liste.is_empty());
        for e in &liste {
            assert!(paquet_valide(&e.paquet), "{}", e.paquet);
            assert!(!a.est_protege(&e.paquet), "{} est protégé", e.paquet);
            for interdit in ["launcherx", "tvlauncher", "setupwraith", "talkback", "tts", "katniss"] {
                assert!(!e.paquet.contains(interdit), "{}", e.paquet);
            }
            assert!(!["com.android.vending", "com.google.android.gms"].contains(&e.paquet.as_str()));
        }
    }

    #[test]
    fn generique_n_est_pas_dans_la_table_des_profils() {
        // le test de parité avec le kit itère sur TOUS : il ne voit que les profils
        assert!(par_id(ID_GENERIQUE).is_none() && TOUS.iter().all(|p| p.id != ID_GENERIQUE));
    }

    /// Parité avec le kit : le vrai `charger_profil` de tv/lib.sh, exécuté par bash.
    #[test]
    fn parite_avec_charger_profil_du_kit() {
        let lib = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tv/lib.sh");
        let script = "set -euo pipefail; . \"$1\"; PROFIL=$2; charger_profil; printf '%s\\n' \"$PRODUIT\" \"$ROLE_HOME\" \"$ACCUEIL\" \"$VEILLE\" \"$DEBLOAT\"";
        for a in TOUS {
            let Ok(sortie) = Command::new("bash").arg("-c").arg(script).arg("bash").arg(&lib).arg(a.id).output() else {
                eprintln!("bash absent : test de parité ignoré");
                return;
            };
            assert!(sortie.status.success(), "{} : {}", a.id, String::from_utf8_lossy(&sortie.stderr));
            let role = if a.mode_accueil == ModeAccueil::RoleHome { "1" } else { "0" };
            let attendu = [a.produits[0], role, a.accueil, a.veille, &format!("debloat-{}.txt", a.id)].join("\n") + "\n";
            assert_eq!(String::from_utf8(sortie.stdout).unwrap(), attendu, "{}", a.id);
        }
    }
}
