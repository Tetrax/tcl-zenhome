//! Erreurs renvoyées à l'interface : `{ code, message }`, message en français prêt à afficher (voir CONTRAT.md).

use serde::Serialize;

use super::surveillance::Interruption;
use crate::moteur::connexion::EchecConnexion;
use crate::moteur::inventaire::ErreurInventaire;
use crate::moteur::session::{Arret, ErreurAction};
use crate::moteur::transport::{ErreurTransport, assainir};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ErreurCommande {
    pub code: &'static str,
    pub message: String,
}

impl ErreurCommande {
    /// Tout message passe par `assainir` : un texte venant de la box ou de l'interface reste borné et sans contrôles.
    pub fn nouvelle(code: &'static str, message: impl AsRef<str>) -> Self {
        Self { code, message: assainir(message.as_ref()) }
    }

    pub fn occupe() -> Self {
        Self::nouvelle("occupe", "Une autre action est déjà en cours : attendez qu'elle se termine.")
    }

    pub fn interne(message: impl AsRef<str>) -> Self {
        Self::nouvelle("interne", message)
    }

    pub fn aucune_box() -> Self {
        Self::nouvelle("injoignable", "Aucune box n'est connectée : connectez-vous d'abord.")
    }

    pub fn annulee() -> Self {
        Self::nouvelle("annule", "Action interrompue à votre demande. Vous pouvez la relancer.")
    }

    pub fn delai() -> Self {
        Self::interne("L'action a pris trop de temps et a été arrêtée. Vous pouvez la relancer.")
    }

    pub fn non_autorise() -> Self {
        Self::nouvelle(
            "non_autorise",
            "La box attend votre autorisation : acceptez « Toujours autoriser cet ordinateur » sur la TV, puis réessayez.",
        )
    }

    pub fn injoignable() -> Self {
        Self::nouvelle("injoignable", "La box ne répond pas. Vérifiez qu'elle est allumée (réveillez-la avec la télécommande) et sur le même Wi-Fi.")
    }

    /// Arrêt d'une action du moteur. `interruption` : annulation ou délai demandés par nous ; ils priment, car la
    /// pause ou l'échec qui en résulte n'en est que la conséquence.
    pub fn depuis_arret(a: Arret, interruption: Option<Interruption>) -> Self {
        match (interruption, a) {
            (Some(Interruption::Annulee), _) => Self::annulee(),
            (Some(Interruption::Delai), _) => Self::delai(),
            (None, Arret::EnPause(p)) if p.non_autorise => Self::non_autorise(),
            (None, Arret::EnPause(_)) => Self::injoignable(),
            (None, Arret::Echec(e)) => e.into(),
        }
    }
}

impl From<ErreurTransport> for ErreurCommande {
    fn from(e: ErreurTransport) -> Self {
        match e {
            ErreurTransport::NonAutorise => Self::non_autorise(),
            ErreurTransport::Coupure(_) | ErreurTransport::Vide => Self::injoignable(),
            ErreurTransport::Lancement(_) => Self::nouvelle("adb_absent", "adb ne peut pas être lancé : réinstallez-le depuis l'écran de démarrage."),
            ErreurTransport::Refusee(m) => Self::nouvelle("choix_invalide", format!("Valeur refusée : {m}")),
            ErreurTransport::Commande { stderr, .. } => Self::interne(format!("La box a refusé une commande : {stderr}")),
        }
    }
}

impl From<ErreurAction> for ErreurCommande {
    fn from(e: ErreurAction) -> Self {
        match e {
            ErreurAction::Transport(t) => t.into(),
            ErreurAction::Refus(m) => Self::nouvelle("choix_invalide", format!("Choix refusé : {m}")),
            ErreurAction::ApkAbsent(_) => Self::interne("Le fichier de Zen Home est introuvable dans l'application."),
            ErreurAction::SignatureIncompatible => Self::interne(
                "Zen Home est déjà installée avec une autre signature. Désinstallez-la sur la box (Paramètres, Applications), puis recommencez.",
            ),
            ErreurAction::Installation(m) => Self::interne(format!("L'installation a échoué : {m}")),
            ErreurAction::Memorisation(_) => Self::nouvelle("lecture", "Vos choix n'ont pas pu être relus sur la box : réessayez."),
            ErreurAction::AutreBox => Self::nouvelle("autre_box", "Ce n'est plus la même box : arrêt, rien n'a été envoyé. Reconnectez-vous à la bonne box."),
        }
    }
}

impl From<ErreurInventaire> for ErreurCommande {
    fn from(e: ErreurInventaire) -> Self {
        match e {
            ErreurInventaire::AttenteUtilisateur { non_autorise: true } => Self::non_autorise(),
            ErreurInventaire::AttenteUtilisateur { .. } => Self::injoignable(),
            ErreurInventaire::Illisible(_) => Self::nouvelle("lecture", "La box a répondu de façon illisible : redémarrez-la puis réessayez."),
            ErreurInventaire::Transport(t) => t.into(),
            ErreurInventaire::Ecriture(m) => Self::interne(format!("Impossible d'écrire la sauvegarde : {m}")),
        }
    }
}

impl From<EchecConnexion> for ErreurCommande {
    fn from(e: EchecConnexion) -> Self {
        match e {
            EchecConnexion::NonAutorise => Self::non_autorise(),
            EchecConnexion::Injoignable(_) => Self::injoignable(),
            EchecConnexion::LectureModele => {
                Self::nouvelle("lecture", "La box ne répond pas correctement : redémarrez-la (débranchez-la quelques secondes) puis réessayez.")
            }
            EchecConnexion::ModeleDifferent(_) => Self::nouvelle("autre_modele", "Cet appareil n'est pas encore pris en charge. Rien n'a été modifié."),
        }
    }
}

impl From<zh_reseau::Erreur> for ErreurCommande {
    fn from(e: zh_reseau::Erreur) -> Self {
        match e {
            zh_reseau::Erreur::VilleVide => Self::nouvelle("choix_invalide", "Saisissez le nom d'une ville."),
            zh_reseau::Erreur::Io(m) => Self::interne(format!("Erreur de fichier : {m}")),
            zh_reseau::Erreur::PlateformeNonSupportee => Self::interne("Cette plateforme n'est pas prise en charge."),
            autre => Self::nouvelle("reseau", format!("Échec réseau : {autre}")),
        }
    }
}
