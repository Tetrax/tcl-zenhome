//! Retour à l'état d'origine : équivalent de `tv/restaurer-tv.sh` pour la Mi Box 4, suivi de la suppression de
//! `zenhome_garder` que fait le kit. Un échec n'arrête pas la restauration (comme `adb_essayer`).

use super::connexion::Politique;
use super::installation::PKG;
use super::origine::{Origine, Reglage};
use super::session::{Arret, Observateur, Reprise, Session};
use super::transport::{Commande, Espace, ErreurTransport, OptionPm, Transport, paquet_valide};
use super::verification::{LAUNCHERX, SPOTIFY};

pub const ACCUEIL_ORIGINE: &str = "com.google.android.tvlauncher/.MainActivity";
pub const VEILLE_ORIGINE: &str = "com.google.android.backdrop/.Backdrop";
/// Réglages globaux du kit, supprimés à la restauration (`zenhome_garder` en dernier, comme le kit).
const REGLAGES_KIT: [&str; 7] =
    ["zenhome_dock", "zenhome_layout", "zenhome_rev", "zenhome_interval", "zenhome_title_mode", "zenhome_title_b64", "zenhome_garder"];

#[derive(Debug, Clone, PartialEq)]
pub struct RapportRestauration {
    /// Faux si Zen Home a été conservé (accueil d'origine non confirmé actif) : la restauration est à refaire.
    pub incomplete: bool,
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

/// Remet un réglage à sa valeur dans la sauvegarde d'origine ; si elle ne la donne pas, `defaut` (`None` : suppression).
fn remettre(s: &mut Session<'_>, espace: Espace, cle: &str, origine: &Reglage, defaut: Option<&str>) -> Result<(), Arret> {
    let valeur = match origine {
        Reglage::Valeur(v) => Some(v.as_str()),
        Reglage::Absent => None,
        Reglage::Inconnu => defaut,
    };
    s.essayer(&match valeur {
        Some(v) => Commande::settings_put(espace, cle, v)?,
        None => Commande::settings_delete(espace, cle)?,
    })?;
    Ok(())
}

/// Fixe l'accueil puis relit celui que la box résout : vrai seulement si le réglage est accepté ET si l'accueil
/// effectif est bien du paquet visé (jamais Zen Home). Le composant d'une sauvegarde n'est pas fiable.
fn fixer_accueil(s: &mut Session<'_>, accueil: &str) -> Result<bool, Arret> {
    if !s.essayer(&Commande::set_home_activity(accueil)?)? {
        return Ok(false);
    }
    let sortie = s.lire(&Commande::resoudre_accueil())?;
    let resolu = sortie.lines().map(str::trim).rfind(|l| !l.is_empty()).and_then(|l| l.split('/').next()).unwrap_or_default();
    let paquet = accueil.split('/').next().unwrap_or_default();
    Ok(resolu == paquet && paquet != PKG)
}

/// `origine` : sauvegarde faite avant Zen Home. Sans elle, restauration générique (tout réactiver, valeurs par défaut).
pub fn restaurer(t: &dyn Transport, pol: &Politique, obs: &mut dyn Observateur, depuis: &Reprise, origine: Option<&Origine>) -> Result<RapportRestauration, Arret> {
    let mut s = Session::nouvelle(t, pol, obs, "restauration", 5, depuis);
    let mut reactives = Vec::new();
    let mut incomplete = false;

    if s.etape(0, "Réactivation des applis désactivées") {
        // Liste relue à chaque passage (reprise) : seuls les noms valides sont utilisés.
        // Celles qui l'étaient déjà avant Zen Home le restent.
        let noms: Vec<String> = noms_valides(&s.lire(&Commande::pm_list(&[OptionPm::Desactives]))?)
            .into_iter()
            .filter(|p| !origine.is_some_and(|o| o.desactives.contains(p)))
            .collect();
        for (i, p) in noms.iter().enumerate() {
            s.progression(i, noms.len(), p);
            if s.essayer(&Commande::pm_enable(p)?)? {
                reactives.push(p.clone());
            }
        }
    }
    if s.etape(1, "Écran de veille, animations et Spotify") {
        let inconnu = Reglage::Inconnu;
        remettre(&mut s, Espace::Secure, "screensaver_components", origine.map_or(&inconnu, |o| &o.veille), Some(VEILLE_ORIGINE))?;
        for (i, (k, defaut)) in [("window_animation_scale", Some("1.0")), ("transition_animation_scale", Some("1.0")), ("animator_duration_scale", None)].into_iter().enumerate() {
            remettre(&mut s, Espace::Global, k, origine.map_or(&inconnu, |o| &o.animations[i]), defaut)?;
        }
        s.essayer(&Commande::appops_set(SPOTIFY, "RUN_ANY_IN_BACKGROUND", "allow")?)?;
        s.essayer(&Commande::standby_bucket(SPOTIFY, "active")?)?;
    }
    if s.etape(2, "Accueil d'origine et désinstallation de Zen Home") {
        // L'accueil de la sauvegarde d'origine n'est repris que s'il est présent sur la box, sinon celui du kit.
        let tous = s.lire_non_vide(&Commande::pm_list(&[]))?;
        let installes = tous.as_deref().map(super::inventaire::noms_paquets);
        let mut accueil = origine
            .and_then(|o| o.accueil.as_deref())
            .filter(|a| installes.as_ref().is_some_and(|l| l.iter().any(|p| a.split('/').next() == Some(p))))
            .unwrap_or(ACCUEIL_ORIGINE);
        let mut effectif = fixer_accueil(&mut s, accueil)?;
        if !effectif && accueil != ACCUEIL_ORIGINE {
            // accueil de la sauvegarde refusé ou non effectif : celui du kit
            accueil = ACCUEIL_ORIGINE;
            effectif = fixer_accueil(&mut s, accueil)?;
        }
        let launcher = accueil.split('/').next().unwrap_or(LAUNCHERX);
        // Zen Home n'est désinstallé que si l'accueil d'origine est PRÉSENT et ACTIF : lectures relues, sinon on le garde.
        let desactives = s.lire(&Commande::pm_list(&[OptionPm::Desactives]))?;
        if installes.as_ref().is_some_and(|l| !l.iter().any(|p| p == PKG)) {
            // déjà désinstallé (reprise) : rien à faire
        } else if effectif
            && installes.as_ref().is_some_and(|l| l.iter().any(|p| p == launcher))
            && !super::inventaire::noms_paquets(&desactives).iter().any(|p| p == launcher)
        {
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
        } else {
            incomplete = true;
            s.avertir(format!(
                "restauration incomplète : Zen Home est conservé, car l'accueil d'origine ({launcher}) n'est pas confirmé présent et actif. \
                 Relancez la restauration."
            ));
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
    Ok(RapportRestauration { incomplete, reactives, avertissements: s.avertissements })
}

fn noms_valides(sortie: &str) -> Vec<String> {
    super::inventaire::noms_paquets(sortie).into_iter().filter(|p| paquet_valide(p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::fake_box::FakeBox;
    use crate::moteur::origine::tests::{TOUS, dossier_temp, sauvegarde};
    use crate::moteur::origine::origine;
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
        let r = restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default(), None).unwrap();
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
        assert!(r.avertissements.is_empty() && !r.incomplete, "{:?}", r.avertissements);
        assert_eq!(f.journal().last().map(String::as_str), Some("shell: settings delete global zenhome_garder"));
    }

    #[test]
    fn keyservice_seul_vide_l_accessibilite() {
        let f = installee();
        f.modifier(|e| {
            e.secure.insert("enabled_accessibility_services".into(), "tv.zenhome/tv.zenhome.KeyService".into());
        });
        restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default(), None).unwrap();
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
        let r = restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default(), None).unwrap();
        assert_eq!(f.journal().iter().filter(|l| l.starts_with("uninstall:")).count(), 1);
        assert!(r.avertissements.is_empty(), "{:?}", r.avertissements);
        assert!(!f.instantane().paquets.contains_key(PKG));
    }

    #[test]
    fn accueil_d_origine_encore_desactive_ne_desinstalle_pas_zen_home() {
        let f = installee();
        // la lecture des applis désactivées (étape 0) revient vide : rien n'est réactivé, l'accueil d'origine reste éteint
        f.lectures_vides(1);
        let r = restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default(), None).unwrap();
        assert!(f.journal().iter().all(|l| !l.starts_with("uninstall:")), "{:?}", f.journal());
        assert!(f.instantane().paquets.contains_key(PKG));
        assert!(r.avertissements.iter().any(|a| a.contains("Zen Home") && a.contains("conservé")), "{:?}", r.avertissements);
        assert!(r.incomplete);
    }

    #[test]
    fn echec_isole_n_arrete_pas_la_restauration() {
        let f = installee();
        // Spotify absent : appops/standby ne sont pas refusés par la fausse box, mais l'accueil d'origine l'est si désinstallé
        f.modifier(|e| {
            e.paquets.remove("com.google.android.tvlauncher");
        });
        let r = restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default(), None).unwrap();
        assert!(r.avertissements.iter().any(|a| a.contains("set-home-activity")), "{:?}", r.avertissements);
        // accueil d'origine absent : Zen Home est conservé (la box resterait sans accueil)
        assert!(f.instantane().paquets.contains_key(PKG));
        assert!(r.incomplete);
    }

    /// Ajoute à la box un second accueil (`com.mitv.tvhome`), copie d'une appli système.
    fn avec_accueil_mitv(e: &mut crate::moteur::fake_box::EtatBox) {
        let mut p = e.paquets["com.google.android.tvlauncher"].clone();
        p.desactive = false;
        e.paquets.insert("com.mitv.tvhome".into(), p);
    }

    /// Origine lue par le vrai analyseur dans une sauvegarde de test (série `S1`).
    fn origine_de(fichiers: &[(&str, &str)]) -> Option<Origine> {
        let r = dossier_temp("restauration");
        let mut tous = vec![("paquets-tous.txt", TOUS)];
        tous.extend_from_slice(fichiers);
        sauvegarde(&r, "20261029-083553-S1", &tous);
        origine(&r, "S1")
    }

    #[test]
    fn depuis_l_origine_seules_les_applis_desactivees_par_zen_home_reviennent() {
        let f = installee();
        let o = origine_de(&[
            ("paquets-desactives.txt", "package:tv.alphonso.alphonso_eula\npackage:com.google.android.tvlauncher\n"),
            ("accueil.txt", "priority=2\ncom.mitv.tvhome/.MainActivity\n"),
            ("reglages-secure.txt", "screensaver_components=com.autre.veille/.Reve\n"),
            ("reglages-global.txt", "window_animation_scale=0.5\ntransition_animation_scale=null\nanimator_duration_scale=2.0\n"),
        ])
        .unwrap();
        f.modifier(|e| {
            e.global.insert("transition_animation_scale".into(), "0".into());
            avec_accueil_mitv(e);
            e.paquets.get_mut("com.google.android.tvlauncher").unwrap().desactive = true;
        });
        let r = restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default(), Some(&o)).unwrap();
        let e = f.instantane();
        assert_eq!(r.reactives.len(), 1, "{:?}", r.reactives);
        assert!(e.paquets["tv.alphonso.alphonso_eula"].desactive, "désactivée avant Zen Home : elle le reste");
        assert!(e.paquets["com.google.android.tvlauncher"].desactive, "désactivé avant Zen Home : il le reste");
        assert!(!e.paquets["com.google.android.tungsten.setupwraith"].desactive);
        assert_eq!(e.secure["screensaver_components"], "com.autre.veille/.Reve");
        assert_eq!(e.global["window_animation_scale"], "0.5");
        assert!(!e.global.contains_key("transition_animation_scale"), "absente dans l'origine : supprimée");
        assert_eq!(e.global["animator_duration_scale"], "2.0");
        assert_eq!(e.accueil_prefere.as_deref(), Some("com.mitv.tvhome/.MainActivity"));
        assert!(!e.paquets.contains_key(PKG), "accueil de l'origine présent et actif : Zen Home désinstallé");
        assert!(r.avertissements.is_empty() && !r.incomplete, "{:?}", r.avertissements);
        assert_eq!(e.secure["enabled_accessibility_services"], "com.autre/.Service", "le reste est inchangé");
        assert!(e.global.keys().all(|k| !k.starts_with("zenhome_")));
    }

    #[test]
    fn accueil_d_origine_absent_de_la_box_ou_desactive_garde_celui_du_kit() {
        let f = installee();
        let o = origine_de(&[("paquets-desactives.txt", ""), ("accueil.txt", "com.absent.home/.Main\n")]).unwrap();
        restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default(), Some(&o)).unwrap();
        assert_eq!(f.instantane().accueil_prefere.as_deref(), Some(ACCUEIL_ORIGINE));
        // accueil d'origine désactivé dans la sauvegarde : jamais retenu
        let o = origine_de(&[("paquets-desactives.txt", "package:com.android.tv.settings\n"), ("accueil.txt", "com.android.tv.settings/.Main\n")]).unwrap();
        assert_eq!(o.accueil, None);
    }

    #[test]
    fn garde_de_desinstallation_sur_l_accueil_de_l_origine() {
        let f = installee();
        let o = origine_de(&[("paquets-desactives.txt", ""), ("accueil.txt", "com.mitv.tvhome/.MainActivity\n")]).unwrap();
        f.modifier(|e| {
            avec_accueil_mitv(e);
            e.paquets.get_mut("com.mitv.tvhome").unwrap().desactive = true;
        });
        // l'accueil d'origine reste éteint (lecture des désactivées vide) : l'accueil effectif n'est pas le sien, Zen Home est conservé
        f.lectures_vides(1);
        let r = restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default(), Some(&o)).unwrap();
        assert!(r.incomplete && f.instantane().paquets.contains_key(PKG), "{:?}", r.avertissements);
        assert!(r.avertissements.iter().any(|a| a.contains("conservé")), "{:?}", r.avertissements);
    }

    #[test]
    fn composant_qui_n_est_pas_un_accueil_est_refuse_puis_accueil_du_kit() {
        let f = installee();
        let o = origine_de(&[("paquets-desactives.txt", ""), ("accueil.txt", "com.android.tv.settings/.MainActivity\n")]).unwrap();
        assert_eq!(o.accueil.as_deref(), Some("com.android.tv.settings/.MainActivity"), "valide en forme, refusé par la box");
        let r = restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default(), Some(&o)).unwrap();
        let e = f.instantane();
        assert_eq!(e.accueil_prefere.as_deref(), Some(ACCUEIL_ORIGINE));
        assert!(!e.paquets.contains_key(PKG), "retombé sur l'accueil du kit, actif : Zen Home désinstallé");
        assert!(r.avertissements.iter().any(|a| a.contains("set-home-activity")), "{:?}", r.avertissements);
        assert!(!r.incomplete);
    }

    #[test]
    fn jamais_de_desinstallation_si_le_reglage_d_accueil_est_refuse() {
        let f = installee();
        // l'accueil du kit lui-même est refusé (paquet absent) : Zen Home reste
        f.modifier(|e| {
            e.paquets.remove("com.google.android.tvlauncher");
        });
        let o = origine_de(&[("paquets-desactives.txt", ""), ("accueil.txt", "com.android.tv.settings/.MainActivity\n")]).unwrap();
        let r = restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default(), Some(&o)).unwrap();
        assert!(r.incomplete && f.instantane().paquets.contains_key(PKG));
    }

    #[test]
    fn fichiers_pieges_donnent_la_restauration_generique() {
        // paquet invalide dans la liste : pas d'origine du tout
        assert!(origine_de(&[("paquets-desactives.txt", "package:tv.alphonso.alphonso_eula; reboot\n")]).is_none());
        // composant, veille et animations invalides : valeurs par défaut du kit
        let enorme = "a=1\n".repeat(2 * 1024 * 1024);
        let o = origine_de(&[
            ("paquets-desactives.txt", "package:tv.alphonso.alphonso_eula\n"),
            ("accueil.txt", "com.exemple/.A; reboot\n"),
            ("reglages-secure.txt", "screensaver_components=a.b/.C;reboot\n"),
            ("reglages-global.txt", &enorme),
        ])
        .unwrap();
        let f = installee();
        let r = restaurer(&f, &Politique::default(), &mut Silencieux, &Reprise::default(), Some(&o)).unwrap();
        let e = f.instantane();
        assert_eq!(e.secure["screensaver_components"], VEILLE_ORIGINE);
        assert_eq!(e.global["window_animation_scale"], "1.0");
        assert!(!e.global.contains_key("animator_duration_scale"));
        assert_eq!(e.accueil_prefere.as_deref(), Some(ACCUEIL_ORIGINE));
        assert!(r.avertissements.is_empty() && !r.incomplete, "{:?}", r.avertissements);
    }
}
