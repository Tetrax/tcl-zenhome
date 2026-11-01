//! Test de parité : les actions Rust du moteur doivent envoyer à la box EXACTEMENT les mêmes commandes de modification
//! que les scripts bash de référence (tv/deployer.sh --sans-build, tv/verifier-tv.sh --fix, tv/restaurer-tv.sh), à
//! partir du même état initial, et laisser la box dans le même état final.
//!
//! Pour chaque scénario : (1) état initial (`EtatBox`) ; (2) action Rust sur `FakeBox` ; (3) script bash sur le faux adb
//! (`adb-faux`, état sur disque, PROFIL=mibox4, DRY_RUN=0, copie de tv/ sans tv.conf, `sleep` neutralisé) ;
//! (4) comparaison des suites de commandes de modification, normalisées (découpage shell : espaces et quotage) ;
//! (5) comparaison de l'état final et du code de sortie (OK / pas OK).
//!
//! ÉCARTS LÉGITIMES (les seuls tolérés ; toute autre différence échoue) :
//!
//! ```text
//! appops          les scripts appellent `appops set ...`, le moteur `cmd appops set ...` (même service, même effet) ;
//!                 normalisé en `cmd appops ...` des deux côtés.
//! apk             le chemin de l'APK diffère (chemin fourni à l'action / chemin de la copie de tv/) ; normalisé en `APK`.
//! garder          après la restauration, le kit (pas restaurer-tv.sh) supprime `zenhome_garder` ; l'action Rust le fait
//!                 en dernière commande. Vérifié puis retiré de la suite Rust avant comparaison.
//! aucun choix     `zenhome_garder` absent : le moteur traite les entrées [option:garder] comme gardées (AVERT), le kit les
//!                 désactive. Les scénarios partent de la sentinelle `-` (ignorée par le kit) ; le cas absent est testé
//!                 dans verification.rs.
//! position        le moteur masque lat/lon/ville (`***`) dans son journal, comme le jeton ; le faux adb du banc journalise
//!                 les valeurs en clair. Valeurs normalisées en `***` des deux côtés : la parité des valeurs elles-mêmes
//!                 n'est plus comparée ici (elle l'est dans les tests unitaires de installation.rs, via la fausse box).
//! lectures        ordre et nombre des lectures différents (non comparées : aucune ne modifie la box).
//! echecs          le moteur ne s'arrête pas sur une commande de réparation refusée (avertissement) là où le script
//!                 s'arrête (`set -e`) ; aucun scénario ne le provoque.
//! retour arriere  le script réactive tvlauncher ET setupwraith dès que l'un des deux a été désactivé par le passage ;
//!                 le moteur réactive ceux qu'il a désactivés. Identique dans les scénarios ci-dessous (les deux sont
//!                 désactivés ensemble).
//! ```
//!
//! Sans bash, les tests s'ignorent avec un message.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use zen_home_lib::moteur::appareils::MIBOX4;
use zen_home_lib::moteur::connexion::Politique;
use zen_home_lib::moteur::fake_box::{EtatBox, FakeBox, PaquetFake, WifiFake};
use zen_home_lib::moteur::installation::{Parametres, installer};
use zen_home_lib::moteur::restauration::restaurer;
use zen_home_lib::moteur::session::{Reprise, Silencieux};
use zen_home_lib::moteur::transport::{Transport, decouper_shell};
use zen_home_lib::moteur::verification::verifier;

const RACINE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const TRACKER: &str = "tv.alphonso.alphonso_eula";
const KATNISS: &str = "com.google.android.katniss";

type Suite = Vec<Vec<String>>;

// ---------------------------------------------------------------- banc d'essai bash

struct Banc {
    dossier: PathBuf,
}

impl Drop for Banc {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dossier);
    }
}

fn bash_disponible() -> bool {
    let ok = Command::new("/bin/bash").arg("--version").stdout(Stdio::null()).status().is_ok_and(|s| s.success());
    if !ok {
        eprintln!("bash absent : test de parité ignoré");
    }
    ok
}

impl Banc {
    fn nouveau() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dossier = std::env::temp_dir().join(format!("zenhome-parite-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&dossier);
        for d in ["tv", "launcher/out", "bin", "etat"] {
            std::fs::create_dir_all(dossier.join(d)).unwrap();
        }
        // copie des seuls fichiers utiles de tv/ : jamais de tv.conf
        for e in std::fs::read_dir(Path::new(RACINE).join("tv")).unwrap() {
            let e = e.unwrap();
            let nom = e.file_name().to_string_lossy().to_string();
            if nom.ends_with(".sh") || (nom.starts_with("debloat-") && nom.ends_with(".txt")) {
                std::fs::copy(e.path(), dossier.join("tv").join(&nom)).unwrap();
            }
        }
        std::fs::write(dossier.join("launcher/out/ZenHome.apk"), "apk de test").unwrap();
        let adb = dossier.join("bin/adb");
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/parite/adb-faux"), &adb).unwrap();
        std::fs::write(dossier.join("bin/sleep"), "#!/bin/sh\nexit 0\n").unwrap();
        for f in ["bin/adb", "bin/sleep"] {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dossier.join(f), std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self { dossier }
    }

    fn apk(&self) -> PathBuf {
        self.dossier.join("launcher/out/ZenHome.apk")
    }

    fn etat(&self) -> PathBuf {
        self.dossier.join("etat")
    }

    /// Lance un script de tv/ ; renvoie (code de sortie, stderr).
    fn lancer(&self, script: &str, args: &[&str], env: &[(&str, &str)]) -> (i32, String) {
        let path = format!("{}:{}", self.dossier.join("bin").display(), std::env::var("PATH").unwrap_or_default());
        let mut c = Command::new("/bin/bash");
        c.arg(self.dossier.join("tv").join(script))
            .args(args)
            .current_dir(&self.dossier)
            .env("PATH", path)
            .env("FAKE_ADB_ETAT", self.etat())
            .env("TV_HOST", "192.168.1.50")
            .env("PROFIL", "mibox4")
            .env("DRY_RUN", "0")
            .env("TMDB_TOKEN_FILE", "")
            .env_remove("LAT")
            .env_remove("LON")
            .env_remove("CITY")
            .stdin(Stdio::null());
        for (k, v) in env {
            c.env(k, v);
        }
        let sortie = c.output().expect("bash");
        (sortie.status.code().unwrap_or(-1), String::from_utf8_lossy(&sortie.stderr).to_string())
    }
}

// ---------------------------------------------------------------- état sur disque

fn ecrire_etat(dir: &Path, e: &EtatBox) {
    let w = |nom: &str, contenu: String| std::fs::write(dir.join(nom), contenu).unwrap();
    w("pkgs", e.paquets.iter().map(|(n, p)| format!("{n} {} {}\n", u8::from(p.systeme), u8::from(p.desactive))).collect());
    w(
        "props",
        format!(
            "ro.product.name={}\nro.product.model={}\nro.build.version.release={}\nro.build.version.sdk={}\nro.serialno={}\n",
            e.produit, e.modele, e.android, e.sdk, e.serie
        ),
    );
    for (nom, m) in [("global.kv", &e.global), ("secure.kv", &e.secure), ("system.kv", &e.system)] {
        w(nom, m.iter().map(|(k, v)| format!("{k}={v}\n")).collect());
    }
    w("accueil_prefere", format!("{}\n", e.accueil_prefere.clone().unwrap_or_default()));
    w("appops", e.appops.iter().map(|((p, o), m)| format!("{p} {o} {m}\n")).collect());
    w("wifi", match e.wifi {
        WifiFake::Libre => "libre\n",
        WifiFake::LieSetupwraith => "lie\n",
        WifiFake::Vide => "vide\n",
    }
    .into());
    w("premier_plan", format!("{}\n", e.premier_plan.clone().unwrap_or_default()));
    w("zen_visible", format!("{}\n", u8::from(e.zen_home_visible)));
    w("journal", String::new());
}

/// État final dans une forme comparable (lignes triées), côté Rust.
fn canonique_rust(e: &EtatBox) -> Vec<String> {
    let mut l: Vec<String> = e.paquets.iter().map(|(n, p)| format!("paquet {n} sys={} dis={}", u8::from(p.systeme), u8::from(p.desactive))).collect();
    for (nom, m) in [("global", &e.global), ("secure", &e.secure), ("system", &e.system)] {
        l.extend(m.iter().map(|(k, v)| format!("{nom} {k}={v}")));
    }
    l.push(format!("accueil {}", e.accueil_prefere.clone().unwrap_or_default()));
    l.extend(e.appops.iter().map(|((p, o), m)| format!("appops {p} {o} {m}")));
    l.push(format!("premier_plan {}", e.premier_plan.clone().unwrap_or_default()));
    l.sort();
    l
}

/// Même forme, lue dans le dossier d'état du faux adb après le script.
fn canonique_dossier(dir: &Path) -> Vec<String> {
    let lire = |nom: &str| std::fs::read_to_string(dir.join(nom)).unwrap();
    let mut l = Vec::new();
    for ligne in lire("pkgs").lines() {
        let m: Vec<&str> = ligne.split(' ').collect();
        l.push(format!("paquet {} sys={} dis={}", m[0], m[1], m[2]));
    }
    for nom in ["global", "secure", "system"] {
        l.extend(lire(&format!("{nom}.kv")).lines().map(|x| format!("{nom} {x}")));
    }
    l.push(format!("accueil {}", lire("accueil_prefere").trim_end()));
    l.extend(lire("appops").lines().map(|x| format!("appops {x}")));
    l.push(format!("premier_plan {}", lire("premier_plan").trim_end()));
    l.sort();
    l
}

// ---------------------------------------------------------------- suites de commandes

/// Commande de modification de la box ? (mêmes verbes que ceux que journalise `adb-faux`)
fn est_modification(m: &[String]) -> bool {
    let m: Vec<&str> = m.iter().map(String::as_str).collect();
    matches!(
        m.as_slice(),
        ["pm", "disable-user" | "enable" | "grant", ..]
            | ["settings", "put" | "delete", ..]
            | ["cmd", "appops", "set", ..]
            | ["appops", "set", ..]
            | ["cmd", "package", "set-home-activity", ..]
            | ["input", "keyevent", ..]
            | ["am", "broadcast" | "set-standby-bucket", ..]
    )
}

fn est_lecture(m: &[String]) -> bool {
    let m: Vec<&str> = m.iter().map(String::as_str).collect();
    matches!(
        m.as_slice(),
        ["getprop", ..]
            | ["pm", "list", ..]
            | ["settings", "get" | "list", ..]
            | ["dumpsys", ..]
            | ["cmd", "package", "resolve-activity", ..]
            | ["cmd", "appops", "get", ..]
    )
}

/// Écarts « appops » et « apk » (voir l'en-tête).
fn normaliser(mut m: Vec<String>) -> Vec<String> {
    if m.first().map(String::as_str) == Some("appops") {
        m.insert(0, "cmd".into());
    }
    if m.first().map(String::as_str) == Some("install") {
        m[1] = "APK".into();
    }
    if m.first().map(String::as_str) == Some("am") {
        for i in 1..m.len() - 1 {
            if matches!(m[i].as_str(), "lat" | "lon" | "city") && m[i - 1] == "--es" {
                m[i + 1] = "***".into();
            }
        }
    }
    m
}

fn suite_rust(f: &FakeBox) -> Suite {
    let mut suite = Vec::new();
    for l in f.journal() {
        let mots = if let Some(c) = l.strip_prefix("shell: ").or_else(|| l.strip_prefix("stdin: ")) {
            decouper_shell(c)
        } else if let Some(a) = l.strip_prefix("install: ") {
            vec!["install".to_string(), a.to_string()]
        } else if let Some(p) = l.strip_prefix("uninstall: ") {
            vec!["uninstall".to_string(), p.to_string()]
        } else {
            continue; // connect, etat, disconnect
        };
        if mots.first().map(String::as_str) == Some("install") || mots.first().map(String::as_str) == Some("uninstall") || est_modification(&mots) {
            suite.push(normaliser(mots));
        } else {
            assert!(est_lecture(&mots), "commande Rust non classée (lecture ou modification ?) : {l}");
        }
    }
    suite
}

fn suite_bash(dir: &Path) -> Suite {
    let j = std::fs::read_to_string(dir.join("journal")).unwrap();
    j.lines().map(|l| normaliser(l.split('\t').map(String::from).collect())).collect()
}

fn comparer(nom: &str, rust: &Suite, bash: &Suite, stderr_bash: &str) {
    assert!(!bash.is_empty(), "{nom} : le script n'a rien modifié (scénario sans objet) ; stderr : {stderr_bash}");
    assert!(
        !bash.iter().chain(rust).any(|m| m[0] == "INCONNU"),
        "{nom} : commande inconnue du faux adb : {bash:?}"
    );
    if rust == bash {
        return;
    }
    let mut msg = format!("{nom} : suites de commandes différentes\n    {:<3} {:<75} | script bash\n", "n°", "moteur Rust");
    for i in 0..rust.len().max(bash.len()) {
        let r = rust.get(i).map(|m| m.join(" ")).unwrap_or_default();
        let b = bash.get(i).map(|m| m.join(" ")).unwrap_or_default();
        msg += &format!("  {} {i:<3} {r:<75} | {b}\n", if r == b { ' ' } else { '!' });
    }
    panic!("{msg}stderr du script :\n{stderr_bash}");
}

// ---------------------------------------------------------------- scénarios

enum Action<'a> {
    Installer { params: &'a Parametres },
    Reparer,
    Restaurer,
}

struct Scenario<'a> {
    nom: &'a str,
    etat: EtatBox,
    action: Action<'a>,
    /// Variables d'environnement du script (LAT, LON, CITY, TMDB_TOKEN_FILE).
    env: Vec<(&'a str, String)>,
    /// Le script doit sortir en 0 (et le moteur réussir) ; sinon les deux doivent échouer ensemble.
    succes: bool,
}

fn avec_zen_home(e: &mut EtatBox) {
    e.paquets.insert(
        "tv.zenhome".into(),
        PaquetFake {
            systeme: false,
            desactive: false,
            chemin: "/data/app/tv.zenhome-1/base.apk".into(),
            octets: (1, 1, 1),
            derniere_utilisation: None,
            libelle: None,
        },
    );
}

fn parite(s: Scenario) {
    if !bash_disponible() {
        return;
    }
    let banc = Banc::nouveau();
    let apk = banc.apk();
    let f = FakeBox::mibox4();
    f.connecter().unwrap();
    let initial = s.etat.clone();
    f.modifier(|e| *e = s.etat.clone());
    ecrire_etat(&banc.etat(), &initial);

    // (2) moteur Rust
    let pol = Politique::default();
    let ok_rust = match &s.action {
        Action::Installer { params } => {
            let p = Parametres { apk: apk.clone(), lat: params.lat.clone(), lon: params.lon.clone(), ville: params.ville.clone(), jeton: params.jeton.clone() };
            installer(&f, &MIBOX4, &pol, &mut Silencieux, &p, &Reprise::default()).map(|_| true).unwrap_or(false)
        }
        Action::Reparer => verifier(&f, &MIBOX4, &pol, &mut Silencieux, true, Some(&apk), &Reprise::default()).expect("vérification").ok,
        Action::Restaurer => restaurer(&f, &MIBOX4, &pol, &mut Silencieux, &Reprise::default(), None).map(|_| true).unwrap_or(false),
    };
    let mut rust = suite_rust(&f);

    // (3) script bash
    let env: Vec<(&str, &str)> = s.env.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let (code, stderr) = match &s.action {
        Action::Installer { .. } => banc.lancer("deployer.sh", &["--sans-build"], &env),
        Action::Reparer => banc.lancer("verifier-tv.sh", &["--fix"], &env),
        Action::Restaurer => banc.lancer("restaurer-tv.sh", &[], &env),
    };
    let bash = suite_bash(&banc.etat());

    // écart « garder » : suppression finale de zenhome_garder par le kit
    if matches!(s.action, Action::Restaurer) {
        let fin: Vec<String> = ["settings", "delete", "global", "zenhome_garder"].map(String::from).to_vec();
        assert_eq!(rust.pop(), Some(fin), "{} : le moteur doit finir par supprimer zenhome_garder", s.nom);
    }

    // (4) et (5)
    comparer(s.nom, &rust, &bash, &stderr);
    let mut final_rust = canonique_rust(&f.instantane());
    let mut final_bash = canonique_dossier(&banc.etat());
    if matches!(s.action, Action::Restaurer) {
        final_bash.retain(|l| l != "global zenhome_garder=x");
        final_rust.retain(|l| l != "global zenhome_garder=x");
    }
    assert_eq!(final_rust, final_bash, "{} : états finaux différents", s.nom);
    assert_eq!(ok_rust, code == 0, "{} : le moteur dit ok={ok_rust}, le script sort avec {code}\n{stderr}", s.nom);
    if s.succes {
        assert!(ok_rust, "{} : succès attendu", s.nom);
    }
}

fn base() -> EtatBox {
    let mut e = EtatBox::mibox4();
    // écart « aucun choix » : clé absente, le moteur garde les [option:garder] là où le kit les désactive ; les scénarios
    // partent donc de « l'utilisateur ne garde rien » (sentinelle `-`, sans paquet valide pour le kit).
    e.global.insert("zenhome_garder".into(), "-".into());
    e
}

fn sans_params() -> Parametres {
    Parametres { apk: PathBuf::new(), lat: None, lon: None, ville: None, jeton: None }
}

#[test]
fn installation_a_neuf() {
    let params = sans_params();
    parite(Scenario { nom: "installation à neuf", etat: base(), action: Action::Installer { params: &params }, env: vec![], succes: true });
}

#[test]
fn installation_avec_configuration_jeton_et_android_13() {
    let banc_jeton = std::env::temp_dir().join(format!("zenhome-parite-jeton-{}", std::process::id()));
    std::fs::write(&banc_jeton, "abc.DEF-123_~+/=\n").unwrap();
    let mut etat = base();
    etat.sdk = "33".into();
    let params = Parametres {
        apk: PathBuf::new(),
        lat: Some("48.86".into()),
        lon: Some("-2.35".into()),
        ville: Some("L'Aigle".into()),
        jeton: Some("abc.DEF-123_~+/=".into()),
    };
    let env = vec![
        ("LAT", "48.86".to_string()),
        ("LON", "-2.35".to_string()),
        ("CITY", "L'Aigle".to_string()),
        ("TMDB_TOKEN_FILE", banc_jeton.display().to_string()),
    ];
    parite(Scenario { nom: "installation avec config, jeton, SDK 33", etat, action: Action::Installer { params: &params }, env, succes: true });
    let _ = std::fs::remove_file(banc_jeton);
}

#[test]
fn installation_keyservice_seul() {
    let params = sans_params();
    let mut etat = base();
    avec_zen_home(&mut etat);
    etat.secure.insert("enabled_accessibility_services".into(), "tv.zenhome/tv.zenhome.KeyService".into());
    etat.secure.insert("accessibility_enabled".into(), "1".into());
    parite(Scenario { nom: "KeyService seul", etat, action: Action::Installer { params: &params }, env: vec![], succes: true });
}

#[test]
fn installation_keyservice_avec_un_autre_service_et_forme_courte() {
    let params = sans_params();
    let mut etat = base();
    avec_zen_home(&mut etat);
    etat.secure.insert("enabled_accessibility_services".into(), "com.autre/.Service:tv.zenhome/.KeyService:com.b/.C".into());
    parite(Scenario { nom: "KeyService parmi d'autres", etat, action: Action::Installer { params: &params }, env: vec![], succes: true });
    // un autre service sans KeyService : ajout à la suite
    let mut etat = base();
    avec_zen_home(&mut etat);
    etat.secure.insert("enabled_accessibility_services".into(), "com.autre/.Service".into());
    parite(Scenario { nom: "autre service seul", etat, action: Action::Installer { params: &params }, env: vec![], succes: true });
}

#[test]
fn verification_fix_sur_box_fraiche() {
    parite(Scenario { nom: "fix box fraîche", etat: base(), action: Action::Reparer, env: vec![], succes: true });
}

#[test]
fn verification_fix_avec_zenhome_garder() {
    let mut etat = base();
    etat.global.insert("zenhome_garder".into(), format!("{TRACKER},{KATNISS}"));
    parite(Scenario { nom: "fix avec zenhome_garder", etat, action: Action::Reparer, env: vec![], succes: true });
}

#[test]
fn verification_fix_wifi_lie_a_setupwraith() {
    for (nom, wifi) in [("Wi-Fi lié", WifiFake::LieSetupwraith), ("Wi-Fi non vérifiable", WifiFake::Vide)] {
        let mut etat = base();
        etat.wifi = wifi;
        parite(Scenario { nom, etat, action: Action::Reparer, env: vec![], succes: true });
    }
}

#[test]
fn verification_fix_zen_home_pas_au_premier_plan() {
    let mut etat = base();
    etat.zen_home_visible = false;
    parite(Scenario { nom: "retour arrière", etat, action: Action::Reparer, env: vec![], succes: false });
}

#[test]
fn verification_fix_apres_mise_a_jour_de_la_box() {
    // Zen Home est là, mais la mise à jour a tout réinitialisé : accueil, services, veille, applis réactivées.
    let mut etat = base();
    avec_zen_home(&mut etat);
    etat.secure.insert("enabled_accessibility_services".into(), "com.autre/.Service".into());
    etat.secure.insert("screensaver_components".into(), "com.google.android.backdrop/.Backdrop".into());
    etat.global.insert("zenhome_garder".into(), TRACKER.into());
    parite(Scenario { nom: "après mise à jour", etat, action: Action::Reparer, env: vec![], succes: true });
}

fn box_installee() -> EtatBox {
    let mut e = base();
    avec_zen_home(&mut e);
    for p in ["com.google.android.tvlauncher", "com.google.android.tungsten.setupwraith", TRACKER, KATNISS, "com.mitv.tvhome.atv"] {
        if let Some(x) = e.paquets.get_mut(p) {
            x.desactive = true;
        }
    }
    e.accueil_prefere = Some("tv.zenhome/.Home".into());
    e.premier_plan = Some("tv.zenhome/.Home".into());
    e.secure.insert("enabled_accessibility_services".into(), "com.autre/.Service:tv.zenhome/tv.zenhome.KeyService".into());
    e.secure.insert("accessibility_enabled".into(), "1".into());
    e.secure.insert("screensaver_components".into(), "tv.zenhome/tv.zenhome.ZenDream".into());
    for k in ["zenhome_dock", "zenhome_layout", "zenhome_rev", "zenhome_title_mode", "zenhome_garder"] {
        e.global.insert(k.into(), "x".into());
    }
    e.global.insert("window_animation_scale".into(), "0.5".into());
    e.appops.insert(("com.spotify.tv.android".into(), "RUN_ANY_IN_BACKGROUND".into()), "ignore".into());
    e
}

#[test]
fn restauration() {
    parite(Scenario { nom: "restauration", etat: box_installee(), action: Action::Restaurer, env: vec![], succes: true });
}

#[test]
fn restauration_keyservice_seul() {
    let mut etat = box_installee();
    etat.secure.insert("enabled_accessibility_services".into(), "tv.zenhome/tv.zenhome.KeyService".into());
    parite(Scenario { nom: "restauration, KeyService seul", etat, action: Action::Restaurer, env: vec![], succes: true });
}

#[test]
fn le_journal_du_faux_adb_ne_contient_jamais_le_jeton() {
    // Contrôle du banc lui-même : le jeton envoyé par stdin est masqué dans les deux journaux.
    if !bash_disponible() {
        return;
    }
    let banc = Banc::nouveau();
    ecrire_etat(&banc.etat(), &base());
    let (code, _) = banc.lancer(
        "deployer.sh",
        &["--sans-build"],
        &[("TMDB_TOKEN_FILE", banc.dossier.join("jeton").to_str().unwrap())],
    );
    // fichier absent : pas de jeton, le script réussit
    assert_eq!(code, 0);
    std::fs::write(banc.dossier.join("jeton"), "TOKEN.secret-9\n").unwrap();
    ecrire_etat(&banc.etat(), &base());
    let (code, _) = banc.lancer("deployer.sh", &["--sans-build"], &[("TMDB_TOKEN_FILE", banc.dossier.join("jeton").to_str().unwrap())]);
    assert_eq!(code, 0);
    let journal = std::fs::read_to_string(banc.etat().join("journal")).unwrap();
    assert!(!journal.contains("TOKEN.secret"), "{journal}");
    assert!(journal.contains("tmdb_token\t***"));
}
