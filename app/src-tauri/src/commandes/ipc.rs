//! Fine couche `#[tauri::command]` au-dessus de `Coeur` : désérialise les intentions, branche le `Channel`,
//! exécute l'action dans `spawn_blocking`. Toute la logique (et les tests) sont dans `coeur.rs`.
//!
//! Arguments côté JS : Tauri v2 renomme les arguments Rust en camelCase (`ville_id` devient `villeId`).

use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::{Manager, State};

use super::coeur::{Coeur, Config};
use super::demo;
use super::erreur::ErreurCommande;
use super::evenements::EvenementUi;
use super::types::*;

type Etat<'a> = State<'a, Arc<Coeur>>;

/// Dossier de données, version et mode (démo ou réel) : l'état est géré par Rust dès le démarrage.
pub fn preparer(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let donnees = app.path().app_data_dir()?;
    let version = app.package_info().version.to_string();
    let cfg = if demo::demo_demandee() {
        Config::demo(version, donnees, demo::RYTHME)
    } else {
        Config::reelle(version, donnees, app.path().resource_dir()?.join("ZenHome.apk"))
    };
    app.manage(Arc::new(Coeur::nouveau(cfg)));
    Ok(())
}

fn emetteur(canal: Channel<EvenementUi>) -> impl Fn(EvenementUi) + Send + Sync {
    move |e| {
        let _ = canal.send(e);
    }
}

async fn bloquant<T: Send + 'static>(f: impl FnOnce() -> Result<T, ErreurCommande> + Send + 'static) -> Result<T, ErreurCommande> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|_| ErreurCommande::interne("L'action s'est arrêtée de façon inattendue."))?
}

#[tauri::command]
pub fn etat_initial(coeur: Etat<'_>) -> EtatInitial {
    coeur.etat_initial()
}

#[tauri::command]
pub async fn installer_adb(coeur: Etat<'_>, accord: bool, canal: Channel<EvenementUi>) -> Result<(), ErreurCommande> {
    let c = coeur.inner().clone();
    bloquant(move || c.installer_adb(accord, &emetteur(canal))).await
}

#[tauri::command]
pub async fn chercher_box(coeur: Etat<'_>) -> Result<Vec<BoxUi>, ErreurCommande> {
    let c = coeur.inner().clone();
    bloquant(move || c.chercher_box()).await
}

#[tauri::command]
pub async fn connecter(coeur: Etat<'_>, cible: Cible, canal: Channel<EvenementUi>) -> Result<Connexion, ErreurCommande> {
    let c = coeur.inner().clone();
    bloquant(move || c.connecter(&cible, &emetteur(canal))).await
}

#[tauri::command]
pub async fn chercher_villes(coeur: Etat<'_>, saisie: String) -> Result<Vec<VilleUi>, ErreurCommande> {
    let c = coeur.inner().clone();
    bloquant(move || c.chercher_villes(&saisie)).await
}

#[tauri::command]
pub async fn inventaire(coeur: Etat<'_>, canal: Channel<EvenementUi>) -> Result<InventaireUi, ErreurCommande> {
    let c = coeur.inner().clone();
    bloquant(move || c.inventaire(&emetteur(canal))).await
}

#[tauri::command]
pub async fn installer(coeur: Etat<'_>, ville_id: Option<u32>, jeton: Option<String>, canal: Channel<EvenementUi>) -> Result<(), ErreurCommande> {
    let c = coeur.inner().clone();
    bloquant(move || c.installer(ville_id, jeton, &emetteur(canal))).await
}

#[tauri::command]
pub async fn appliquer(coeur: Etat<'_>, choix: ChoixUi, canal: Channel<EvenementUi>) -> Result<RapportApplication, ErreurCommande> {
    let c = coeur.inner().clone();
    bloquant(move || c.appliquer(&choix, &emetteur(canal))).await
}

#[tauri::command]
pub async fn verifier(coeur: Etat<'_>, reparer: bool, canal: Channel<EvenementUi>) -> Result<VerificationUi, ErreurCommande> {
    let c = coeur.inner().clone();
    bloquant(move || c.verifier(reparer, &emetteur(canal))).await
}

#[tauri::command]
pub async fn restaurer(coeur: Etat<'_>, canal: Channel<EvenementUi>) -> Result<RestaurationUi, ErreurCommande> {
    let c = coeur.inner().clone();
    bloquant(move || c.restaurer(&emetteur(canal))).await
}

#[tauri::command]
pub fn decider(coeur: Etat<'_>, demande: u64, decision: String) -> Result<(), ErreurCommande> {
    coeur.decider(demande, &decision)
}

#[tauri::command]
pub fn annuler(coeur: Etat<'_>) {
    coeur.annuler();
}

#[tauri::command]
pub async fn couper_debogage(coeur: Etat<'_>) -> Result<(), ErreurCommande> {
    let c = coeur.inner().clone();
    bloquant(move || c.couper_debogage()).await
}

#[tauri::command]
pub async fn ouvrir_sauvegardes(coeur: Etat<'_>) -> Result<(), ErreurCommande> {
    let c = coeur.inner().clone();
    bloquant(move || c.ouvrir_sauvegardes()).await
}
