//! Box déjà connectées, mémorisées dans le dossier de données (`boxes.json`) pour les retrouver tout de suite :
//! la recherche sonde d'abord ces IP, sans attendre le mDNS (lent sur une box en Wi-Fi économe).
//!
//! Le fichier est lu comme non fiable : taille, nombre et forme des entrées sont revalidés (série, IP locale,
//! nom déjà nettoyé) ; une entrée invalide fait ignorer tout le fichier. La connexion revérifie toujours modèle et série.

use std::path::Path;

use serde::{Deserialize, Serialize};
use zh_reseau::BoxTrouvee;

use crate::moteur::appareils::{MIBOX4, TOUS, par_id};

use super::persistance::{ecrire_atomique, serie_valide};

const FICHIER: &str = "boxes.json";
const TAILLE_MAX: u64 = 64 * 1024;
const BOXES_MAX: usize = 16;

#[derive(Serialize, Deserialize)]
struct Memorisee {
    serie: String,
    nom: String,
    ip: String,
    /// Identifiant de l'appareil (absent des anciens fichiers, qui ne connaissaient que la Mi Box).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    appareil: Option<String>,
}

fn charger(racine: &Path) -> Vec<Memorisee> {
    let Ok(f) = std::fs::File::open(racine.join(FICHIER)) else { return vec![] };
    if f.metadata().map_or(true, |m| m.len() > TAILLE_MAX) {
        return vec![];
    }
    let Ok(entrees) = serde_json::from_reader::<_, Vec<Memorisee>>(f) else { return vec![] };
    let valide = entrees.len() <= BOXES_MAX
        && entrees.iter().enumerate().all(|(i, m)| {
            serie_valide(&m.serie)
                && zh_reseau::valider_ipv4(&m.ip).is_some_and(|ip| ip.to_string() == m.ip)
                && m.appareil.as_deref().is_none_or(|a| par_id(a).is_some())
                && !m.nom.is_empty()
                && zh_reseau::nettoyer_nom(&m.nom) == m.nom
                && entrees[..i].iter().all(|q| q.serie != m.serie && q.ip != m.ip)
        });
    if valide { entrees } else { vec![] }
}

/// Box mémorisées (la plus récente en dernier) ; vide si le fichier est absent ou invalide.
pub fn lire(racine: &Path) -> Vec<BoxTrouvee> {
    charger(racine).into_iter().map(|m| BoxTrouvee { libelle: par_defaut(m.appareil.as_deref()).into(), nom: m.nom, ip: m.ip, appareil: m.appareil }).collect()
}

/// Nom par défaut de l'appareil (la Mi Box pour les anciennes entrées).
fn par_defaut(appareil: Option<&str>) -> &'static str {
    appareil.and_then(par_id).unwrap_or(&MIBOX4).nom_defaut
}

/// Nom de la box une fois sa série lue : celui mémorisé pour CETTE série, sinon `propose` (nom mDNS) s'il ne vient pas
/// de la mémoire, sinon `defaut` (nom par défaut de l'appareil). Le nom mémorisé d'une IP dont la série diffère (box
/// remplacée) n'est jamais repris.
pub fn nom_pour(racine: &Path, serie: &str, ip: &str, propose: &str, defaut: &str) -> String {
    let connues = charger(racine);
    let reel = |n: &str| !n.is_empty() && !TOUS.iter().any(|a| a.nom_defaut == n) && n != "Appareil à vérifier";
    if let Some(m) = connues.iter().find(|m| m.serie == serie && reel(&m.nom)) {
        return m.nom.clone();
    }
    let de_memoire = connues.iter().any(|m| m.ip == ip && m.nom == propose);
    if reel(propose) && !de_memoire { propose.into() } else { defaut.into() }
}

/// Mémorise la box (remplace l'entrée de même série ou de même IP ; la plus ancienne saute au-delà du plafond).
pub fn memoriser(racine: &Path, serie: &str, nom: &str, ip: &str, appareil: Option<&str>) -> std::io::Result<()> {
    let mut entrees = charger(racine);
    entrees.retain(|m| m.serie != serie && m.ip != ip);
    let nom = if nom == "Appareil à vérifier" { String::new() } else { zh_reseau::nettoyer_nom(nom) };
    let nom = if nom.is_empty() { par_defaut(appareil).into() } else { nom };
    entrees.push(Memorisee { serie: serie.into(), nom, ip: ip.into(), appareil: appareil.map(String::from) });
    let trop = entrees.len().saturating_sub(BOXES_MAX);
    entrees.drain(..trop);
    ecrire_atomique(racine, FICHIER, &serde_json::to_vec(&entrees).map_err(std::io::Error::other)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dossier(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("zenhome-memoire-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn memorisee_puis_relue_sans_doublon() {
        let d = dossier("aller-retour");
        assert!(lire(&d).is_empty());
        memoriser(&d, "A", "SALON", "192.168.50.14", None).unwrap();
        memoriser(&d, "B", "CHAMBRE", "192.168.50.15", None).unwrap();
        memoriser(&d, "A", "Salon bis", "192.168.50.20", None).unwrap();
        memoriser(&d, "C", "Autre", "192.168.50.15", None).unwrap();
        let l: Vec<_> = lire(&d).into_iter().map(|b| (b.nom, b.ip)).collect();
        assert_eq!(l, [("Salon bis".into(), "192.168.50.20".into()), ("Autre".into(), "192.168.50.15".into())]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn nom_par_defaut_memorise_lisible_et_nom_de_cette_serie_prioritaire() {
        let d = dossier("defaut");
        memoriser(&d, "A", "Appareil à vérifier", "10.0.0.1", None).unwrap();
        assert_eq!(lire(&d).len(), 1, "le fichier reste valide");
        assert_eq!(nom_pour(&d, "A", "10.0.0.1", "Appareil à vérifier", "Mi Box"), "Mi Box");
        memoriser(&d, "B", "SALON", "10.0.0.2", None).unwrap();
        assert_eq!(nom_pour(&d, "B", "10.0.0.2", "Mi Box", "Mi Box"), "SALON");
        assert_eq!(nom_pour(&d, "C", "10.0.0.2", "SALON", "Mi Box"), "Mi Box", "autre série : nom mémorisé non repris");
        assert_eq!(nom_pour(&d, "C", "10.0.0.2", "Chambre", "Mi Box"), "Chambre", "nom mDNS conservé");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn plafond() {
        let d = dossier("plafond");
        for n in 0..BOXES_MAX + 3 {
            memoriser(&d, &format!("S{n}"), "Box", &format!("10.0.0.{}", n + 1), None).unwrap();
        }
        let l = lire(&d);
        assert_eq!(l.len(), BOXES_MAX);
        assert_eq!(l[0].ip, "10.0.0.4", "les plus anciennes ont sauté");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn fichier_modifie_ignore() {
        let d = dossier("hostile");
        std::fs::create_dir_all(&d).unwrap();
        let trop = format!("[{}]", (0..=BOXES_MAX).map(|n| format!(r#"{{"serie":"S{n}","nom":"B","ip":"10.0.0.1"}}"#)).collect::<Vec<_>>().join(","));
        let bon = r#"{"serie":"A","nom":"Salon","ip":"192.168.1.20"}"#;
        for contenu in [
            r#"[{"serie":"A","nom":"Salon","ip":"8.8.8.8"}]"#,
            r#"[{"serie":"A","nom":"Salon","ip":"192.168.1.20:5555"}]"#,
            r#"[{"serie":"A","nom":"Salon; rm -rf /","ip":"192.168.1.20"}]"#,
            r#"[{"serie":"A","nom":"","ip":"192.168.1.20"}]"#,
            r#"[{"serie":"A B","nom":"Salon","ip":"192.168.1.20"}]"#,
            &format!("[{bon},{bon}]"),
            &format!("[{bon},{{\"serie\":\"B\",\"nom\":\"X\",\"ip\":\"1.1.1.1\"}}]"),
            &format!("[{bon},{{\"serie\":\"B\",\"nom\":\"X\",\"ip\":\"192.168.1.20\"}}]"),
            &trop,
            "pas du json",
        ] {
            std::fs::write(d.join(FICHIER), contenu).unwrap();
            assert!(lire(&d).is_empty(), "{contenu}");
        }
        std::fs::write(d.join(FICHIER), format!("[{bon}]")).unwrap();
        assert_eq!(lire(&d).len(), 1);
        std::fs::write(d.join(FICHIER), vec![b' '; (TAILLE_MAX + 1) as usize]).unwrap();
        assert!(lire(&d).is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }
}
