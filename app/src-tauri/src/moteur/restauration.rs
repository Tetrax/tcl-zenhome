//! Retour à l'état d'origine : équivalent de `tv/restaurer-tv.sh` pour la Mi Box 4, suivi de la suppression de
//! `zenhome_garder` que fait le kit. Un échec n'arrête pas la restauration (comme `adb_essayer`).

use super::connexion::Politique;
use super::installation::PKG;
use super::session::{Arret, Observateur, Reprise, Session};
use super::transport::{Commande, Espace, ErreurTransport, OptionPm, Transport, paquet_valide};
use super::verification::SPOTIFY;

pub const ACCUEIL_ORIGINE: &str = "com.google.android.tvlauncher/.MainActivity";
pub const VEILLE_ORIGINE: &str = "com.google.android.backdrop/.Backdrop";
/// Réglages globaux du kit, supprimés à la restauration (`zenhome_garder` en dernier, comme le kit).
const REGLAGES_KIT: [&str; 7] =
    ["zenhome_dock", "zenhome_layout", "zenhome_rev", "zenhome_interval", "zenhome_title_mode", "zenhome_title_b64", "zenhome_garder"];

#[derive(Debug, Clone, PartialEq)]
pub struct RapportRestauration {
    pub reactives: Vec<String>,
    pub avertissements: Vec<String>,
}

/// Services d'accessibilité à conserver : tout sauf ceux de Zen Home (`tv.zenhome/...`), « null » et les valeurs vides.
/// `""` (valeur écrite par le kit pour « vide » pendant une relance de KeyService) est aussi ignoré.
fn services_restants(brut: &str) -> String {
    brut.trim()
        .split(':')
        .filter(|e| !e.starts_with("tv.zenhome/") && !["null", "", "\"\""].contains(e))
        .collect::<Vec<_>>()
        .join(":")
}

pub fn restaurer(t: &dyn Transport, pol: &Politique, obs: &mut dyn Observateur, depuis: &Reprise) -> Result<RapportRestauration, Arret> {
    let mut s = Session::nouvelle(t, pol, obs, "restauration", 5, depuis);
    let mut reactives = Vec::new();

    if s.etape(0, "Réactivation des applis désactivées") {
        // Liste relue à chaque passage (reprise) : seuls les noms valides sont utilisés.
        let noms: Vec<String> = noms_valides(&s.lire(&Commande::pm_list(&[OptionPm::Desactives]))?);
        for (i, p) in noms.iter().enumerate() {
            s.progression(i, noms.len(), p);
            if s.essayer(&Commande::pm_enable(p)?)? {
                reactives.push(p.clone());
            }
        }
    }
    if s.etape(1, "Écran de veille, animations et Spotify") {
        s.essayer(&Commande::settings_put(Espace::Secure, "screensaver_components", VEILLE_ORIGINE)?)?;
        for k in ["window_animation_scale", "transition_animation_scale"] {
            s.essayer(&Commande::settings_put(Espace::Global, k, "1.0")?)?;
        }
        s.essayer(&Commande::settings_delete(Espace::Global, "animator_duration_scale")?)?;
        s.essayer(&Commande::appops_set(SPOTIFY, "RUN_ANY_IN_BACKGROUND", "allow")?)?;
        s.essayer(&Commande::standby_bucket(SPOTIFY, "active")?)?;
    }
    if s.etape(2, "Accueil d'origine et désinstallation de Zen Home") {
        s.essayer(&Commande::set_home_activity(ACCUEIL_ORIGINE)?)?;
        // Idempotent : si Zen Home n'est plus installé (reprise), rien à désinstaller.
        let liste = Commande::pm_list(&[]);
        let resultat = s.op("désinstallation de Zen Home", |t| {
            let l = t.shell(&liste)?.stdout;
            if !l.trim().is_empty() && !super::inventaire::noms_paquets(&l).iter().any(|p| p == PKG) {
                return Ok(None);
            }
            match t.uninstall(PKG) {
                Ok(_) => Ok(None),
                Err(ErreurTransport::Commande { stderr, .. }) => Ok(Some(stderr)),
                Err(e) => Err(e),
            }
        })?;
        if let Some(m) = resultat {
            s.avertir(format!("refusé ou sans effet : désinstallation de {PKG} ({})", m.lines().next().unwrap_or_default()));
        }
        s.essayer(&Commande::touche_accueil())?;
    }
    if s.etape(3, "Accessibilité") {
        // Retire seulement KeyService : les autres services sont conservés.
        let reste = services_restants(&s.lire(&Commande::settings_get(Espace::Secure, "enabled_accessibility_services")?)?);
        if reste.is_empty() {
            s.essayer(&Commande::settings_delete(Espace::Secure, "enabled_accessibility_services")?)?;
            s.essayer(&Commande::settings_put(Espace::Secure, "accessibility_enabled", "0")?)?;
        } else {
            s.essayer(&Commande::settings_put(Espace::Secure, "enabled_accessibility_services", &reste)?)?;
        }
    }
    if s.etape(4, "Réglages de Zen Home") {
        for k in REGLAGES_KIT {
            s.essayer(&Commande::settings_delete(Espace::Global, k)?)?;
        }
    }
    Ok(RapportRestauration { reactives, avertissements: s.avertissements })
}

fn noms_valides(sortie: &str) -> Vec<String> {
    super::inventaire::noms_paquets(sortie).into_iter().filter(|p| paquet_valide(p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::fake_box::FakeBox;
    use crate::moteur::session::Silencieux;

    fn installee() -> FakeBox {
        let f = FakeBox::mibox4();
        f.connecter().unwrap();
        f.install(std::path::Path::new("ZenHome.apk")).unwrap();
        f.modifier(|e| {
            for p in ["com.google.android.tvlauncher", "com.google.android.tungsten.setupwraith", "tv.alphonso.alphonso_eula"] {
                e.paquets.get_mut(p).unwrap().desactive = true;
            }
            e.accueil_prefere = Some("tv.zenhome/.Home".into());
            e.secure.insert("enabled_accessibility_services".into(), "com.autre/.Service:tv.zenhome/tv.zenhome.KeyService".into());
            e.secure.insert("accessibility_enabled".into(), "1".into());
            e.secure.insert("screensaver_components".into(), "tv.zenhome/tv.zenhome.ZenDream".into());
            for k in ["zenhome_dock", "zenhome_layout", "zenhome_garder"] {
                e.global.insert(k.into(), "x".into());
            }
        });
        f
    }

    #[test]
    fn restauration_complete() {
        let f = installee();
        let r = restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default()).unwrap();
        let e = f.instantane();
        assert_eq!(r.reactives.len(), 3);
        assert!(e.paquets.values().all(|p| !p.desactive));
        assert!(!e.paquets.contains_key(PKG));
        assert_eq!(e.secure["screensaver_components"], VEILLE_ORIGINE);
        assert_eq!(e.accueil_prefere.as_deref(), Some(ACCUEIL_ORIGINE));
        assert_eq!(e.secure["enabled_accessibility_services"], "com.autre/.Service", "les autres services sont conservés");
        assert_eq!(e.secure["accessibility_enabled"], "1");
        assert!(e.global.keys().all(|k| !k.starts_with("zenhome_")));
        assert_eq!(e.global["window_animation_scale"], "1.0");
        assert_eq!(e.appops[&(SPOTIFY.to_string(), "RUN_ANY_IN_BACKGROUND".to_string())], "allow");
        assert!(r.avertissements.is_empty(), "{:?}", r.avertissements);
        assert_eq!(f.journal().last().map(String::as_str), Some("shell: settings delete global zenhome_garder"));
    }

    #[test]
    fn keyservice_seul_vide_l_accessibilite() {
        let f = installee();
        f.modifier(|e| {
            e.secure.insert("enabled_accessibility_services".into(), "tv.zenhome/tv.zenhome.KeyService".into());
        });
        restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default()).unwrap();
        let e = f.instantane();
        assert!(!e.secure.contains_key("enabled_accessibility_services"));
        assert_eq!(e.secure["accessibility_enabled"], "0");
        assert_eq!(services_restants("null"), "");
        assert_eq!(services_restants("\"\""), "");
        assert_eq!(services_restants("a/.B:tv.zenhome/.KeyService:c/.D"), "a/.B:c/.D");
    }

    #[test]
    fn coupure_puis_reprise_ne_desinstalle_pas_deux_fois() {
        let f = installee();
        f.perdre_reponse_desinstallation(1);
        let r = restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default()).unwrap();
        assert_eq!(f.journal().iter().filter(|l| l.starts_with("uninstall:")).count(), 1);
        assert!(r.avertissements.is_empty(), "{:?}", r.avertissements);
        assert!(!f.instantane().paquets.contains_key(PKG));
    }

    #[test]
    fn echec_isole_n_arrete_pas_la_restauration() {
        let f = installee();
        // Spotify absent : appops/standby ne sont pas refusés par la fausse box, mais l'accueil d'origine l'est si désinstallé
        f.modifier(|e| {
            e.paquets.remove("com.google.android.tvlauncher");
        });
        let r = restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default()).unwrap();
        assert!(r.avertissements.iter().any(|a| a.contains("set-home-activity")), "{:?}", r.avertissements);
        assert!(!f.instantane().paquets.contains_key(PKG));
    }
}
