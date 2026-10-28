//! Client HTTP commun : HTTPS seul, hôtes figés, aucune redirection, taille et délai bornés.

use crate::Erreur;
use std::io::Read;
use std::time::Duration;

/// Vérifie qu'une URL est en HTTPS, sans identifiants, vers l'un des hôtes autorisés.
pub(crate) fn url_autorisee(url: &str, hotes: &[&str]) -> Result<(), Erreur> {
    let refus = || Erreur::UrlRefusee(url.chars().take(80).collect());
    let reste = url.strip_prefix("https://").ok_or_else(refus)?;
    let autorite = reste.split(['/', '?', '#']).next().unwrap_or("");
    if hotes.contains(&autorite) { Ok(()) } else { Err(refus()) }
}

/// GET borné. `requete` ajoute les paramètres ; renvoie le corps (≤ `max` octets).
pub(crate) fn get(
    url: &str,
    hotes: &[&str],
    delai: Duration,
    max: u64,
    requete: impl FnOnce(ureq::RequestBuilder<ureq::typestate::WithoutBody>) -> ureq::RequestBuilder<ureq::typestate::WithoutBody>,
    progression: &mut dyn FnMut(u64, Option<u64>),
) -> Result<Vec<u8>, Erreur> {
    url_autorisee(url, hotes)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_global(Some(delai))
        .build()
        .into();
    let mut rep = requete(agent.get(url)).call().map_err(|e| Erreur::Reseau(e.to_string()))?;
    if rep.status().as_u16() != 200 {
        return Err(Erreur::StatutHttp(rep.status().as_u16()));
    }
    let total = rep.body().content_length();
    if total.is_some_and(|t| t > max) {
        return Err(Erreur::TropGros);
    }
    lire_borne(rep.body_mut().as_reader(), max, total, progression)
}

/// Lit au plus `max` octets ; échoue si le flux en contient davantage.
pub(crate) fn lire_borne(
    mut flux: impl Read,
    max: u64,
    total: Option<u64>,
    progression: &mut dyn FnMut(u64, Option<u64>),
) -> Result<Vec<u8>, Erreur> {
    let mut sortie = Vec::new();
    let mut tampon = [0u8; 64 * 1024];
    loop {
        let n = flux.read(&mut tampon).map_err(|e| Erreur::Reseau(e.to_string()))?;
        if n == 0 {
            return Ok(sortie);
        }
        if sortie.len() as u64 + n as u64 > max {
            return Err(Erreur::TropGros);
        }
        sortie.extend_from_slice(&tampon[..n]);
        progression(sortie.len() as u64, total);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_refusees() {
        let h = ["dl.google.com"];
        assert!(url_autorisee("https://dl.google.com/android/x.zip", &h).is_ok());
        for u in [
            "http://dl.google.com/x",
            "https://evil.com/x",
            "https://dl.google.com.evil.com/x",
            "https://user@dl.google.com/x",
            "https://dl.google.com@evil.com/x",
            "https://dl.google.com:444/x",
            "ftp://dl.google.com/x",
            "",
        ] {
            assert!(url_autorisee(u, &h).is_err(), "{u}");
        }
    }

    #[test]
    fn lecture_bornee() {
        let donnees = vec![7u8; 1000];
        let mut vu = 0;
        let ok = lire_borne(&donnees[..], 1000, Some(1000), &mut |n, _| vu = n).unwrap();
        assert_eq!((ok.len(), vu), (1000, 1000));
        assert!(matches!(lire_borne(&donnees[..], 999, None, &mut |_, _| {}), Err(Erreur::TropGros)));
    }
}
