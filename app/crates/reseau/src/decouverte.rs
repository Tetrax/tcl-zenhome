//! Découverte mDNS des appareils Android TV / Google TV (service Google Cast). Les annonces sont non fiables.

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
/// Appareils retenus au plus.
const ANNONCES_MAX: usize = 64;
/// Annonces examinées au plus (toutes confondues) : borne le travail et la mémoire face à un flot d'annonces.
const VUES_MAX: usize = 1000;
/// IP d'appareils Cast ne correspondant à aucun indice, retenues au plus.
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
/// Box déjà rencontrées : tentatives courtes et rapprochées (une box en Wi-Fi économe ne répond qu'à certains paquets).
const DELAI_CONNUE: Duration = Duration::from_millis(500);
const PASSE_CONNUE: Duration = Duration::from_millis(200);
/// Connexions simultanées du balayage.
const FILS_BALAYAGE: usize = 64;
/// Réseaux /24 balayés au plus (un par interface).
const RESEAUX_MAX: usize = 4;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BoxTrouvee {
    pub nom: String,
    pub ip: String,
    /// Identifiant de l'indice reconnu (simple supposition : la connexion vérifie le modèle) ; `None` pour le balayage de repli.
    pub appareil: Option<String>,
    /// Libellé par défaut de l'appareil supposé.
    pub libelle: String,
}

/// Indice fourni par l'appelant pour reconnaître un type d'appareil dans les annonces Cast. Une annonce est reconnue si
/// son nom d'instance ET son champ `md` commencent par un préfixe du même indice.
#[derive(Debug, Clone, Copy)]
pub struct IndiceAppareil<'a> {
    pub id: &'a str,
    pub libelle: &'a str,
    pub prefixes_md: &'a [&'a str],
    pub prefixes_instance: &'a [&'a str],
}

/// Indice de la Mi Box (compatibilité : `chercher_box_mibox`).
pub const INDICE_MIBOX: IndiceAppareil<'static> =
    IndiceAppareil { id: "mibox4", libelle: "Mi Box", prefixes_md: &["MIBOX"], prefixes_instance: &["MIBOX"] };

/// Annonce réduite aux champs utiles (permet de tester sans réseau).
struct Annonce {
    fullname: String,
    md: Option<String>,
    fn_: Option<String>,
    ipv4: Vec<String>,
}

pub fn nettoyer_nom(brut: &str) -> String {
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

/// Nom d'instance de l'annonce s'il est valide : lettres, chiffres ou tirets, rien d'autre.
fn instance_valide(fullname: &str) -> Option<&str> {
    let instance = fullname.strip_suffix(&format!(".{SERVICE}"))?;
    (instance.len() <= INSTANCE_MAX && instance.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')).then_some(instance)
}

/// Premier indice dont un préfixe d'instance ET un préfixe `md` correspondent à l'annonce.
fn reconnaitre<'a>(a: &Annonce, indices: &'a [IndiceAppareil<'a>]) -> Option<&'a IndiceAppareil<'a>> {
    let instance = instance_valide(&a.fullname)?;
    let md = a.md.as_deref()?;
    indices.iter().find(|i| {
        i.prefixes_instance.iter().any(|p| instance.starts_with(p)) && i.prefixes_md.iter().any(|p| md.starts_with(p))
    })
}

/// Garde les annonces reconnues par un indice, dédoublonnées par IP.
fn filtrer_annonces(annonces: &[Annonce], indices: &[IndiceAppareil]) -> Vec<BoxTrouvee> {
    let mut trouvees: Vec<BoxTrouvee> = Vec::new();
    for a in annonces {
        let Some(indice) = reconnaitre(a, indices) else {
            continue;
        };
        let Some(ip) = a.ipv4.iter().find_map(|s| valider_ipv4(s)) else {
            continue;
        };
        let ip = ip.to_string();
        if trouvees.iter().any(|t| t.ip == ip) {
            continue;
        }
        let nom = nettoyer_nom(a.fn_.as_deref().unwrap_or(""));
        let nom = if nom.is_empty() { indice.libelle.into() } else { nom };
        trouvees.push(BoxTrouvee { nom, ip, appareil: Some(indice.id.into()), libelle: indice.libelle.into() });
    }
    trouvees
}

/// Annonces retenues pendant l'écoute.
#[derive(Default)]
struct Collecte {
    /// Nombre total d'annonces reçues, retenues ou non.
    vues: usize,
    /// Instances reconnues déjà retenues (dédoublonnage).
    vus: HashSet<String>,
    annonces: Vec<Annonce>,
    /// IPv4 d'annonces Cast résolues qu'aucun indice ne reconnaît (exclues du balayage).
    autres: HashSet<Ipv4Addr>,
}

impl Collecte {
    /// Filtre à la réception (instance ET modèle d'un même indice) : seules les annonces retenues sont conservées et comptées
    /// dans `ANNONCES_MAX` ; le total des annonces examinées est borné à part par `VUES_MAX`.
    fn recevoir(&mut self, a: Annonce, indices: &[IndiceAppareil]) {
        if self.vues >= VUES_MAX {
            return;
        }
        self.vues += 1;
        if reconnaitre(&a, indices).is_none() {
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
fn balayer(hotes: &[Ipv4Addr], trouves: &Mutex<Vec<Ipv4Addr>>, stop: &AtomicBool, delai: Duration, passe_min: Duration) {
    while !stop.load(Ordering::Relaxed) && !hotes.is_empty() {
        let restants: Vec<Ipv4Addr> = {
            let t = trouves.lock().unwrap();
            hotes.iter().filter(|h| !t.contains(h)).copied().collect()
        };
        if restants.is_empty() {
            std::thread::sleep(Duration::from_millis(100));
        } else {
            let debut = Instant::now();
            passe(&restants, trouves, stop, delai);
            finir_passe(debut, passe_min, stop);
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

fn passe(hotes: &[Ipv4Addr], trouves: &Mutex<Vec<Ipv4Addr>>, stop: &AtomicBool, delai: Duration) {
    let suivant = AtomicUsize::new(0);
    std::thread::scope(|sc| {
        for _ in 0..FILS_BALAYAGE.min(hotes.len()) {
            sc.spawn(|| {
                while !stop.load(Ordering::Relaxed) {
                    let Some(h) = hotes.get(suivant.fetch_add(1, Ordering::Relaxed)) else { break };
                    if TcpStream::connect_timeout(&SocketAddr::from((*h, PORT_ADB)), delai).is_ok() {
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

/// Toutes les box mémorisées ont répondu : inutile d'attendre davantage.
fn tout_repondu(connues: usize, repondu: usize) -> bool {
    connues > 0 && repondu >= connues
}

/// Fusionne : si mDNS ou la mémoire ont trouvé des appareils, on ne rend que celles-ci (mDNS d'abord, il fait foi pour le
/// nom ; dédoublonnage par IP). Sinon (repli pur), on rend les hôtes du balayage qui ne sont pas connus comme autres
/// appareils Cast, sous le nom « Appareil à vérifier ».
fn fusionner(mut mdns: Vec<BoxTrouvee>, connues: &[BoxTrouvee], balayage: &[Ipv4Addr], exclus: &HashSet<Ipv4Addr>) -> Vec<BoxTrouvee> {
    for b in connues {
        if !mdns.iter().any(|m| m.ip == b.ip) {
            mdns.push(b.clone());
        }
    }
    if !mdns.is_empty() {
        return mdns;
    }
    let mut tout: Vec<BoxTrouvee> = Vec::new();
    for ip in balayage.iter().filter(|ip| !exclus.contains(ip)) {
        let ip = ip.to_string();
        if !tout.iter().any(|b| b.ip == ip) {
            tout.push(BoxTrouvee { nom: "Appareil à vérifier".into(), ip, appareil: None, libelle: "Appareil à vérifier".into() });
        }
    }
    tout
}

/// Transition : `chercher_box` limitée à la Mi Box (`INDICE_MIBOX`), comme avant l'ajout des indices.
pub fn chercher_box_mibox(duree: Duration, connues: &[BoxTrouvee], arret: &AtomicBool) -> Result<Vec<BoxTrouvee>, Erreur> {
    chercher_box(duree, connues, &[INDICE_MIBOX], arret)
}

/// Cherche les appareils décrits par `indices` pendant `duree` au plus (plafonnée à 30 s) : écoute mDNS et balayage TCP du port adb en parallèle.
/// `connues` : box déjà rencontrées (nom, IP, appareil), sondées tout de suite en parallèle ; une qui répond compte comme trouvée.
/// Rend la main `GRACE` après le premier appareil reconnu par mDNS ou répondant depuis `connues` (le balayage ne
/// l'écourte jamais) ; bloquant.
/// `arret` levé (même d'avance) : la recherche s'arrête aussitôt (au plus le délai d'une connexion TCP en cours) et rend
/// ce qu'elle a déjà trouvé.
pub fn chercher_box(duree: Duration, connues: &[BoxTrouvee], indices: &[IndiceAppareil], arret: &AtomicBool) -> Result<Vec<BoxTrouvee>, Erreur> {
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
    let connues: Vec<(Ipv4Addr, &BoxTrouvee)> = connues.iter().filter_map(|b| Some((valider_ipv4(&b.ip)?, b))).take(ANNONCES_MAX).collect();
    let ips_connues: Vec<Ipv4Addr> = connues.iter().map(|(ip, _)| *ip).collect();
    let connues_ok = Mutex::new(Vec::new());
    let stop = AtomicBool::new(false);
    // Le balayage ne sert qu'en repli : on l'arrête dès qu'une box est trouvée, sans attendre la fin de la grâce.
    let stop_balayage = AtomicBool::new(false);
    let mut mdns = Vec::new();
    let mut grace_lancee = false;
    std::thread::scope(|sc| {
        sc.spawn(|| balayer(&hotes, &trouves, &stop_balayage, DELAI_CONNEXION, PASSE_MIN));
        sc.spawn(|| balayer(&ips_connues, &connues_ok, &stop, DELAI_CONNUE, PASSE_CONNUE));
        while let Some(reste) = limite.checked_duration_since(Instant::now()).filter(|_| !arret.load(Ordering::Relaxed)) {
            match rx.recv_timeout(reste.min(Duration::from_millis(100))) {
                Ok(ServiceEvent::ServiceResolved(s)) => {
                    collecte.recevoir(
                        Annonce {
                            fullname: s.fullname.clone(),
                            md: s.get_property_val_str("md").map(String::from),
                            fn_: s.get_property_val_str("fn").map(String::from),
                            ipv4: s.get_addresses_v4().iter().map(|a| a.to_string()).collect(),
                        },
                        indices,
                    );
                }
                Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            mdns = filtrer_annonces(&collecte.annonces, indices);
            let repondu = connues_ok.lock().unwrap().len();
            if tout_repondu(ips_connues.len(), repondu) {
                limite = Instant::now();
            }
            if !grace_lancee && (!mdns.is_empty() || repondu > 0) {
                grace_lancee = true;
                stop_balayage.store(true, Ordering::Relaxed);
                limite = limite.min(Instant::now() + GRACE);
            }
        }
        stop.store(true, Ordering::Relaxed);
        stop_balayage.store(true, Ordering::Relaxed);
    });
    let _ = daemon.shutdown();
    let ok = connues_ok.into_inner().unwrap();
    let connues: Vec<BoxTrouvee> = connues.into_iter().filter(|(ip, _)| ok.contains(ip)).map(|(_, b)| b.clone()).collect();
    Ok(fusionner(mdns, &connues, &trouves.into_inner().unwrap(), &collecte.autres))
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

    const INDICE_TCL: IndiceAppareil = IndiceAppareil {
        id: "tcl-g08",
        libelle: "TV TCL",
        prefixes_md: &["Smart TV Pro"],
        prefixes_instance: &["Smart-TV-Pro"],
    };

    fn mibox(nom: &str, ip: &str) -> BoxTrouvee {
        BoxTrouvee { nom: nom.into(), ip: ip.into(), appareil: Some("mibox4".into()), libelle: "Mi Box".into() }
    }

    fn tcl(nom: &str, ip: &str) -> BoxTrouvee {
        BoxTrouvee { nom: nom.into(), ip: ip.into(), appareil: Some("tcl-g08".into()), libelle: "TV TCL".into() }
    }

    /// Résultat du repli ou box mémorisée d'origine inconnue.
    fn inconnue(nom: &str, ip: &str) -> BoxTrouvee {
        BoxTrouvee { nom: nom.into(), ip: ip.into(), appareil: None, libelle: nom.into() }
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
    fn arret_leve_d_avance_rend_la_main_vite() {
        let arret = AtomicBool::new(true);
        let t = Instant::now();
        let r = chercher_box_mibox(Duration::from_secs(15), &[mibox("SALON", "192.168.1.20")], &arret);
        assert!(t.elapsed() < Duration::from_secs(3), "{:?}", t.elapsed());
        assert!(r.map_or(true, |l| l.len() <= 1)); // erreur mDNS possible en bac à sable, sinon au plus la box connue
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
        assert_eq!(filtrer_annonces(&l, &[INDICE_MIBOX]), vec![mibox("Salon", "192.168.1.20")]);
    }

    #[test]
    fn les_annonces_etrangeres_ne_comptent_pas_dans_la_limite() {
        let mut c = Collecte::default();
        for i in 0..100 {
            c.recevoir(annonce(&format!("Chromecast-{i}"), Some("Chromecast"), Some("TV"), &["192.168.1.30"]), &[INDICE_MIBOX]);
        }
        c.recevoir(annonce("MIBOX4-a", Some("MIBOX4"), Some("Salon"), &["192.168.1.20"]), &[INDICE_MIBOX]);
        assert_eq!(filtrer_annonces(&c.annonces, &[INDICE_MIBOX]), vec![mibox("Salon", "192.168.1.20")]);
        assert!(c.annonces.len() <= 1, "rien d'étranger n'est conservé : {}", c.annonces.len());
        assert!(c.vus.len() <= 1, "ni retenu pour le dédoublonnage : {}", c.vus.len());
    }

    #[test]
    fn le_nombre_total_d_annonces_vues_est_borne() {
        let mut c = Collecte::default();
        for i in 0..(VUES_MAX + 10) {
            c.recevoir(annonce(&format!("Chromecast-{i}"), Some("Chromecast"), None, &["192.168.1.30"]), &[INDICE_MIBOX]);
        }
        c.recevoir(annonce("MIBOX4-a", Some("MIBOX4"), None, &["192.168.1.20"]), &[INDICE_MIBOX]);
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
        assert_eq!(filtrer_annonces(&l, &[INDICE_MIBOX]), vec![mibox("Mi Box", "192.168.1.20"), mibox("Mi Box", "192.168.1.21")]);
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
        let f = fusionner(vec![mibox("Salon", "192.168.50.14")], &[], &[ip("192.168.50.14"), ip("192.168.50.30")], &exclus);
        assert_eq!(f, vec![mibox("Salon", "192.168.50.14")]);
    }

    #[test]
    fn fusion_connues_apres_mdns_sans_doublon_et_sans_repli() {
        let connues = [mibox("Ancien nom", "192.168.50.14"), mibox("Chambre", "192.168.50.15")];
        let f = fusionner(vec![mibox("Salon", "192.168.50.14")], &connues, &[ip("192.168.50.30")], &HashSet::new());
        assert_eq!(f, vec![mibox("Salon", "192.168.50.14"), mibox("Chambre", "192.168.50.15")]);
        let f = fusionner(vec![], &connues[1..], &[ip("192.168.50.30")], &HashSet::new());
        assert_eq!(f, vec![mibox("Chambre", "192.168.50.15")], "une box connue joignable évite « Appareil à vérifier »");
    }

    #[test]
    fn arret_immediat_seulement_si_toutes_les_connues_ont_repondu() {
        assert!(tout_repondu(2, 2) && tout_repondu(1, 1));
        assert!(!tout_repondu(2, 1) && !tout_repondu(1, 0));
        assert!(!tout_repondu(0, 0), "aucune box mémorisée : la grâce habituelle s'applique");
    }

    #[test]
    fn repli_pur_exclut_les_autres_appareils_cast_et_dedoublonne() {
        let exclus = HashSet::from([ip("192.168.50.17")]);
        let f = fusionner(vec![], &[], &[ip("192.168.50.17"), ip("192.168.50.30"), ip("192.168.50.30")], &exclus);
        assert_eq!(f, vec![inconnue("Appareil à vérifier", "192.168.50.30")]);
        assert!(fusionner(vec![], &[], &[], &exclus).is_empty());
    }

    #[test]
    fn les_annonces_non_mibox_alimentent_les_exclusions_bornees() {
        let mut c = Collecte::default();
        c.recevoir(annonce("Smart-TV-Pro-x", Some("Smart TV Pro"), Some("TCL VAL"), &["192.168.50.17", "8.8.8.8", "pas-ip"]), &[INDICE_MIBOX]);
        c.recevoir(annonce("MIBOX4-a", Some("MIBOX4"), Some("SALON"), &["192.168.50.14"]), &[INDICE_MIBOX]);
        assert_eq!(c.autres, HashSet::from([ip("192.168.50.17")]));
        assert_eq!(c.annonces.len(), 1);
        for i in 0..600u32 {
            c.recevoir(annonce(&format!("T{i}"), Some("TV"), None, &[&format!("10.0.{}.{}", i / 250, i % 250)]), &[INDICE_MIBOX]);
        }
        assert!(c.autres.len() <= AUTRES_MAX);
    }

    #[test]
    fn indices_reconnaissent_chaque_type_et_portent_l_identifiant() {
        let l = [
            annonce("MIBOX4-abc", Some("MIBOX4"), Some("Salon"), &["192.168.1.20"]),
            annonce("Smart-TV-Pro-1a2b", Some("Smart TV Pro"), Some("TV Chambre"), &["192.168.1.21"]),
            annonce("Smart-TV-Pro-3c4d", Some("Smart TV Pro"), None, &["192.168.1.22"]),
        ];
        let deux = [INDICE_MIBOX, INDICE_TCL];
        assert_eq!(
            filtrer_annonces(&l, &deux),
            vec![mibox("Salon", "192.168.1.20"), tcl("TV Chambre", "192.168.1.21"), tcl("TV TCL", "192.168.1.22")]
        );
        assert_eq!(filtrer_annonces(&l, &[INDICE_TCL]).len(), 2);
    }

    #[test]
    fn tcl_exclue_sans_son_indice_et_ip_exclue_du_repli() {
        let l = annonce("Smart-TV-Pro-1a2b", Some("Smart TV Pro"), Some("TV"), &["192.168.1.21"]);
        assert!(filtrer_annonces(std::slice::from_ref(&l), &[INDICE_MIBOX]).is_empty());
        let mut c = Collecte::default();
        c.recevoir(l, &[INDICE_MIBOX]);
        assert_eq!(c.autres, HashSet::from([ip("192.168.1.21")]));
        assert!(c.annonces.is_empty());
    }

    #[test]
    fn instance_et_md_doivent_venir_du_meme_indice() {
        let deux = [INDICE_MIBOX, INDICE_TCL];
        let l = [
            annonce("MIBOX4-a", Some("Smart TV Pro"), Some("Croisé 1"), &["192.168.1.30"]),
            annonce("Smart-TV-Pro-a", Some("MIBOX4"), Some("Croisé 2"), &["192.168.1.31"]),
            annonce("Smart-TV-Pro-b", Some("Autre"), Some("md faux"), &["192.168.1.32"]),
            annonce("Smart-TV-Pro-c", None, Some("sans md"), &["192.168.1.33"]),
            annonce("Smart-TV-Pro;rm -rf", Some("Smart TV Pro"), Some("Piégée"), &["192.168.1.34"]),
            annonce("Smart-TV-Pro\n.evil", Some("Smart TV Pro"), Some("Piégée 2"), &["192.168.1.35"]),
            annonce(&format!("Smart-TV-Pro-{}", "a".repeat(100)), Some("Smart TV Pro"), Some("Trop longue"), &["192.168.1.36"]),
            annonce("Smart-TV-Pro-d", Some("Smart TV Pro"), Some("IP publique"), &["8.8.8.8"]),
        ];
        assert!(filtrer_annonces(&l, &deux).is_empty());
    }

    #[test]
    fn fusion_garde_l_identifiant_des_connues() {
        let connues = [tcl("TV", "192.168.50.15")];
        let f = fusionner(vec![mibox("Salon", "192.168.50.14")], &connues, &[], &HashSet::new());
        assert_eq!(f, vec![mibox("Salon", "192.168.50.14"), tcl("TV", "192.168.50.15")]);
    }
}
