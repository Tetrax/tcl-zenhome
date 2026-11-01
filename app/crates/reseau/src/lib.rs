//! Réseau de Zen Home. Tout le réseau de l'appli passe par cette crate, vers des
//! hôtes figés uniquement ; toute donnée reçue est traitée comme non fiable.

mod decouverte;
mod erreur;
mod http;
mod meteo;
mod platform_tools;

pub use decouverte::{BoxTrouvee, INDICE_MIBOX, IndiceAppareil, chercher_box, chercher_box_mibox, nettoyer_nom, valider_ipv4};
pub use erreur::Erreur;
pub use meteo::{Ville, chercher_villes};
pub use platform_tools::{adb_installe, installer_adb};
