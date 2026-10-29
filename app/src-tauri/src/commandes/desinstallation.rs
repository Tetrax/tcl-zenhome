//! « Désinstaller Zen Home de ce Mac » : copie facultative des sauvegardes, suppression du dossier de données de
//! l'appli (avec garde stricte) et mise à la Corbeille du bundle. Chaque étape est une fonction à part, testable
//! sur des dossiers temporaires ; `Coeur::desinstaller_mac` les enchaîne.

use std::io;
use std::path::{Path, PathBuf};

/// Identifiant de l'appli (tauri.conf.json) : son dossier de données est `<données utilisateur>/<identifiant>`.
pub const IDENTIFIANT: &str = "fr.tetrax.zenhome";

/// Crée un dossier NEUF `Zen Home – sauvegardes <horodatage>` dans `documents` (suffixe ` (2)`, ` (3)`… s'il existe
/// déjà) : `create_dir` échoue si le chemin existe, rien n'est donc jamais écrasé.
pub fn creer_dossier_copie(documents: &Path, horodatage: &str) -> io::Result<PathBuf> {
    let base = format!("Zen Home – sauvegardes {horodatage}");
    for n in 1..100 {
        let nom = if n == 1 { base.clone() } else { format!("{base} ({n})") };
        let chemin = documents.join(nom);
        match std::fs::create_dir(&chemin) {
            Ok(()) => return Ok(chemin),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(io::ErrorKind::AlreadyExists, "trop de copies existantes"))
}

/// Copie récursive de `source` dans `destination` (déjà créé). Fichiers et dossiers ordinaires seulement : les liens
/// symboliques ne sont jamais suivis. Les droits du dossier d'origine (privé) sont conservés.
pub fn copier_contenu(source: &Path, destination: &Path) -> io::Result<()> {
    for e in std::fs::read_dir(source)? {
        let e = e?;
        let vers = destination.join(e.file_name());
        let type_ = e.file_type()?;
        if type_.is_dir() {
            std::fs::create_dir(&vers)?;
            copier_contenu(&e.path(), &vers)?;
        } else if type_.is_file() {
            std::fs::copy(e.path(), &vers)?;
        }
    }
    std::fs::set_permissions(destination, std::fs::metadata(source)?.permissions())
}

/// Supprime le dossier de données, mais seulement s'il s'agit EXACTEMENT de `<parent>/fr.tetrax.zenhome` : un vrai
/// dossier (pas un lien symbolique), dont le chemin canonique est celui attendu. Absent : rien à faire.
/// `remove_dir_all` ne suit pas les liens symboliques rencontrés à l'intérieur.
pub fn supprimer_donnees(donnees: &Path, parent: &Path) -> Result<(), String> {
    let meta = match std::fs::symlink_metadata(donnees) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("dossier de données inaccessible : {e}")),
    };
    if !meta.is_dir() {
        return Err("le dossier de données n'est pas un dossier ordinaire".into());
    }
    let reel = donnees.canonicalize().map_err(|e| format!("dossier de données inaccessible : {e}"))?;
    let attendu = parent.join(IDENTIFIANT).canonicalize().map_err(|e| format!("dossier de données attendu inaccessible : {e}"))?;
    if reel != attendu {
        return Err("ce dossier n'est pas le dossier de données de Zen Home".into());
    }
    std::fs::remove_dir_all(&reel).map_err(|e| format!("suppression impossible : {e}"))
}

/// `X.app` si `exe` est bien `X.app/Contents/MacOS/<exécutable>`.
pub fn bundle_depuis(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    let nom = |p: &Path| p.file_name().and_then(|n| n.to_str()).map(str::to_owned);
    (nom(macos).as_deref() == Some("MacOS") && nom(contents).as_deref() == Some("Contents") && bundle.extension().is_some_and(|e| e == "app"))
        .then(|| bundle.to_path_buf())
}

/// Bundle de l'appli en cours, en release seulement (en développement, l'exécutable n'est pas dans un bundle).
pub fn bundle_courant() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        return None;
    }
    bundle_depuis(&std::env::current_exe().ok()?.canonicalize().ok()?)
}

/// Met le bundle à la Corbeille (NSFileManager : pas de suppression définitive, pas d'Apple Events).
#[cfg(target_os = "macos")]
pub fn mettre_a_la_corbeille(bundle: &Path) -> Result<(), String> {
    use objc2_foundation::{NSFileManager, NSString, NSURL};
    let chemin = NSString::from_str(bundle.to_str().ok_or("chemin illisible")?);
    let url = NSURL::fileURLWithPath_isDirectory(&chemin, true);
    NSFileManager::defaultManager()
        .trashItemAtURL_resultingItemURL_error(&url, None)
        .map_err(|e| e.localizedDescription().to_string().chars().take(200).collect::<String>())
}

#[cfg(not(target_os = "macos"))]
pub fn mettre_a_la_corbeille(_bundle: &Path) -> Result<(), String> {
    Err("la mise à la Corbeille n'est prise en charge que sur macOS".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::origine::tests::dossier_temp;

    fn donnees_factices(parent: &Path) -> PathBuf {
        let d = parent.join(IDENTIFIANT);
        std::fs::create_dir_all(d.join("sauvegardes/20261029-083553-S1")).unwrap();
        std::fs::write(d.join("sauvegardes/20261029-083553-S1/accueil.txt"), "x").unwrap();
        std::fs::write(d.join("reprise.json"), "{}").unwrap();
        d
    }

    #[test]
    fn supprime_exactement_le_dossier_de_donnees() {
        let parent = dossier_temp("garde-ok");
        let d = donnees_factices(&parent);
        let voisin = parent.join("autre");
        std::fs::create_dir(&voisin).unwrap();
        supprimer_donnees(&d, &parent).unwrap();
        assert!(!d.exists() && voisin.is_dir() && parent.is_dir());
        supprimer_donnees(&d, &parent).unwrap(); // déjà absent : sans effet
    }

    #[test]
    fn refuse_un_chemin_hors_du_dossier_attendu() {
        let parent = dossier_temp("garde-hors");
        donnees_factices(&parent);
        let autre = parent.join("autre");
        std::fs::create_dir_all(autre.join("sauvegardes")).unwrap();
        for interdit in [&autre, &parent, Path::new("/"), &parent.join(IDENTIFIANT).join("sauvegardes"), &parent.join(IDENTIFIANT).join("..")] {
            assert!(supprimer_donnees(interdit, &parent).is_err(), "{interdit:?}");
        }
        assert!(autre.join("sauvegardes").is_dir() && parent.join(IDENTIFIANT).join("reprise.json").is_file());
        // un parent qui n'a pas de dossier de l'appli ne valide rien non plus
        let vide = dossier_temp("garde-vide");
        assert!(supprimer_donnees(&autre, &vide).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn refuse_un_lien_symbolique_et_ne_le_suit_pas() {
        let parent = dossier_temp("garde-lien");
        let reel = dossier_temp("garde-lien-cible");
        std::fs::write(reel.join("precieux.txt"), "x").unwrap();
        // le dossier de données est lui-même un lien vers un autre dossier
        std::os::unix::fs::symlink(&reel, parent.join(IDENTIFIANT)).unwrap();
        assert!(supprimer_donnees(&parent.join(IDENTIFIANT), &parent).is_err());
        assert!(reel.join("precieux.txt").is_file());

        // un lien à l'intérieur est retiré sans que sa cible soit touchée
        let parent = dossier_temp("garde-lien-interne");
        let d = donnees_factices(&parent);
        std::os::unix::fs::symlink(&reel, d.join("lien")).unwrap();
        supprimer_donnees(&d, &parent).unwrap();
        assert!(!d.exists() && reel.join("precieux.txt").is_file());
    }

    #[test]
    fn dossier_de_copie_jamais_ecrase() {
        let documents = dossier_temp("copie-nom");
        let a = creer_dossier_copie(&documents, "2026-10-29 08h35").unwrap();
        std::fs::write(a.join("garde.txt"), "x").unwrap();
        let b = creer_dossier_copie(&documents, "2026-10-29 08h35").unwrap();
        assert_eq!(a.file_name().unwrap(), "Zen Home – sauvegardes 2026-10-29 08h35");
        assert_eq!(b.file_name().unwrap(), "Zen Home – sauvegardes 2026-10-29 08h35 (2)");
        assert!(a.join("garde.txt").is_file());
        assert!(creer_dossier_copie(&documents.join("absent"), "x").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn copie_recursive_sans_suivre_les_liens() {
        use std::os::unix::fs::PermissionsExt;
        let parent = dossier_temp("copie");
        let d = donnees_factices(&parent);
        let source = d.join("sauvegardes");
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o700)).unwrap();
        let hors = dossier_temp("copie-hors");
        std::fs::write(hors.join("secret.txt"), "x").unwrap();
        std::os::unix::fs::symlink(&hors, source.join("lien")).unwrap();
        let dest = creer_dossier_copie(&dossier_temp("copie-docs"), "t").unwrap();
        copier_contenu(&source, &dest).unwrap();
        assert_eq!(std::fs::read_to_string(dest.join("20261029-083553-S1/accueil.txt")).unwrap(), "x");
        assert!(!dest.join("lien").exists(), "lien ignoré");
        assert_eq!(std::fs::metadata(&dest).unwrap().permissions().mode() & 0o777, 0o700);
    }

    #[test]
    fn bundle_reconnu_seulement_dans_un_app() {
        assert_eq!(bundle_depuis(Path::new("/Applications/Zen Home.app/Contents/MacOS/zen-home")), Some(PathBuf::from("/Applications/Zen Home.app")));
        for non in ["/usr/bin/zen-home", "/x/Zen Home.app/Contents/Resources/zen-home", "/x/dossier/Contents/MacOS/zen-home", "/zen-home", "zen-home"] {
            assert_eq!(bundle_depuis(Path::new(non)), None, "{non}");
        }
        assert_eq!(bundle_courant(), None, "jamais en test (build debug)");
    }
}
