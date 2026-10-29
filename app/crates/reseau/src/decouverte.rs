//! Découverte mDNS des Mi Box (service Google Cast). Les annonces sont non fiables.

use crate::Erreur;
use mdns_sd::{RecvTimeoutError, ServiceDaemon, ServiceEvent};
use serde::Serialize;
use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const SERVICE: &str = "_googlecast._tcp.local.";
const DUREE_MAX: Duration = Duration::from_secs(30);
/// Mi Box retenues au plus.
const ANNONCES_MAX: usize = 64;
/// Annonces examinées au plus (toutes confondues) : borne le travail et la mémoire face à un flot d'annonces.
const VUES_MAX: usize = 1000;
/// IP d'appareils Cast qui ne sont pas des Mi Box, retenues au plus.
const AUTRES_MAX: usize = 256;
const NOM_MAX: usize = 30;
const INSTANCE_MAX: usize = 80;
/// Après la première box trouvée, on attend encore un peu pour en attraper une seconde.
const GRACE: Duration = Duration::from_millis(1500);
/// Port adb sondé par le balayage de repli.
const PORT_ADB: u16 = 5555;
/// Large : une box en Wi-Fi économe met plusieurs centaines de ms à se réveiller au premier paquet.
/// Durée minimale d'une passe de balayage.
const PASSE_MIN: Duration = Duration::from_secs(2);
const DELAI_CONNEXION: Duration = Duration::from_millis(1500);
/// Connexions simultanées du balayage.
const FILS_BALAYAGE: usize = 64;
/// Réseaux /24 balayés au plus (un par interface).
const RESEAUX_MAX: usize = 4;

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
pub fn valider_ipv4(s: &str) -> Option<Ipv4Addr> {
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

/// Annonces retenues pendant l'écoute.
#[derive(Default)]
struct Collecte {
    /// Nombre total d'annonces reçues, retenues ou non.
    vues: usize,
    /// Instances Mi Box déjà retenues (dédoublonnage).
    vus: HashSet<String>,
    annonces: Vec<Annonce>,
    /// IPv4 d'annonces Cast résolues qui ne sont pas des Mi Box (exclues du balayage).
    autres: HashSet<Ipv4Addr>,
}

impl Collecte {
    /// Filtre à la réception (instance ET modèle Mi Box) : seules les annonces retenues sont conservées et comptées
    /// dans `ANNONCES_MAX` ; le total des annonces examinées est borné à part par `VUES_MAX`.
    fn recevoir(&mut self, a: Annonce) {
        if self.vues >= VUES_MAX {
            return;
        }
        self.vues += 1;
        let mibox = instance_mibox(&a.fullname) && a.md.as_deref().is_some_and(|m| m.starts_with("MIBOX"));
        if !mibox {
            for ip in a.ipv4.iter().filter_map(|s| valider_ipv4(s)) {
                if self.autres.len() < AUTRES_MAX {
                    self.autres.insert(ip);
                }
            }
        } else if self.annonces.len() < ANNONCES_MAX && self.vus.insert(a.fullname.clone()) {
            self.annonces.push(a);
        }
    }
}

/// Hôtes à sonder : le /24 de chaque IP locale privée ou link-local (un réseau une seule fois, `RESEAUX_MAX` au plus),
/// sans les adresses du réseau et de broadcast ni les IP de la machine elle-même.
fn hotes_a_balayer(locales: &[Ipv4Addr]) -> Vec<Ipv4Addr> {
    let mut reseaux: Vec<[u8; 3]> = Vec::new();
    for ip in locales.iter().filter(|ip| valider_ipv4(&ip.to_string()).is_some()) {
        let [a, b, c, _] = ip.octets();
        if !reseaux.contains(&[a, b, c]) && reseaux.len() < RESEAUX_MAX {
            reseaux.push([a, b, c]);
        }
    }
    reseaux
        .iter()
        .flat_map(|&[a, b, c]| (1..=254).map(move |d| Ipv4Addr::new(a, b, c, d)))
        .filter(|h| !locales.contains(h))
        .collect()
}

/// IPv4 des interfaces actives (hors loopback).
fn ips_locales() -> Vec<Ipv4Addr> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback() && i.is_oper_up())
        .filter_map(|i| match i.addr {
            if_addrs::IfAddr::V4(v4) => Some(v4.ip),
            _ => None,
        })
        .collect()
}

/// Sonde le port adb de `hotes` en parallèle (borné) et note dans `trouves` ceux qui répondent.
/// Repasse en continu sur les hôtes pas encore trouvés jusqu'à ce que `stop` soit levé.
fn balayer(hotes: &[Ipv4Addr], trouves: &Mutex<Vec<Ipv4Addr>>, stop: &AtomicBool) {
    while !stop.load(Ordering::Relaxed) && !hotes.is_empty() {
        let restants: Vec<Ipv4Addr> = {
            let t = trouves.lock().unwrap();
            hotes.iter().filter(|h| !t.contains(h)).copied().collect()
        };
        if restants.is_empty() {
            std::thread::sleep(Duration::from_millis(100));
        } else {
            let debut = Instant::now();
            passe(&restants, trouves, stop);
            finir_passe(debut, PASSE_MIN, stop);
        }
    }
}

/// Attend que `min` se soit écoulé depuis `debut` (une passe qui échoue vite ne doit pas tourner en boucle serrée),
/// en restant réactif à `stop`.
fn finir_passe(debut: Instant, min: Duration, stop: &AtomicBool) {
    while !stop.load(Ordering::Relaxed) {
        let Some(reste) = (debut + min).checked_duration_since(Instant::now()) else { break };
        std::thread::sleep(reste.min(Duration::from_millis(50)));
    }
}

fn passe(hotes: &[Ipv4Addr], trouves: &Mutex<Vec<Ipv4Addr>>, stop: &AtomicBool) {
    let suivant = AtomicUsize::new(0);
    std::thread::scope(|sc| {
        for _ in 0..FILS_BALAYAGE.min(hotes.len()) {
            sc.spawn(|| {
                while !stop.load(Ordering::Relaxed) {
                    let Some(h) = hotes.get(suivant.fetch_add(1, Ordering::Relaxed)) else { break };
                    if TcpStream::connect_timeout(&SocketAddr::from((*h, PORT_ADB)), DELAI_CONNEXION).is_ok() {
                        let mut t = trouves.lock().unwrap();
                        if t.len() < ANNONCES_MAX {
                            t.push(*h);
                        }
                    }
                }
            });
        }
    });
}

/// Fusionne : si mDNS a trouvé des Mi Box, on ne rend que celles-ci. Sinon (repli pur), on rend les hôtes du balayage
/// qui ne sont pas connus comme autres appareils Cast, sous le nom « Appareil à vérifier ».
fn fusionner(mdns: Vec<BoxTrouvee>, balayage: &[Ipv4Addr], exclus: &HashSet<Ipv4Addr>) -> Vec<BoxTrouvee> {
    if !mdns.is_empty() {
        return mdns;
    }
    let mut tout: Vec<BoxTrouvee> = Vec::new();
    for ip in balayage.iter().filter(|ip| !exclus.contains(ip)) {
        let ip = ip.to_string();
        if !tout.iter().any(|b| b.ip == ip) {
            tout.push(BoxTrouvee { nom: "Appareil à vérifier".into(), ip });
        }
    }
    tout
}

/// Cherche les Mi Box pendant `duree` au plus (plafonnée à 30 s) : écoute mDNS et balayage TCP du port adb en parallèle.
/// Rend la main `GRACE` après la première Mi Box reconnue par mDNS (le balayage ne l'écourte jamais) ; bloquant.
pub fn chercher_box(duree: Duration) -> Result<Vec<BoxTrouvee>, Erreur> {
    let daemon = ServiceDaemon::new().map_err(|e| Erreur::Mdns(e.to_string()))?;
    let rx = match daemon.browse(SERVICE) {
        Ok(rx) => rx,
        Err(e) => {
            let _ = daemon.shutdown();
            return Err(Erreur::Mdns(e.to_string()));
        }
    };
    let mut limite = Instant::now() + duree.min(DUREE_MAX);
    let mut collecte = Collecte::default();
    let hotes = hotes_a_balayer(&ips_locales());
    let trouves = Mutex::new(Vec::new());
    let stop = AtomicBool::new(false);
    let mut mdns = Vec::new();
    let mut grace_lancee = false;
    std::thread::scope(|sc| {
        sc.spawn(|| balayer(&hotes, &trouves, &stop));
        while let Some(reste) = limite.checked_duration_since(Instant::now()) {
            match rx.recv_timeout(reste.min(Duration::from_millis(100))) {
                Ok(ServiceEvent::ServiceResolved(s)) => {
                    collecte.recevoir(Annonce {
                        fullname: s.fullname.clone(),
                        md: s.get_property_val_str("md").map(String::from),
                        fn_: s.get_property_val_str("fn").map(String::from),
                        ipv4: s.get_addresses_v4().iter().map(|a| a.to_string()).collect(),
                    });
                }
                Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            mdns = filtrer_mibox(&collecte.annonces);
            if !grace_lancee && !mdns.is_empty() {
                grace_lancee = true;
                limite = limite.min(Instant::now() + GRACE);
            }
        }
        stop.store(true, Ordering::Relaxed);
    });
    let _ = daemon.shutdown();
    Ok(fusionner(mdns, &trouves.into_inner().unwrap(), &collecte.autres))
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
    fn une_passe_dure_au_moins_le_minimum_et_reste_reactive_a_stop() {
        let stop = AtomicBool::new(false);
        let t = Instant::now();
        finir_passe(t, Duration::from_millis(300), &stop);
        assert!(t.elapsed() >= Duration::from_millis(300));
        stop.store(true, Ordering::Relaxed);
        let t = Instant::now();
        finir_passe(t, Duration::from_secs(10), &stop);
        assert!(t.elapsed() < Duration::from_secs(1));
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
    fn les_annonces_etrangeres_ne_comptent_pas_dans_la_limite() {
        let mut c = Collecte::default();
        for i in 0..100 {
            c.recevoir(annonce(&format!("Chromecast-{i}"), Some("Chromecast"), Some("TV"), &["192.168.1.30"]));
        }
        c.recevoir(annonce("MIBOX4-a", Some("MIBOX4"), Some("Salon"), &["192.168.1.20"]));
        assert_eq!(filtrer_mibox(&c.annonces), vec![mibox("Salon", "192.168.1.20")]);
        assert!(c.annonces.len() <= 1, "rien d'étranger n'est conservé : {}", c.annonces.len());
        assert!(c.vus.len() <= 1, "ni retenu pour le dédoublonnage : {}", c.vus.len());
    }

    #[test]
    fn le_nombre_total_d_annonces_vues_est_borne() {
        let mut c = Collecte::default();
        for i in 0..(VUES_MAX + 10) {
            c.recevoir(annonce(&format!("Chromecast-{i}"), Some("Chromecast"), None, &["192.168.1.30"]));
        }
        c.recevoir(annonce("MIBOX4-a", Some("MIBOX4"), None, &["192.168.1.20"]));
        assert!(c.annonces.is_empty(), "au-delà de la borne, plus rien n'est examiné");
        assert!(c.vues <= VUES_MAX + 1);
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

    fn ip(s: &str) -> Ipv4Addr {
        s.parse().unwrap()
    }

    #[test]
    fn hotes_du_24_sans_reseau_broadcast_ni_soi_meme() {
        let h = hotes_a_balayer(&[ip("192.168.50.20")]);
        assert_eq!(h.len(), 253);
        assert!(!h.contains(&ip("192.168.50.0")) && !h.contains(&ip("192.168.50.255")));
        assert!(!h.contains(&ip("192.168.50.20")));
        assert!(h.contains(&ip("192.168.50.14")) && h.contains(&ip("192.168.50.1")) && h.contains(&ip("192.168.50.254")));
    }

    #[test]
    fn pas_de_balayage_pour_les_ip_non_locales() {
        let h = hotes_a_balayer(&[ip("8.8.8.8"), ip("127.0.0.1"), ip("0.0.0.0"), ip("224.0.0.251")]);
        assert!(h.is_empty());
        let h = hotes_a_balayer(&[ip("8.8.8.8"), ip("10.0.0.5")]);
        assert!(h.iter().all(|x| x.octets()[..3] == [10, 0, 0]));
    }

    #[test]
    fn un_reseau_une_fois_et_nombre_de_reseaux_borne() {
        assert_eq!(hotes_a_balayer(&[ip("192.168.1.2"), ip("192.168.1.3")]).len(), 252);
        let beaucoup: Vec<Ipv4Addr> = (0..10).map(|i| Ipv4Addr::new(10, 0, i, 5)).collect();
        assert_eq!(hotes_a_balayer(&beaucoup).len(), RESEAUX_MAX * 254 - RESEAUX_MAX);
    }

    #[test]
    fn fusion_mdns_seul_si_present() {
        let exclus = HashSet::new();
        let f = fusionner(vec![mibox("Salon", "192.168.50.14")], &[ip("192.168.50.14"), ip("192.168.50.30")], &exclus);
        assert_eq!(f, vec![mibox("Salon", "192.168.50.14")]);
    }

    #[test]
    fn repli_pur_exclut_les_autres_appareils_cast_et_dedoublonne() {
        let exclus = HashSet::from([ip("192.168.50.17")]);
        let f = fusionner(vec![], &[ip("192.168.50.17"), ip("192.168.50.30"), ip("192.168.50.30")], &exclus);
        assert_eq!(f, vec![mibox("Appareil à vérifier", "192.168.50.30")]);
        assert!(fusionner(vec![], &[], &exclus).is_empty());
    }

    #[test]
    fn les_annonces_non_mibox_alimentent_les_exclusions_bornees() {
        let mut c = Collecte::default();
        c.recevoir(annonce("Smart-TV-Pro-x", Some("Smart TV Pro"), Some("TCL VAL"), &["192.168.50.17", "8.8.8.8", "pas-ip"]));
        c.recevoir(annonce("MIBOX4-a", Some("MIBOX4"), Some("SALON"), &["192.168.50.14"]));
        assert_eq!(c.autres, HashSet::from([ip("192.168.50.17")]));
        assert_eq!(c.annonces.len(), 1);
        for i in 0..600u32 {
            c.recevoir(annonce(&format!("T{i}"), Some("TV"), None, &[&format!("10.0.{}.{}", i / 250, i % 250)]));
        }
        assert!(c.autres.len() <= AUTRES_MAX);
    }
}
