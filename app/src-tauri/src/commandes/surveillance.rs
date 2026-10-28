//! Garde-fous d'une action : une seule à la fois, annulation, délai global, attente d'une décision de l'utilisateur.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::moteur::session::Decision;
use crate::moteur::transport::{Commande, ErreurTransport, EtatLien, Sortie, Transport};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interruption {
    Annulee,
    Delai,
}

/// Drapeau d'annulation (commun à toute l'appli) et échéance de l'action en cours.
#[derive(Clone)]
pub struct Surveillance {
    annule: Arc<AtomicBool>,
    fin: Instant,
}

impl Surveillance {
    pub fn nouvelle(annule: Arc<AtomicBool>, delai: Duration) -> Self {
        Self { annule, fin: Instant::now() + delai }
    }

    pub fn interruption(&self) -> Option<Interruption> {
        if self.annule.load(Ordering::SeqCst) {
            Some(Interruption::Annulee)
        } else if Instant::now() >= self.fin {
            Some(Interruption::Delai)
        } else {
            None
        }
    }
}

/// Transport qui refuse tout nouvel envoi une fois l'action annulée ou son délai dépassé.
/// Couvre les lectures qui ne passent pas par `Session` (inventaire, connexion) ; `Session` s'arrête proprement
/// de son côté (`Observateur::annule`).
pub struct Surveille<'a> {
    pub t: &'a dyn Transport,
    pub surveillance: &'a Surveillance,
}

impl Surveille<'_> {
    fn controler(&self) -> Result<(), ErreurTransport> {
        match self.surveillance.interruption() {
            Some(_) => Err(ErreurTransport::Refusee("action interrompue".into())),
            None => Ok(()),
        }
    }
}

impl Transport for Surveille<'_> {
    fn shell(&self, cmd: &Commande) -> Result<Sortie, ErreurTransport> {
        self.controler()?;
        self.t.shell(cmd)
    }
    fn shell_stdin(&self, cmd: &Commande) -> Result<Sortie, ErreurTransport> {
        self.controler()?;
        self.t.shell_stdin(cmd)
    }
    fn install(&self, apk: &Path) -> Result<Sortie, ErreurTransport> {
        self.controler()?;
        self.t.install(apk)
    }
    fn uninstall(&self, paquet: &str) -> Result<Sortie, ErreurTransport> {
        self.controler()?;
        self.t.uninstall(paquet)
    }
    fn etat(&self) -> Result<EtatLien, ErreurTransport> {
        self.controler()?;
        self.t.etat()
    }
    fn connecter(&self) -> Result<(), ErreurTransport> {
        self.controler()?;
        self.t.connecter()
    }
    fn deconnecter(&self) {
        // une action interrompue ne remue plus le lien (la fin de session passe par le transport lui-même)
        if self.surveillance.interruption().is_none() {
            self.t.deconnecter();
        }
    }
    fn arreter_serveur(&self) {
        self.t.arreter_serveur();
    }
    fn attendre(&self, duree: Duration) {
        if self.surveillance.interruption().is_none() {
            self.t.attendre(duree);
        }
    }
}

/// Une seule action à la fois : le garde se libère à la fin de l'action (y compris en cas de panique).
pub struct Occupation<'a>(&'a AtomicBool);

impl<'a> Occupation<'a> {
    pub fn prendre(drapeau: &'a AtomicBool) -> Option<Self> {
        drapeau.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).ok().map(|_| Self(drapeau))
    }
}

impl Drop for Occupation<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

#[derive(Default)]
struct Demande {
    prochaine: u64,
    /// (identifiant, décision reçue) de la demande ouverte, s'il y en a une.
    ouverte: Option<(u64, Option<Decision>)>,
}

/// Rendez-vous entre l'action (qui attend) et `decider` (qui répond). Chaque demande a un identifiant à usage unique.
#[derive(Default)]
pub struct Attente {
    demande: Mutex<Demande>,
    signal: Condvar,
}

impl Attente {
    fn verrou(&self) -> std::sync::MutexGuard<'_, Demande> {
        self.demande.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Ouvre une demande (à faire AVANT d'en informer l'interface, pour qu'une réponse rapide trouve son rendez-vous).
    pub fn ouvrir(&self) -> u64 {
        let mut d = self.verrou();
        d.prochaine += 1;
        let id = d.prochaine;
        d.ouverte = Some((id, None));
        id
    }

    /// Réponse de l'interface : acceptée une seule fois, et seulement pour la demande ouverte.
    pub fn repondre(&self, id: u64, decision: Decision) -> bool {
        let mut d = self.verrou();
        match d.ouverte.as_mut() {
            Some((ouvert, recue @ None)) if *ouvert == id => {
                *recue = Some(decision);
                self.signal.notify_all();
                true
            }
            _ => false,
        }
    }

    /// Attend la réponse à `id` ; sans réponse dans `delai`, ou si l'action est interrompue : arrêter.
    pub fn attendre(&self, id: u64, delai: Duration, interrompue: impl Fn() -> bool) -> Decision {
        let fin = Instant::now() + delai;
        let mut d = self.verrou();
        let decision = loop {
            if let Some((_, Some(decision))) = d.ouverte.filter(|(ouvert, _)| *ouvert == id) {
                break decision;
            }
            let reste = fin.saturating_duration_since(Instant::now());
            if reste.is_zero() || interrompue() {
                break Decision::Arreter;
            }
            d = self.signal.wait_timeout(d, reste.min(Duration::from_millis(100))).unwrap_or_else(|e| e.into_inner()).0;
        };
        if d.ouverte.is_some_and(|(ouvert, _)| ouvert == id) {
            d.ouverte = None;
        }
        decision
    }
}
