//! Cœur des commandes : l'état géré par Rust (box découvertes, villes proposées, session connectée) et les actions.
//! Aucune dépendance à Tauri : `ipc.rs` n'est qu'une fine couche de `#[tauri::command]` au-dessus.
//!
//! Règles : l'interface n'envoie que des intentions (ids, noms de paquets recoupés par le moteur) ; une seule action à la
//! fois ; chaque action a un délai global et peut être annulée ; la reprise (paquets désactivés) survit à une fermeture.

use std::collections::HashMap;
use std::io::Write;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use sha2::{Digest, Sha256};
use zh_reseau::{BoxTrouvee, Ville};

use super::demo;
use super::desinstallation;
use super::erreur::ErreurCommande;
use super::evenements::{Emetteur, EvenementUi, ObservateurUi};
use super::{memoire, persistance, rapport};
use super::surveillance::{Attente, Interruption, Occupation, Surveillance, Surveille};
use super::types::*;
use crate::moteur::appareils::{self, Appareil};
use crate::moteur::choix::{self, Choix};
use crate::moteur::connexion::{Connecte, EchecConnexion, Politique, connecter_box, lire_serie};
use crate::moteur::installation::{self, Parametres};
use crate::moteur::inventaire::{self, ErreurInventaire, Inventaire, libelles_broadcast};
use crate::moteur::origine;
use crate::moteur::restauration;
use crate::moteur::session::{Arret, Decision, ErreurAction, Reprise, Session};
use crate::moteur::transport::{AdbProcess, ArgConfig, Commande, Espace, ErreurTransport, Transport};
use crate::moteur::verification;

include!(concat!(env!("OUT_DIR"), "/apk_empreinte.rs"));

const APK_TAILLE_MAX: u64 = 200 * 1024 * 1024;

pub type TransportPartage = Arc<dyn Transport + Send + Sync>;
/// Fabrique le transport vers la box à cette IP (adb réel, FakeBox en démo ou en test).
pub type Fabrique = Box<dyn Fn(Ipv4Addr) -> Result<TransportPartage, ErreurCommande> + Send + Sync>;

pub struct Config {
    pub version: String,
    /// Dossier de données de l'appli (`app_data_dir`, ou son sous-dossier `demo`).
    pub donnees: PathBuf,
    /// APK embarqué comme ressource du bundle.
    pub apk_ressource: PathBuf,
    /// Empreinte SHA-256 attendue de l'APK (constante compilée par build.rs).
    pub apk_sha256: String,
    pub demo: bool,
    pub fabrique: Fabrique,
    pub politique: Politique,
    pub delai_action: Duration,
    pub delai_decision: Duration,
    /// Durée des pauses artificielles de la démo (nulle dans les tests).
    pub rythme_demo: Duration,
    /// Jeton TMDB du projet, injecté à la compilation (`ZENHOME_TMDB_JETON`). Jamais renvoyé à l'interface ni journalisé.
    pub jeton_projet: Option<String>,
}

impl Config {
    pub fn reelle(version: String, donnees: PathBuf, apk_ressource: PathBuf) -> Self {
        let dossier = donnees.clone();
        let fabrique: Fabrique = Box::new(move |ip| {
            let adb = zh_reseau::adb_installe(&dossier)
                .ok_or_else(|| ErreurCommande::nouvelle("adb_absent", "adb n'est pas installé : installez-le depuis l'écran de démarrage."))?;
            Ok(Arc::new(AdbProcess::nouveau(adb, ip)) as TransportPartage)
        });
        let mut cfg = Self::commune(version, donnees, apk_ressource, APK_SHA256.to_string(), false, fabrique, Duration::ZERO);
        cfg.jeton_projet = option_env!("ZENHOME_TMDB_JETON").map(|j| j.trim().to_string()).filter(|j| !j.is_empty());
        cfg
    }

    pub fn demo(version: String, donnees: PathBuf, rythme: Duration) -> Self {
        let boxes = demo::BOXES.map(|(_, ip, serie)| (ip, demo::DemoBox::nouvelle(rythme, serie)));
        let fabrique: Fabrique = Box::new(move |ip| {
            let (_, b) = boxes.iter().find(|(i, _)| *i == ip.to_string()).unwrap_or(&boxes[0]);
            Ok(b.clone() as TransportPartage)
        });
        Self::commune(version, donnees.join("demo"), PathBuf::new(), String::new(), true, fabrique, rythme)
    }

    fn commune(version: String, donnees: PathBuf, apk_ressource: PathBuf, apk_sha256: String, demo: bool, fabrique: Fabrique, rythme_demo: Duration) -> Self {
        Self {
            version,
            donnees,
            apk_ressource,
            apk_sha256,
            demo,
            fabrique,
            politique: Politique::default(),
            delai_action: Duration::from_secs(30 * 60),
            delai_decision: Duration::from_secs(5 * 60),
            rythme_demo,
            jeton_projet: None,
        }
    }
}

struct SessionBox {
    t: TransportPartage,
    /// Appareil reconnu à la connexion : liste d'allègement, accueil, veille et paquets protégés en dépendent.
    appareil: &'static Appareil,
    /// Numéro de série et paquets désactivés par un passage interrompu (l'étape repart toujours de zéro : toutes les
    /// opérations du moteur sont rejouables, et un nouveau choix ne doit jamais sauter la mémorisation).
    reprise: Reprise,
    inventaire: Option<Inventaire>,
}

#[derive(Default)]
struct Etat {
    boxes: Vec<BoxTrouvee>,
    /// Id de la première box de `boxes` ; `prochain_id` : base de la prochaine recherche (un id n'est jamais réutilisé).
    base_id: u32,
    prochain_id: u32,
    villes: Vec<Ville>,
    adb_demo: bool,
    session: Option<SessionBox>,
}

pub struct Coeur {
    cfg: Config,
    etat: Mutex<Etat>,
    occupe: AtomicBool,
    annule: Arc<AtomicBool>,
    attente: Attente,
}

/// APK vérifié, prêt à installer : copie privée dont l'empreinte a été contrôlée (supprimée à la fin de l'action).
struct ApkVerifie {
    chemin: PathBuf,
    supprimer: bool,
}

impl Drop for ApkVerifie {
    fn drop(&mut self) {
        if self.supprimer {
            let _ = std::fs::remove_file(&self.chemin);
        }
    }
}

/// Ce qu'une action sur la box emprunte à la session pendant qu'elle tourne.
pub struct Contexte<'a> {
    coeur: &'a Coeur,
    t: TransportPartage,
    appareil: &'static Appareil,
    reprise: Reprise,
    inventaire: Option<Inventaire>,
    surveillance: Surveillance,
    emettre: Emetteur<'a>,
    conserver: &'a (dyn Fn(&Reprise) -> std::io::Result<()> + Send + Sync),
    delai_decision: Duration,
}

impl Contexte<'_> {
    fn transport(&self) -> Surveille<'_> {
        Surveille { t: &*self.t, surveillance: &self.surveillance }
    }

    fn observateur(&self) -> ObservateurUi<'_> {
        ObservateurUi {
            emettre: self.emettre,
            attente: &self.coeur.attente,
            surveillance: &self.surveillance,
            delai_decision: self.delai_decision,
            conserver: self.conserver,
        }
    }

    fn issue<T>(&self, r: Result<T, Arret>) -> Result<T, ErreurCommande> {
        r.map_err(|a| ErreurCommande::depuis_arret(a, self.surveillance.interruption()))
    }
}

fn interruption(s: &Surveillance) -> Option<ErreurCommande> {
    s.interruption().map(|i| match i {
        Interruption::Annulee => ErreurCommande::annulee(),
        Interruption::Delai => ErreurCommande::delai(),
    })
}

/// Fin de session côté adb : déconnexion puis arrêt du serveur dédié.
fn clore(t: &dyn Transport) {
    t.deconnecter();
    t.arreter_serveur();
}

/// Numéro de série réduit aux caractères d'un nom de dossier de sauvegarde.
fn nom_serie(r: &Reprise) -> String {
    r.serie.iter().flat_map(|s| s.chars()).filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_').collect()
}

fn sha256_hex(octets: &[u8]) -> String {
    Sha256::digest(octets).iter().map(|b| format!("{b:02x}")).collect()
}

impl Coeur {
    pub fn nouveau(cfg: Config) -> Self {
        if cfg.demo {
            demo::semer_origine(&cfg.donnees);
        }
        Self { cfg, etat: Mutex::default(), occupe: AtomicBool::new(false), annule: Arc::new(AtomicBool::new(false)), attente: Attente::default() }
    }

    fn etat(&self) -> MutexGuard<'_, Etat> {
        self.etat.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Une seule action à la fois : `occupe` sinon. Remet l'annulation à zéro et arme le délai global.
    pub(crate) fn prendre(&self) -> Result<(Occupation<'_>, Surveillance), ErreurCommande> {
        let o = Occupation::prendre(&self.occupe).ok_or_else(ErreurCommande::occupe)?;
        self.annule.store(false, Ordering::SeqCst);
        Ok((o, Surveillance::nouvelle(self.annule.clone(), self.cfg.delai_action)))
    }

    fn conserver(&self, r: &Reprise) -> std::io::Result<()> {
        if let Some(s) = self.etat().session.as_mut() {
            s.reprise.desactives.clone_from(&r.desactives);
        }
        persistance::ecrire(&self.cfg.donnees, r)
    }

    /// Action sur la box connectée : session empruntée, observateur branché sur le canal.
    fn agir<T>(&self, emettre: Emetteur, f: impl FnOnce(&Contexte) -> Result<T, ErreurCommande>) -> Result<T, ErreurCommande> {
        self.agir_avec(emettre, self.cfg.delai_decision, f)
    }

    /// `delai_decision` : temps laissé à l'utilisateur pour répondre à une demande (nul : on arrête aussitôt).
    fn agir_avec<T>(&self, emettre: Emetteur, delai_decision: Duration, f: impl FnOnce(&Contexte) -> Result<T, ErreurCommande>) -> Result<T, ErreurCommande> {
        let (_occupation, surveillance) = self.prendre()?;
        let (t, appareil, reprise, inventaire) = {
            let e = self.etat();
            let s = e.session.as_ref().ok_or_else(ErreurCommande::aucune_box)?;
            (s.t.clone(), s.appareil, s.reprise.clone(), s.inventaire.clone())
        };
        let conserver = |r: &Reprise| self.conserver(r);
        f(&Contexte { coeur: self, t, appareil, reprise, inventaire, surveillance, emettre, conserver: &conserver, delai_decision })
    }

    #[cfg(test)]
    pub(crate) fn reprise_session(&self) -> Option<Reprise> {
        self.etat().session.as_ref().map(|s| s.reprise.clone())
    }

    // ------------------------------------------------------------------ commandes sans box

    /// Avec une session : l'origine de la box connectée (celle que `restaurer` utilisera) ; sinon la plus récente.
    fn origine_courante(&self) -> Option<origine::Origine> {
        let sauvegardes = self.cfg.donnees.join("sauvegardes");
        match self.etat().session.as_ref() {
            Some(s) => origine::origine(&sauvegardes, &nom_serie(&s.reprise)),
            None => origine::derniere_origine(&sauvegardes),
        }
    }

    pub fn etat_initial(&self) -> EtatInitial {
        let adb_installe = if self.cfg.demo { self.etat().adb_demo } else { zh_reseau::adb_installe(&self.cfg.donnees).is_some() };
        EtatInitial {
            version: self.cfg.version.clone(),
            adb_installe,
            demo: self.cfg.demo,
            sauvegardes_dir_existe: self.cfg.donnees.join("sauvegardes").is_dir(),
            reprise_en_attente: persistance::en_attente(&self.cfg.donnees),
            origine: self.origine_courante().map(|o| OrigineUi { date: o.date }),
            boxes_memorisees: self.boxes_memorisees(),
            films_inclus: self.cfg.jeton_projet.is_some(),
        }
    }

    /// Box mémorisées, la plus récente d'abord. En démo, SALON joue celle d'une installation précédente.
    fn boxes_memorisees(&self) -> Vec<BoxMemoriseeUi> {
        let vue = |b: BoxTrouvee| BoxMemoriseeUi { nom: b.nom, ip: b.ip };
        if self.cfg.demo {
            let (nom, ip, _) = demo::BOXES[0];
            return vec![vue(BoxTrouvee { nom: nom.into(), ip: ip.into(), appareil: Some(appareils::MIBOX4.id.into()), libelle: appareils::MIBOX4.nom_defaut.into() })];
        }
        memoire::lire(&self.cfg.donnees).into_iter().rev().map(vue).collect()
    }

    pub fn installer_adb(&self, accord: bool, emettre: Emetteur) -> Result<(), ErreurCommande> {
        let (_occupation, _) = self.prendre()?;
        if !accord {
            return Err(ErreurCommande::nouvelle(
                "refus_licence",
                "Il faut accepter la licence d'Android SDK Platform-Tools (Google) pour télécharger adb.",
            ));
        }
        let progression = |fraction: f64| emettre(EvenementUi::Progression { fraction });
        if self.cfg.demo {
            for i in 0..=20 {
                progression(f64::from(i) / 20.0);
                std::thread::sleep(self.cfg.rythme_demo * 2);
            }
            self.etat().adb_demo = true;
            return Ok(());
        }
        if zh_reseau::adb_installe(&self.cfg.donnees).is_none() {
            let mut dernier = -1.0;
            zh_reseau::installer_adb(&self.cfg.donnees, |recu, total| {
                if let Some(total) = total.filter(|t| *t > 0) {
                    let f = (recu as f64 / total as f64).min(1.0);
                    if f - dernier >= 0.01 {
                        dernier = f;
                        progression(f);
                    }
                }
            })?;
        }
        progression(1.0);
        Ok(())
    }

    pub fn chercher_box(&self) -> Result<Vec<BoxUi>, ErreurCommande> {
        let (_occupation, _) = self.prendre()?;
        let resultat = if self.cfg.demo {
            std::thread::sleep(self.cfg.rythme_demo * 10);
            Ok(demo::BOXES
                .iter()
                .map(|(nom, ip, _)| BoxTrouvee { nom: (*nom).into(), ip: (*ip).into(), appareil: Some(appareils::MIBOX4.id.into()), libelle: appareils::MIBOX4.nom_defaut.into() })
                .collect())
        } else {
            // `annuler` lève le drapeau : la recherche rend alors ce qu'elle a trouvé, sans erreur
            let indices: Vec<_> = appareils::TOUS.iter().map(|a| a.indice()).collect();
            zh_reseau::chercher_box(Duration::from_secs(15), &memoire::lire(&self.cfg.donnees), &indices, &self.annule)
        };
        self.garder_recherche(resultat)
    }

    /// Garde le résultat d'une recherche. Même en cas d'échec, les ids de la recherche précédente ne sont plus valables.
    pub(crate) fn garder_recherche(&self, resultat: Result<Vec<BoxTrouvee>, zh_reseau::Erreur>) -> Result<Vec<BoxUi>, ErreurCommande> {
        let mut e = self.etat();
        let base = e.prochain_id;
        e.base_id = base;
        let trouvees: Vec<BoxTrouvee> = match resultat {
            Ok(t) => t,
            Err(err) => {
                e.boxes.clear();
                return Err(err.into());
            }
        };
        let vue = trouvees.iter().zip(base..).map(|(b, id)| BoxUi { id, nom: b.nom.clone(), ip: b.ip.clone() }).collect();
        e.prochain_id = base.saturating_add(trouvees.len() as u32);
        e.boxes = trouvees;
        Ok(vue)
    }

    pub fn chercher_villes(&self, saisie: &str) -> Result<Vec<VilleUi>, ErreurCommande> {
        let (_occupation, _) = self.prendre()?;
        let villes = if self.cfg.demo {
            if saisie.trim().is_empty() {
                return Err(zh_reseau::Erreur::VilleVide.into());
            }
            let v = |nom: &str, region: &str, pays: &str, latitude, longitude| Ville {
                nom: nom.into(),
                region: Some(region.into()),
                pays: Some(pays.into()),
                latitude,
                longitude,
            };
            vec![
                v("Montpellier", "Occitanie", "France", 43.6119, 3.8772),
                v("Montpellier", "Vermont", "États-Unis", 44.2601, -72.5754),
                v("Montpelier", "Idaho", "États-Unis", 42.3238, -111.2988),
            ]
        } else {
            zh_reseau::chercher_villes(saisie)?
        };
        let vue = villes
            .iter()
            .enumerate()
            .map(|(i, v)| VilleUi { id: i as u32, nom: v.nom.clone(), region: v.region.clone(), pays: v.pays.clone() })
            .collect();
        self.etat().villes = villes;
        Ok(vue)
    }

    pub fn annuler(&self) {
        if self.occupe.load(Ordering::SeqCst) {
            self.annule.store(true, Ordering::SeqCst);
        }
    }

    /// Réponse à une demande `attente_utilisateur` (usage unique).
    pub fn decider(&self, demande: u64, decision: &str) -> Result<(), ErreurCommande> {
        let decision = match decision {
            "reessayer" => Decision::Reessayer,
            "arreter" => Decision::Arreter,
            _ => return Err(ErreurCommande::nouvelle("choix_invalide", "Décision inconnue : « réessayer » ou « arrêter » attendu.")),
        };
        if self.attente.repondre(demande, decision) {
            Ok(())
        } else {
            Err(ErreurCommande::nouvelle("choix_invalide", "Cette demande n'est plus en attente."))
        }
    }

    pub fn ouvrir_sauvegardes(&self) -> Result<(), ErreurCommande> {
        let dossier = self.cfg.donnees.join("sauvegardes");
        std::fs::create_dir_all(&dossier).map_err(|e| ErreurCommande::interne(format!("Dossier des sauvegardes inaccessible : {e}")))?;
        if self.cfg.demo {
            return Ok(());
        }
        // Chemin fixe de l'appli, jamais fourni par l'interface.
        let programme = if cfg!(target_os = "macos") {
            "open"
        } else if cfg!(windows) {
            "explorer"
        } else {
            "xdg-open"
        };
        let statut = std::process::Command::new(programme)
            .arg(&dossier)
            .status()
            .map_err(|e| ErreurCommande::interne(format!("Impossible d'ouvrir le dossier des sauvegardes : {e}")))?;
        // `explorer` rend 1 même quand il réussit.
        if !statut.success() && !cfg!(windows) {
            return Err(ErreurCommande::interne("Impossible d'ouvrir le dossier des sauvegardes."));
        }
        Ok(())
    }

    /// Rapport à copier : version, système, erreur et résumé de la dernière action (filtrés, sans secret).
    pub fn rapport_texte(&self, erreur: &str, resume: &str) -> String {
        rapport::complet(&self.cfg.version, &rapport::systeme(), erreur, resume)
    }

    /// Prépare un e-mail à l'adresse fixe de l'appli (corps borné) dans la messagerie du Mac. En démo : rien n'est ouvert.
    pub fn envoyer_rapport(&self, erreur: &str, resume: &str) -> Result<(), ErreurCommande> {
        let corps = rapport::corps_mail(&self.cfg.version, &rapport::systeme(), erreur, resume);
        if self.cfg.demo {
            return Ok(());
        }
        rapport::ouvrir(&rapport::mailto(&corps))
    }

    pub fn en_demo(&self) -> bool {
        self.cfg.demo
    }

    /// Retire Zen Home de ce Mac (la box n'est pas touchée) : copie des sauvegardes si demandée, arrêt d'adb, bundle à la
    /// Corbeille, puis suppression des données de l'appli. L'appelant quitte ensuite. En démo : rien n'est copié ni supprimé.
    /// `donnees_parent` : dossier des données utilisateur ; `bundle` : l'appli à mettre à la Corbeille, si elle y est.
    pub fn desinstaller_mac(&self, garder_sauvegardes: bool, documents: &Path, donnees_parent: &Path, bundle: Option<&Path>) -> Result<(), ErreurCommande> {
        let (_occupation, _) = self.prendre()?;
        if self.cfg.demo {
            std::thread::sleep(self.cfg.rythme_demo * 10);
            return Ok(());
        }
        let copie_impossible = |e: &std::io::Error| ErreurCommande::interne(format!("Copie des sauvegardes dans Documents impossible ({e}). Rien n'a été supprimé."));
        let sauvegardes = self.cfg.donnees.join("sauvegardes");
        if garder_sauvegardes && sauvegardes.is_dir() {
            let horodatage = chrono::Local::now().format("%Y-%m-%d %Hh%M").to_string();
            let copie = desinstallation::creer_dossier_copie(documents, &horodatage)
                .map_err(|e| copie_impossible(&e))?;
            desinstallation::copier_contenu(&sauvegardes, &copie).map_err(|e| {
                let _ = std::fs::remove_dir_all(&copie); // dossier créé à l'instant : une copie partielle ne reste pas
                copie_impossible(&e)
            })?;
        }
        self.fermer(); // adb (serveur du port 5038) tourne depuis le dossier qu'on va supprimer
        // Corbeille avant la suppression des données : en cas d'échec, les données restent intactes.
        if let Some(b) = bundle {
            desinstallation::mettre_a_la_corbeille(b).map_err(|m| {
                ErreurCommande::interne(format!("Zen Home n'a pas pu être mis à la Corbeille ({m}). Glissez l'application à la Corbeille vous-même. Vos données n'ont pas été supprimées."))
            })?;
        }
        desinstallation::supprimer_donnees(&self.cfg.donnees, donnees_parent)
            .map_err(|m| ErreurCommande::interne(format!("Suppression des données de Zen Home impossible ({m}).{}", if bundle.is_some() { " L'application est déjà dans la Corbeille." } else { "" })))?;
        Ok(())
    }

    /// Fin de session (et sortie de l'appli) : `adb disconnect` puis arrêt du serveur adb.
    pub fn fermer(&self) {
        let session = self.etat().session.take();
        if let Some(s) = session {
            clore(&*s.t);
        }
    }

    // ------------------------------------------------------------------ connexion

    fn resoudre(&self, cible: &Cible) -> Result<(String, Ipv4Addr), ErreurCommande> {
        let e = self.etat();
        let ip_invalide = || {
            ErreurCommande::nouvelle(
                "injoignable",
                "Adresse IP refusée : indiquez l'adresse locale de la box (par exemple 192.168.1.20), pas une adresse publique.",
            )
        };
        match cible {
            Cible::Id { id } => {
                let b = e
                    .boxes
                    .get(id.checked_sub(e.base_id).map_or(usize::MAX, |i| i as usize))
                    .ok_or_else(|| ErreurCommande::nouvelle("choix_invalide", "Cette box n'est plus dans la liste : relancez la recherche."))?;
                Ok((b.nom.clone(), zh_reseau::valider_ipv4(&b.ip).ok_or_else(ip_invalide)?))
            }
            Cible::Ip { ip } => {
                let ip = zh_reseau::valider_ipv4(ip).ok_or_else(ip_invalide)?;
                let nom = e.boxes.iter().find(|b| b.ip == ip.to_string()).map_or_else(String::new, |b| b.nom.clone());
                Ok((nom, ip))
            }
        }
    }

    pub fn connecter(&self, cible: &Cible, emettre: Emetteur) -> Result<Connexion, ErreurCommande> {
        let (_occupation, surveillance) = self.prendre()?;
        let (nom, ip) = self.resoudre(cible)?;
        // Seule l'ancienne box est déconnectée : le serveur adb reste, pour enchaîner les box sans le relancer.
        let ancienne = self.etat().session.take();
        if let Some(s) = ancienne {
            s.t.deconnecter();
        }
        emettre(EvenementUi::Etape { nom: "Connexion à la box".into() });
        let t = (self.cfg.fabrique)(ip)?;
        let r = self.etablir(&t, emettre, &surveillance);
        match r {
            Ok((serie, appareil)) => {
                let mut reprise = Reprise::pour_box(serie);
                // Paquets désactivés par un passage interrompu sur CETTE box : la prochaine vérification les reprend.
                if let Some(d) = persistance::lire(&self.cfg.donnees, reprise.serie.as_deref().unwrap_or_default()) {
                    reprise.desactives = d.desactives;
                }
                let mut nom = if nom.is_empty() { appareil.nom_defaut.to_string() } else { nom };
                if let (false, Some(serie)) = (self.cfg.demo, reprise.serie.as_deref()) {
                    nom = memoire::nom_pour(&self.cfg.donnees, serie, &ip.to_string(), &nom, appareil.nom_defaut);
                    let _ = memoire::memoriser(&self.cfg.donnees, serie, &nom, &ip.to_string(), Some(appareil.id));
                }
                self.etat().session = Some(SessionBox { t, appareil, reprise, inventaire: None });
                Ok(Connexion { nom, ip: ip.to_string(), modele: appareil.nom.into(), appareil: appareil.id.into() })
            }
            Err(e) => {
                clore(&*t);
                Err(e)
            }
        }
    }

    /// Connexion + numéro de série (lu juste après, il identifie la box pour toute la session) + appareil reconnu.
    fn etablir(&self, t: &TransportPartage, emettre: Emetteur, surveillance: &Surveillance) -> Result<(String, &'static Appareil), ErreurCommande> {
        let w = Surveille { t: &**t, surveillance };
        let pol = &self.cfg.politique;
        let conserver = |_: &Reprise| Ok(());
        let obs = ObservateurUi { emettre, attente: &self.attente, surveillance, delai_decision: self.cfg.delai_decision, conserver: &conserver };
        let Connecte { appareil, sdk_recent } = loop {
            match connecter_box(&w, pol) {
                Ok(c) => break c,
                Err(e @ (EchecConnexion::NonAutorise | EchecConnexion::Injoignable(_))) => {
                    if let Some(i) = interruption(surveillance) {
                        return Err(i);
                    }
                    if obs.demander(matches!(e, EchecConnexion::NonAutorise)) != Decision::Reessayer {
                        return Err(e.into());
                    }
                }
                Err(e) => return Err(interruption(surveillance).unwrap_or_else(|| e.into())),
            }
        };
        if sdk_recent {
            emettre(EvenementUi::Avertissement {
                texte: format!("Cette {} utilise une version d'Android plus récente que celles testées : vérifiez le résultat à la fin.", appareil.mot),
            });
        }
        for essai in 0..3 {
            match lire_serie(&w) {
                Ok(s) => return Ok((s, appareil)),
                Err(ErreurTransport::NonAutorise) => return Err(ErreurCommande::non_autorise()),
                Err(_) if essai < 2 => w.attendre(pol.pause),
                Err(_) => {}
            }
        }
        Err(interruption(surveillance)
            .unwrap_or_else(|| ErreurCommande::nouvelle("lecture", "La box ne donne pas son numéro de série : redémarrez-la puis réessayez.")))
    }

    // ------------------------------------------------------------------ lecture

    pub fn inventaire(&self, emettre: Emetteur) -> Result<InventaireUi, ErreurCommande> {
        self.agir(emettre, |c| {
            (c.emettre)(EvenementUi::Etape { nom: "Lecture de l'état de la box".into() });
            let w = c.transport();
            let obs = c.observateur();
            let serie = nom_serie(&c.reprise);
            let maintenant = chrono::Local::now();
            let base = format!("{}-{serie}", maintenant.format("%Y%m%d-%H%M%S"));
            let sauvegardes = self.cfg.donnees.join("sauvegardes");
            // dossier toujours neuf : deux lectures dans la même seconde ne se partagent pas le même
            let dossier = (1..100)
                .map(|n| if n == 1 { base.clone() } else { format!("{base}+{n}") })
                .map(|nom| sauvegardes.join(nom))
                .find(|d| !d.exists())
                .ok_or_else(|| ErreurCommande::interne("Trop de sauvegardes créées en même temps."))?;
            let horodatage = maintenant.format("%d/%m/%Y %H:%M").to_string();
            let inv = loop {
                match inventaire::inventaire(&w, &dossier, &horodatage) {
                    Ok(i) => break i,
                    Err(ErreurInventaire::AttenteUtilisateur { non_autorise }) => {
                        if let Some(i) = interruption(&c.surveillance) {
                            return Err(i);
                        }
                        if obs.demander(non_autorise) != Decision::Reessayer {
                            return Err(ErreurInventaire::AttenteUtilisateur { non_autorise }.into());
                        }
                    }
                    Err(e) => return Err(interruption(&c.surveillance).unwrap_or_else(|| e.into())),
                }
            };
            for a in &inv.avertissements {
                (c.emettre)(EvenementUi::Avertissement { texte: crate::moteur::transport::assainir(a) });
            }
            (c.emettre)(EvenementUi::Etape { nom: "Lecture des noms des applis".into() });
            let libelles = Self::libelles(&w);
            let vue = inventaire_ui(c.appareil, &inv, &libelles);
            if let Some(s) = self.etat().session.as_mut() {
                s.inventaire = Some(inv);
            }
            Ok(vue)
        })
    }

    /// Noms lisibles des applis de l'utilisateur, demandés à Zen Home (absent ou muet : on garde les noms de paquets).
    fn libelles(t: &dyn Transport) -> HashMap<String, String> {
        let Ok(commande) = Commande::broadcast_config(&[ArgConfig::Libelles]) else { return HashMap::new() };
        t.shell(&commande).ok().and_then(|s| libelles_broadcast(&s.stdout)).unwrap_or_default().into_iter().collect()
    }

    // ------------------------------------------------------------------ actions

    /// APK à installer : en démo un fichier factice ; sinon la ressource du bundle, dont l'empreinte est comparée à la
    /// constante compilée AVANT toute installation. Le contrôle porte sur une copie privée, celle qu'adb enverra.
    fn apk_verifie(&self) -> Result<ApkVerifie, ErreurCommande> {
        if self.cfg.demo {
            let chemin = self.cfg.donnees.join("ZenHome-demo.apk");
            std::fs::create_dir_all(&self.cfg.donnees)
                .and_then(|()| std::fs::write(&chemin, b"APK de demonstration"))
                .map_err(|e| ErreurCommande::interne(format!("Fichier de démonstration impossible à écrire : {e}")))?;
            return Ok(ApkVerifie { chemin, supprimer: false });
        }
        let manquant = || {
            ErreurCommande::interne(
                "Cette version de l'application ne contient pas Zen Home (l'APK manquait à sa construction) : installez une version complète.",
            )
        };
        if self.cfg.apk_sha256.is_empty() {
            return Err(manquant());
        }
        let octets = match std::fs::metadata(&self.cfg.apk_ressource) {
            Ok(m) if m.len() > APK_TAILLE_MAX => return Err(ErreurCommande::interne("Le fichier de Zen Home embarqué est anormalement gros : réinstallez l'application.")),
            Ok(_) => std::fs::read(&self.cfg.apk_ressource).map_err(|_| manquant())?,
            Err(_) => return Err(manquant()),
        };
        if octets.is_empty() {
            return Err(manquant());
        }
        if !sha256_hex(&octets).eq_ignore_ascii_case(&self.cfg.apk_sha256) {
            return Err(ErreurCommande::interne(
                "Le fichier de Zen Home embarqué ne correspond pas à l'empreinte attendue (altéré ou corrompu) : réinstallez l'application. Rien n'a été envoyé à la box.",
            ));
        }
        let dossier = self.cfg.donnees.join("apk-verifie");
        let chemin = dossier.join(format!("ZenHome-{}.apk", std::process::id()));
        let copie = || -> std::io::Result<()> {
            std::fs::create_dir_all(&dossier)?;
            let _ = std::fs::remove_file(&chemin);
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
            options.open(&chemin)?.write_all(&octets)
        };
        let apk = ApkVerifie { chemin: chemin.clone(), supprimer: true };
        copie().map_err(|e| ErreurCommande::interne(format!("Copie de Zen Home impossible : {e}")))?;
        Ok(apk)
    }

    pub fn installer(&self, ville_id: Option<u32>, jeton: Option<String>, emettre: Emetteur) -> Result<(), ErreurCommande> {
        self.agir(emettre, |c| {
            let ville = match ville_id {
                Some(id) => Some(
                    self.etat()
                        .villes
                        .get(id as usize)
                        .cloned()
                        .ok_or_else(|| ErreurCommande::nouvelle("choix_invalide", "Cette ville n'est plus dans la liste : relancez la recherche."))?,
                ),
                None => None,
            };
            let apk = self.apk_verifie()?;
            let p = Parametres {
                apk: apk.chemin.clone(),
                lat: ville.as_ref().map(|v| v.latitude.to_string()),
                lon: ville.as_ref().map(|v| v.longitude.to_string()),
                ville: ville.map(|v| v.nom),
                jeton: jeton.map(|j| j.trim().to_string()).filter(|j| !j.is_empty()).or_else(|| self.cfg.jeton_projet.clone()),
            };
            let r = installation::installer(&c.transport(), c.appareil, &self.cfg.politique, &mut c.observateur(), &p, &c.reprise);
            c.issue(r).map(|_| ())
        })
    }

    pub fn appliquer(&self, choix: &ChoixUi, emettre: Emetteur) -> Result<RapportApplication, ErreurCommande> {
        self.agir(emettre, |c| {
            let inv = c.inventaire.as_ref().ok_or_else(|| ErreurCommande::nouvelle("lecture", "Lisez d'abord l'état de la box avant d'appliquer vos choix."))?;
            let demande = Choix { garder: choix.garder.clone(), desactiver: choix.desactiver.clone(), desinstaller: choix.desinstaller.clone() };
            let r = choix::appliquer(&c.transport(), c.appareil, &self.cfg.politique, &mut c.observateur(), inv, &demande, &c.reprise);
            let rapport = c.issue(r)?;
            // L'inventaire de la session suit ce qui vient d'être fait : une même désinstallation n'est pas rejouée.
            if let Some(i) = self.etat().session.as_mut().and_then(|s| s.inventaire.as_mut()) {
                let partis: Vec<&String> = rapport.desinstalles.iter().chain(&rapport.deja_absents).collect();
                i.utilisateur.retain(|p| !partis.contains(&p));
                i.applis.retain(|a| !partis.contains(&&a.paquet));
                i.desactives.retain(|p| !rapport.reactives.contains(p));
                i.garder.clone_from(&rapport.gardes);
            }
            Ok(rapport_application(&rapport))
        })
    }

    pub fn verifier(&self, reparer: bool, emettre: Emetteur) -> Result<VerificationUi, ErreurCommande> {
        self.agir(emettre, |c| {
            // L'APK ne sert qu'à réinstaller Zen Home s'il manque ; un APK refusé n'empêche pas le reste de la réparation.
            let apk = if reparer {
                self.apk_verifie()
                    .inspect_err(|e| emettre(EvenementUi::Avertissement { texte: e.message.clone() }))
                    .ok()
            } else {
                None
            };
            let r = verification::verifier(&c.transport(), c.appareil, &self.cfg.politique, &mut c.observateur(), reparer, apk.as_ref().map(|a| a.chemin.as_path()), &c.reprise);
            let rapport = c.issue(r)?;
            if reparer && rapport.ok {
                self.oublier_reprise();
            }
            Ok(verification_ui(rapport.final_()))
        })
    }

    pub fn restaurer(&self, emettre: Emetteur) -> Result<RestaurationUi, ErreurCommande> {
        self.agir(emettre, |c| {
            // Sauvegarde faite avant Zen Home pour cette box, s'il y en a une : sinon restauration générique.
            let origine = origine::origine(&self.cfg.donnees.join("sauvegardes"), &nom_serie(&c.reprise));
            let r = restauration::restaurer(&c.transport(), c.appareil, &self.cfg.politique, &mut c.observateur(), &c.reprise, origine.as_ref());
            let rapport = c.issue(r)?;
            if !rapport.incomplete {
                self.oublier_reprise();
            }
            Ok(RestaurationUi {
                incomplete: rapport.incomplete,
                avertissements: rapport.avertissements.iter().map(|a| crate::moteur::transport::assainir(a)).collect(),
                depuis_sauvegarde: origine.map(|o| o.date),
            })
        })
    }

    /// Coupe le débogage USB de la box (conseillé en fin d'installation), puis ferme la session.
    /// Sans canal d'événements : une demande à l'utilisateur ne pourrait pas être affichée, l'action s'arrête donc aussitôt.
    pub fn couper_debogage(&self) -> Result<(), ErreurCommande> {
        self.agir_avec(&|_| {}, Duration::ZERO, |c| {
            let mut obs = c.observateur();
            let w = c.transport();
            let mut s = Session::nouvelle(&w, &self.cfg.politique, &mut obs, "debogage", 1, &c.reprise);
            s.etape(0, "Coupure du débogage");
            let r = match s.ecrire(&Commande::settings_put(Espace::Global, "adb_enabled", "0")?) {
                // la box coupe le lien en appliquant le réglage : la réponse peut se perdre
                Err(Arret::Echec(ErreurAction::Transport(ErreurTransport::Coupure(_) | ErreurTransport::Vide))) => Ok(()),
                autre => autre.map(|_| ()),
            };
            c.issue(r)?;
            self.fermer();
            Ok(())
        })
    }

    /// La vérification (ou la restauration) est allée au bout : plus de retour arrière à garder.
    fn oublier_reprise(&self) {
        let serie = {
            let mut e = self.etat();
            let Some(s) = e.session.as_mut() else { return };
            s.reprise.desactives.clear();
            s.reprise.serie.clone()
        };
        if let Some(serie) = serie {
            persistance::effacer(&self.cfg.donnees, &serie);
        }
    }
}
