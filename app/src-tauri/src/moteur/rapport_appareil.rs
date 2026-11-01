//! Rapport d'appareil : ce qu'il faut savoir d'un appareil non reconnu pour l'ajouter plus tard.
//! Uniquement des lectures, toutes sur liste blanche (voir `composer`). Jamais le numéro de série (seulement « lisible ou
//! non »), l'IP, le nom Cast, le Wi-Fi ni le nom des applis de l'utilisateur (seulement leur nombre).

use std::collections::BTreeSet;

use super::transport::{Commande, Espace, ErreurTransport, OptionPm, Transport, assainir};

/// Propriétés lues (valeurs courtes, sans donnée personnelle).
const PROPRIETES: [&str; 9] = [
    "ro.product.name",
    "ro.product.model",
    "ro.product.brand",
    "ro.product.manufacturer",
    "ro.product.device",
    "ro.build.version.sdk",
    "ro.build.version.release",
    "ro.build.version.security_patch",
    "ro.build.characteristics",
];
/// Noms de paquets système listés au plus, par liste (les listes sont triées).
const PAQUETS_MAX: usize = 400;
/// Taille maximale du rapport (en caractères).
const TEXTE_MAX: usize = 16_000;

/// Une lecture : une commande refusée par la box (code non nul) donne « rien », une coupure remonte.
fn lire(t: &dyn Transport, c: &Commande) -> Result<String, ErreurTransport> {
    match t.shell(c) {
        Ok(s) => Ok(s.stdout),
        Err(ErreurTransport::Commande { .. }) => Ok(String::new()),
        Err(e) => Err(e),
    }
}

fn valeur(brut: &str) -> String {
    let v = assainir(brut);
    if v.is_empty() { "(vide)".into() } else { v }
}

/// Noms de paquets de `pm list packages` (`package:nom` par ligne), sans doublon, triés.
fn paquets(brut: &str) -> BTreeSet<String> {
    brut.lines().filter_map(|l| l.trim().strip_prefix("package:")).map(|p| assainir(p).replace(' ', "")).filter(|p| !p.is_empty()).collect()
}

fn liste(titre: &str, noms: &BTreeSet<String>) -> String {
    let mut s = format!("{titre} ({}) :\n", noms.len());
    s.extend(noms.iter().take(PAQUETS_MAX).map(|n| format!("{n}\n")));
    if noms.len() > PAQUETS_MAX {
        s.push_str("…\n");
    }
    s
}

/// Lit l'appareil connecté et rend le texte du rapport (sans en-tête : version et système sont ajoutés par l'appelant).
pub fn composer(t: &dyn Transport) -> Result<String, ErreurTransport> {
    let mut s = String::from("Rapport d'appareil\n\n");
    let mut sdk = None;
    for cle in PROPRIETES {
        let v = valeur(&lire(t, &Commande::getprop(cle)?)?);
        if cle == "ro.build.version.sdk" {
            sdk = v.parse::<u32>().ok();
        }
        s.push_str(&format!("{cle} : {v}\n"));
    }
    // le numéro de série n'est jamais recopié : on note seulement si la box le donne
    let serie = super::connexion::lire_serie(t);
    s.push_str(&format!("numéro de série : {}\n", if serie.is_ok() { "lisible" } else { "illisible" }));

    let systeme = paquets(&lire(t, &Commande::pm_list(&[OptionPm::Systeme]))?);
    // « désactivés » : seulement ceux du système (un paquet de l'utilisateur désactivé ne figure pas au rapport)
    let desactives: BTreeSet<String> = paquets(&lire(t, &Commande::pm_list(&[OptionPm::Desactives]))?).intersection(&systeme).cloned().collect();
    let tiers = paquets(&lire(t, &Commande::pm_list(&[OptionPm::Tiers]))?);
    s.push_str(&format!("applis de l'utilisateur : {}\n", tiers.len()));
    // un accueil, un rôle HOME ou un écran de veille peut être une appli de l'utilisateur : son nom n'est pas recopié
    let anonyme = |v: String| if tiers.iter().any(|p| v.contains(p.as_str())) { "(appli de l'utilisateur)".to_string() } else { v };

    let accueil = lire(t, &Commande::resoudre_accueil())?;
    s.push_str(&format!("accueil résolu : {}\n", anonyme(valeur(accueil.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("")))));
    if sdk.is_some_and(|v| v >= 29) {
        s.push_str(&format!("détenteur du rôle HOME : {}\n", anonyme(valeur(&lire(t, &Commande::role_home_detenteur())?))));
    }
    let veille = lire(t, &Commande::settings_get(Espace::Secure, "screensaver_components")?)?;
    s.push_str(&format!("screensaver_components : {}\n\n", anonyme(valeur(&veille))));

    s.push_str(&liste("Paquets système", &systeme));
    s.push('\n');
    s.push_str(&liste("Paquets système désactivés", &desactives));
    Ok(s.chars().take(TEXTE_MAX).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::fake_box::FakeBox;
    use crate::moteur::transport::Transport;

    fn connectee(f: FakeBox) -> FakeBox {
        f.connecter().unwrap();
        f
    }

    #[test]
    fn que_des_lectures_sur_liste_blanche() {
        for f in [connectee(FakeBox::mibox4()), connectee(FakeBox::tcl_g08())] {
            composer(&f).unwrap();
            let journal = f.journal();
            let shell: Vec<_> = journal.iter().filter_map(|l| l.strip_prefix("shell: ")).collect();
            assert!(!shell.is_empty());
            for c in &shell {
                assert!(
                    ["getprop ro.", "pm list packages", "settings get secure screensaver_components", "cmd package resolve-activity", "cmd role get-role-holders"]
                        .iter()
                        .any(|p| c.starts_with(p)),
                    "commande hors liste blanche : {c}"
                );
            }
            assert!(journal.iter().all(|l| l.starts_with("shell: ") || ["connect", "etat"].contains(&l.as_str())), "{journal:?}");
        }
    }

    #[test]
    fn ni_serie_ni_ip_ni_nom_ni_applis_perso() {
        let f = connectee(FakeBox::tcl_g08());
        f.modifier(|e| {
            e.serie = "SERIE-SECRETE-42".into();
            e.global.insert("device_name".into(), "Salon de Valentin".into());
        });
        let texte = composer(&f).unwrap();
        for interdit in ["SERIE-SECRETE-42", "Valentin", "192.168", "com.spotify.tv.android", "com.exemple.lecteur", "MyTF1"] {
            assert!(!texte.contains(interdit), "{interdit} dans le rapport");
        }
        assert!(texte.contains("numéro de série : lisible") && texte.contains("applis de l'utilisateur : 4"));
        assert!(texte.contains("ro.product.name : G08_4K_GB") && texte.contains("détenteur du rôle HOME : com.google.android.apps.tv.launcherx"));
        assert!(texte.contains("com.android.vending"));
        f.serie_illisible(true);
        assert!(composer(&f).unwrap().contains("numéro de série : illisible"));
    }

    #[test]
    fn accueil_role_et_veille_d_une_appli_de_l_utilisateur_sont_masques() {
        let f = connectee(FakeBox::tcl_g08());
        f.modifier(|e| {
            e.role_home = Some("com.exemple.lecteur".into());
            e.secure.insert("screensaver_components".into(), "com.exemple.lecteur/.Veille".into());
        });
        let texte = composer(&f).unwrap();
        assert!(!texte.contains("com.exemple.lecteur"), "{texte}");
        assert!(texte.contains("détenteur du rôle HOME : (appli de l'utilisateur)\nscreensaver_components : (appli de l'utilisateur)"), "{texte}");
    }

    #[test]
    fn android_9_sans_role_et_desactives_systeme_seulement() {
        let f = connectee(FakeBox::mibox4());
        f.modifier(|e| {
            e.paquets.get_mut("com.mitv.tvhome.atv").unwrap().desactive = true;
            e.paquets.get_mut("com.spotify.tv.android").unwrap().desactive = true;
        });
        let texte = composer(&f).unwrap();
        assert!(!texte.contains("rôle HOME"), "pas de rôle avant Android 10");
        assert!(texte.contains("Paquets système désactivés (1)") && !texte.contains("spotify"), "{texte}");
        assert!(f.journal().iter().all(|l| !l.contains("role")));
    }

    #[test]
    fn texte_borne_et_nettoye() {
        let f = connectee(FakeBox::mibox4());
        f.modifier(|e| {
            e.modele = format!("Modèle\u{7}{}", "x".repeat(1000));
            for i in 0..2000 {
                e.paquets.insert(format!("com.exemple.paquet{i}"), crate::moteur::fake_box::PaquetFake {
                    systeme: true, desactive: false, chemin: String::new(), octets: (0, 0, 0), derniere_utilisation: None, libelle: None,
                });
            }
        });
        let texte = composer(&f).unwrap();
        assert!(texte.chars().count() <= TEXTE_MAX && !texte.contains('\u{7}'));
        assert!(texte.lines().filter(|l| l.starts_with("com.exemple.paquet")).count() <= PAQUETS_MAX);
        assert!(texte.lines().all(|l| l.chars().count() <= 340), "ligne trop longue");
    }

    #[test]
    fn une_coupure_remonte() {
        let f = FakeBox::mibox4(); // pas connectée
        assert!(composer(&f).is_err());
    }
}
