//! Test réseau (étape 3a) : découverte mDNS des Mi Box puis sondage adb.
//! Code de test : sert à vérifier la permission macOS « Réseau local ».

use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::Serialize;
use std::collections::HashSet;
use std::io::Read;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const SERVICE: &str = "_googlecast._tcp.local.";
const DUREE_MDNS: Duration = Duration::from_secs(5);
const DELAI_ADB: Duration = Duration::from_secs(15);
const PORT_SERVEUR_ADB: &str = "5038";
const SORTIE_MAX: usize = 2000;
const NOM_MAX: usize = 30;

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct MiBox {
    nom: String,
    ip: String,
}

#[derive(Serialize, Debug)]
pub struct ResultatAdb {
    commande: String,
    code: Option<i32>,
    stdout: String,
    stderr: String,
    delai_depasse: bool,
}

#[derive(Serialize, Debug, Default)]
pub struct Rapport {
    annonces_googlecast: usize,
    mi_box: Vec<MiBox>,
    ip_ciblee: Option<String>,
    adb_present: bool,
    adb: Vec<ResultatAdb>,
    erreurs: Vec<String>,
}

/// Annonce mDNS réduite aux champs utiles (permet de tester sans réseau).
pub struct Annonce {
    pub fullname: String,
    pub md: Option<String>,
    pub fn_: Option<String>,
    pub ipv4: Vec<String>,
}

pub fn nettoyer_nom(brut: &str) -> String {
    brut.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '_' | '-'))
        .take(NOM_MAX)
        .collect()
}

pub fn valider_ipv4(s: &str) -> Option<Ipv4Addr> {
    s.trim().parse::<Ipv4Addr>().ok()
}

/// Garde les annonces dont l'instance ET le modèle (`md`) commencent par « MIBOX ».
pub fn filtrer_mibox(annonces: &[Annonce]) -> Vec<MiBox> {
    let mut trouvees = Vec::new();
    for a in annonces {
        let instance_ok = a.fullname.starts_with("MIBOX");
        let md_ok = a.md.as_deref().is_some_and(|m| m.starts_with("MIBOX"));
        if !(instance_ok && md_ok) {
            continue;
        }
        let Some(ip) = a.ipv4.iter().find_map(|s| valider_ipv4(s)) else {
            continue;
        };
        trouvees.push(MiBox {
            nom: nettoyer_nom(a.fn_.as_deref().unwrap_or("")),
            ip: ip.to_string(),
        });
    }
    trouvees
}

fn tronquer(octets: &[u8]) -> String {
    let s = String::from_utf8_lossy(octets);
    s.chars().take(SORTIE_MAX).collect()
}

fn decouvrir(rapport: &mut Rapport) -> Vec<Annonce> {
    let mut annonces = Vec::new();
    let daemon = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            rapport.erreurs.push(format!("mDNS : démarrage impossible : {e}"));
            return annonces;
        }
    };
    match daemon.browse(SERVICE) {
        Ok(rx) => {
            let fin = Instant::now() + DUREE_MDNS;
            let mut vus = HashSet::new();
            while let Some(reste) = fin.checked_duration_since(Instant::now()) {
                match rx.recv_timeout(reste) {
                    Ok(ServiceEvent::ServiceResolved(s)) => {
                        if vus.insert(s.fullname.clone()) {
                            annonces.push(Annonce {
                                fullname: s.fullname.clone(),
                                md: s.get_property_val_str("md").map(String::from),
                                fn_: s.get_property_val_str("fn").map(String::from),
                                ipv4: s.get_addresses_v4().iter().map(|a| a.to_string()).collect(),
                            });
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
        Err(e) => rapport.erreurs.push(format!("mDNS : écoute impossible : {e}")),
    }
    let _ = daemon.shutdown();
    rapport.annonces_googlecast = annonces.len();
    annonces
}

fn chemin_adb() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/ZenHome/platform-tools/adb"))
}

fn lancer_adb(adb: &PathBuf, args: &[&str]) -> ResultatAdb {
    let commande = format!("adb {}", args.join(" "));
    let mut res = ResultatAdb { commande, code: None, stdout: String::new(), stderr: String::new(), delai_depasse: false };
    let child = Command::new(adb)
        .args(args)
        .env("ANDROID_ADB_SERVER_PORT", PORT_SERVEUR_ADB)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            res.stderr = format!("lancement impossible : {e}");
            return res;
        }
    };
    let lire = |flux: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(f) = flux {
                let _ = f.take(64 * 1024).read_to_end(&mut buf);
            }
            buf
        })
    };
    let out = lire(child.stdout.take().map(|s| Box::new(s) as _));
    let err = lire(child.stderr.take().map(|s| Box::new(s) as _));
    let fin = Instant::now() + DELAI_ADB;
    loop {
        match child.try_wait() {
            Ok(Some(st)) => {
                res.code = st.code();
                break;
            }
            Ok(None) if Instant::now() >= fin => {
                let _ = child.kill();
                let _ = child.wait();
                res.delai_depasse = true;
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => {
                res.stderr = format!("attente impossible : {e}");
                break;
            }
        }
    }
    res.stdout = tronquer(&out.join().unwrap_or_default());
    if res.stderr.is_empty() {
        res.stderr = tronquer(&err.join().unwrap_or_default());
    }
    res
}

fn sonder(rapport: &mut Rapport, ip: &str) {
    rapport.ip_ciblee = Some(ip.to_string());
    let Some(adb) = chemin_adb().filter(|p| p.is_file()) else {
        rapport.erreurs.push("adb absent (platform-tools non installé) : étape adb sautée".into());
        return;
    };
    rapport.adb_present = true;
    let cible = format!("{ip}:5555");
    rapport.adb.push(lancer_adb(&adb, &["connect", &cible]));
    rapport.adb.push(lancer_adb(&adb, &["-s", &cible, "get-state"]));
    rapport.adb.push(lancer_adb(&adb, &["-s", &cible, "shell", "getprop", "ro.product.name"]));
    rapport.adb.push(lancer_adb(&adb, &["kill-server"]));
}

fn executer(ip_manuelle: Option<String>) -> Rapport {
    let mut rapport = Rapport::default();
    let annonces = decouvrir(&mut rapport);
    rapport.mi_box = filtrer_mibox(&annonces);
    let ip = match ip_manuelle.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(saisie) => match valider_ipv4(saisie) {
            Some(ip) => Some(ip.to_string()),
            None => {
                rapport.erreurs.push("IP manuelle invalide (IPv4 attendue) : adb non lancé".into());
                None
            }
        },
        None => rapport.mi_box.first().map(|b| b.ip.clone()),
    };
    match ip {
        Some(ip) => sonder(&mut rapport, &ip),
        None => rapport.erreurs.push("Aucune cible pour adb".into()),
    }
    rapport
}

#[tauri::command]
pub async fn test_reseau(ip_manuelle: Option<String>) -> Result<Rapport, String> {
    tauri::async_runtime::spawn_blocking(move || executer(ip_manuelle))
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn annonce(fullname: &str, md: Option<&str>, fn_: Option<&str>, ips: &[&str]) -> Annonce {
        Annonce {
            fullname: fullname.into(),
            md: md.map(Into::into),
            fn_: fn_.map(Into::into),
            ipv4: ips.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn nettoie_les_noms() {
        assert_eq!(nettoyer_nom("Salon <b>TV</b>; rm -rf /"), "Salon bTVb rm -rf ");
        assert_eq!(nettoyer_nom("Mi_Box-4.0 é\n\x1b[0m"), "Mi_Box-4.0 0m");
        assert_eq!(nettoyer_nom(&"a".repeat(100)).len(), 30);
    }

    #[test]
    fn valide_ipv4() {
        assert!(valider_ipv4("192.168.1.20").is_some());
        assert!(valider_ipv4(" 10.0.0.1 ").is_some());
        for mauvais in ["", "256.1.1.1", "1.2.3", "::1", "1.2.3.4:5555", "1.2.3.4; ls", "a.b.c.d"] {
            assert!(valider_ipv4(mauvais).is_none(), "{mauvais}");
        }
    }

    #[test]
    fn filtre_les_mibox() {
        let l = [
            annonce("MIBOX4-abc._googlecast._tcp.local.", Some("MIBOX4"), Some("Salon"), &["192.168.1.20"]),
            annonce("Chromecast-1._googlecast._tcp.local.", Some("Chromecast"), Some("Cuisine"), &["192.168.1.30"]),
            annonce("MIBOX4-x._googlecast._tcp.local.", Some("Chromecast"), Some("Faux md"), &["192.168.1.31"]),
            annonce("Autre._googlecast._tcp.local.", Some("MIBOX4"), Some("Faux nom"), &["192.168.1.32"]),
            annonce("MIBOX4-y._googlecast._tcp.local.", None, Some("Sans md"), &["192.168.1.33"]),
            annonce("MIBOX4-z._googlecast._tcp.local.", Some("MIBOX4"), Some("Sans ip"), &["pas-une-ip"]),
        ];
        assert_eq!(filtrer_mibox(&l), vec![MiBox { nom: "Salon".into(), ip: "192.168.1.20".into() }]);
    }
}
