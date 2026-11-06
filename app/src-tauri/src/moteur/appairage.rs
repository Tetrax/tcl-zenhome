//! Appairage par code (« Débogage sans fil » d'Android 11+) : certaines TV (Chromecast avec Google TV, Google TV Streamer,
//! quelques Sony ou Philips) n'acceptent pas adb sur le port 5555. Elles demandent un appairage par code à 6 chiffres, puis
//! se connectent sur un port TLS qui change à chaque activation. Les ports viennent de `adb mdns services`.
//!
//! Tout passe par le trait `Appairage` (adb réel : `AdbServeur` ; tests et démo : `FakeAppairage`). Aucune commande n'est
//! construite par un shell : `adb pair` reçoit ses arguments séparés.

use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use super::transport::{ErreurTransport, assainir};

/// Chiffres du code d'association affiché sur la TV.
const CODE_CHIFFRES: usize = 6;
/// Pause entre deux lectures de `mdns services`.
const PAUSE_MDNS: Duration = Duration::from_secs(1);
/// Lectures de `mdns services` pour trouver la fenêtre d'association (~15 s).
const ESSAIS_FENETRE: u32 = 15;
/// Lectures pour retrouver le service de connexion après l'appairage (quelques secondes).
const ESSAIS_CONNEXION: u32 = 6;
const INSTANCE_MAX: usize = 64;

const SERVICE_APPAIRAGE: &str = "_adb-tls-pairing._tcp";
const SERVICE_CONNEXION: &str = "_adb-tls-connect._tcp";

pub trait Appairage {
    /// Sortie brute de `adb mdns services`.
    fn services_mdns(&self) -> Result<String, ErreurTransport>;
    /// `adb pair ip:port code`. Une fenêtre ouverte reste ouverte après un code refusé.
    fn appairer(&self, adresse: SocketAddrV4, code: &str) -> Result<(), ErreurAppairage>;
    /// Pause entre deux lectures ; les faux la rendent instantanée.
    fn attendre(&self, duree: Duration) {
        std::thread::sleep(duree);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErreurAppairage {
    /// « protocol fault » ou « Wrong password » : la fenêtre reste ouverte, on peut ressaisir.
    CodeRefuse,
    /// Aucune fenêtre d'association annoncée (ou plus joignable).
    FenetreIntrouvable,
    /// Plusieurs TV affichent une fenêtre et aucune adresse n'a été donnée.
    PlusieursFenetres,
    Delai,
    /// Appairage réussi, mais la TV n'annonce pas (encore) son service de connexion.
    ConnexionIntrouvable,
    /// Annulation ou délai global demandés par l'appelant.
    Interrompu,
    Autre(String),
}

/// Service annoncé par `adb mdns services`.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceMdns {
    pub instance: String,
    pub adresse: SocketAddrV4,
}

#[derive(Debug, Default, PartialEq)]
pub struct ServicesMdns {
    pub appairage: Vec<ServiceMdns>,
    pub connexion: Vec<ServiceMdns>,
}

/// Code d'association : exactement 6 chiffres ; les blancs (espaces du collage « 745 385 ») sont ignorés.
pub fn valider_code(saisie: &str) -> Option<String> {
    let code: String = saisie.chars().filter(|c| !c.is_whitespace()).collect();
    (code.len() == CODE_CHIFFRES && code.chars().all(|c| c.is_ascii_digit())).then_some(code)
}

fn adresse(s: &str) -> Option<SocketAddrV4> {
    let (ip, port) = s.rsplit_once(':')?;
    let port = port.parse::<u16>().ok().filter(|p| *p != 0)?;
    Some(SocketAddrV4::new(zh_reseau::valider_ipv4(ip)?, port))
}

/// Analyse la sortie de `adb mdns services` (champs séparés par des blancs : instance, type, IP:port). Les lignes
/// inattendues (en-tête, champs manquants, IP publique, port invalide, instance suspecte) sont ignorées ; seuls les
/// services d'appairage et de connexion sont retenus (`_adb._tcp`, le 5555 classique, ne nous concerne pas).
pub fn analyser_mdns(sortie: &str) -> ServicesMdns {
    let mut services = ServicesMdns::default();
    for ligne in sortie.lines() {
        let champs: Vec<&str> = ligne.split_whitespace().collect();
        let [instance, genre, adr] = champs[..] else { continue };
        let instance_ok = (1..=INSTANCE_MAX).contains(&instance.len()) && instance.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
        let Some(adresse) = adresse(adr).filter(|_| instance_ok) else { continue };
        let service = ServiceMdns { instance: instance.into(), adresse };
        let liste = match genre {
            SERVICE_APPAIRAGE => &mut services.appairage,
            SERVICE_CONNEXION => &mut services.connexion,
            _ => continue,
        };
        if !liste.contains(&service) {
            liste.push(service);
        }
    }
    services
}

/// Classe un échec de `adb pair` d'après son texte (stderr + stdout).
pub fn classer_erreur_appairage(texte: &str) -> ErreurAppairage {
    let t = texte.to_lowercase();
    let a = |m: &str| t.contains(m);
    if a("protocol fault") || a("wrong password") {
        ErreurAppairage::CodeRefuse
    } else if a("timed out") || a("timeout") || a("délai dépassé") {
        ErreurAppairage::Delai
    } else if a("connection refused") || a("failed to connect") || a("unable to connect") || a("no route to host") || a("unreachable") {
        ErreurAppairage::FenetreIntrouvable
    } else {
        ErreurAppairage::Autre(assainir(texte))
    }
}

/// Services de connexion annoncés (box sans fil déjà appairées) ; vide si adb ne répond pas.
pub fn connexions_mdns(a: &dyn Appairage) -> Vec<ServiceMdns> {
    a.services_mdns().map(|s| analyser_mdns(&s).connexion).unwrap_or_default()
}

/// Cherche la fenêtre d'association (une seule, ou celle de `ip` si l'utilisateur l'a donnée).
fn attendre_fenetre(a: &dyn Appairage, ip: Option<Ipv4Addr>, interrompu: &dyn Fn() -> bool) -> Result<ServiceMdns, ErreurAppairage> {
    for essai in 0..ESSAIS_FENETRE {
        if interrompu() {
            return Err(ErreurAppairage::Interrompu);
        }
        let sortie = a.services_mdns().map_err(|e| ErreurAppairage::Autre(e.to_string()))?;
        let mut fenetres: Vec<ServiceMdns> = analyser_mdns(&sortie).appairage.into_iter().filter(|s| ip.is_none_or(|ip| *s.adresse.ip() == ip)).collect();
        match fenetres.len() {
            0 => {}
            1 => return Ok(fenetres.remove(0)),
            _ if ip.is_some() => return Ok(fenetres.remove(0)),
            _ => return Err(ErreurAppairage::PlusieursFenetres),
        }
        if essai + 1 < ESSAIS_FENETRE {
            a.attendre(PAUSE_MDNS);
        }
    }
    Err(ErreurAppairage::FenetreIntrouvable)
}

/// Parcours d'appairage : trouve la fenêtre, appaire, puis retrouve le service de connexion du même nom d'instance ET de
/// la même IP (un autre appareil du réseau peut annoncer le même nom pour détourner la connexion).
/// Rend l'adresse (IP:port) sur laquelle faire `adb connect`. `interrompu` : annulation ou délai global.
pub fn appairer(a: &dyn Appairage, code: &str, ip: Option<Ipv4Addr>, interrompu: &dyn Fn() -> bool) -> Result<SocketAddrV4, ErreurAppairage> {
    let code = valider_code(code).ok_or_else(|| ErreurAppairage::Autre("code invalide".into()))?;
    let fenetre = attendre_fenetre(a, ip, interrompu)?;
    a.appairer(fenetre.adresse, &code)?;
    for essai in 0..ESSAIS_CONNEXION {
        if interrompu() {
            return Err(ErreurAppairage::Interrompu);
        }
        let service = a.services_mdns().ok().and_then(|s| analyser_mdns(&s).connexion.into_iter().find(|s| s.instance == fenetre.instance && s.adresse.ip() == fenetre.adresse.ip()));
        if let Some(s) = service {
            return Ok(s.adresse);
        }
        if essai + 1 < ESSAIS_CONNEXION {
            a.attendre(PAUSE_MDNS);
        }
    }
    Err(ErreurAppairage::ConnexionIntrouvable)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SORTIE: &str = "List of discovered mdns services\n\
        adb-2C0E003207E0230DE-LpiJ1e\t_adb-tls-pairing._tcp\t192.168.50.17:44437\n\
        adb-2C0E003207E0230DE\t_adb._tcp\t192.168.50.17:5555\n\
        adb-2C0E003207E0230DE-LpiJ1e\t_adb-tls-connect._tcp\t192.168.50.17:45509\n";

    fn sock(s: &str) -> SocketAddrV4 {
        s.parse().unwrap()
    }

    /// Sortie `mdns services` fixe, appairage toujours accepté.
    struct MdnsFixe(&'static str);
    impl Appairage for MdnsFixe {
        fn services_mdns(&self) -> Result<String, ErreurTransport> {
            Ok(self.0.into())
        }
        fn appairer(&self, _: SocketAddrV4, _: &str) -> Result<(), ErreurAppairage> {
            Ok(())
        }
        fn attendre(&self, _: Duration) {}
    }

    #[test]
    fn service_de_connexion_d_une_autre_ip_ignore() {
        // Un autre appareil annonce le nom d'instance de la TV avec sa propre IP : on ne s'y connecte pas.
        let a = MdnsFixe("List of discovered mdns services\nadb-TV-x\t_adb-tls-pairing._tcp\t192.168.50.17:44437\nadb-TV-x\t_adb-tls-connect._tcp\t192.168.50.66:45509\n");
        assert_eq!(appairer(&a, "123456", None, &|| false), Err(ErreurAppairage::ConnexionIntrouvable));
        let a = MdnsFixe("adb-TV-x\t_adb-tls-pairing._tcp\t192.168.50.17:44437\nadb-TV-x\t_adb-tls-connect._tcp\t192.168.50.66:45509\nadb-TV-x\t_adb-tls-connect._tcp\t192.168.50.17:45509\n");
        assert_eq!(appairer(&a, "123456", None, &|| false), Ok(sock("192.168.50.17:45509")));
    }

    #[test]
    fn analyse_la_sortie_mesuree() {
        let s = analyser_mdns(SORTIE);
        assert_eq!(s.appairage, [ServiceMdns { instance: "adb-2C0E003207E0230DE-LpiJ1e".into(), adresse: sock("192.168.50.17:44437") }]);
        assert_eq!(s.connexion, [ServiceMdns { instance: "adb-2C0E003207E0230DE-LpiJ1e".into(), adresse: sock("192.168.50.17:45509") }]);
    }

    #[test]
    fn ignore_les_lignes_inattendues() {
        let sortie = "List of discovered mdns services\n\
            \n\
            seul\n\
            a _adb-tls-pairing._tcp\n\
            a _adb-tls-pairing._tcp 192.168.50.17:44437 en-trop\n\
            a _adb-tls-pairing._tcp 8.8.8.8:44437\n\
            a _adb-tls-pairing._tcp 192.168.50.17:0\n\
            a _adb-tls-pairing._tcp 192.168.50.17:70000\n\
            a _adb-tls-pairing._tcp 192.168.50.17\n\
            a _adb-tls-pairing._tcp pas-une-ip:1234\n\
            a;rm _adb-tls-pairing._tcp 192.168.50.18:1111\n\
            a _autre._tcp 192.168.50.19:2222\n\
            bon _adb-tls-connect._tcp 192.168.50.20:3333\n\
            bon _adb-tls-connect._tcp 192.168.50.20:3333\r\n";
        let s = analyser_mdns(sortie);
        assert!(s.appairage.is_empty(), "{:?}", s.appairage);
        assert_eq!(s.connexion, [ServiceMdns { instance: "bon".into(), adresse: sock("192.168.50.20:3333") }], "dédoublonné");
        assert_eq!(analyser_mdns(""), ServicesMdns::default());
        assert_eq!(analyser_mdns("\u{0}\u{1b}[0m binaire"), ServicesMdns::default());
    }

    #[test]
    fn code_six_chiffres_exactement() {
        for (saisie, attendu) in [("745385", Some("745385")), (" 745 385 ", Some("745385")), ("745\u{a0}385", Some("745385")), ("000000", Some("000000"))] {
            assert_eq!(valider_code(saisie).as_deref(), attendu, "{saisie:?}");
        }
        for mauvais in ["", "74538", "7453856", "74538a", "745-385", "٧٤٥٣٨٥", "７４５３８５", "74538\n5;id", "+74538"] {
            assert_eq!(valider_code(mauvais), None, "{mauvais:?}");
        }
    }

    #[test]
    fn erreurs_d_appairage_classees() {
        let c = classer_erreur_appairage;
        assert_eq!(c("error: protocol fault (couldn't read status message): Undefined error: 0"), ErreurAppairage::CodeRefuse);
        assert_eq!(c("Failed: Wrong password or password and guid mismatch"), ErreurAppairage::CodeRefuse);
        assert_eq!(c("error: connection refused"), ErreurAppairage::FenetreIntrouvable);
        assert_eq!(c("failed to connect to 192.168.50.17:44437"), ErreurAppairage::FenetreIntrouvable);
        assert_eq!(c("error: No route to host"), ErreurAppairage::FenetreIntrouvable);
        assert_eq!(c("error: connection timed out"), ErreurAppairage::Delai);
        assert_eq!(c("délai dépassé"), ErreurAppairage::Delai);
        assert_eq!(c("error: truc\u{1b}[0m inconnu"), ErreurAppairage::Autre("error: truc[0m inconnu".into()));
    }
}
