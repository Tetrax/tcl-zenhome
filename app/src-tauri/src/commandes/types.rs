//! Types échangés avec l'interface (voir CONTRAT.md) et conversion depuis les types du moteur.
//! Tout texte venant de la box est passé par `assainir` (ou `nettoyer_libelle`) avant de sortir d'ici.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::moteur::appareils::Appareil;
use crate::moteur::choix::RapportChoix;
use crate::moteur::debloat::{Balise, Recommandation};
use crate::moteur::inventaire::{Inventaire, nettoyer_libelle, taille_lisible};
use crate::moteur::transport::assainir;
use crate::moteur::verification::{Controle, Statut};

const TEXTE_VIDE: &str = "—";

#[derive(Serialize)]
pub struct EtatInitial {
    pub version: String,
    pub adb_installe: bool,
    pub demo: bool,
    pub sauvegardes_dir_existe: bool,
    pub reprise_en_attente: bool,
    /// Sauvegarde d'origine la plus récente trouvée (toutes box confondues), si elle existe.
    pub origine: Option<OrigineUi>,
    /// Box déjà configurées (la plus récente d'abord), à proposer tout de suite pendant la recherche.
    pub boxes_memorisees: Vec<BoxMemoriseeUi>,
    /// Vrai si l'appli embarque un jeton TMDB (le jeton lui-même n'est jamais exposé).
    pub films_inclus: bool,
}

#[derive(Serialize)]
pub struct BoxMemoriseeUi {
    pub nom: String,
    pub ip: String,
}

#[derive(Serialize)]
pub struct OrigineUi {
    pub date: String,
}

#[derive(Serialize)]
pub struct BoxUi {
    pub id: u32,
    pub nom: String,
    pub ip: String,
    /// Appareil supposé d'après l'annonce réseau (simple indice, la connexion vérifie) ; `None` si inconnu.
    pub appareil: Option<String>,
    /// Libellé de cet appareil supposé.
    pub libelle: String,
}

#[derive(Serialize)]
pub struct VilleUi {
    pub id: u32,
    pub nom: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pays: Option<String>,
}

/// Cible d'une connexion : une box de la dernière recherche, ou une adresse IP locale saisie.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Cible {
    Id { id: u32 },
    Ip { ip: String },
}

#[derive(Serialize)]
pub struct Connexion {
    pub nom: String,
    pub ip: String,
    /// Modèle affiché de l'appareil reconnu.
    pub modele: String,
    /// Identifiant de l'appareil reconnu (PROFIL du kit : « mibox4 », « tcl-g08 »).
    pub appareil: String,
}

#[derive(Serialize)]
pub struct AppliSysteme {
    pub paquet: String,
    pub libelle: String,
    pub explication: String,
    pub recommande: &'static str,
    /// « pistage » (lignes sans balise de la liste) ou « option » ([option], [option:garder]).
    pub groupe: &'static str,
    pub etat: &'static str,
}

#[derive(Serialize)]
pub struct AppliUtilisateur {
    pub paquet: String,
    pub libelle: String,
    pub taille: String,
    pub derniere_utilisation: String,
}

#[derive(Serialize)]
pub struct Resume {
    pub modele: String,
    pub android: String,
    pub nb_applis: usize,
    pub nb_desactivees: usize,
}

#[derive(Serialize)]
pub struct InventaireUi {
    pub applis_systeme: Vec<AppliSysteme>,
    pub applis_utilisateur: Vec<AppliUtilisateur>,
    pub resume: Resume,
}

/// Choix reçu de l'interface : des noms de paquets seulement ; le moteur les recoupe avec l'inventaire de la session.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ChoixUi {
    pub garder: Vec<String>,
    pub desactiver: Vec<String>,
    pub desinstaller: Vec<String>,
}

#[derive(Serialize)]
pub struct RapportApplication {
    pub desactivees: usize,
    pub gardees: usize,
    pub desinstallees: Vec<String>,
    pub echecs: Vec<String>,
}

#[derive(Serialize)]
pub struct LigneUi {
    pub etat: &'static str,
    pub controle: String,
    pub detail: String,
}

#[derive(Serialize)]
pub struct VerificationUi {
    pub lignes: Vec<LigneUi>,
    pub ko: usize,
}

#[derive(Serialize)]
pub struct RestaurationUi {
    pub incomplete: bool,
    pub avertissements: Vec<String>,
    /// Date de la sauvegarde d'origine utilisée, `None` pour la restauration générique.
    pub depuis_sauvegarde: Option<String>,
}

fn court(texte: &str) -> String {
    let t = assainir(texte);
    if t.is_empty() { TEXTE_VIDE.to_string() } else { t }
}

/// Inventaire de la box mis en forme : applis de la liste présentes sur la box, applis de l'utilisateur, résumé.
/// `libelles` : noms lisibles obtenus de Zen Home (bruts), à défaut le nom du paquet.
pub fn inventaire_ui(app: &Appareil, inv: &Inventaire, libelles: &HashMap<String, String>) -> InventaireUi {
    let applis_systeme = app.liste()
        .into_iter()
        .filter(|e| inv.tous.contains(&e.paquet))
        .map(|e| {
            let etat = if inv.desactives.contains(&e.paquet) {
                "desactive"
            } else if inv.garder.contains(&e.paquet) {
                "garde"
            } else {
                "actif"
            };
            AppliSysteme {
                recommande: if e.recommandation() == Recommandation::Garder { "garder" } else { "desactiver" },
                groupe: if e.balise == Balise::Aucune { "pistage" } else { "option" },
                paquet: e.paquet,
                libelle: e.libelle,
                explication: e.explication,
                etat,
            }
        })
        .collect();
    let applis_utilisateur = inv
        .applis
        .iter()
        .filter(|a| !app.est_protege(&a.paquet))
        .map(|a| AppliUtilisateur {
            paquet: a.paquet.clone(),
            libelle: nettoyer_libelle(libelles.get(&a.paquet).map_or("", String::as_str), &a.paquet),
            taille: a.octets.map_or_else(|| TEXTE_VIDE.to_string(), taille_lisible),
            derniere_utilisation: a.derniere_utilisation.as_deref().map_or_else(|| TEXTE_VIDE.to_string(), court),
        })
        .collect();
    InventaireUi {
        applis_systeme,
        applis_utilisateur,
        resume: Resume {
            modele: court(&inv.modele),
            android: court(&inv.android),
            nb_applis: inv.tous.len(),
            nb_desactivees: inv.desactives.len(),
        },
    }
}

pub fn verification_ui(c: &Controle) -> VerificationUi {
    VerificationUi {
        lignes: c
            .lignes
            .iter()
            .map(|l| LigneUi {
                etat: match l.statut {
                    Statut::Ok => "ok",
                    Statut::Ko => "ko",
                    Statut::Avert => "avert",
                },
                controle: assainir(&l.nom),
                detail: assainir(&l.detail),
            })
            .collect(),
        ko: c.ko,
    }
}

pub fn rapport_application(r: &RapportChoix) -> RapportApplication {
    RapportApplication {
        desactivees: r.a_desactiver.len(),
        gardees: r.gardes.len(),
        desinstallees: r.desinstalles.iter().chain(&r.deja_absents).map(|p| assainir(p)).collect(),
        echecs: r.echecs.iter().map(|p| assainir(p)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::appareils::{MIBOX4, TCL_G08};
    use crate::moteur::fake_box::FakeBox;
    use crate::moteur::inventaire::{AppUtilisateur, inventaire};
    use crate::moteur::transport::Transport;

    #[test]
    fn aucun_paquet_protege_n_est_propose_a_la_desinstallation() {
        for (f, app, proteges) in [
            (FakeBox::tcl_g08(), &TCL_G08, &["com.tcl.tv", "com.tcl.micmanager", "com.tcl.framework.custom", "com.tcl.suspension"][..]),
            (FakeBox::mibox4(), &MIBOX4, &["com.google.android.tvlauncher"][..]),
        ] {
            f.connecter().unwrap();
            let d = std::env::temp_dir().join(format!("zenhome-types-{}-{}", app.id, std::process::id()));
            let mut inv = inventaire(&f, &d.join("sauvegarde"), "2026-10-31 10:00").unwrap();
            inv.applis.extend(proteges.iter().map(|p| AppUtilisateur { paquet: (*p).into(), octets: None, derniere_utilisation: None }));
            let ui = inventaire_ui(app, &inv, &HashMap::new());
            assert!(!ui.applis_utilisateur.is_empty());
            assert!(ui.applis_utilisateur.iter().all(|a| !proteges.contains(&a.paquet.as_str())), "{}", app.id);
            let _ = std::fs::remove_dir_all(d);
        }
    }
}
