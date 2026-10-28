//! Géocodage Open-Meteo : propositions de villes (lat/lon validées, noms nettoyés).

use crate::{Erreur, http};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const HOTE: &str = "geocoding-api.open-meteo.com";
const URL: &str = "https://geocoding-api.open-meteo.com/v1/search";
const DELAI: Duration = Duration::from_secs(15);
const REPONSE_MAX: u64 = 256 * 1024;
const PROPOSITIONS_MAX: usize = 5;
const NOM_MAX: usize = 60;
const SAISIE_MAX: usize = 100;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ville {
    pub nom: String,
    pub region: Option<String>,
    pub pays: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Deserialize)]
struct Reponse {
    results: Option<Vec<Brut>>,
}

#[derive(Deserialize)]
struct Brut {
    name: String,
    latitude: f64,
    longitude: f64,
    admin1: Option<String>,
    country: Option<String>,
}

/// Retire contrôles, séparateurs de ligne, caractères invisibles et marques bidirectionnelles,
/// puis borne la longueur (en caractères).
fn nettoyer(s: &str, max: usize) -> String {
    let net: String = s
        .chars()
        .filter(|c| {
            !c.is_control()
                && !matches!(*c, '\u{061C}' | '\u{200B}'..='\u{200F}' | '\u{2028}'..='\u{202E}' | '\u{2060}'..='\u{2069}' | '\u{FEFF}')
        })
        .take(max)
        .collect();
    net.trim().to_string()
}

fn optionnel(s: Option<String>) -> Option<String> {
    s.map(|v| nettoyer(&v, NOM_MAX)).filter(|v| !v.is_empty())
}

/// Analyse une réponse brute ; les entrées invalides sont ignorées, le JSON invalide est une erreur.
fn analyser(octets: &[u8]) -> Result<Vec<Ville>, Erreur> {
    let rep: Reponse = serde_json::from_slice(octets).map_err(|e| Erreur::ReponseInvalide(e.to_string()))?;
    Ok(rep
        .results
        .unwrap_or_default()
        .into_iter()
        .filter(|b| (-90.0..=90.0).contains(&b.latitude) && (-180.0..=180.0).contains(&b.longitude))
        .filter_map(|b| {
            let nom = nettoyer(&b.name, NOM_MAX);
            (!nom.is_empty()).then(|| Ville {
                nom,
                region: optionnel(b.admin1),
                pays: optionnel(b.country),
                latitude: b.latitude,
                longitude: b.longitude,
            })
        })
        .take(PROPOSITIONS_MAX)
        .collect())
}

/// Cherche jusqu'à 5 villes correspondant à la saisie (bloquant).
pub fn chercher_villes(saisie: &str) -> Result<Vec<Ville>, Erreur> {
    let ville = nettoyer(saisie, SAISIE_MAX);
    if ville.is_empty() {
        return Err(Erreur::VilleVide);
    }
    let corps = http::get(
        URL,
        &[HOTE],
        DELAI,
        REPONSE_MAX,
        |r| r.query("name", &ville).query("count", "5").query("language", "fr").query("format", "json"),
        &mut |_, _| {},
    )?;
    analyser(&corps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reponse_normale() {
        let j = br#"{"results":[
            {"id":1,"name":"Saint-Martin","latitude":45.5,"longitude":4.2,"admin1":"Loire","country":"France","extra":1},
            {"name":"Saint-Martin","latitude":18.07,"longitude":-63.05,"country":"Saint-Martin"}],"generationtime_ms":0.5}"#;
        let v = analyser(j).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].region.as_deref(), Some("Loire"));
        assert_eq!(v[1].region, None);
        assert_eq!(v[1].longitude, -63.05);
    }

    #[test]
    fn sans_resultat() {
        assert!(analyser(br#"{"generationtime_ms":0.2}"#).unwrap().is_empty());
    }

    #[test]
    fn json_invalide() {
        for j in [&b""[..], b"pas du json", b"[1,2]", br#"{"results":"x"}"#, br#"{"results":[{"name":"A"}]}"#, br#"{"results":[{"name":"A","latitude":"1","longitude":2}]}"#] {
            assert!(matches!(analyser(j), Err(Erreur::ReponseInvalide(_))), "{}", String::from_utf8_lossy(j));
        }
    }

    #[test]
    fn coordonnees_hors_bornes_ignorees() {
        let j = br#"{"results":[
            {"name":"A","latitude":90.1,"longitude":0},
            {"name":"B","latitude":0,"longitude":-180.5},
            {"name":"C","latitude":1e999,"longitude":0},
            {"name":"D","latitude":-90,"longitude":180}]}"#;
        // 1e999 est refusé par serde_json (infini) : JSON invalide, pas de panic.
        assert!(analyser(j).is_err());
        let j = br#"{"results":[
            {"name":"A","latitude":90.1,"longitude":0},
            {"name":"B","latitude":0,"longitude":-180.5},
            {"name":"D","latitude":-90,"longitude":180}]}"#;
        let v = analyser(j).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].nom, "D");
    }

    #[test]
    fn noms_nettoyes() {
        let j = serde_json::json!({"results":[
            {"name":"Pa\u{0}ri\ns\u{1b}[0m","latitude":1,"longitude":1,"admin1":"\u{202E}evil\u{2066}","country":"\u{200B}\n"},
            {"name":"\u{202E}\u{200F}","latitude":1,"longitude":1},
            {"name":"é".repeat(100),"latitude":1,"longitude":1}
        ]})
        .to_string();
        let v = analyser(j.as_bytes()).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].nom, "Paris[0m");
        assert_eq!(v[0].region.as_deref(), Some("evil"));
        assert_eq!(v[0].pays, None);
        assert_eq!(v[1].nom.chars().count(), 60);
    }

    #[test]
    fn cinq_propositions_au_plus() {
        let r: Vec<_> = (0..9).map(|i| serde_json::json!({"name":format!("V{i}"),"latitude":1,"longitude":1})).collect();
        let j = serde_json::json!({ "results": r }).to_string();
        assert_eq!(analyser(j.as_bytes()).unwrap().len(), 5);
    }

    #[test]
    fn saisie_vide_ou_controles() {
        assert!(matches!(chercher_villes(""), Err(Erreur::VilleVide)));
        assert!(matches!(chercher_villes(" \n\t\u{202E} "), Err(Erreur::VilleVide)));
    }

    #[test]
    fn reponse_trop_grosse() {
        let enorme = vec![b' '; REPONSE_MAX as usize + 1];
        assert!(matches!(http::lire_borne(&enorme[..], REPONSE_MAX, None, &mut |_, _| {}), Err(Erreur::TropGros)));
    }

    #[test]
    #[ignore = "contacte Open-Meteo"]
    fn reel_saint_martin() {
        let v = chercher_villes("Saint-Martin").unwrap();
        assert!(!v.is_empty() && v.len() <= 5);
    }
}
