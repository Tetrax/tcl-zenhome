//! Rapport à envoyer par e-mail (lien `mailto:` vers une adresse fixe) ou à copier.
//!
//! L'interface fournit l'erreur et le résumé de la dernière action (textes déjà nettoyés par le cœur) ; ce module les
//! filtre encore, ajoute la version et le système, borne le corps et construit lui-même le lien : l'interface ne choisit
//! jamais l'URL. Aucun secret n'a de raison d'y figurer ; par prudence, toute longue suite de lettres, chiffres, `_` et `-`
//! (un jeton, une clé) est masquée.

use crate::commandes::erreur::ErreurCommande;

pub const ADRESSE: &str = "hebertval34@gmail.com";
const SUJET: &str = "Zen Home : rapport";
/// Longueur maximale du corps une fois encodé (au-delà, les clients mail tronquent le lien).
const CORPS_MAX: usize = 1500;
const ENTREE_MAX: usize = 20_000;
const JETON_MIN: usize = 24;
const PIED: &str = "\n\nRapport complet : dossier des sauvegardes de Zen Home (bouton « Ouvrir les sauvegardes »).";

/// Texte sans caractère de contrôle (sauf saut de ligne), borné (la fin est gardée), avec les longues suites « jeton » masquées.
fn nettoyer(brut: &str) -> String {
    let mut sortie = String::new();
    let mut suite = String::new();
    let vider = |sortie: &mut String, suite: &mut String| {
        sortie.push_str(if suite.chars().count() >= JETON_MIN || suite.starts_with("eyJ") { "[masqué]" } else { suite });
        suite.clear();
    };
    for c in brut.chars().skip(brut.chars().count().saturating_sub(ENTREE_MAX)) { // on garde la fin
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            suite.push(c);
            continue;
        }
        vider(&mut sortie, &mut suite);
        if c == '\n' || !c.is_control() {
            sortie.push(c);
        }
    }
    vider(&mut sortie, &mut suite);
    sortie
}

/// Version de macOS (ou nom du système ailleurs), pour le rapport.
pub fn systeme() -> String {
    let version = if cfg!(target_os = "macos") {
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).chars().filter(|c| c.is_ascii_digit() || *c == '.').take(20).collect::<String>())
            .filter(|v| !v.is_empty())
    } else {
        None
    };
    match version {
        Some(v) => format!("macOS {v} ({})", std::env::consts::ARCH),
        None => format!("{} ({})", std::env::consts::OS, std::env::consts::ARCH),
    }
}

fn entete(version: &str, systeme: &str, erreur: &str) -> String {
    let erreur = nettoyer(erreur);
    let erreur = erreur.trim().chars().take(500).collect::<String>();
    let mut s = format!("Zen Home {}\nSystème : {}\n", nettoyer(version), nettoyer(systeme));
    if !erreur.is_empty() {
        s.push_str(&format!("Erreur : {erreur}\n"));
    }
    s
}

/// Rapport complet (pour « Copier le rapport »).
pub fn complet(version: &str, systeme: &str, erreur: &str, rapport: &str) -> String {
    format!("{}\n{}\n", entete(version, systeme, erreur), nettoyer(rapport).trim())
}

/// Encodage pour un lien `mailto:` : tout sauf les caractères non réservés, octet par octet ; saut de ligne en CRLF.
fn encoder(texte: &str) -> String {
    let mut s = String::new();
    for c in texte.chars() {
        match c {
            c if c.is_ascii_alphanumeric() || "-._~".contains(c) => s.push(c),
            '\n' => s.push_str("%0D%0A"),
            c => c.to_string().bytes().for_each(|b| s.push_str(&format!("%{b:02X}"))),
        }
    }
    s
}

/// Corps du mail : en-tête et erreur toujours, puis la fin du rapport dans ce qui reste de la place.
pub fn corps_mail(version: &str, systeme: &str, erreur: &str, rapport: &str) -> String {
    let tete = entete(version, systeme, erreur);
    let place = CORPS_MAX.saturating_sub(encoder(&tete).len() + encoder(PIED).len() + encoder("…\n").len());
    let rapport = nettoyer(rapport);
    let rapport = rapport.trim();
    // la fin du rapport, au plus `place` caractères une fois encodée
    let mut utilise = 0;
    let garde = rapport.char_indices().rev().take_while(|&(_, c)| {
        utilise += encoder(c.encode_utf8(&mut [0; 4])).len();
        utilise <= place
    });
    let debut = garde.last().map_or(rapport.len(), |(i, _)| i);
    let mut fin = &rapport[debut..];
    let tronque = debut > 0;
    if tronque {
        // on repart d'un début de ligne quand c'est possible
        fin = fin.split_once('\n').map_or(fin, |(_, reste)| reste);
    }
    format!("{tete}\n{}{fin}{PIED}", if tronque { "…\n" } else { "" })
}

/// Lien `mailto:` complet (adresse et sujet fixes, corps encodé).
pub fn mailto(corps: &str) -> String {
    format!("mailto:{ADRESSE}?subject={}&body={}", encoder(SUJET), encoder(corps))
}

/// Ouvre le lien avec le programme du système (même principe que l'ouverture du dossier des sauvegardes).
/// Refuse tout ce qui n'est pas un lien `mailto:` vers l'adresse fixe.
pub fn ouvrir(url: &str) -> Result<(), ErreurCommande> {
    let echec = || ErreurCommande::interne("Impossible d'ouvrir votre messagerie : utilisez « Copier le rapport » et collez-le dans un message.");
    if !url.starts_with(&format!("mailto:{ADRESSE}?")) {
        return Err(echec());
    }
    let programme = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let statut = std::process::Command::new(programme).arg(url).status().map_err(|_| echec())?;
    // `explorer` rend 1 même quand il réussit.
    if !statut.success() && !cfg!(windows) {
        return Err(echec());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const JETON: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijklmnopqrstuvwxyz0123456789ABCD";

    #[test]
    fn jetons_masques_noms_de_paquets_gardes() {
        let t = nettoyer(&format!("clé {JETON} et 0123456789abcdef0123456789abcdef ; com.google.android.tvrecommendations OK"));
        assert!(!t.contains("eyJ") && !t.contains("0123456789abcdef") && !t.contains("abcdefghijklmnop"), "{t}");
        assert!(t.contains("com.google.android.tvrecommendations") && t.contains("[masqué]"), "{t}");
        assert_eq!(nettoyer("a\u{0}b\tc\nd"), "abc\nd");
    }

    #[test]
    fn corps_borne_avec_fin_du_rapport_et_sans_jeton() {
        let rapport: String = (0..400).map(|i| format!("ligne {i} éèà {JETON}\n")).collect();
        let corps = corps_mail("0.3.0", "macOS 15.1 (aarch64)", "interne : la box ne répond pas", &rapport);
        assert!(encoder(&corps).len() <= CORPS_MAX, "{}", encoder(&corps).len());
        assert!(corps.starts_with("Zen Home 0.3.0\nSystème : macOS 15.1 (aarch64)\nErreur : interne : la box ne répond pas\n"));
        assert!(corps.contains("ligne 399 ") && !corps.contains("ligne 0 "), "la fin est gardée : {corps}");
        assert!(corps.contains("…\n") && corps.ends_with(PIED));
        assert!(!corps.contains("eyJ") && !corps.contains("abcdefghijklmnop"));
        assert!(!mailto(&corps).contains("eyJ"));
    }

    #[test]
    fn rapport_court_garde_en_entier() {
        let corps = corps_mail("0.3.0", "linux", "", "OK : tout est en ordre");
        assert!(corps.contains("OK : tout est en ordre") && !corps.contains('…') && !corps.contains("Erreur"));
        assert!(complet("0.3.0", "linux", "x : y", "reste").contains("Erreur : x : y"));
    }

    #[test]
    fn lien_fixe_et_caracteres_encodes() {
        let url = mailto("é & = ? #\nà 日");
        assert!(url.starts_with("mailto:hebertval34@gmail.com?subject=Zen%20Home%20%3A%20rapport&body="), "{url}");
        let corps = &url[url.find("body=").unwrap() + 5..];
        assert_eq!(corps, "%C3%A9%20%26%20%3D%20%3F%20%23%0D%0A%C3%A0%20%E6%97%A5");
        assert!(url.is_ascii() && !url.contains(' ') && !url.contains('\n'));
    }

    #[test]
    fn seul_le_mailto_fixe_est_ouvert() {
        for url in ["https://exemple.fr", "mailto:autre@exemple.fr?body=x", "file:///etc/passwd", "-a Calculator", ""] {
            assert_eq!(ouvrir(url).unwrap_err().code, "interne", "{url}");
        }
    }
}
