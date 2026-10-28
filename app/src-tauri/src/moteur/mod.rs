//! Moteur Zen Home : réécriture en Rust de la logique du kit bash (étape 4 : transport et lecture ;
//! étape 5 : actions installation, choix, vérification, restauration).
//! Référence fonctionnelle : kit/Zen Home.command et tv/*.sh.

pub mod choix;
pub mod connexion;
pub mod debloat;
pub mod fake_box;
pub mod installation;
pub mod inventaire;
pub mod restauration;
pub mod session;
pub mod transport;
pub mod verification;
