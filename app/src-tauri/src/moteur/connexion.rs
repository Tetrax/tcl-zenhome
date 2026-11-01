//! Connexion à la box avec la politique de reprise du kit (`connecter`, `assurer_connexion`).
//! Le lien adb de la Mi Box décroche souvent (« offline », lectures vides) : on repart d'un lien neuf à chaque reprise.

use std::time::Duration;

use super::appareils::{Appareil, Proprietes, identifier};
use super::transport::{Commande, ErreurTransport, EtatLien, Transport};

#[derive(Debug, Clone)]
pub struct Politique {
    /// Pause entre deux tentatives (`PAUSE` du kit).
    pub pause: Duration,
    /// Tentatives de connexion initiale.
    pub essais_connexion: u32,
    /// Lectures du modèle.
    pub essais_modele: u32,
    /// Reconnexions de `assurer_connexion` avant de rendre la main à l'utilisateur.
    pub reconnexions: u32,
}

impl Default for Politique {
    fn default() -> Self {
        Self { pause: Duration::from_secs(3), essais_connexion: 5, essais_modele: 3, reconnexions: 5 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EchecConnexion {
    /// La box affiche une autorisation à accepter (« Toujours autoriser cet ordinateur ») : l'appelant affiche la consigne
    /// puis rappelle `connecter_box`.
    NonAutorise,
    /// Aucune réponse (éteinte, en veille, mauvaise IP, autre Wi-Fi...). Contient le dernier message d'adb.
    Injoignable(String),
    /// Lien établi mais modèle illisible (réponses vides) : box à redémarrer.
    LectureModele,
    /// Appareil non pris en charge : on s'arrête sans rien modifier. Contient le résumé nettoyé (`Inconnu::resume`).
    ModeleDifferent(String),
}

/// Appareil reconnu à la connexion.
#[derive(Debug, Clone, Copy)]
pub struct Connecte {
    pub appareil: &'static Appareil,
    /// Android plus récent que le dernier SDK testé : à signaler, pas à refuser.
    pub sdk_recent: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EtatConnexion {
    Connectee,
    /// Reconnexions épuisées : l'utilisateur doit réveiller la box (ou accepter l'autorisation) puis réessayer.
    AttenteUtilisateur { non_autorise: bool },
}

/// Modèle affichable : lettres, chiffres, `._ -` et 40 caractères au plus.
pub(super) fn nettoyer_modele(brut: &str) -> String {
    brut.chars().filter(|c| c.is_ascii_alphanumeric() || "._ -".contains(*c)).take(40).collect()
}

/// Numéro de série (`ro.serialno`) : 1 à 40 caractères parmi `A-Za-z0-9._-`, sinon illisible (`Vide`).
/// Mémorisé à la connexion (`Reprise::pour_box`), relu après chaque reconnexion et avant d'envoyer un secret ou un APK.
pub fn lire_serie(t: &dyn Transport) -> Result<String, ErreurTransport> {
    let lecture = Commande::getprop("ro.serialno").expect("propriété constante valide");
    let brut = t.shell(&lecture)?.stdout;
    let serie = brut.trim();
    if (1..=40).contains(&serie.len()) && serie.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)) {
        Ok(serie.to_string())
    } else {
        Err(ErreurTransport::Vide)
    }
}

/// Lit une propriété : une réponse vide est une lecture ratée (lien instable), relue après reconnexion.
fn lire_propriete(t: &dyn Transport, pol: &Politique, cle: &str) -> Result<String, EchecConnexion> {
    let lecture = Commande::getprop(cle).expect("propriété constante valide");
    for essai in 1..=pol.essais_modele {
        match t.shell(&lecture) {
            Ok(s) if !s.stdout.trim().is_empty() => return Ok(s.stdout.trim().to_string()),
            Err(ErreurTransport::NonAutorise) => return Err(EchecConnexion::NonAutorise),
            _ => {}
        }
        if essai < pol.essais_modele {
            t.attendre(pol.pause);
            let _ = t.connecter();
        }
    }
    Err(EchecConnexion::LectureModele)
}

/// Connexion à partir de zéro : déconnexion préalable, `connect` (jusqu'à 5 essais), lecture du nom, de la marque, du
/// device et du SDK (3 essais chacune : vide = lecture ratée, pas un autre modèle), refus si l'appareil n'est pas reconnu.
pub fn connecter_box(t: &dyn Transport, pol: &Politique) -> Result<Connecte, EchecConnexion> {
    t.deconnecter();
    let mut dernier = String::new();
    let mut lien = false;
    for _ in 0..pol.essais_connexion {
        match t.connecter() {
            Err(ErreurTransport::NonAutorise) => return Err(EchecConnexion::NonAutorise),
            Err(e) => dernier = e.to_string(),
            Ok(()) => match t.etat() {
                Ok(EtatLien::Device) => {
                    lien = true;
                    break;
                }
                Ok(EtatLien::NonAutorise) | Err(ErreurTransport::NonAutorise) => return Err(EchecConnexion::NonAutorise),
                Ok(autre) => dernier = format!("état du lien : {autre:?}"),
                Err(e) => dernier = e.to_string(),
            },
        }
        t.attendre(pol.pause);
    }
    if !lien {
        return Err(EchecConnexion::Injoignable(dernier));
    }
    let nom = lire_propriete(t, pol, "ro.product.name")?;
    let marque = lire_propriete(t, pol, "ro.product.brand")?;
    let device = lire_propriete(t, pol, "ro.product.device")?;
    let sdk = lire_propriete(t, pol, "ro.build.version.sdk")?;
    match identifier(&Proprietes { nom: &nom, marque: &marque, device: &device, sdk: &sdk }) {
        Ok(appareil) => Ok(Connecte { appareil, sdk_recent: appareil.plus_recent_que_teste(sdk.parse().unwrap_or_default()) }),
        Err(inconnu) => Err(EchecConnexion::ModeleDifferent(inconnu.resume)),
    }
}

/// Vérifie le lien et le rétablit si la box a décroché : jusqu'à `pol.reconnexions` cycles déconnexion + connexion.
/// Ensuite l'appelant demande à l'utilisateur de réveiller la box (« Réessayer » rappelle cette fonction).
/// Une autorisation en attente rend la main tout de suite : réessayer sans l'utilisateur ne servirait à rien.
pub fn assurer_connexion(t: &dyn Transport, pol: &Politique) -> EtatConnexion {
    match t.etat() {
        Ok(EtatLien::Device) => return EtatConnexion::Connectee,
        Ok(EtatLien::NonAutorise) | Err(ErreurTransport::NonAutorise) => {
            return EtatConnexion::AttenteUtilisateur { non_autorise: true };
        }
        _ => {}
    }
    for _ in 0..pol.reconnexions {
        t.deconnecter();
        if matches!(t.connecter(), Err(ErreurTransport::NonAutorise)) {
            return EtatConnexion::AttenteUtilisateur { non_autorise: true };
        }
        t.attendre(pol.pause);
        match t.etat() {
            Ok(EtatLien::Device) => return EtatConnexion::Connectee,
            Ok(EtatLien::NonAutorise) | Err(ErreurTransport::NonAutorise) => {
                return EtatConnexion::AttenteUtilisateur { non_autorise: true };
            }
            _ => {}
        }
    }
    EtatConnexion::AttenteUtilisateur { non_autorise: false }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::fake_box::FakeBox;

    fn compte(f: &FakeBox, motif: &str) -> usize {
        f.journal().iter().filter(|l| l.as_str() == motif).count()
    }

    #[test]
    fn connexion_ok() {
        let f = FakeBox::mibox4();
        assert_eq!(connecter_box(&f, &Politique::default()).unwrap().appareil.id, "mibox4");
        assert_eq!(
            f.journal(),
            [
                "disconnect",
                "connect",
                "etat",
                "shell: getprop ro.product.name",
                "shell: getprop ro.product.brand",
                "shell: getprop ro.product.device",
                "shell: getprop ro.build.version.sdk"
            ]
        );
    }

    #[test]
    fn modele_different_refuse_et_nettoye() {
        let f = FakeBox::mibox4();
        f.modele("G08_4K_GB\u{7}; rm -rf /");
        let r = connecter_box(&f, &Politique::default());
        assert_eq!(r.unwrap_err(), EchecConnexion::ModeleDifferent("G08_4K_GB rm -rf  / Xiaomi / oneday / 28".to_string()));
        // rien d'autre que la lecture des propriétés n'a été envoyé
        assert!(f.journal().iter().filter(|l| l.starts_with("shell:")).all(|l| l.starts_with("shell: getprop ")));
    }

    #[test]
    fn autorisation_en_attente_puis_acceptee() {
        let f = FakeBox::mibox4();
        f.autorisation_en_attente();
        assert_eq!(connecter_box(&f, &Politique::default()).unwrap_err(), EchecConnexion::NonAutorise);
        assert!(f.journal().iter().all(|l| !l.starts_with("shell:")), "aucune commande avant l'autorisation");
        f.accepter_autorisation();
        assert!(connecter_box(&f, &Politique::default()).is_ok());
    }

    #[test]
    fn box_injoignable_apres_cinq_essais() {
        let f = FakeBox::mibox4();
        f.injoignable(true);
        let r = connecter_box(&f, &Politique::default());
        assert!(matches!(r, Err(EchecConnexion::Injoignable(m)) if m.contains("Connection refused")));
        assert_eq!(compte(&f, "connect"), 5);
    }

    #[test]
    fn lien_offline_persistant_est_injoignable() {
        let f = FakeBox::mibox4();
        f.perdre_lien(99);
        assert!(matches!(connecter_box(&f, &Politique::default()), Err(EchecConnexion::Injoignable(_))));
    }

    #[test]
    fn modele_vide_relu_trois_fois() {
        let f = FakeBox::mibox4();
        f.lectures_vides(2);
        assert!(connecter_box(&f, &Politique::default()).is_ok());
        assert_eq!(f.journal().iter().filter(|l| l.starts_with("shell:")).count(), 3 + 3, "3 lectures du nom, puis une par propriété");
        // vide = lecture ratée, jamais « un autre modèle »
        let g = FakeBox::mibox4();
        g.lectures_vides(3);
        assert_eq!(connecter_box(&g, &Politique::default()).unwrap_err(), EchecConnexion::LectureModele);
    }

    #[test]
    fn coupure_pendant_la_lecture_du_modele_puis_reprise() {
        let f = FakeBox::mibox4();
        f.couper(1);
        assert!(connecter_box(&f, &Politique::default()).is_ok());
        assert_eq!(compte(&f, "connect"), 2, "reconnexion entre deux lectures, comme le kit");
    }

    #[test]
    fn assurer_connexion_sans_rien_faire_si_ok() {
        let f = FakeBox::mibox4();
        connecter_box(&f, &Politique::default()).unwrap();
        let avant = f.journal().len();
        assert_eq!(assurer_connexion(&f, &Politique::default()), EtatConnexion::Connectee);
        assert_eq!(f.journal().len(), avant + 1, "un seul get-state");
    }

    #[test]
    fn assurer_connexion_reconnecte_apres_coupure() {
        let f = FakeBox::mibox4();
        connecter_box(&f, &Politique::default()).unwrap();
        f.perdre_lien(3);
        assert_eq!(assurer_connexion(&f, &Politique::default()), EtatConnexion::Connectee);
        assert_eq!(compte(&f, "connect"), 1 + 3);
    }

    #[test]
    fn assurer_connexion_rend_la_main_apres_cinq_reconnexions() {
        let f = FakeBox::mibox4();
        connecter_box(&f, &Politique::default()).unwrap();
        f.perdre_lien(6);
        let r = assurer_connexion(&f, &Politique::default());
        assert_eq!(r, EtatConnexion::AttenteUtilisateur { non_autorise: false });
        assert_eq!(compte(&f, "connect"), 1 + 5);
        // l'utilisateur réveille la box : le prochain appel réussit
        assert_eq!(assurer_connexion(&f, &Politique::default()), EtatConnexion::Connectee);
    }

    #[test]
    fn assurer_connexion_signale_une_autorisation_en_attente() {
        let f = FakeBox::mibox4();
        connecter_box(&f, &Politique::default()).unwrap();
        f.autorisation_en_attente();
        assert_eq!(assurer_connexion(&f, &Politique::default()), EtatConnexion::AttenteUtilisateur { non_autorise: true });
        assert_eq!(compte(&f, "connect"), 1, "pas de reconnexion inutile");
    }

    #[test]
    fn assurer_connexion_apres_deconnexion_totale() {
        let f = FakeBox::mibox4();
        assert_eq!(assurer_connexion(&f, &Politique::default()), EtatConnexion::Connectee);
    }

    #[test]
    fn la_tcl_est_reconnue_et_un_sdk_recent_est_signale() {
        let f = FakeBox::tcl_g08();
        let c = connecter_box(&f, &Politique::default()).unwrap();
        assert_eq!((c.appareil.id, c.sdk_recent), ("tcl-g08", false));
        f.modifier(|e| e.sdk = "35".into());
        let c = connecter_box(&f, &Politique::default()).unwrap();
        assert!(c.sdk_recent, "reconnue malgré un Android plus récent : simple avertissement");
    }

    #[test]
    fn marque_ou_sdk_inattendus_sont_refuses() {
        for (cle, valeur) in [("marque", "Autre"), ("device", "autre"), ("sdk", "27")] {
            let f = FakeBox::mibox4();
            f.modifier(|e| match cle {
                "marque" => e.marque = valeur.into(),
                "device" => e.device = valeur.into(),
                _ => e.sdk = valeur.into(),
            });
            assert!(matches!(connecter_box(&f, &Politique::default()), Err(EchecConnexion::ModeleDifferent(_))));
        }
    }
}
