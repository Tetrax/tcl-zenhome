//! Téléchargement et installation d'adb (Android SDK Platform-Tools).
//!
//! Ordre imposé : téléchargement borné, vérification SHA-256, puis extraction en liste
//! blanche dans un dossier temporaire, enfin renommage atomique.

use crate::{Erreur, http};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;
use zip::ZipArchive;

const HOTES: &[&str] = &["dl.google.com"];
const DELAI: Duration = Duration::from_secs(600);
const ZIP_MAX: u64 = 40 * 1024 * 1024;
const FICHIER_MAX: u64 = 40 * 1024 * 1024;
const ENTREES_MAX: usize = 1000;
const DOSSIER: &str = "platform-tools";

/// Source figée d'une plateforme : URL, empreinte et fichiers autorisés (chemins dans l'archive).
pub(crate) struct Source {
    url: &'static str,
    sha256: &'static str,
    fichiers: &'static [&'static str],
    /// Nom de l'exécutable adb dans le dossier `platform-tools`.
    adb: &'static str,
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) const MACOS: Source = Source {
    url: "https://dl.google.com/android/repository/platform-tools_r37.0.1-darwin.zip",
    sha256: "ee39ad5967e95c2a07f04dbcbde96b1a0c916ba376096db5d2f498b7727a5d1d",
    fichiers: &["platform-tools/adb"],
    adb: "adb",
};

#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const WINDOWS: Source = Source {
    url: "https://dl.google.com/android/repository/platform-tools_r37.0.1-win.zip",
    sha256: "45f4d63113e895ebde0c90f194099a4676b6ac653bd28d54314a9e022bbc1a99",
    fichiers: &["platform-tools/adb.exe", "platform-tools/AdbWinApi.dll", "platform-tools/AdbWinUsbApi.dll"],
    adb: "adb.exe",
};

fn source_courante() -> Result<&'static Source, Erreur> {
    if cfg!(target_os = "macos") {
        Ok(&MACOS)
    } else if cfg!(windows) {
        Ok(&WINDOWS)
    } else {
        Err(Erreur::PlateformeNonSupportee)
    }
}

fn est_executable(m: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        m.permissions().mode() & 0o100 != 0
    }
    #[cfg(not(unix))]
    {
        let _ = m;
        true
    }
}

fn present(src: &Source, dossier: &Path) -> Option<PathBuf> {
    let base = dossier.join(DOSSIER);
    for f in src.fichiers {
        let nom = f.rsplit('/').next()?;
        let m = fs::symlink_metadata(base.join(nom)).ok()?;
        if !m.is_file() || (nom == src.adb && !est_executable(&m)) {
            return None;
        }
    }
    Some(base.join(src.adb))
}

/// Chemin d'adb s'il est installé dans `dossier` (fichiers réguliers, adb exécutable).
pub fn adb_installe(dossier: &Path) -> Option<PathBuf> {
    present(source_courante().ok()?, dossier)
}

/// Télécharge, vérifie et installe adb dans `dossier/platform-tools` (bloquant).
/// `progression` reçoit (octets reçus, total si connu). Renvoie le chemin d'adb.
pub fn installer_adb(dossier: &Path, mut progression: impl FnMut(u64, Option<u64>)) -> Result<PathBuf, Erreur> {
    let src = source_courante()?;
    let zip = http::get(src.url, HOTES, DELAI, ZIP_MAX, |r| r, &mut progression)?;
    installer_archive(src, &zip, src.sha256, dossier)
}

fn sha256_hex(octets: &[u8]) -> String {
    Sha256::digest(octets).iter().map(|b| format!("{b:02x}")).collect()
}

/// Nom d'entrée sûr : relatif, sans `..`, sans antislash, sans lettre de lecteur ni NUL.
fn nom_sur(nom: &str) -> bool {
    !nom.is_empty()
        && !nom.starts_with('/')
        && !nom.contains(['\\', '\0', ':'])
        && nom.split('/').all(|c| c != "..")
}

fn archive_invalide(msg: impl Into<String>) -> Erreur {
    Erreur::ArchiveInvalide(msg.into())
}

fn installer_archive(src: &Source, zip: &[u8], sha_attendu: &str, dossier: &Path) -> Result<PathBuf, Erreur> {
    if !sha256_hex(zip).eq_ignore_ascii_case(sha_attendu) {
        return Err(Erreur::EmpreinteInvalide);
    }
    fs::create_dir_all(dossier)?;
    let pid = std::process::id();
    let tmp = dossier.join(format!(".{DOSSIER}.tmp-{pid}"));
    let ancien = dossier.join(format!(".{DOSSIER}.old-{pid}"));
    let _ = fs::remove_dir_all(&tmp);
    let _ = fs::remove_dir_all(&ancien);
    fs::create_dir(&tmp)?;
    if let Err(e) = extraire(src, zip, &tmp) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(e);
    }
    let final_ = dossier.join(DOSSIER);
    let remplace = fs::symlink_metadata(&final_).is_ok();
    if remplace && let Err(e) = fs::rename(&final_, &ancien) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(e.into());
    }
    if let Err(e) = fs::rename(&tmp, &final_) {
        if remplace {
            let _ = fs::rename(&ancien, &final_);
        }
        let _ = fs::remove_dir_all(&tmp);
        return Err(e.into());
    }
    let _ = fs::remove_dir_all(&ancien);
    Ok(final_.join(src.adb))
}

/// Extrait uniquement les fichiers de la liste blanche, à plat dans `cible`, sous un nom
/// qui vient de la liste blanche (jamais de l'archive). Toute entrée dangereuse refuse l'archive.
fn extraire(src: &Source, zip: &[u8], cible: &Path) -> Result<(), Erreur> {
    let mut archive = ZipArchive::new(Cursor::new(zip)).map_err(|e| archive_invalide(e.to_string()))?;
    if archive.len() > ENTREES_MAX {
        return Err(archive_invalide("trop d'entrées"));
    }
    let mut vus = HashSet::new();
    for i in 0..archive.len() {
        let mut entree = archive.by_index(i).map_err(|e| archive_invalide(e.to_string()))?;
        let nom = entree.name().to_string();
        if !nom_sur(&nom) {
            return Err(archive_invalide("chemin d'entrée dangereux"));
        }
        if entree.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err(archive_invalide("lien symbolique"));
        }
        let Some(autorise) = src.fichiers.iter().find(|f| **f == nom) else {
            continue;
        };
        if !vus.insert(*autorise) {
            return Err(archive_invalide("entrée en double"));
        }
        if entree.size() > FICHIER_MAX {
            return Err(Erreur::TropGros);
        }
        let fichier = cible.join(autorise.rsplit('/').next().unwrap_or(autorise));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(if *autorise == "platform-tools/adb" { 0o755 } else { 0o644 });
        }
        let mut sortie = options.open(&fichier)?;
        let ecrits = io::copy(&mut (&mut entree).take(FICHIER_MAX + 1), &mut sortie)?;
        if ecrits > FICHIER_MAX {
            return Err(Erreur::TropGros);
        }
    }
    match src.fichiers.iter().find(|f| !vus.contains(*f)) {
        Some(f) => Err(archive_invalide(format!("{f} absent de l'archive"))),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::{ZipWriter, write::SimpleFileOptions};

    /// Construit un zip en mémoire ; `None` comme contenu = lien symbolique vers `/etc/passwd`.
    fn fabriquer(entrees: &[(&str, Option<&[u8]>)]) -> Vec<u8> {
        let mut z = ZipWriter::new(Cursor::new(Vec::new()));
        let o = SimpleFileOptions::default().unix_permissions(0o644);
        for (nom, contenu) in entrees {
            match contenu {
                Some(c) => {
                    z.start_file(*nom, o).unwrap();
                    z.write_all(c).unwrap();
                }
                None => z.add_symlink(*nom, "/etc/passwd", o).unwrap(),
            }
        }
        z.finish().unwrap().into_inner()
    }

    fn installer(src: &Source, zip: &[u8], dossier: &Path) -> Result<PathBuf, Erreur> {
        installer_archive(src, zip, &sha256_hex(zip), dossier)
    }

    fn vide(d: &Path) -> bool {
        fs::read_dir(d).unwrap().next().is_none()
    }

    const ADB: &[u8] = b"binaire adb";

    #[test]
    fn installe_la_liste_blanche_mac() {
        let d = tempfile::tempdir().unwrap();
        let zip = fabriquer(&[
            ("platform-tools/", Some(b"")),
            ("platform-tools/adb", Some(ADB)),
            ("platform-tools/fastboot", Some(b"x")),
            ("platform-tools/lib64/libc++.so", Some(b"x")),
        ]);
        let p = installer(&MACOS, &zip, d.path()).unwrap();
        assert_eq!(p, d.path().join("platform-tools/adb"));
        assert_eq!(fs::read(&p).unwrap(), ADB);
        let noms: Vec<_> = fs::read_dir(d.path().join("platform-tools")).unwrap().collect();
        assert_eq!(noms.len(), 1, "seul adb est extrait");
        assert_eq!(fs::read_dir(d.path()).unwrap().count(), 1, "aucun résidu temporaire");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o755);
        }
    }

    #[test]
    fn installe_la_liste_blanche_windows() {
        let d = tempfile::tempdir().unwrap();
        let zip = fabriquer(&[
            ("platform-tools/adb.exe", Some(b"exe")),
            ("platform-tools/AdbWinApi.dll", Some(b"d1")),
            ("platform-tools/AdbWinUsbApi.dll", Some(b"d2")),
            ("platform-tools/fastboot.exe", Some(b"x")),
        ]);
        installer(&WINDOWS, &zip, d.path()).unwrap();
        assert_eq!(fs::read_dir(d.path().join("platform-tools")).unwrap().count(), 3);
    }

    #[test]
    fn mauvaise_empreinte_rien_extrait() {
        let d = tempfile::tempdir().unwrap();
        let zip = fabriquer(&[("platform-tools/adb", Some(ADB))]);
        let r = installer_archive(&MACOS, &zip, &"0".repeat(64), d.path());
        assert!(matches!(r, Err(Erreur::EmpreinteInvalide)));
        assert!(vide(d.path()));
    }

    #[test]
    fn archives_malveillantes_refusees() {
        let cas: Vec<(&str, Vec<u8>)> = vec![
            ("traversée", fabriquer(&[("../../evil", Some(b"x")), ("platform-tools/adb", Some(ADB))])),
            ("traversée imbriquée", fabriquer(&[("platform-tools/../../evil", Some(b"x")), ("platform-tools/adb", Some(ADB))])),
            ("absolu", fabriquer(&[("/tmp/evil", Some(b"x")), ("platform-tools/adb", Some(ADB))])),
            ("antislash", fabriquer(&[("..\\evil", Some(b"x")), ("platform-tools/adb", Some(ADB))])),
            ("lecteur", fabriquer(&[("C:/evil", Some(b"x")), ("platform-tools/adb", Some(ADB))])),
            ("lien symbolique", fabriquer(&[("platform-tools/adb", None)])),
            ("lien hors liste", fabriquer(&[("platform-tools/lien", None), ("platform-tools/adb", Some(ADB))])),
            ("adb manquant", fabriquer(&[("platform-tools/fastboot", Some(b"x"))])),
            ("adb mal placé", fabriquer(&[("adb", Some(ADB)), ("autre/platform-tools/adb", Some(ADB))])),
            ("pas un zip", b"PK pas vraiment".to_vec()),
        ];
        for (nom, zip) in cas {
            let d = tempfile::tempdir().unwrap();
            let r = installer(&MACOS, &zip, d.path());
            assert!(matches!(r, Err(Erreur::ArchiveInvalide(_))), "{nom} : {r:?}");
            assert!(vide(d.path()), "{nom} : résidus");
            assert!(adb_installe_dans(&MACOS, d.path()).is_none(), "{nom}");
        }
    }

    #[test]
    fn windows_exige_les_trois_fichiers() {
        let d = tempfile::tempdir().unwrap();
        let zip = fabriquer(&[("platform-tools/adb.exe", Some(b"exe"))]);
        assert!(matches!(installer(&WINDOWS, &zip, d.path()), Err(Erreur::ArchiveInvalide(_))));
        assert!(vide(d.path()));
    }

    #[test]
    fn echec_conserve_l_installation_existante() {
        let d = tempfile::tempdir().unwrap();
        installer(&MACOS, &fabriquer(&[("platform-tools/adb", Some(b"v1"))]), d.path()).unwrap();
        let mauvais = fabriquer(&[("../evil", Some(b"x"))]);
        assert!(installer(&MACOS, &mauvais, d.path()).is_err());
        assert_eq!(fs::read(d.path().join("platform-tools/adb")).unwrap(), b"v1");
        installer(&MACOS, &fabriquer(&[("platform-tools/adb", Some(b"v2"))]), d.path()).unwrap();
        assert_eq!(fs::read(d.path().join("platform-tools/adb")).unwrap(), b"v2");
        assert_eq!(fs::read_dir(d.path()).unwrap().count(), 1);
    }

    fn adb_installe_dans(src: &Source, d: &Path) -> Option<PathBuf> {
        present(src, d)
    }

    #[test]
    fn adb_installe_reverifie() {
        let d = tempfile::tempdir().unwrap();
        assert!(adb_installe_dans(&MACOS, d.path()).is_none());
        installer(&MACOS, &fabriquer(&[("platform-tools/adb", Some(ADB))]), d.path()).unwrap();
        let p = adb_installe_dans(&MACOS, d.path()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
            assert!(adb_installe_dans(&MACOS, d.path()).is_none(), "non exécutable");
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
            fs::remove_file(&p).unwrap();
            std::os::unix::fs::symlink("/bin/ls", &p).unwrap();
            assert!(adb_installe_dans(&MACOS, d.path()).is_none(), "lien symbolique");
        }
        #[cfg(not(unix))]
        {
            fs::remove_file(&p).unwrap();
            assert!(adb_installe_dans(&MACOS, d.path()).is_none());
        }
    }

    #[test]
    fn constantes_coherentes() {
        for s in [&MACOS, &WINDOWS] {
            assert!(http::url_autorisee(s.url, HOTES).is_ok());
            assert_eq!(s.sha256.len(), 64);
            assert!(s.sha256.chars().all(|c| c.is_ascii_hexdigit()));
            assert!(s.fichiers.iter().all(|f| nom_sur(f)));
        }
    }

    #[test]
    #[ignore = "télécharge ~16 Mo depuis dl.google.com"]
    fn telechargement_reel() {
        let d = tempfile::tempdir().unwrap();
        let mut dernier = 0;
        let p = installer_adb(d.path(), |n, _| dernier = n).unwrap();
        assert!(dernier > 1_000_000);
        assert_eq!(adb_installe(d.path()), Some(p));
    }
}
