use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::{env, fs};

/// APK de Zen Home embarqué dans l'appli : copié depuis `launcher/out/ZenHome.apk` vers `resources/ZenHome.apk`
/// (ressource du bundle), et son empreinte SHA-256 est figée dans le binaire (vérifiée avant toute installation).
/// Si l'APK n'a pas été construit, le build continue avec un fichier vide : l'appli refusera d'installer Zen Home
/// et le dira clairement.
fn embarquer_apk() {
    let racine = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let source = racine.join("../../launcher/out/ZenHome.apk");
    let destination = racine.join("resources/ZenHome.apk");
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed={}", destination.display());
    fs::create_dir_all(destination.parent().expect("dossier parent")).expect("création de resources/");
    if source.is_file() {
        fs::copy(&source, &destination).expect("copie de ZenHome.apk");
    } else if !destination.is_file() {
        println!(
            "cargo:warning=launcher/out/ZenHome.apk est absent : construisez l'APK (launcher/) avant de livrer l'appli. \
             Sans lui, Zen Home ne pourra pas être installée sur la box."
        );
        fs::write(&destination, b"").expect("création de resources/ZenHome.apk");
    } else {
        println!("cargo:warning=launcher/out/ZenHome.apk est absent : l'APK déjà copié dans resources/ est conservé.");
    }
    let octets = fs::read(&destination).expect("lecture de resources/ZenHome.apk");
    let empreinte: String = if octets.is_empty() { String::new() } else { Sha256::digest(&octets).iter().map(|b| format!("{b:02x}")).collect() };
    let sortie = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("apk_empreinte.rs");
    fs::write(sortie, format!("/// SHA-256 de l'APK embarqué (vide si l'APK manquait au build).\npub const APK_SHA256: &str = \"{empreinte}\";\n"))
        .expect("écriture de apk_empreinte.rs");
}

fn main() {
    embarquer_apk();
    // Les commandes de l'appli sont déclarées ici : chacune exige une permission explicite
    // (allow-<nom>) dans capabilities/. Ajouter les nouvelles commandes à cette liste.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
        "ping",
        "etat_initial",
        "installer_adb",
        "chercher_box",
        "connecter",
        "chercher_villes",
        "inventaire",
        "installer",
        "appliquer",
        "verifier",
        "restaurer",
        "decider",
        "annuler",
        "couper_debogage",
        "ouvrir_sauvegardes",
        "rapport_texte",
        "envoyer_rapport",
        "desinstaller_mac",
    ])))
    .expect("échec de tauri-build");
}
