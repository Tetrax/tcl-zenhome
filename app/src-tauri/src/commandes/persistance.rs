//! Reprise conservée sur disque (dossier de données de l'appli) tant qu'une vérification n'est pas terminée :
//! les paquets désactivés par un passage interrompu, pour que le retour arrière survive à une fermeture.
//!
//! Le fichier est lu comme non fiable (il peut avoir été modifié) : taille, nombre et forme des paquets sont revalidés,
//! et seuls ces paquets reviennent dans une `Reprise` (l'étape repart toujours de zéro).

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::moteur::session::Reprise;
use crate::moteur::transport::paquet_valide;

const FICHIER: &str = "reprise.json";
const TAILLE_MAX: u64 = 256 * 1024;
const PAQUETS_MAX: usize = 1000;

#[derive(Serialize, Deserialize)]
struct Persistee {
    serie: String,
    desactives: Vec<String>,
}

fn chemin(racine: &Path) -> PathBuf {
    racine.join(FICHIER)
}

fn serie_valide(s: &str) -> bool {
    (1..=40).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

/// Reprise en attente, si le fichier existe et est valide.
pub fn lire(racine: &Path) -> Option<Reprise> {
    let f = std::fs::File::open(chemin(racine)).ok()?;
    if f.metadata().ok()?.len() > TAILLE_MAX {
        return None;
    }
    let p: Persistee = serde_json::from_reader(f).ok()?;
    let valide = serie_valide(&p.serie) && p.desactives.len() <= PAQUETS_MAX && p.desactives.iter().all(|d| paquet_valide(d));
    (valide && !p.desactives.is_empty()).then_some(Reprise { etape: 0, desactives: p.desactives, serie: Some(p.serie) })
}

/// Écriture atomique (fichier temporaire puis renommage), lisible par l'utilisateur seul.
pub fn ecrire(racine: &Path, reprise: &Reprise) -> std::io::Result<()> {
    let Some(serie) = reprise.serie.clone() else { return Ok(()) };
    std::fs::create_dir_all(racine)?;
    let contenu = serde_json::to_vec(&Persistee { serie, desactives: reprise.desactives.clone() }).map_err(std::io::Error::other)?;
    let tmp = racine.join(format!(".{FICHIER}.tmp"));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut f = options.open(&tmp)?;
    f.write_all(&contenu)?;
    f.sync_all()?;
    std::fs::rename(&tmp, chemin(racine))
}

pub fn effacer(racine: &Path) {
    let _ = std::fs::remove_file(chemin(racine));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dossier(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("zenhome-persist-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn ecrite_puis_relue() {
        let d = dossier("aller-retour");
        let r = Reprise { etape: 2, desactives: vec!["a.b".into(), "c.d".into()], serie: Some("ABC-1".into()) };
        ecrire(&d, &r).unwrap();
        assert_eq!(lire(&d), Some(Reprise { etape: 0, ..r }));
        effacer(&d);
        assert_eq!(lire(&d), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn fichier_modifie_refuse() {
        let d = dossier("hostile");
        std::fs::create_dir_all(&d).unwrap();
        for contenu in [
            r#"{"serie":"X","desactives":["a.b; rm -rf /"]}"#,
            r#"{"serie":"X Y","desactives":["a.b"]}"#,
            r#"{"serie":"X","desactives":[]}"#,
            "pas du json",
        ] {
            std::fs::write(chemin(&d), contenu).unwrap();
            assert_eq!(lire(&d), None, "{contenu}");
        }
        std::fs::write(chemin(&d), vec![b' '; (TAILLE_MAX + 1) as usize]).unwrap();
        assert_eq!(lire(&d), None);
        let _ = std::fs::remove_dir_all(&d);
    }
}
