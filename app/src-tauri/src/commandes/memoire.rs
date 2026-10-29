//! Box déjà connectées, mémorisées dans le dossier de données (`boxes.json`) pour les retrouver tout de suite :
//! la recherche sonde d'abord ces IP, sans attendre le mDNS (lent sur une box en Wi-Fi économe).
//!
//! Le fichier est lu comme non fiable : taille, nombre et forme des entrées sont revalidés (série, IP locale,
//! nom déjà nettoyé) ; une entrée invalide fait ignorer tout le fichier. La connexion revérifie toujours modèle et série.

use std::path::Path;

use serde::{Deserialize, Serialize};
use zh_reseau::BoxTrouvee;

use super::persistance::{ecrire_atomique, serie_valide};

const FICHIER: &str = "boxes.json";
const TAILLE_MAX: u64 = 64 * 1024;
const BOXES_MAX: usize = 16;

#[derive(Serialize, Deserialize)]
struct Memorisee {
    serie: String,
    nom: String,
    ip: String,
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
                && !m.nom.is_empty()
                && zh_reseau::nettoyer_nom(&m.nom) == m.nom
                && entrees[..i].iter().all(|q| q.serie != m.serie)
        });
    if valide { entrees } else { vec![] }
}

/// Box mémorisées (la plus récente en dernier) ; vide si le fichier est absent ou invalide.
pub fn lire(racine: &Path) -> Vec<BoxTrouvee> {
    charger(racine).into_iter().map(|m| BoxTrouvee { nom: m.nom, ip: m.ip }).collect()
}

/// Nom mémorisé de cette série, s'il est autre chose que le nom par défaut.
pub fn nom(racine: &Path, serie: &str) -> Option<String> {
    charger(racine).into_iter().find(|m| m.serie == serie && m.nom != "Mi Box").map(|m| m.nom)
}

/// Mémorise la box (remplace l'entrée de même série ou de même IP ; la plus ancienne saute au-delà du plafond).
pub fn memoriser(racine: &Path, serie: &str, nom: &str, ip: &str) -> std::io::Result<()> {
    let mut entrees = charger(racine);
    entrees.retain(|m| m.serie != serie && m.ip != ip);
    entrees.push(Memorisee { serie: serie.into(), nom: nom.into(), ip: ip.into() });
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
        memoriser(&d, "A", "SALON", "192.168.50.14").unwrap();
        memoriser(&d, "B", "CHAMBRE", "192.168.50.15").unwrap();
        memoriser(&d, "A", "Salon bis", "192.168.50.20").unwrap();
        memoriser(&d, "C", "Autre", "192.168.50.15").unwrap();
        let l: Vec<_> = lire(&d).into_iter().map(|b| (b.nom, b.ip)).collect();
        assert_eq!(l, [("Salon bis".into(), "192.168.50.20".into()), ("Autre".into(), "192.168.50.15".into())]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn plafond() {
        let d = dossier("plafond");
        for n in 0..BOXES_MAX + 3 {
            memoriser(&d, &format!("S{n}"), "Box", &format!("10.0.0.{}", n + 1)).unwrap();
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
