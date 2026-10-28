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

use serde::Serialize;

use super::connexion::{EtatConnexion, Politique, assurer_connexion, lire_serie};
use super::transport::{Commande, ErreurTransport, Sortie, Transport, assainir};

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
    /// Vrai quand l'appelant demande l'arrêt (annulation, délai global) : l'opération suivante n'est pas lancée et
    /// l'action se met en pause.
    fn annule(&self) -> bool {
        false
    }
    /// La reprise a changé (paquet désactivé noté) : l'appelant peut la conserver pour qu'elle survive à une fermeture.
    fn reprise_changee(&mut self, _reprise: &Reprise) {}
}

/// Observateur qui ignore tout et arrête à la première attente utilisateur.
pub struct Silencieux;

impl Observateur for Silencieux {
    fn notifier(&mut self, _evenement: Evenement) {}
}

/// Où repartir. `etape` : première étape à rejouer ; `desactives` : paquets désactivés par le passage interrompu
/// (la vérification en a besoin pour son retour arrière) ; `serie` : numéro de série mémorisé de la box.
/// Jamais `Deserialize` : une reprise ne vient que du moteur, l'interface ne peut pas faire sauter d'étape.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Reprise {
    pub etape: usize,
    pub desactives: Vec<String>,
    pub serie: Option<String>,
}

impl Reprise {
    /// Départ d'une action sur une box précise : `lire_serie` juste après `connecter_box`.
    /// Sans numéro, il est mémorisé à la première vérification de l'action (voir `Session::verifier_identite`).
    pub fn pour_box(serie: String) -> Self {
        Self { serie: Some(serie), ..Self::default() }
    }
}

/// État « en pause » : l'action s'est arrêtée proprement, rien n'est à défaire.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
    /// Numéro de série différent de celui mémorisé, ou illisible : rien n'est envoyé.
    AutreBox,
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
            Self::AutreBox => write!(f, "ce n'est plus la même box (numéro de série différent ou illisible) : arrêt, rien n'est envoyé"),
        }
    }
}

impl std::error::Error for ErreurAction {}

#[derive(Debug, Clone, PartialEq)]
pub enum Arret {
    EnPause(Box<Pause>),
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

    /// Les avertissements peuvent citer du texte de la box : nettoyé et borné ici.
    pub fn avertir(&mut self, message: impl Into<String>) {
        let message = assainir(&message.into());
        self.obs.notifier(Evenement::Avertissement { message: message.clone() });
        self.avertissements.push(message);
    }

    /// Prévient l'observateur que `reprise` a changé.
    pub fn signaler_reprise(&mut self) {
        self.obs.reprise_changee(&self.reprise);
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

    /// Compare le numéro de série lu (`None` : illisible) au numéro mémorisé ; sans numéro mémorisé, le retient.
    fn comparer_serie(&mut self, lu: Option<String>) -> Result<(), Arret> {
        match (&self.reprise.serie, lu) {
            (Some(m), Some(l)) if *m == l => Ok(()),
            (None, Some(l)) => {
                self.reprise.serie = Some(l);
                Ok(())
            }
            _ => Err(ErreurAction::AutreBox.into()),
        }
    }

    /// Contrôle du numéro de série juste après une reconnexion : lecture directe (pas de `op`, on est déjà dans une reprise).
    /// Une coupure ou une autorisation retirée remontent telles quelles ; une lecture vide est relue.
    fn controler_serie(&mut self) -> Result<(), Arret> {
        let mut lu = None;
        for _ in 0..ESSAIS_OPERATION {
            match lire_serie(self.t) {
                Ok(s) => {
                    lu = Some(s);
                    break;
                }
                Err(ErreurTransport::Vide | ErreurTransport::Commande { .. }) => self.t.attendre(self.pol.pause),
                Err(e) => return Err(e.into()),
            }
        }
        self.comparer_serie(lu)
    }

    /// À appeler juste avant d'envoyer un secret ou un APK : est-ce toujours la même box ?
    pub fn verifier_identite(&mut self) -> Result<(), Arret> {
        let lu = match self.op("identité de la box", lire_serie) {
            Ok(s) => Some(s),
            Err(Arret::Echec(ErreurAction::Transport(ErreurTransport::Vide | ErreurTransport::Commande { .. }))) => None,
            Err(e) => return Err(e),
        };
        self.comparer_serie(lu)
    }

    /// Rétablit le lien (et vérifie que c'est la même box) ; si `assurer_connexion` rend la main, demande à l'observateur.
    fn rattraper(&mut self, operation: &str) -> Result<(), Arret> {
        let mut vains = 0;
        loop {
            let non_autorise = match assurer_connexion(self.t, self.pol) {
                EtatConnexion::Connectee => match self.controler_serie() {
                    Ok(()) => {
                        self.obs.notifier(Evenement::Reconnectee);
                        return Ok(());
                    }
                    Err(Arret::Echec(ErreurAction::Transport(ErreurTransport::NonAutorise))) => true,
                    Err(Arret::Echec(ErreurAction::Transport(ErreurTransport::Coupure(_)))) => {
                        // le lien est retombé aussitôt : on le rétablit à nouveau, sans boucler indéfiniment
                        vains += 1;
                        if vains < ESSAIS_OPERATION {
                            continue;
                        }
                        false
                    }
                    Err(e) => return Err(e),
                },
                EtatConnexion::AttenteUtilisateur { non_autorise } => non_autorise,
            };
            self.attente(operation, non_autorise)?;
        }
    }

    /// Demande à l'observateur : `Ok` pour réessayer, `EnPause` pour arrêter.
    fn attente(&mut self, operation: &str, non_autorise: bool) -> Result<(), Arret> {
        self.obs.notifier(Evenement::AttenteUtilisateur { non_autorise });
        if self.obs.attente_utilisateur(non_autorise) == Decision::Arreter {
            return Err(Arret::EnPause(Box::new(self.pause(operation, non_autorise))));
        }
        Ok(())
    }

    /// Exécute une opération idempotente ; la rejoue après reconnexion si le lien a décroché.
    /// Les autres erreurs (commande en échec, valeur refusée) sont renvoyées telles quelles.
    pub fn op<T>(&mut self, nom: &str, mut f: impl FnMut(&dyn Transport) -> Result<T, ErreurTransport>) -> Result<T, Arret> {
        let mut essai = 0;
        loop {
            if self.obs.annule() {
                return Err(Arret::EnPause(Box::new(self.pause(nom, false))));
            }
            let e = match f(self.t) {
                Ok(v) => return Ok(v),
                Err(e) => e,
            };
            let autorisation = e == ErreurTransport::NonAutorise;
            if !(autorisation || matches!(e, ErreurTransport::Coupure(_) | ErreurTransport::Vide)) {
                return Err(e.into());
            }
            essai += 1;
            if essai >= ESSAIS_OPERATION {
                if !autorisation {
                    return Err(e.into());
                }
                // autorisation toujours refusée alors que le lien semble bon : pas de boucle, l'utilisateur doit l'accepter
                self.attente(nom, true)?;
                essai = 0;
                continue;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::fake_box::FakeBox;
    use std::marker::PhantomData;

    #[test]
    fn autorisation_refusee_en_boucle_met_l_action_en_pause() {
        let f = FakeBox::mibox4();
        f.connecter().unwrap();
        f.shell_non_autorise(true);
        let (pol, mut obs) = (Politique::default(), Silencieux);
        let mut s = Session::nouvelle(&f, &pol, &mut obs, "test", 1, &Reprise::default());
        s.etape(0, "Essai");
        let r = s.ecrire(&Commande::touche_accueil());
        assert!(matches!(&r, Err(Arret::EnPause(p)) if p.non_autorise), "{r:?}");
        assert!(f.journal().iter().filter(|l| l.starts_with("shell:")).count() <= ESSAIS_OPERATION as usize);
    }

    /// `Reprise` et `Pause` ne doivent jamais être construites depuis l'extérieur (l'interface ne peut pas sauter d'étape).
    #[test]
    fn la_reprise_ne_se_deserialise_pas() {
        struct Sonde<T>(PhantomData<T>);
        trait Non {
            fn deserialisable(&self) -> bool {
                false
            }
        }
        impl<T> Non for Sonde<T> {}
        impl<T: serde::de::DeserializeOwned> Sonde<T> {
            fn deserialisable(&self) -> bool {
                true
            }
        }
        assert!(!Sonde::<Reprise>(PhantomData).deserialisable());
        assert!(!Sonde::<Pause>(PhantomData).deserialisable());
        assert!(Sonde::<Vec<u8>>(PhantomData).deserialisable(), "la sonde elle-même fonctionne");
    }
}
