//! Classification en lecture seule d'un appareil : vérifié (fiche stricte), compatible probable ou non supporté.
//! Fonction pure sur ce que le rapport d'appareil a déjà lu ; elle ne change rien à l'installation.

use std::collections::BTreeSet;

use super::appareils::{self, ModeAccueil, Proprietes};
use super::transport::paquet_valide;

#[derive(Debug, Clone, PartialEq)]
pub enum Classification {
    /// `identifier()` réussit (id de la fiche).
    Verifie(&'static str),
    CompatibleProbable { mode_accueil: ModeAccueil, accueil_origine: String },
    /// Raison courte, lisible.
    NonSupporte(String),
}

/// Lectures nécessaires (sorties brutes, non assainies : les lignes comptent).
pub struct Lecture<'a> {
    pub proprietes: Proprietes<'a>,
    /// `ro.product.manufacturer`
    pub fabricant: &'a str,
    /// `ro.build.version.release`
    pub release: &'a str,
    /// `ro.build.characteristics`
    pub caracteristiques: &'a str,
    /// Sortie de `pm list features`.
    pub features: &'a str,
    /// Paquets système (`pm list packages -s`).
    pub systeme: &'a BTreeSet<String>,
    /// Sortie de `cmd package resolve-activity --brief` (HOME).
    pub resolution: &'a str,
    /// Sortie de `cmd role get-role-holders` (vide si non lu).
    pub role: &'a str,
}

/// Accueils qui ne sont jamais un accueil d'origine (sélecteur, repli, assistant d'installation).
const ECARTES: [&str; 3] = ["android", "com.android.settings", "com.google.android.tungsten.setupwraith"];

fn non(raison: impl Into<String>) -> Classification {
    Classification::NonSupporte(raison.into())
}

fn est_amazon(v: &str) -> bool {
    v.trim().eq_ignore_ascii_case("amazon")
}

/// Premier critère en échec, dans l'ordre : TV, SDK, Amazon, accueil d'origine.
pub fn classer(l: &Lecture) -> Classification {
    if let Ok(a) = appareils::identifier(&l.proprietes) {
        return Classification::Verifie(a.id);
    }
    let tv = l.caracteristiques.split(',').any(|c| c.trim().eq_ignore_ascii_case("tv"))
        || l.features.lines().any(|f| f.trim().split('=').next() == Some("feature:android.software.leanback"));
    if !tv {
        return non("pas une TV Android");
    }
    let Ok(sdk) = l.proprietes.sdk.parse::<u32>() else { return non("version d'Android illisible") };
    if sdk < 28 {
        return non(format!("Android trop ancien ({})", if l.release.is_empty() { format!("SDK {sdk}") } else { l.release.to_string() }));
    }
    if est_amazon(l.fabricant) || est_amazon(l.proprietes.marque) || l.systeme.contains("com.amazon.tv.launcher") {
        return non("Fire TV : l'accueil ne peut pas être remplacé");
    }
    // accueil d'origine : le détenteur du rôle HOME s'il y en a un (Android 10+), sinon la résolution HOME
    let detenteur = if sdk >= 29 { l.role.lines().map(str::trim).find(|x| !x.is_empty()) } else { None };
    let candidat = detenteur.or_else(|| {
        let ligne = l.resolution.lines().rev().map(str::trim).find(|x| !x.is_empty())?;
        ligne.split('/').next()
    });
    let Some(paquet) = candidat else { return non("accueil d'origine introuvable") };
    if paquet == "tv.zenhome" {
        return non("Zen Home déjà installé : accueil d'origine inconnu");
    }
    if ECARTES.contains(&paquet) || !paquet_valide(paquet) || !l.systeme.contains(paquet) {
        return non("accueil d'origine introuvable");
    }
    let mode_accueil = if detenteur.is_some() { ModeAccueil::RoleHome } else { ModeAccueil::AccueilPrefere };
    Classification::CompatibleProbable { mode_accueil, accueil_origine: paquet.to_string() }
}

impl Classification {
    /// Ligne du rapport d'appareil (après « classification : »).
    pub fn libelle(&self) -> String {
        match self {
            Self::Verifie(id) => format!("vérifié ({id})"),
            Self::CompatibleProbable { mode_accueil, accueil_origine } => {
                let mode = if *mode_accueil == ModeAccueil::RoleHome { "rôle HOME" } else { "accueil préféré" };
                format!("compatible probable ({mode}, accueil d'origine : {accueil_origine})")
            }
            Self::NonSupporte(raison) => format!("non supporté : {raison}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::appareils::{MIBOX4, TCL_G08};

    const LEANBACK: &str = "feature:android.hardware.wifi\nfeature:android.software.leanback\n";
    const TV_GOOGLE: &str = "com.google.android.tvlauncher";
    const LAUNCHERX: &str = "com.google.android.apps.tv.launcherx";

    struct Cas {
        nom: &'static str,
        marque: &'static str,
        device: &'static str,
        sdk: &'static str,
        release: &'static str,
        fabricant: &'static str,
        caracteristiques: &'static str,
        features: &'static str,
        systeme: Vec<&'static str>,
        resolution: &'static str,
        role: &'static str,
    }

    /// TV Android 11 inconnue, accueil Google par résolution, sans rôle.
    fn base() -> Cas {
        Cas {
            nom: "inconnu",
            marque: "Marque",
            device: "dev",
            sdk: "30",
            release: "11",
            fabricant: "Marque",
            caracteristiques: "tv",
            features: LEANBACK,
            systeme: vec![TV_GOOGLE, LAUNCHERX, "com.android.settings"],
            resolution: "priority=0 preferredOrder=0\ncom.google.android.tvlauncher/.MainActivity\n",
            role: "",
        }
    }

    fn classe(c: &Cas) -> Classification {
        let systeme: BTreeSet<String> = c.systeme.iter().map(|s| s.to_string()).collect();
        classer(&Lecture {
            proprietes: Proprietes { nom: c.nom, marque: c.marque, device: c.device, sdk: c.sdk },
            fabricant: c.fabricant,
            release: c.release,
            caracteristiques: c.caracteristiques,
            features: c.features,
            systeme: &systeme,
            resolution: c.resolution,
            role: c.role,
        })
    }

    fn refus(c: &Cas) -> String {
        match classe(c) {
            Classification::NonSupporte(r) => r,
            autre => panic!("attendu non supporté : {autre:?}"),
        }
    }

    fn probable(mode_accueil: ModeAccueil, p: &str) -> Classification {
        Classification::CompatibleProbable { mode_accueil, accueil_origine: p.into() }
    }

    fn est_probable(c: &Cas) -> bool {
        matches!(classe(c), Classification::CompatibleProbable { .. })
    }

    #[test]
    fn appareils_verifies() {
        for a in [&MIBOX4, &TCL_G08] {
            let c = Cas { nom: a.produits[0], marque: a.marque, device: a.device.unwrap(), sdk: "34", caracteristiques: "", features: "", ..base() };
            assert_eq!(classe(&c), Classification::Verifie(a.id));
        }
        let c = Cas { nom: "oneday", marque: "Xiaomi", device: "oneday", sdk: "28", ..base() };
        assert_eq!(classe(&c).libelle(), "vérifié (mibox4)");
    }

    #[test]
    fn chromecast_google_tv_role_home() {
        let c = Cas {
            nom: "sabrina",
            marque: "google",
            device: "sabrina",
            sdk: "34",
            release: "14",
            fabricant: "Google",
            caracteristiques: "nosdcard,tv",
            role: "com.google.android.apps.tv.launcherx\n",
            ..base()
        };
        assert_eq!(classe(&c), probable(ModeAccueil::RoleHome, LAUNCHERX));
        assert_eq!(classe(&c).libelle(), format!("compatible probable (rôle HOME, accueil d'origine : {LAUNCHERX})"));
        assert_eq!(classe(&Cas { sdk: "31", ..c }), probable(ModeAccueil::RoleHome, LAUNCHERX));
    }

    #[test]
    fn nvidia_shield_sans_role_utilise_la_resolution() {
        let c = Cas { nom: "foster_e", marque: "NVIDIA", fabricant: "NVIDIA", sdk: "30", ..base() };
        assert_eq!(classe(&c), probable(ModeAccueil::AccueilPrefere, TV_GOOGLE));
    }

    #[test]
    fn vieille_box_android_9_sans_role() {
        // Android 9 : le rôle n'est pas lu, même si une sortie traîne
        let c = Cas { sdk: "28", release: "9", role: LAUNCHERX, ..base() };
        assert_eq!(classe(&c), probable(ModeAccueil::AccueilPrefere, TV_GOOGLE));
    }

    #[test]
    fn leanback_ou_caracteristique_tv_suffit() {
        assert!(est_probable(&Cas { features: "", ..base() }));
        assert!(est_probable(&Cas { caracteristiques: "default", ..base() }));
        assert!(est_probable(&Cas { caracteristiques: "nosdcard, TV", features: "", ..base() }));
        assert_eq!(refus(&Cas { caracteristiques: "tvbox", features: "feature:android.software.leanback_only\n", ..base() }), "pas une TV Android");
    }

    #[test]
    fn fire_tv_refusee() {
        let raison = "Fire TV : l'accueil ne peut pas être remplacé";
        assert_eq!(refus(&Cas { fabricant: "Amazon", ..base() }), raison);
        assert_eq!(refus(&Cas { marque: "amazon", ..base() }), raison);
        let mut c = base();
        c.systeme.push("com.amazon.tv.launcher");
        assert_eq!(refus(&c), raison);
    }

    #[test]
    fn telephone_et_android_ancien_refuses() {
        assert_eq!(refus(&Cas { caracteristiques: "default", features: "feature:android.hardware.wifi\n", ..base() }), "pas une TV Android");
        assert_eq!(refus(&Cas { sdk: "26", release: "8", ..base() }), "Android trop ancien (8)");
        assert_eq!(refus(&Cas { sdk: "", ..base() }), "version d'Android illisible");
    }

    #[test]
    fn accueil_inutilisable_refuse() {
        let introuvable = "accueil d'origine introuvable";
        let resolution = |r| Cas { resolution: r, ..base() };
        assert_eq!(refus(&resolution("android/com.android.internal.app.ResolverActivity\n")), introuvable);
        assert_eq!(refus(&resolution("com.android.settings/.FallbackHome\n")), introuvable);
        assert_eq!(refus(&resolution("com.google.android.tungsten.setupwraith/.Main\n")), introuvable);
        assert_eq!(refus(&resolution("")), introuvable);
        assert_eq!(refus(&resolution("pas un paquet/.A\n")), introuvable);
        // appli de l'utilisateur : absente des paquets système
        assert_eq!(refus(&resolution("com.exemple.lanceur/.Accueil\n")), introuvable);
        assert_eq!(refus(&Cas { role: "com.exemple.lanceur\n", ..base() }), introuvable);
        // Zen Home déjà installé
        let zen = "Zen Home déjà installé : accueil d'origine inconnu";
        assert_eq!(refus(&Cas { role: "tv.zenhome\n", ..base() }), zen);
        assert_eq!(refus(&resolution("tv.zenhome/.Accueil\n")), zen);
    }
}
