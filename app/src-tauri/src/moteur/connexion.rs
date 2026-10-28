//! Connexion à la box avec la politique de reprise du kit (`connecter`, `assurer_connexion`).
//! Le lien adb de la Mi Box décroche souvent (« offline », lectures vides) : on repart d'un lien neuf à chaque reprise.

use std::time::Duration;

use super::transport::{Commande, ErreurTransport, EtatLien, Transport};

/// `ro.product.name` de la Xiaomi Mi Box 4 (seul appareil vérifié).
pub const PRODUIT_MIBOX4: &str = "oneday";

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
    /// Ce n'est pas une Mi Box 4 : on s'arrête sans rien modifier. Contient le nom nettoyé.
    ModeleDifferent(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EtatConnexion {
    Connectee,
    /// Reconnexions épuisées : l'utilisateur doit réveiller la box (ou accepter l'autorisation) puis réessayer.
    AttenteUtilisateur { non_autorise: bool },
}

/// Modèle affichable : lettres, chiffres, `._ -` et 40 caractères au plus.
fn nettoyer_modele(brut: &str) -> String {
    brut.chars().filter(|c| c.is_ascii_alphanumeric() || "._ -".contains(*c)).take(40).collect()
}

/// Connexion à partir de zéro : déconnexion préalable, `connect` (jusqu'à 5 essais), lecture du modèle (3 essais :
/// vide = lecture ratée, pas un autre modèle), refus si ce n'est pas une Mi Box 4. Renvoie `ro.product.name`.
pub fn connecter_box(t: &dyn Transport, pol: &Politique) -> Result<String, EchecConnexion> {
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
    let lecture = Commande::getprop("ro.product.name").expect("propriété constante valide");
    let mut produit = String::new();
    for essai in 1..=pol.essais_modele {
        match t.shell(&lecture) {
            Ok(s) if !s.stdout.trim().is_empty() => {
                produit = s.stdout.trim().to_string();
                break;
            }
            Err(ErreurTransport::NonAutorise) => return Err(EchecConnexion::NonAutorise),
            _ => {}
        }
        if essai < pol.essais_modele {
            t.attendre(pol.pause);
            let _ = t.connecter();
        }
    }
    if produit.is_empty() {
        return Err(EchecConnexion::LectureModele);
    }
    if produit != PRODUIT_MIBOX4 {
        return Err(EchecConnexion::ModeleDifferent(nettoyer_modele(&produit)));
    }
    Ok(produit)
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
        assert_eq!(connecter_box(&f, &Politique::default()), Ok("oneday".to_string()));
        assert_eq!(f.journal(), ["disconnect", "connect", "etat", "shell: getprop ro.product.name"]);
    }

    #[test]
    fn modele_different_refuse_et_nettoye() {
        let f = FakeBox::mibox4();
        f.modele("G08_4K_GB\u{7}; rm -rf /");
        let r = connecter_box(&f, &Politique::default());
        assert_eq!(r, Err(EchecConnexion::ModeleDifferent("G08_4K_GB rm -rf ".to_string())));
        // rien d'autre que la lecture du modèle n'a été envoyé
        assert_eq!(f.journal().iter().filter(|l| l.starts_with("shell:")).count(), 1);
    }

    #[test]
    fn autorisation_en_attente_puis_acceptee() {
        let f = FakeBox::mibox4();
        f.autorisation_en_attente();
        assert_eq!(connecter_box(&f, &Politique::default()), Err(EchecConnexion::NonAutorise));
        assert!(f.journal().iter().all(|l| !l.starts_with("shell:")), "aucune commande avant l'autorisation");
        f.accepter_autorisation();
        assert_eq!(connecter_box(&f, &Politique::default()), Ok("oneday".to_string()));
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
        assert_eq!(connecter_box(&f, &Politique::default()), Ok("oneday".to_string()));
        assert_eq!(f.journal().iter().filter(|l| l.starts_with("shell:")).count(), 3);
        // vide = lecture ratée, jamais « un autre modèle »
        let g = FakeBox::mibox4();
        g.lectures_vides(3);
        assert_eq!(connecter_box(&g, &Politique::default()), Err(EchecConnexion::LectureModele));
    }

    #[test]
    fn coupure_pendant_la_lecture_du_modele_puis_reprise() {
        let f = FakeBox::mibox4();
        f.couper(1);
        assert_eq!(connecter_box(&f, &Politique::default()), Ok("oneday".to_string()));
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
}
