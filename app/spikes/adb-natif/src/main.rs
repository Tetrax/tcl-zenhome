//! Essai (étape 3b) : piloter la box avec un client adb natif, sans le binaire adb.
//! Les opérations de la crate `adb_client` n'ont pas de délai global : chacune est donc
//! exécutée dans un thread, et on abandonne l'attente au bout de `DELAI`.

use std::fs::File;
use std::io::{ErrorKind, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use adb_client::tcp::ADBTcpDevice;
use adb_client::{ADBDeviceExt, RustADBError};

const PORT: u16 = 5555;
const DELAI: Duration = Duration::from_secs(15);
const DELAI_INSTALL: Duration = Duration::from_secs(60);
const VEILLE_DUREE: Duration = Duration::from_secs(600);
const VEILLE_PAS: Duration = Duration::from_secs(10);

struct Cible {
    ip: Ipv4Addr,
    cle: Option<PathBuf>,
}

fn main() {
    let code = match lancer(std::env::args().skip(1).collect()) {
        Ok(()) => 0,
        Err(Echec::Usage(m)) => {
            eprintln!("Erreur : {m}\n\n{}", USAGE);
            2
        }
        Err(Echec::Autre(m)) => {
            eprintln!("Erreur : {m}");
            1
        }
    };
    // Termine aussi les threads abandonnés (la crate peut en laisser tourner).
    std::process::exit(code);
}

const USAGE: &str = "Usage :
  adb-natif connecter    --ip A.B.C.D [--cle FICHIER]
  adb-natif shell \"CMD\"  --ip A.B.C.D [--cle FICHIER]
  adb-natif installer APK [--sans-r] --ip A.B.C.D [--cle FICHIER]
  adb-natif veille-test  --ip A.B.C.D [--cle FICHIER]";

enum Echec {
    Usage(String),
    Autre(String),
}

impl From<String> for Echec {
    fn from(m: String) -> Self {
        Echec::Autre(m)
    }
}

fn lancer(args: Vec<String>) -> Result<(), Echec> {
    let mut ip = None;
    let mut cle = None;
    let mut sans_r = false;
    let mut positionnels = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--ip" => {
                let v = it
                    .next()
                    .ok_or_else(|| Echec::Usage("--ip attend une valeur".into()))?;
                ip = Some(v.parse::<Ipv4Addr>().map_err(|_| {
                    Echec::Usage(format!("« {v} » n'est pas une adresse IPv4 valide"))
                })?);
            }
            "--cle" => {
                let v = it
                    .next()
                    .ok_or_else(|| Echec::Usage("--cle attend un fichier".into()))?;
                cle = Some(PathBuf::from(v));
            }
            "--sans-r" => sans_r = true,
            _ => positionnels.push(a),
        }
    }
    let ip = ip.ok_or_else(|| Echec::Usage("--ip est obligatoire".into()))?;
    let cible = Cible { ip, cle };
    let mut pos = positionnels.into_iter();
    let sous_commande = pos
        .next()
        .ok_or_else(|| Echec::Usage("sous-commande manquante".into()))?;
    let argument = pos.next();
    if pos.next().is_some() {
        return Err(Echec::Usage(
            "trop d'arguments (mettre la commande shell entre guillemets)".into(),
        ));
    }
    match (sous_commande.as_str(), argument) {
        ("connecter", None) => Ok(connecter(&cible)?),
        ("shell", Some(cmd)) => Ok(shell(&cible, &cmd)?),
        ("installer", Some(apk)) => Ok(installer(&cible, &apk, sans_r)?),
        ("veille-test", None) => {
            veille_test(&cible);
            Ok(())
        }
        ("shell" | "installer", None) => Err(Echec::Usage("argument manquant".into())),
        _ => Err(Echec::Usage(
            "sous-commande inconnue ou argument en trop".into(),
        )),
    }
}

// ---------------------------------------------------------------- utilitaires

/// Exécute `f` dans un thread ; renvoie son résultat, ou une erreur si le délai est dépassé.
fn avec_delai<T, F>(delai: Duration, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(delai).map_err(|e| match e {
        mpsc::RecvTimeoutError::Timeout => format!(
            "délai de {} s dépassé (opération abandonnée, le thread de la crate peut rester bloqué)",
            delai.as_secs()
        ),
        mpsc::RecvTimeoutError::Disconnected => "panique interne dans la crate adb_client".into(),
    })
}

/// Exécute `f` sur l'appareil avec un délai. L'appareil est rendu avec le résultat ;
/// en cas de dépassement il est perdu (il reste dans le thread abandonné).
fn sur_appareil<T, F>(
    mut dev: ADBTcpDevice,
    delai: Duration,
    f: F,
) -> Result<(ADBTcpDevice, T), String>
where
    T: Send + 'static,
    F: FnOnce(&mut ADBTcpDevice) -> T + Send + 'static,
{
    avec_delai(delai, move || {
        let r = f(&mut dev);
        (dev, r)
    })
}

/// Traduit une erreur de la crate en diagnostic lisible.
fn diagnostic(e: &RustADBError) -> String {
    match e {
        RustADBError::IOError(io)
            if matches!(io.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) =>
        {
            "en attente d'autorisation sur la TV (aucune réponse de la box après l'envoi de la clé)"
                .into()
        }
        RustADBError::IOError(io)
            if matches!(
                io.kind(),
                ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionAborted
                    | ErrorKind::BrokenPipe
                    | ErrorKind::UnexpectedEof
            ) =>
        {
            format!("refusé ou connexion coupée par la box ({io})")
        }
        autre => format!("erreur adb_client : {autre}"),
    }
}

fn ouvrir(c: &Cible) -> Result<ADBTcpDevice, String> {
    let adresse = SocketAddr::from((c.ip, PORT));
    // Sonde TCP avec délai : `ADBTcpDevice::new` utilise `TcpStream::connect` sans délai.
    TcpStream::connect_timeout(&adresse, DELAI)
        .map_err(|e| format!("injoignable : {adresse} ne répond pas ({e})"))?;
    let cle = c.cle.clone();
    if let Some(p) = &cle {
        // Sans fichier, la crate génère une clé en mémoire, jamais sauvegardée.
        if !p.is_file() {
            return Err(format!("fichier de clé introuvable : {}", p.display()));
        }
    }
    let r = avec_delai(DELAI, move || match cle {
        Some(p) => ADBTcpDevice::new_with_custom_private_key(adresse, p),
        None => ADBTcpDevice::new(adresse),
    })?;
    r.map_err(|e| diagnostic(&e))
}

fn pas_vide(sortie: Vec<u8>) -> String {
    String::from_utf8_lossy(&sortie).trim().to_string()
}

fn executer(dev: ADBTcpDevice, cmd: &str) -> Result<(ADBTcpDevice, String, Option<u8>), String> {
    let cmd = cmd.to_string();
    let (dev, r) = sur_appareil(dev, DELAI, move |d| {
        let mut sortie = Vec::new();
        d.shell_command(&cmd, Some(&mut sortie), None)
            .map(|code| (sortie, code))
    })?;
    let (sortie, code) = r.map_err(|e| format!("commande échouée : {e}"))?;
    Ok((dev, pas_vide(sortie), code))
}

// ---------------------------------------------------------------- sous-commandes

fn connecter(c: &Cible) -> Result<(), String> {
    let debut = Instant::now();
    let dev = match ouvrir(c) {
        Ok(d) => d,
        Err(m) => {
            println!("État : {m}");
            return Err("connexion impossible".into());
        }
    };
    println!("État : autorisé ({} ms)", debut.elapsed().as_millis());
    let (_dev, nom, _) = executer(dev, "getprop ro.product.name")?;
    println!("ro.product.name = {nom}");
    Ok(())
}

fn shell(c: &Cible, cmd: &str) -> Result<(), String> {
    let dev = ouvrir(c)?;
    let debut = Instant::now();
    let (_dev, sortie, code) = executer(dev, cmd)?;
    println!("{sortie}");
    match code {
        Some(c) => println!("Code de retour : {c}"),
        None => println!("Code de retour : indisponible (shell v1 : la crate ne le fournit pas)"),
    }
    println!("Durée : {} ms", debut.elapsed().as_millis());
    Ok(())
}

/// Reçoit la sortie de `cmd package install` (arrivée de façon asynchrone via `exec`).
#[derive(Clone)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("verrou empoisonné"))?
            .extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn installer(c: &Cible, apk: &str, sans_r: bool) -> Result<(), String> {
    let mut fichier = File::open(apk).map_err(|e| format!("APK illisible ({apk}) : {e}"))?;
    let taille = fichier.metadata().map_err(|e| e.to_string())?.len();
    let dev = ouvrir(c)?;
    let debut = Instant::now();
    let apk = PathBuf::from(apk);
    let resultat = if sans_r {
        // Chemin de la crate : `cmd package install -S <taille>` (sans -r).
        let (_dev, r) = sur_appareil(dev, DELAI_INSTALL, move |d| d.install(&apk, None))?;
        r.map(|()| "Success".to_string())
            .map_err(|e| format!("{e}"))
    } else {
        // Équivalent de `adb install -r` : la crate n'expose pas -r, on passe par `exec`.
        let sortie = Capture(Arc::new(Mutex::new(Vec::new())));
        let lecture = sortie.clone();
        let (_dev, r) = sur_appareil(dev, DELAI_INSTALL, move |d| {
            let commande = format!("cmd package install -r -S {taille}");
            d.exec(&commande, &mut fichier, Box::new(sortie))?;
            // Le résultat arrive après la fin de l'envoi : on attend « Success » ou « Failure ».
            let fin = Instant::now() + DELAI;
            loop {
                let texte = lecture
                    .0
                    .lock()
                    .map(|b| pas_vide(b.clone()))
                    .unwrap_or_default();
                if texte.contains("Success") || texte.contains("Failure") {
                    return Ok::<String, RustADBError>(texte);
                }
                if Instant::now() > fin {
                    return Ok(format!(
                        "aucun résultat reçu après l'envoi (sortie : « {texte} »)"
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        })?;
        r.map_err(|e| format!("{e}"))
    };
    println!(
        "Durée : {} ms pour {} octets",
        debut.elapsed().as_millis(),
        taille
    );
    match resultat {
        Ok(t) if t.starts_with("Success") => {
            println!("Résultat : installation réussie");
            Ok(())
        }
        Ok(t) => Err(format!("installation non confirmée : {t}")),
        Err(e) => Err(format!("installation échouée : {e}")),
    }
}

fn horodatage() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

fn veille_test(c: &Cible) {
    let debut = Instant::now();
    let mut dev = match ouvrir(c) {
        Ok(d) => Some(d),
        Err(m) => {
            println!("[{}] connexion initiale impossible : {m}", horodatage());
            None
        }
    };
    let mut coupures = 0u32;
    while debut.elapsed() < VEILLE_DUREE {
        let tour = Instant::now();
        let mut panne = None;
        if let Some(d) = dev.take() {
            match executer(d, "getprop ro.product.name") {
                Ok((d, nom, _)) if !nom.is_empty() => {
                    println!("[{}] OK : {nom}", horodatage());
                    dev = Some(d);
                }
                Ok((_, _, _)) => panne = Some("réponse vide".to_string()),
                Err(e) => panne = Some(e),
            }
        } else {
            panne = Some("pas de connexion".to_string());
        }
        if let Some(raison) = panne {
            coupures += 1;
            println!("[{}] COUPURE : {raison}", horodatage());
            println!("[{}] reconnexion...", horodatage());
            match ouvrir(c) {
                Ok(d) => {
                    println!("[{}] reconnecté", horodatage());
                    dev = Some(d);
                }
                Err(m) => println!("[{}] reconnexion échouée : {m}", horodatage()),
            }
        }
        if let Some(reste) = VEILLE_PAS.checked_sub(tour.elapsed()) {
            std::thread::sleep(reste);
        }
    }
    println!(
        "[{}] fin du test : {coupures} coupure(s) en {} s",
        horodatage(),
        debut.elapsed().as_secs()
    );
}
