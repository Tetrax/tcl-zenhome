//! Découverte mDNS des Mi Box (service Google Cast). Les annonces sont non fiables.

use crate::Erreur;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::Serialize;
use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

const SERVICE: &str = "_googlecast._tcp.local.";
const DUREE_MAX: Duration = Duration::from_secs(30);
const ANNONCES_MAX: usize = 64;
const NOM_MAX: usize = 30;
const INSTANCE_MAX: usize = 80;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BoxTrouvee {
    pub nom: String,
    pub ip: String,
}

/// Annonce réduite aux champs utiles (permet de tester sans réseau).
struct Annonce {
    fullname: String,
    md: Option<String>,
    fn_: Option<String>,
    ipv4: Vec<String>,
}

fn nettoyer_nom(brut: &str) -> String {
    brut.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '_' | '-'))
        .take(NOM_MAX)
        .collect::<String>()
        .trim()
        .to_string()
}

/// IPv4 d'un réseau local (privée ou link-local) : une box ne peut pas être ailleurs.
fn valider_ipv4(s: &str) -> Option<Ipv4Addr> {
    let ip = s.trim().parse::<Ipv4Addr>().ok()?;
    (ip.is_private() || ip.is_link_local()).then_some(ip)
}

/// Nom d'instance de l'annonce : `MIBOX` suivi de lettres, chiffres ou tirets, rien d'autre.
fn instance_mibox(fullname: &str) -> bool {
    let Some(instance) = fullname.strip_suffix(&format!(".{SERVICE}")) else {
        return false;
    };
    instance.len() <= INSTANCE_MAX
        && instance.starts_with("MIBOX")
        && instance.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Garde les annonces dont l'instance ET le modèle (`md`) commencent par « MIBOX », dédoublonnées par IP.
fn filtrer_mibox(annonces: &[Annonce]) -> Vec<BoxTrouvee> {
    let mut trouvees: Vec<BoxTrouvee> = Vec::new();
    for a in annonces {
        if !instance_mibox(&a.fullname) || !a.md.as_deref().is_some_and(|m| m.starts_with("MIBOX")) {
            continue;
        }
        let Some(ip) = a.ipv4.iter().find_map(|s| valider_ipv4(s)) else {
            continue;
        };
        let ip = ip.to_string();
        if trouvees.iter().any(|t| t.ip == ip) {
            continue;
        }
        let nom = nettoyer_nom(a.fn_.as_deref().unwrap_or(""));
        trouvees.push(BoxTrouvee { nom: if nom.is_empty() { "Mi Box".into() } else { nom }, ip });
    }
    trouvees
}

/// Écoute le réseau local pendant `duree` (plafonnée à 30 s) et renvoie les Mi Box trouvées (bloquant).
pub fn chercher_box(duree: Duration) -> Result<Vec<BoxTrouvee>, Erreur> {
    let daemon = ServiceDaemon::new().map_err(|e| Erreur::Mdns(e.to_string()))?;
    let rx = match daemon.browse(SERVICE) {
        Ok(rx) => rx,
        Err(e) => {
            let _ = daemon.shutdown();
            return Err(Erreur::Mdns(e.to_string()));
        }
    };
    let fin = Instant::now() + duree.min(DUREE_MAX);
    let mut vus = HashSet::new();
    let mut annonces = Vec::new();
    while let Some(reste) = fin.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(reste) {
            Ok(ServiceEvent::ServiceResolved(s)) => {
                if annonces.len() < ANNONCES_MAX && vus.insert(s.fullname.clone()) {
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
    let _ = daemon.shutdown();
    Ok(filtrer_mibox(&annonces))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn annonce(instance: &str, md: Option<&str>, fn_: Option<&str>, ips: &[&str]) -> Annonce {
        Annonce {
            fullname: format!("{instance}.{SERVICE}"),
            md: md.map(Into::into),
            fn_: fn_.map(Into::into),
            ipv4: ips.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn mibox(nom: &str, ip: &str) -> BoxTrouvee {
        BoxTrouvee { nom: nom.into(), ip: ip.into() }
    }

    #[test]
    fn nettoie_les_noms() {
        assert_eq!(nettoyer_nom("Salon <b>TV</b>; rm -rf /"), "Salon bTVb rm -rf");
        assert_eq!(nettoyer_nom("Mi_Box-4.0 é\n\x1b[0m"), "Mi_Box-4.0 0m");
        assert_eq!(nettoyer_nom("\u{202E}Salon"), "Salon");
        assert_eq!(nettoyer_nom(&"a".repeat(100)).len(), 30);
    }

    #[test]
    fn valide_ipv4() {
        for bon in ["192.168.1.20", " 10.0.0.1 ", "172.16.5.4", "169.254.1.1"] {
            assert!(valider_ipv4(bon).is_some(), "{bon}");
        }
        for mauvais in [
            "", "256.1.1.1", "1.2.3", "::1", "1.2.3.4:5555", "1.2.3.4; ls", "a.b.c.d",
            "8.8.8.8", "127.0.0.1", "0.0.0.0", "224.0.0.251", "255.255.255.255",
        ] {
            assert!(valider_ipv4(mauvais).is_none(), "{mauvais}");
        }
    }

    #[test]
    fn filtre_les_annonces_forgees() {
        let l = [
            annonce("MIBOX4-abc", Some("MIBOX4"), Some("Salon"), &["192.168.1.20"]),
            annonce("Chromecast-1", Some("Chromecast"), Some("Cuisine"), &["192.168.1.30"]),
            annonce("MIBOX4-x", Some("Chromecast"), Some("Faux md"), &["192.168.1.31"]),
            annonce("Autre", Some("MIBOX4"), Some("Faux nom"), &["192.168.1.32"]),
            annonce("MIBOX4-y", None, Some("Sans md"), &["192.168.1.33"]),
            annonce("MIBOX4-z", Some("MIBOX4"), Some("Sans ip"), &["pas-une-ip"]),
            annonce("MIBOX4-pub", Some("MIBOX4"), Some("IP publique"), &["8.8.8.8"]),
            annonce("MIBOX4;rm -rf", Some("MIBOX4"), Some("Instance piégée"), &["192.168.1.34"]),
            annonce("MIBOX4\n.evil", Some("MIBOX4"), Some("Instance piégée 2"), &["192.168.1.35"]),
            annonce(&format!("MIBOX{}", "a".repeat(100)), Some("MIBOX4"), Some("Trop longue"), &["192.168.1.36"]),
            Annonce { fullname: "MIBOX4-q._autre._tcp.local.".into(), md: Some("MIBOX4".into()), fn_: None, ipv4: vec!["192.168.1.37".into()] },
        ];
        assert_eq!(filtrer_mibox(&l), vec![mibox("Salon", "192.168.1.20")]);
    }

    #[test]
    fn dedoublonne_et_nomme_par_defaut() {
        let l = [
            annonce("MIBOX4-a", Some("MIBOX4"), Some("\u{202E}"), &["bad", "192.168.1.20"]),
            annonce("MIBOX4-b", Some("MIBOX4"), Some("Doublon"), &["192.168.1.20"]),
            annonce("MIBOX4-c", Some("MIBOX4"), None, &["192.168.1.21"]),
        ];
        assert_eq!(filtrer_mibox(&l), vec![mibox("Mi Box", "192.168.1.20"), mibox("Mi Box", "192.168.1.21")]);
    }
}
