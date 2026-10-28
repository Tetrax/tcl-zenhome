mod reseau;

use serde::Serialize;

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
        .invoke_handler(tauri::generate_handler![ping, reseau::test_reseau])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de Zen Home");
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
