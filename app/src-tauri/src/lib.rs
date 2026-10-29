// Moteur public : les tests d'intégration (tests/parite) le pilotent.
pub mod commandes;
pub mod moteur;

use serde::Serialize;
use tauri::Manager;

#[derive(Serialize, Debug, PartialEq)]
struct PingInfo {
    version: String,
    os: &'static str,
    arch: &'static str,
}

fn ping_info(version: &str) -> PingInfo {
    PingInfo {
        version: version.to_string(),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
    }
}

#[tauri::command]
fn ping(app: tauri::AppHandle) -> PingInfo {
    ping_info(&app.package_info().version.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(commandes::preparer)
        .invoke_handler(tauri::generate_handler![
            ping,
            commandes::etat_initial,
            commandes::installer_adb,
            commandes::chercher_box,
            commandes::connecter,
            commandes::chercher_villes,
            commandes::inventaire,
            commandes::installer,
            commandes::appliquer,
            commandes::verifier,
            commandes::restaurer,
            commandes::decider,
            commandes::annuler,
            commandes::couper_debogage,
            commandes::ouvrir_sauvegardes,
            commandes::desinstaller_mac,
        ])
        .build(tauri::generate_context!())
        .expect("erreur au lancement de Zen Home")
        .run(|app, evenement| {
            // Fin de session adb à la sortie : déconnexion et arrêt du serveur adb dédié.
            if let (tauri::RunEvent::Exit, Some(coeur)) = (evenement, app.try_state::<std::sync::Arc<commandes::Coeur>>()) {
                coeur.fermer();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_renvoie_version_os_arch() {
        let p = ping_info("0.1.0");
        assert_eq!(p.version, "0.1.0");
        assert_eq!(p.os, std::env::consts::OS);
        assert_eq!(p.arch, std::env::consts::ARCH);
    }
}
