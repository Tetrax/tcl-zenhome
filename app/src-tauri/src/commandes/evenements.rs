//! Événements envoyés à l'interface (`Channel<EvenementUi>`, forme fixée par CONTRAT.md) et observateur du moteur.

use std::time::Duration;

use serde::Serialize;

use super::surveillance::{Attente, Surveillance};
use crate::moteur::session::{Decision, Evenement, Observateur, Reprise};
use crate::moteur::transport::assainir;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Raison {
    Injoignable,
    Autorisation,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvenementUi {
    Etape { nom: String },
    Progression { fraction: f64 },
    ConnexionPerdue { essai: u32 },
    Reconnectee,
    AttenteUtilisateur { demande: u64, raison: Raison },
    Avertissement { texte: String },
}

/// Où partent les événements : un `Channel` en production, un enregistreur dans les tests.
pub type Emetteur<'a> = &'a (dyn Fn(EvenementUi) + Send + Sync);

pub struct ObservateurUi<'a> {
    pub emettre: Emetteur<'a>,
    pub attente: &'a Attente,
    pub surveillance: &'a Surveillance,
    pub delai_decision: Duration,
    /// Conserve la reprise (disque) à chaque changement ; un échec devient un avertissement.
    pub conserver: &'a (dyn Fn(&Reprise) -> std::io::Result<()> + Send + Sync),
}

impl ObservateurUi<'_> {
    /// Demande à l'utilisateur de réveiller la box ou d'accepter l'autorisation, et attend sa réponse.
    pub fn demander(&self, non_autorise: bool) -> Decision {
        let id = self.attente.ouvrir();
        (self.emettre)(EvenementUi::AttenteUtilisateur { demande: id, raison: if non_autorise { Raison::Autorisation } else { Raison::Injoignable } });
        self.attente.attendre(id, self.delai_decision, || self.surveillance.interruption().is_some())
    }
}

impl Observateur for ObservateurUi<'_> {
    fn notifier(&mut self, evenement: Evenement) {
        let ui = match evenement {
            Evenement::Etape { nom, .. } => EvenementUi::Etape { nom: assainir(&nom) },
            Evenement::Progression { fait, total, .. } if total > 0 => EvenementUi::Progression { fraction: (fait as f64 / total as f64).clamp(0.0, 1.0) },
            Evenement::Progression { .. } => return,
            Evenement::ConnexionPerdue { essai } => EvenementUi::ConnexionPerdue { essai },
            Evenement::Reconnectee => EvenementUi::Reconnectee,
            // l'attente est signalée par `attente_utilisateur`, avec son identifiant de demande
            Evenement::AttenteUtilisateur { .. } => return,
            Evenement::Avertissement { message } => EvenementUi::Avertissement { texte: assainir(&message) },
        };
        (self.emettre)(ui);
    }

    fn attente_utilisateur(&mut self, non_autorise: bool) -> Decision {
        self.demander(non_autorise)
    }

    fn annule(&self) -> bool {
        self.surveillance.interruption().is_some()
    }

    fn reprise_changee(&mut self, reprise: &Reprise) {
        if (self.conserver)(reprise).is_err() {
            (self.emettre)(EvenementUi::Avertissement {
                texte: "La reprise n'a pas pu être enregistrée : si l'application se ferme, le retour arrière ne sera pas possible.".into(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forme_des_evenements_selon_le_contrat() {
        let json = |e: EvenementUi| serde_json::to_string(&e).unwrap();
        assert_eq!(json(EvenementUi::Etape { nom: "Contrôle".into() }), r#"{"type":"etape","nom":"Contrôle"}"#);
        assert_eq!(json(EvenementUi::Progression { fraction: 0.5 }), r#"{"type":"progression","fraction":0.5}"#);
        assert_eq!(json(EvenementUi::ConnexionPerdue { essai: 2 }), r#"{"type":"connexion_perdue","essai":2}"#);
        assert_eq!(json(EvenementUi::Reconnectee), r#"{"type":"reconnectee"}"#);
        assert_eq!(
            json(EvenementUi::AttenteUtilisateur { demande: 7, raison: Raison::Autorisation }),
            r#"{"type":"attente_utilisateur","demande":7,"raison":"autorisation"}"#
        );
        assert_eq!(json(EvenementUi::Avertissement { texte: "x".into() }), r#"{"type":"avertissement","texte":"x"}"#);
    }
}
