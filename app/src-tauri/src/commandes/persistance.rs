//! Reprise conservée sur disque (dossier de données de l'appli) tant qu'une vérification n'est pas terminée :
//! les paquets désactivés par un passage interrompu, pour que le retour arrière survive à une fermeture.
//! Un seul fichier garde une entrée par série de box (plafonnée), pour que traiter une box n'écrase pas celle d'une autre.
//!
//! Le fichier est lu comme non fiable (il peut avoir été modifié) : taille, nombre et forme des entrées sont revalidés,
//! et seuls ces paquets reviennent dans une `Reprise` (l'étape repart toujours de zéro). Une entrée invalide fait
//! ignorer tout le fichier. L'ancien format (une seule box, objet sans liste) reste lisible.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::moteur::session::Reprise;
use crate::moteur::transport::paquet_valide;

const FICHIER: &str = "reprise.json";
const TAILLE_MAX: u64 = 256 * 1024;
const PAQUETS_MAX: usize = 1000;
const SERIES_MAX: usize = 32;

#[derive(Serialize, Deserialize)]
struct Persistee {
    serie: String,
    desactives: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Fichier {
    Liste(Vec<Persistee>),
    Ancien(Persistee),
}

fn chemin(racine: &Path) -> PathBuf {
    racine.join(FICHIER)
}

pub(super) fn serie_valide(s: &str) -> bool {
    (1..=40).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

/// Entrées du fichier ; vide s'il est absent, trop gros ou invalide (en entier).
fn charger(racine: &Path) -> Vec<Persistee> {
    let Ok(f) = std::fs::File::open(chemin(racine)) else { return vec![] };
    if f.metadata().map_or(true, |m| m.len() > TAILLE_MAX) {
        return vec![];
    }
    let entrees = match serde_json::from_reader(f) {
        Ok(Fichier::Liste(l)) => l,
        Ok(Fichier::Ancien(p)) => vec![p],
        Err(_) => return vec![],
    };
    let valide = entrees.len() <= SERIES_MAX
        && entrees.iter().enumerate().all(|(i, p)| {
            serie_valide(&p.serie)
                && !p.desactives.is_empty()
                && p.desactives.len() <= PAQUETS_MAX
                && p.desactives.iter().all(|d| paquet_valide(d))
                && entrees[..i].iter().all(|q| q.serie != p.serie)
        });
    if valide { entrees } else { vec![] }
}

/// Reprise en attente pour cette série, si le fichier est valide et en contient une.
pub fn lire(racine: &Path, serie: &str) -> Option<Reprise> {
    let p = charger(racine).into_iter().find(|p| p.serie == serie)?;
    Some(Reprise { etape: 0, desactives: p.desactives, serie: Some(p.serie) })
}

/// Au moins une série a une reprise en attente.
pub fn en_attente(racine: &Path) -> bool {
    !charger(racine).is_empty()
}

/// Écriture atomique (fichier temporaire puis renommage), lisible par l'utilisateur seul.
pub(super) fn ecrire_atomique(racine: &Path, fichier: &str, contenu: &[u8]) -> std::io::Result<()> {
    std::fs::create_dir_all(racine)?;
    let tmp = racine.join(format!(".{fichier}.tmp"));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut f = options.open(&tmp)?;
    f.write_all(contenu)?;
    f.sync_all()?;
    std::fs::rename(&tmp, racine.join(fichier))
}

fn sauver(racine: &Path, entrees: &[Persistee]) -> std::io::Result<()> {
    if entrees.is_empty() {
        let _ = std::fs::remove_file(chemin(racine));
        return Ok(());
    }
    let contenu = serde_json::to_vec(entrees).map_err(std::io::Error::other)?;
    ecrire_atomique(racine, FICHIER, &contenu)
}

/// Mémorise la reprise de sa série (les autres sont gardées, la plus ancienne saute au-delà du plafond) ; vide : l'efface.
pub fn ecrire(racine: &Path, reprise: &Reprise) -> std::io::Result<()> {
    let Some(serie) = reprise.serie.clone() else { return Ok(()) };
    let mut entrees = charger(racine);
    entrees.retain(|p| p.serie != serie);
    if !reprise.desactives.is_empty() {
        entrees.push(Persistee { serie, desactives: reprise.desactives.clone() });
        let trop = entrees.len().saturating_sub(SERIES_MAX);
        entrees.drain(..trop);
    }
    sauver(racine, &entrees)
}

pub fn effacer(racine: &Path, serie: &str) {
    let mut entrees = charger(racine);
    if entrees.iter().any(|p| p.serie == serie) {
        entrees.retain(|p| p.serie != serie);
        let _ = sauver(racine, &entrees);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dossier(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("zenhome-persist-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn reprise(serie: &str, p: &[&str]) -> Reprise {
        Reprise { etape: 2, desactives: p.iter().map(|x| x.to_string()).collect(), serie: Some(serie.into()) }
    }

    #[test]
    fn ecrite_puis_relue() {
        let d = dossier("aller-retour");
        let r = reprise("ABC-1", &["a.b", "c.d"]);
        ecrire(&d, &r).unwrap();
        assert_eq!(lire(&d, "ABC-1"), Some(Reprise { etape: 0, ..r }));
        assert_eq!(lire(&d, "AUTRE"), None);
        effacer(&d, "ABC-1");
        assert_eq!(lire(&d, "ABC-1"), None);
        assert!(!en_attente(&d) && !chemin(&d).exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn une_entree_par_serie() {
        let d = dossier("series");
        ecrire(&d, &reprise("A", &["a.a"])).unwrap();
        ecrire(&d, &reprise("B", &["b.b"])).unwrap();
        ecrire(&d, &reprise("A", &["a.a", "a.c"])).unwrap();
        assert_eq!(lire(&d, "A").unwrap().desactives, ["a.a", "a.c"]);
        assert_eq!(lire(&d, "B").unwrap().desactives, ["b.b"]);
        effacer(&d, "A");
        assert_eq!(lire(&d, "A"), None);
        assert!(en_attente(&d), "B reste en attente");
        ecrire(&d, &reprise("B", &[])).unwrap();
        assert!(!en_attente(&d));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn plafond_de_series() {
        let d = dossier("plafond");
        for n in 0..SERIES_MAX + 3 {
            ecrire(&d, &reprise(&format!("S{n}"), &["a.b"])).unwrap();
        }
        assert_eq!(charger(&d).len(), SERIES_MAX);
        assert_eq!(lire(&d, "S0"), None, "la plus ancienne a sauté");
        assert!(lire(&d, &format!("S{}", SERIES_MAX + 2)).is_some());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn ancien_format_lisible() {
        let d = dossier("ancien");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(chemin(&d), r#"{"serie":"X","desactives":["a.b"]}"#).unwrap();
        assert_eq!(lire(&d, "X").unwrap().desactives, ["a.b"]);
        ecrire(&d, &reprise("Y", &["c.d"])).unwrap();
        assert!(lire(&d, "X").is_some() && lire(&d, "Y").is_some(), "réécrit au nouveau format sans rien perdre");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn fichier_modifie_refuse() {
        let d = dossier("hostile");
        std::fs::create_dir_all(&d).unwrap();
        let trop = format!("[{}]", (0..=SERIES_MAX).map(|n| format!(r#"{{"serie":"S{n}","desactives":["a.b"]}}"#)).collect::<Vec<_>>().join(","));
        for contenu in [
            r#"{"serie":"X","desactives":["a.b; rm -rf /"]}"#,
            r#"{"serie":"X Y","desactives":["a.b"]}"#,
            r#"{"serie":"X","desactives":[]}"#,
            r#"[{"serie":"A","desactives":["a.b"]},{"serie":"B C","desactives":["a.b"]}]"#,
            r#"[{"serie":"A","desactives":["a.b"]},{"serie":"A","desactives":["a.b"]}]"#,
            r#"[{"serie":"A","desactives":["a.b"]},{"serie":"B","desactives":["x; y"]}]"#,
            &trop,
            "pas du json",
        ] {
            std::fs::write(chemin(&d), contenu).unwrap();
            assert_eq!(lire(&d, "A"), None, "{contenu}");
            assert!(!en_attente(&d), "{contenu}");
        }
        std::fs::write(chemin(&d), vec![b' '; (TAILLE_MAX + 1) as usize]).unwrap();
        assert!(!en_attente(&d));
        let _ = std::fs::remove_dir_all(&d);
    }
}
