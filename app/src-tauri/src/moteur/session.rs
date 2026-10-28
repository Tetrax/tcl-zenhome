//! Socle des actions longues (installation, choix, vérification, restauration) : observateur, reprise sur coupure.
//!
//! Chaque action est une suite d'étapes ; chaque étape est une suite d'opérations IDEMPOTENTES (rejouables sans effet
//! en double). Une coupure du lien (`Coupure`, réponse vide, autorisation retirée) déclenche `assurer_connexion` puis la
//! reprise de l'opération en cours seulement. Quand `assurer_connexion` rend la main (box à réveiller, autorisation à
//! accepter), l'observateur choisit : réessayer, ou arrêter. Arrêter donne `Arret::EnPause(Pause)` : l'appelant relance
//! la même action en lui passant `Pause::reprise` pour repartir de l'étape interrompue.
//!
//! Aucune dépendance à Tauri : `Evenement` est `Serialize`, un `tauri::ipc::Channel` s'y branchera dans `notifier`.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::connexion::{EtatConnexion, Politique, assurer_connexion};
use super::transport::{Commande, ErreurTransport, Sortie, Transport};

/// Essais d'une même opération face à des coupures (comme les 3 passes de `lancer_script` du kit).
const ESSAIS_OPERATION: u32 = 3;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Evenement {
    Etape { numero: usize, total: usize, nom: String },
    Progression { fait: usize, total: usize, detail: String },
    /// Le lien a décroché pendant une opération ; `essai` = numéro de la reprise de cette opération.
    ConnexionPerdue { essai: u32 },
    Reconnectee,
    /// `assurer_connexion` a rendu la main : l'observateur va devoir décider (`attente_utilisateur`).
    AttenteUtilisateur { non_autorise: bool },
    Avertissement { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Decision {
    Reessayer,
    Arreter,
}

pub trait Observateur {
    fn notifier(&mut self, evenement: Evenement);
    /// La box ne répond plus malgré les reconnexions. Par défaut : arrêter (action mise en pause).
    fn attente_utilisateur(&mut self, _non_autorise: bool) -> Decision {
        Decision::Arreter
    }
}

/// Observateur qui ignore tout et arrête à la première attente utilisateur.
pub struct Silencieux;

impl Observateur for Silencieux {
    fn notifier(&mut self, _evenement: Evenement) {}
}

/// Où repartir. `etape` : première étape à rejouer ; `desactives` : paquets désactivés par le passage interrompu
/// (la vérification en a besoin pour son retour arrière).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Reprise {
    pub etape: usize,
    pub desactives: Vec<String>,
}

/// État « en pause » : l'action s'est arrêtée proprement, rien n'est à défaire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pause {
    pub action: String,
    pub etape: String,
    pub operation: String,
    pub non_autorise: bool,
    pub reprise: Reprise,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErreurAction {
    Transport(ErreurTransport),
    /// Choix ou paramètre refusé avant tout envoi.
    Refus(String),
    ApkAbsent(String),
    /// `INSTALL_FAILED_UPDATE_INCOMPATIBLE` : rien n'a été désinstallé, l'utilisateur doit décider.
    SignatureIncompatible,
    Installation(String),
    /// Les choix n'ont pas pu être relus sur la box.
    Memorisation(String),
}

impl fmt::Display for ErreurAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "{e}"),
            Self::Refus(m) => write!(f, "refusé : {m}"),
            Self::ApkAbsent(p) => write!(f, "APK absent : {p}"),
            Self::SignatureIncompatible => write!(f, "signature différente de la version installée (INSTALL_FAILED_UPDATE_INCOMPATIBLE)"),
            Self::Installation(m) => write!(f, "échec de l'installation : {m}"),
            Self::Memorisation(m) => write!(f, "choix non mémorisés sur la box : {m}"),
        }
    }
}

impl std::error::Error for ErreurAction {}

#[derive(Debug, Clone, PartialEq)]
pub enum Arret {
    EnPause(Pause),
    Echec(ErreurAction),
}

impl fmt::Display for Arret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EnPause(p) => write!(f, "en pause ({} : {})", p.etape, p.operation),
            Self::Echec(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Arret {}

impl From<ErreurTransport> for Arret {
    fn from(e: ErreurTransport) -> Self {
        Self::Echec(ErreurAction::Transport(e))
    }
}

impl From<ErreurAction> for Arret {
    fn from(e: ErreurAction) -> Self {
        Self::Echec(e)
    }
}

pub struct Session<'a> {
    t: &'a dyn Transport,
    pol: &'a Politique,
    obs: &'a mut dyn Observateur,
    action: &'static str,
    total: usize,
    etape_nom: String,
    /// Reprise à renvoyer si l'action est mise en pause maintenant.
    pub reprise: Reprise,
    pub avertissements: Vec<String>,
}

impl<'a> Session<'a> {
    pub fn nouvelle(
        t: &'a dyn Transport,
        pol: &'a Politique,
        obs: &'a mut dyn Observateur,
        action: &'static str,
        total: usize,
        depuis: &Reprise,
    ) -> Self {
        Self { t, pol, obs, action, total, etape_nom: String::new(), reprise: depuis.clone(), avertissements: Vec::new() }
    }

    /// Entre dans l'étape `numero` (0, 1, ...). Faux si elle est déjà faite (reprise) : l'appelant la saute.
    pub fn etape(&mut self, numero: usize, nom: &str) -> bool {
        if numero < self.reprise.etape {
            return false;
        }
        self.reprise.etape = numero;
        self.etape_nom = nom.to_string();
        self.obs.notifier(Evenement::Etape { numero: numero + 1, total: self.total, nom: nom.to_string() });
        true
    }

    pub fn progression(&mut self, fait: usize, total: usize, detail: &str) {
        self.obs.notifier(Evenement::Progression { fait, total, detail: detail.to_string() });
    }

    pub fn avertir(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.obs.notifier(Evenement::Avertissement { message: message.clone() });
        self.avertissements.push(message);
    }

    pub fn attendre(&self, duree: std::time::Duration) {
        self.t.attendre(duree);
    }

    fn pause(&self, operation: &str, non_autorise: bool) -> Pause {
        Pause {
            action: self.action.to_string(),
            etape: self.etape_nom.clone(),
            operation: operation.to_string(),
            non_autorise,
            reprise: self.reprise.clone(),
        }
    }

    /// Rétablit le lien ; si `assurer_connexion` rend la main, demande à l'observateur.
    fn rattraper(&mut self, operation: &str) -> Result<(), Arret> {
        loop {
            match assurer_connexion(self.t, self.pol) {
                EtatConnexion::Connectee => {
                    self.obs.notifier(Evenement::Reconnectee);
                    return Ok(());
                }
                EtatConnexion::AttenteUtilisateur { non_autorise } => {
                    self.obs.notifier(Evenement::AttenteUtilisateur { non_autorise });
                    if self.obs.attente_utilisateur(non_autorise) == Decision::Arreter {
                        return Err(Arret::EnPause(self.pause(operation, non_autorise)));
                    }
                }
            }
        }
    }

    /// Exécute une opération idempotente ; la rejoue après reconnexion si le lien a décroché.
    /// Les autres erreurs (commande en échec, valeur refusée) sont renvoyées telles quelles.
    pub fn op<T>(&mut self, nom: &str, mut f: impl FnMut(&dyn Transport) -> Result<T, ErreurTransport>) -> Result<T, Arret> {
        let mut essai = 0;
        loop {
            let e = match f(self.t) {
                Ok(v) => return Ok(v),
                Err(e) => e,
            };
            let autorisation = e == ErreurTransport::NonAutorise;
            if !(autorisation || matches!(e, ErreurTransport::Coupure(_) | ErreurTransport::Vide)) {
                return Err(e.into());
            }
            essai += 1;
            if essai >= ESSAIS_OPERATION && !autorisation {
                return Err(e.into());
            }
            self.obs.notifier(Evenement::ConnexionPerdue { essai });
            self.rattraper(nom)?;
        }
    }

    /// Lecture à la manière d'`adb_lire` : une commande en échec donne un texte vide.
    pub fn lire(&mut self, cmd: &Commande) -> Result<String, Arret> {
        match self.op(cmd.afficher(), |t| t.shell(cmd)) {
            Ok(s) => Ok(s.stdout),
            Err(Arret::Echec(ErreurAction::Transport(ErreurTransport::Commande { .. }))) => Ok(String::new()),
            Err(e) => Err(e),
        }
    }

    /// Lecture dont une réponse vide est un lien instable : relue après reconnexion ; `None` si elle reste vide.
    pub fn lire_non_vide(&mut self, cmd: &Commande) -> Result<Option<String>, Arret> {
        match self.op(cmd.afficher(), |t| t.shell(cmd)?.non_vide()) {
            Ok(s) => Ok(Some(s.stdout)),
            Err(Arret::Echec(ErreurAction::Transport(ErreurTransport::Commande { .. } | ErreurTransport::Vide))) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Commande qui modifie la box (`adb_tv` du kit) : un échec de la commande est une erreur.
    pub fn ecrire(&mut self, cmd: &Commande) -> Result<Sortie, Arret> {
        self.op(cmd.afficher(), |t| t.shell(cmd))
    }

    /// `adb_essayer` du kit : un refus ou une sortie « Exception/Error/Unknown » devient un avertissement.
    /// Renvoie vrai si la commande a abouti sans avertissement.
    pub fn essayer(&mut self, cmd: &Commande) -> Result<bool, Arret> {
        match self.ecrire(cmd) {
            Ok(s) if ["Exception", "Error", "Unknown"].iter().any(|m| s.stdout.contains(m) || s.stderr.contains(m)) => {
                let premiere = s.stdout.lines().chain(s.stderr.lines()).next().unwrap_or_default().to_string();
                self.avertir(format!("refusé ou sans effet : {cmd} ({premiere})"));
                Ok(false)
            }
            Ok(_) => Ok(true),
            Err(Arret::Echec(ErreurAction::Transport(ErreurTransport::Commande { stderr, .. }))) => {
                let premiere = stderr.lines().next().unwrap_or_default().to_string();
                self.avertir(format!("refusé ou sans effet : {cmd} ({premiere})"));
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }
}
