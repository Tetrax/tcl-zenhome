fn main() {
    // Les commandes de l'appli sont déclarées ici : chacune exige une permission explicite
    // (allow-<nom>) dans capabilities/. Ajouter les nouvelles commandes à cette liste.
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&["ping", "test_reseau"])),
    )
    .expect("échec de tauri-build");
}
