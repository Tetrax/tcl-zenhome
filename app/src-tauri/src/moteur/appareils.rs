//! Table des appareils pris en charge : identification stricte, accueil et veille d'origine, liste d'allègement,
//! paquets protégés. Référence : `charger_profil` de tv/lib.sh (l'`id` est le PROFIL du kit).

use super::connexion::nettoyer_modele;
use super::debloat::{self, Entree};

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

#[derive(Debug)]
pub struct Appareil {
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

pub static MIBOX4: Appareil = Appareil {
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

pub static TCL_G08: Appareil = Appareil {
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

pub static TOUS: [&Appareil; 2] = [&MIBOX4, &TCL_G08];

pub fn par_id(id: &str) -> Option<&'static Appareil> {
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

impl Appareil {
    /// Liste d'allègement annotée de l'appareil.
    pub fn liste(&self) -> Vec<Entree> {
        debloat::analyser(self.liste).unwrap_or_else(|_| panic!("tv/debloat-{}.txt est vide ou illisible", self.id))
    }

    /// Paquet de l'accueil d'origine.
    pub fn launcher(&self) -> &'static str {
        self.accueil.split('/').next().unwrap_or_default()
    }

    pub fn est_protege(&self, paquet: &str) -> bool {
        SOCLE.contains(&paquet) || paquet == self.launcher() || self.proteges.contains(&paquet)
    }

    /// Indice pour la découverte mDNS (`zh_reseau::chercher_box`).
    pub fn indice(&self) -> zh_reseau::IndiceAppareil<'static> {
        zh_reseau::IndiceAppareil { id: self.id, libelle: self.nom_defaut, prefixes_md: self.prefixes_md, prefixes_instance: self.prefixes_instance }
    }

    pub fn plus_recent_que_teste(&self, sdk: u32) -> bool {
        sdk > self.sdk_teste
    }

    fn reconnait(&self, p: &Proprietes, sdk: Option<u32>) -> bool {
        self.marque == p.marque
            && self.produits.contains(&p.nom)
            && self.device.is_none_or(|d| d == p.device)
            && sdk.is_some_and(|s| s >= self.sdk_min)
    }
}

/// Identification stricte : toutes les conditions de la fiche doivent être remplies.
pub fn identifier(p: &Proprietes) -> Result<&'static Appareil, Inconnu> {
    let sdk = p.sdk.parse::<u32>().ok();
    TOUS.iter()
        .copied()
        .find(|a| a.reconnait(p, sdk))
        .ok_or_else(|| Inconnu { resume: [p.nom, p.marque, p.device, p.sdk].map(nettoyer_modele).join(" / ") })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::transport::{composant_valide, paquet_valide};
    use std::path::Path;
    use std::process::Command;

    fn bonnes(a: &Appareil) -> Proprietes<'static> {
        Proprietes { nom: a.produits[0], marque: a.marque, device: a.device.unwrap_or("x"), sdk: "40" }
    }

    #[test]
    fn identification_stricte() {
        for a in TOUS {
            let sdk_min = a.sdk_min.to_string();
            let sdk_bas = (a.sdk_min - 1).to_string();
            assert_eq!(identifier(&Proprietes { sdk: &sdk_min, ..bonnes(a) }).map(|r| r.id), Ok(a.id));
            assert_eq!(identifier(&bonnes(a)).map(|r| r.id), Ok(a.id), "SDK au-delà du dernier testé : reconnu");
            assert!(a.plus_recent_que_teste(40) && !a.plus_recent_que_teste(a.sdk_teste));
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
            for e in a.liste() {
                assert!(paquet_valide(&e.paquet), "{}", e.paquet);
                assert!(!a.est_protege(&e.paquet), "{} : {} est protégé", a.id, e.paquet);
            }
            assert!(a.est_protege("tv.zenhome") && a.est_protege(a.launcher()) && !a.est_protege("com.exemple.lecteur"));
        }
        assert!(par_id("inconnu").is_none());
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
