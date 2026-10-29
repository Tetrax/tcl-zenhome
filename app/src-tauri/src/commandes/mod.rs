//! Commandes de l'application (interface <-> cœur). Contrat : app/CONTRAT.md.

mod coeur;
mod demo;
mod desinstallation;
mod erreur;
mod evenements;
mod ipc;
mod persistance;
mod surveillance;
mod types;
#[cfg(test)]
mod tests;

pub use coeur::{Config, Coeur};
pub use demo::{RYTHME as RYTHME_DEMO, demo_demandee};
pub use ipc::*;
