//! Application des choix de l'utilisateur : équivalent de `appliquer` du kit (mémorisation de `zenhome_garder`,
//! réactivation des applis gardées, désinstallations). La désactivation proprement dite reste faite par
//! `verification::verifier` (comme le kit : `verifier-tv.sh --fix` désactive la liste moins `zenhome_garder`).

use std::collections::HashSet;

use super::connexion::Politique;
use super::debloat;
use super::inventaire::{Inventaire, PROTEGES, noms_paquets};
use super::session::{Arret, ErreurAction, Observateur, Reprise, Session};
use super::transport::{Commande, Espace, ErreurTransport, OptionPm, paquet_valide};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Choix {
    /// Applis système de la liste à garder actives.
    pub garder: Vec<String>,
    /// Applis système de la liste à désactiver (informatif ici : la vérification les désactive).
    pub desactiver: Vec<String>,
    /// Applis de l'utilisateur (`pm list packages -3`) à désinstaller, définitivement.
    pub desinstaller: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RapportChoix {
    pub gardes: Vec<String>,
    pub a_desactiver: Vec<String>,
    pub reactives: Vec<String>,
    pub desinstalles: Vec<String>,
    /// Déjà absentes au moment de désinstaller (reprise après coupure).
    pub deja_absents: Vec<String>,
    pub echecs: Vec<String>,
    pub avertissements: Vec<String>,
}

fn sans_doublons(l: &[String]) -> Vec<String> {
    let mut vus = HashSet::new();
    l.iter().filter(|p| vus.insert(p.as_str())).cloned().collect()
}

/// Refuse tout le choix si un seul paquet est hors règles : rien n'est envoyé.
fn valider(inv: &Inventaire, c: &Choix) -> Result<Choix, ErreurAction> {
    let liste: HashSet<String> = debloat::liste_mibox4().into_iter().map(|e| e.paquet).collect();
    let mut refus = Vec::new();
    for p in c.garder.iter().chain(&c.desactiver) {
        if !paquet_valide(p) {
            refus.push(format!("{p} : nom invalide"));
        } else if !liste.contains(p) {
            refus.push(format!("{p} : hors de la liste des applis à alléger"));
        } else if !inv.tous.contains(p) {
            refus.push(format!("{p} : absent de la box"));
        }
    }
    for p in &c.desinstaller {
        if !paquet_valide(p) {
            refus.push(format!("{p} : nom invalide"));
        } else if PROTEGES.contains(&p.as_str()) {
            refus.push(format!("{p} : protégé, jamais désinstallé"));
        } else if !inv.utilisateur.contains(p) {
            refus.push(format!("{p} : pas une appli de l'utilisateur de la box"));
        }
    }
    let (garder, desactiver) = (sans_doublons(&c.garder), sans_doublons(&c.desactiver));
    refus.extend(garder.iter().filter(|p| desactiver.contains(p)).map(|p| format!("{p} : à la fois gardé et désactivé")));
    if !refus.is_empty() {
        return Err(ErreurAction::Refus(refus.join(" ; ")));
    }
    Ok(Choix { garder, desactiver, desinstaller: sans_doublons(&c.desinstaller) })
}

enum Desinstallation {
    Faite,
    DejaAbsente,
    Echec(String),
}

pub fn appliquer(
    t: &dyn super::transport::Transport,
    pol: &Politique,
    obs: &mut dyn Observateur,
    inv: &Inventaire,
    choix: &Choix,
    depuis: &Reprise,
) -> Result<RapportChoix, Arret> {
    let choix = valider(inv, choix)?;
    let mut rapport = RapportChoix { gardes: choix.garder.clone(), a_desactiver: choix.desactiver.clone(), ..Default::default() };
    let mut s = Session::nouvelle(t, pol, obs, "choix", 3, depuis);

    if s.etape(0, "Mémorisation de vos choix") {
        let g = choix.garder.join(",");
        let cle = "zenhome_garder";
        let ecrire = if g.is_empty() { Commande::settings_delete(Espace::Global, cle)? } else { Commande::settings_put(Espace::Global, cle, &g)? };
        let lire = Commande::settings_get(Espace::Global, cle)?;
        let mut relu = None;
        // écriture puis relecture, recommencée si la box a décroché entre les deux (3 essais, comme le kit)
        for _ in 0..3 {
            let v = s.op("mémorisation des choix", |t| {
                t.shell(&ecrire)?;
                t.shell(&lire).map(|r| r.stdout.trim().to_string())
            })?;
            let v = if v == "null" { String::new() } else { v };
            let ok = v == g;
            relu = Some(v);
            if ok {
                break;
            }
        }
        if relu.as_deref() != Some(g.as_str()) {
            return Err(ErreurAction::Memorisation(format!("écrit « {g} », relu « {} »", relu.unwrap_or_default())).into());
        }
    }

    if s.etape(1, "Réactivation des applis gardées") {
        let a_reactiver: Vec<&String> = choix.garder.iter().filter(|p| inv.desactives.contains(p)).collect();
        for (i, p) in a_reactiver.iter().enumerate() {
            s.progression(i, a_reactiver.len(), p);
            if s.essayer(&Commande::pm_enable(p)?)? {
                rapport.reactives.push((*p).clone());
            }
        }
    }

    if s.etape(2, "Désinstallation") {
        for (i, p) in choix.desinstaller.iter().enumerate() {
            s.progression(i, choix.desinstaller.len(), p);
            let liste = Commande::pm_list(&[OptionPm::Tiers]);
            // Idempotent : une désinstallation dont la réponse s'est perdue ne serait pas rejouée (elle échouerait).
            let r = s.op("désinstallation", |t| {
                let l = t.shell(&liste)?.stdout;
                if !l.trim().is_empty() && !noms_paquets(&l).contains(p) {
                    return Ok(Desinstallation::DejaAbsente);
                }
                match t.uninstall(p) {
                    Ok(_) => Ok(Desinstallation::Faite),
                    Err(ErreurTransport::Commande { stderr, .. }) => Ok(Desinstallation::Echec(stderr)),
                    Err(e) => Err(e),
                }
            })?;
            match r {
                Desinstallation::Faite => rapport.desinstalles.push(p.clone()),
                Desinstallation::DejaAbsente => rapport.deja_absents.push(p.clone()),
                Desinstallation::Echec(m) => {
                    s.avertir(format!("échec de la désinstallation de {p} : {m}"));
                    rapport.echecs.push(p.clone());
                }
            }
        }
    }
    rapport.avertissements = s.avertissements;
    Ok(rapport)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::fake_box::FakeBox;
    use crate::moteur::inventaire::inventaire;
    use crate::moteur::session::Silencieux;
    use crate::moteur::transport::Transport;

    const TRACKER: &str = "tv.alphonso.alphonso_eula";
    const AUTRE: &str = "com.google.android.katniss";

    fn prete() -> (FakeBox, Inventaire, std::path::PathBuf) {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let f = FakeBox::mibox4();
        f.connecter().unwrap();
        let d = std::env::temp_dir().join(format!("zenhome-choix-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
        let inv = inventaire(&f, &d.join("sauvegarde"), "2026-10-29 10:00").unwrap();
        (f, inv, d)
    }

    fn choix() -> Choix {
        Choix {
            garder: vec![TRACKER.into(), AUTRE.into()],
            desactiver: vec!["com.mitv.tvhome.atv".into()],
            desinstaller: vec!["fr.tf1.mytf1".into(), "com.exemple.lecteur".into()],
        }
    }

    #[test]
    fn applique_garder_reactive_et_desinstalle() {
        let (f, inv, d) = prete();
        f.modifier(|e| e.paquets.get_mut(TRACKER).unwrap().desactive = true);
        let inv = Inventaire { desactives: vec![TRACKER.into()], ..inv };
        let r = appliquer(&f, &Politique::default(), &mut Silencieux, &inv, &choix(), &Reprise::default()).unwrap();
        let e = f.instantane();
        assert_eq!(e.global["zenhome_garder"], format!("{TRACKER},{AUTRE}"));
        assert!(!e.paquets[TRACKER].desactive);
        assert_eq!(r.reactives, [TRACKER]);
        assert_eq!(r.desinstalles, ["fr.tf1.mytf1", "com.exemple.lecteur"]);
        assert!(!e.paquets.contains_key("fr.tf1.mytf1") && !e.paquets.contains_key("com.exemple.lecteur"));
        assert!(e.paquets.contains_key("com.exemple.musique"));
        // la désactivation n'est pas faite ici
        assert!(!e.paquets["com.mitv.tvhome.atv"].desactive);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn liste_vide_supprime_la_cle() {
        let (f, inv, d) = prete();
        f.modifier(|e| {
            e.global.insert("zenhome_garder".into(), "a.b".into());
        });
        appliquer(&f, &Politique::default(), &mut Silencieux, &inv, &Choix::default(), &Reprise::default()).unwrap();
        assert!(!f.instantane().global.contains_key("zenhome_garder"));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn refus_protege_hors_liste_systeme_ou_inconnu() {
        let (f, inv, d) = prete();
        let avant = f.journal().len();
        for (nom, c) in [
            ("protégé", Choix { desinstaller: vec!["tv.zenhome".into()], ..Default::default() }),
            ("launcher", Choix { desinstaller: vec!["com.google.android.tvlauncher".into()], ..Default::default() }),
            ("setupwraith", Choix { desinstaller: vec!["com.google.android.tungsten.setupwraith".into()], ..Default::default() }),
            ("vending", Choix { desinstaller: vec!["com.android.vending".into()], ..Default::default() }),
            ("settings", Choix { desinstaller: vec!["com.android.tv.settings".into()], ..Default::default() }),
            ("système, hors -3", Choix { desinstaller: vec![AUTRE.into()], ..Default::default() }),
            ("absent", Choix { desinstaller: vec!["com.pas.installe".into()], ..Default::default() }),
            ("injection", Choix { desinstaller: vec!["fr.tf1.mytf1; reboot".into()], ..Default::default() }),
            ("garder hors liste", Choix { garder: vec!["com.google.android.gms".into()], ..Default::default() }),
            ("désactiver hors liste", Choix { desactiver: vec!["com.spotify.tv.android".into()], ..Default::default() }),
            ("garder absent de la box", Choix { garder: vec!["com.absent.dubox".into()], ..Default::default() }),
            ("conflit", Choix { garder: vec![AUTRE.into()], desactiver: vec![AUTRE.into()], ..Default::default() }),
        ] {
            let r = appliquer(&f, &Politique::default(), &mut Silencieux, &inv, &c, &Reprise::default());
            assert!(matches!(r, Err(Arret::Echec(ErreurAction::Refus(_)))), "{nom} : {r:?}");
        }
        assert_eq!(f.journal().len(), avant, "aucune commande envoyée");
        // un choix à moitié bon est refusé en entier
        let c = Choix { desinstaller: vec!["fr.tf1.mytf1".into(), "tv.zenhome".into()], ..Default::default() };
        assert!(appliquer(&f, &Politique::default(), &mut Silencieux, &inv, &c, &Reprise::default()).is_err());
        assert!(f.instantane().paquets.contains_key("fr.tf1.mytf1"));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn coupure_au_milieu_puis_reprise_sans_double_desinstallation() {
        let (f, inv, d) = prete();
        // la 1re désinstallation est exécutée mais sa réponse se perd : la reprise ne doit pas la rejouer en échec
        f.perdre_reponse_desinstallation(1);
        let r = appliquer(&f, &Politique::default(), &mut Silencieux, &inv, &choix(), &Reprise::default()).unwrap();
        assert_eq!(r.deja_absents, ["fr.tf1.mytf1"]);
        assert_eq!(r.desinstalles, ["com.exemple.lecteur"]);
        assert!(r.echecs.is_empty());
        let uninstalls = f.journal().iter().filter(|l| l.starts_with("uninstall:")).count();
        assert_eq!(uninstalls, 2, "une désinstallation par appli, aucune en double");
        assert_eq!(f.instantane().global["zenhome_garder"], format!("{TRACKER},{AUTRE}"));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn coupure_pendant_la_memorisation() {
        let (f, inv, d) = prete();
        // la coupure tombe entre l'écriture et la relecture : l'opération (écrire + relire) est rejouée
        f.couper_apres(1, 1);
        appliquer(&f, &Politique::default(), &mut Silencieux, &inv, &choix(), &Reprise::default()).unwrap();
        assert_eq!(f.instantane().global["zenhome_garder"], format!("{TRACKER},{AUTRE}"));
        let ecritures = f.journal().iter().filter(|l| l.contains("settings put global zenhome_garder")).count();
        assert_eq!(ecritures, 2, "écriture idempotente rejouée une fois");
        let _ = std::fs::remove_dir_all(d);
    }
}
