//! Fausse Mi Box 4 : un `Transport` avec un vrai état mutable (paquets, réglages, accueil, appops...) et un journal
//! des commandes reçues. Sert aux tests des étapes 4-5 et au mode démo. Les commandes sont interprétées à partir du
//! texte réellement envoyé à adb, comme le ferait le shell de la box.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use super::debloat;
use super::transport::{Commande, ErreurTransport, EtatLien, Sortie, Transport, decouper_shell, paquet_valide};

pub const ACCUEIL_ORIGINE: &str = "com.google.android.tvlauncher/.MainActivity";
pub const ZEN_HOME: &str = "tv.zenhome";
const SETUPWRAITH: &str = "com.google.android.tungsten.setupwraith";

#[derive(Debug, Clone)]
pub struct PaquetFake {
    pub systeme: bool,
    pub desactive: bool,
    pub chemin: String,
    /// Octets de l'appli, des données et du cache.
    pub octets: (u64, u64, u64),
    /// « AAAA-MM-JJ HH:MM:SS » comme dans `dumpsys usagestats`.
    pub derniere_utilisation: Option<String>,
    pub libelle: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WifiFake {
    /// Réseau configuré par le système : setupwraith peut être désactivé.
    Libre,
    /// Réseau créé par setupwraith : ne PAS le désactiver.
    LieSetupwraith,
    /// `dumpsys wifi` sans réseau configuré : non vérifiable.
    Vide,
}

#[derive(Debug, Clone)]
pub struct EtatBox {
    pub produit: String,
    pub modele: String,
    pub android: String,
    /// `ro.build.version.sdk`.
    pub sdk: String,
    pub serie: String,
    pub paquets: BTreeMap<String, PaquetFake>,
    pub global: BTreeMap<String, String>,
    pub secure: BTreeMap<String, String>,
    pub system: BTreeMap<String, String>,
    /// Accueil préféré (`set-home-activity`), composant `paquet/.Classe`.
    pub accueil_prefere: Option<String>,
    pub appops: BTreeMap<(String, String), String>,
    pub wifi: WifiFake,
    /// Composant au premier plan.
    pub premier_plan: Option<String>,
    /// Faux : Zen Home n'apparaît jamais au premier plan, même avec la touche Accueil.
    pub zen_home_visible: bool,
    /// Dernière configuration reçue par le ConfigReceiver (le jeton n'est jamais conservé : `***`).
    pub config: BTreeMap<String, String>,
}

#[derive(Debug, Default)]
struct Scenario {
    coupures: u32,
    /// Commandes qui passent avant que `coupures` ne commence.
    sauter: u32,
    /// `uninstall` exécuté sur la box mais dont la réponse se perd (coupure).
    reponse_perdue_desinstallation: u32,
    lectures_vides: u32,
    autorisation_en_attente: bool,
    /// Les commandes répondent « unauthorized » alors que `get-state` dit « device » ; `refus_non_autorise` les compte.
    shell_non_autorise: bool,
    refus_non_autorise: u32,
    /// `ro.serialno` répond vide.
    serie_illisible: bool,
    injoignable: bool,
    signature_incompatible: bool,
    /// Reconnexions encore nécessaires avant que le lien repasse de « offline » à « device ».
    lien_perdu: u32,
    connecte: bool,
}

struct Interne {
    etat: EtatBox,
    scenario: Scenario,
    journal: Vec<String>,
}

pub struct FakeBox(Mutex<Interne>);

fn paquet_systeme(chemin_dossier: &str, p: &str) -> PaquetFake {
    PaquetFake {
        systeme: true,
        desactive: false,
        chemin: format!("/system/{chemin_dossier}/{p}/{p}.apk"),
        octets: (20_000_000, 5_000_000, 100_000),
        derniere_utilisation: None,
        libelle: None,
    }
}

fn paquet_tiers(p: &str, mo: u64, derniere: &str, libelle: &str) -> PaquetFake {
    PaquetFake {
        systeme: false,
        desactive: false,
        chemin: format!("/data/app/{p}-1/base.apk"),
        octets: (mo * 1_048_576 / 2, mo * 1_048_576 / 4, mo * 1_048_576 / 4),
        derniere_utilisation: Some(derniere.to_string()),
        libelle: Some(libelle.to_string()),
    }
}

impl EtatBox {
    /// Mi Box 4 d'origine (Android 9) : accueil d'origine actif, liste de debloat présente, quatre applis tierces.
    pub fn mibox4() -> Self {
        let mut paquets = BTreeMap::new();
        for e in debloat::liste_mibox4() {
            paquets.insert(e.paquet.clone(), paquet_systeme("app", &e.paquet));
        }
        for p in [
            "com.google.android.tvlauncher",
            SETUPWRAITH,
            "com.android.vending",
            "com.android.tv.settings",
            "com.google.android.gms",
        ] {
            paquets.insert(p.to_string(), paquet_systeme("priv-app", p));
        }
        for (p, mo, d, l) in [
            ("com.spotify.tv.android", 19, "2026-10-27 21:03:12", "Spotify"),
            ("com.exemple.lecteur", 200, "2026-10-28 22:00:00", "Lecteur"),
            ("com.exemple.musique", 595, "2026-10-28 20:15:00", "Musique"),
            ("fr.tf1.mytf1", 143, "2026-10-14 20:00:01", "MyTF1"),
        ] {
            paquets.insert(p.to_string(), paquet_tiers(p, mo, d, l));
        }
        let mut secure = BTreeMap::new();
        secure.insert("android_id".to_string(), "0123456789abcdef".to_string());
        Self {
            produit: "oneday".into(),
            modele: "MIBOX4".into(),
            android: "9".into(),
            sdk: "28".into(),
            serie: "TESTSERIAL0001".into(),
            paquets,
            global: BTreeMap::from([("device_name".to_string(), "BOX-TEST".to_string())]),
            secure,
            system: BTreeMap::new(),
            accueil_prefere: None,
            appops: BTreeMap::new(),
            wifi: WifiFake::Libre,
            premier_plan: Some(ACCUEIL_ORIGINE.into()),
            zen_home_visible: true,
            config: BTreeMap::new(),
        }
    }

    fn actif(&self, paquet: &str) -> bool {
        self.paquets.get(paquet).is_some_and(|p| !p.desactive)
    }

    /// Ce que la touche Accueil affiche : l'accueil d'origine tant qu'il est actif (priorité système plus haute que
    /// l'accueil préféré, comme sur la vraie Mi Box), sinon l'accueil préféré.
    fn accueil_affiche(&self) -> Option<String> {
        let origine = ACCUEIL_ORIGINE.split('/').next().unwrap_or_default();
        if self.actif(origine) {
            return Some(ACCUEIL_ORIGINE.into());
        }
        let c = self.accueil_prefere.clone()?;
        let p = c.split('/').next().unwrap_or_default();
        (self.actif(p) && (p != ZEN_HOME || self.zen_home_visible)).then_some(c)
    }
}

fn coupure() -> ErreurTransport {
    ErreurTransport::Coupure("error: device offline".into())
}

fn echec(code: i32, msg: &str) -> ErreurTransport {
    ErreurTransport::Commande { code, stderr: msg.into() }
}

fn sortie(stdout: impl Into<String>) -> Result<Sortie, ErreurTransport> {
    Ok(Sortie { stdout: stdout.into(), stderr: String::new() })
}

fn reglages(m: &BTreeMap<String, String>) -> String {
    m.iter().map(|(k, v)| format!("{k}={v}\n")).collect()
}

fn liste_nombres(valeurs: impl Iterator<Item = u64>) -> String {
    format!("[{}]", valeurs.map(|v| v.to_string()).collect::<Vec<_>>().join(","))
}

impl Interne {
    /// Pannes simulées communes aux commandes (shell, install, uninstall) ; renvoie `Some(sortie vide)` pour une lecture vide.
    fn pannes(&mut self) -> Result<bool, ErreurTransport> {
        let s = &mut self.scenario;
        if !s.connecte || s.injoignable {
            return Err(ErreurTransport::Coupure("error: device '192.168.1.50:5555' not found".into()));
        }
        if s.autorisation_en_attente {
            return Err(ErreurTransport::NonAutorise);
        }
        if s.shell_non_autorise {
            s.refus_non_autorise += 1;
            assert!(s.refus_non_autorise < 50, "la box refuse en boucle : l'appelant ne s'arrête pas");
            return Err(ErreurTransport::NonAutorise);
        }
        if s.lien_perdu > 0 {
            return Err(coupure());
        }
        if s.coupures > 0 {
            if s.sauter > 0 {
                s.sauter -= 1;
            } else {
                s.coupures -= 1;
                return Err(coupure());
            }
        }
        if s.lectures_vides > 0 {
            s.lectures_vides -= 1;
            return Ok(true);
        }
        Ok(false)
    }

    fn executer(&mut self, texte: &str) -> Result<Sortie, ErreurTransport> {
        let mots = decouper_shell(texte);
        let a: Vec<&str> = mots.iter().map(String::as_str).collect();
        let e = &mut self.etat;
        match a.as_slice() {
            ["getprop"] => sortie(
                [
                    ("ro.product.name", &e.produit),
                    ("ro.product.model", &e.modele),
                    ("ro.build.version.release", &e.android),
                    ("ro.build.version.sdk", &e.sdk),
                    ("ro.serialno", &e.serie),
                ]
                .iter()
                .map(|(k, v)| format!("[{k}]: [{v}]\n"))
                .collect::<String>(),
            ),
            ["getprop", k] => sortie(match *k {
                "ro.product.name" => format!("{}\n", e.produit),
                "ro.product.model" => format!("{}\n", e.modele),
                "ro.build.version.release" => format!("{}\n", e.android),
                "ro.build.version.sdk" => format!("{}\n", e.sdk),
                "ro.serialno" if !self.scenario.serie_illisible => format!("{}\n", e.serie),
                _ => "\n".into(),
            }),
            ["pm", "list", "packages", opts @ ..] => {
                let (d, t, s, f) = (opts.contains(&"-d"), opts.contains(&"-3"), opts.contains(&"-s"), opts.contains(&"-f"));
                sortie(
                    e.paquets
                        .iter()
                        .filter(|(_, p)| (!d || p.desactive) && (!t || !p.systeme) && (!s || p.systeme))
                        .map(|(n, p)| if f { format!("package:{}={n}\n", p.chemin) } else { format!("package:{n}\n") })
                        .collect::<String>(),
                )
            }
            ["pm", "disable-user", "--user", "0", p] => match e.paquets.get_mut(*p) {
                Some(x) => {
                    x.desactive = true;
                    sortie(format!("Package {p} new state: disabled-user\n"))
                }
                None => Err(echec(1, &format!("Exception: Unknown package: {p}"))),
            },
            ["pm", "enable", p] => match e.paquets.get_mut(*p) {
                Some(x) => {
                    x.desactive = false;
                    sortie(format!("Package {p} new state: enabled\n"))
                }
                None => Err(echec(1, &format!("Exception: Unknown package: {p}"))),
            },
            ["pm", "grant", p, _permission] => {
                if e.paquets.contains_key(*p) { sortie("") } else { Err(echec(1, &format!("Exception: Unknown package: {p}"))) }
            }
            ["am", "set-standby-bucket", p, _bucket] => {
                if e.paquets.contains_key(*p) { sortie("") } else { Err(echec(1, &format!("Exception: Unknown package: {p}"))) }
            }
            ["settings", "get", ns, k] => {
                let m = espace(e, ns)?;
                sortie(format!("{}\n", m.get(*k).map_or("null", String::as_str)))
            }
            ["settings", "put", ns, k, v] => {
                espace(e, ns)?.insert((*k).into(), (*v).into());
                sortie("")
            }
            ["settings", "delete", ns, k] => {
                let existait = espace(e, ns)?.remove(*k).is_some();
                sortie(format!("Deleted {} rows\n", u8::from(existait)))
            }
            ["settings", "list", ns] => sortie(reglages(espace(e, ns)?)),
            ["dumpsys", "diskstats"] => {
                let noms = e.paquets.keys().map(|n| format!("\"{n}\"")).collect::<Vec<_>>().join(",");
                sortie(format!(
                    "Latency: 1ms [512B Data Write]\nData-Free: 3062280K / 5292456K total = 57% free\n\
                     Package Names: [{noms}]\nApp Sizes: {}\nApp Data Sizes: {}\nCache Sizes: {}\n",
                    liste_nombres(e.paquets.values().map(|p| p.octets.0)),
                    liste_nombres(e.paquets.values().map(|p| p.octets.1)),
                    liste_nombres(e.paquets.values().map(|p| p.octets.2)),
                ))
            }
            ["dumpsys", "usagestats"] => {
                let lignes: String = e
                    .paquets
                    .iter()
                    .filter_map(|(n, p)| {
                        p.derniere_utilisation.as_ref().map(|d| format!("      package={n} totalTime=\"00:10:00\" lastTime=\"{d}\" appLaunchCount=1\n"))
                    })
                    .collect();
                sortie(format!(
                    "USAGE STATS\nIn-memory daily stats\n  user=0\n    packages\nIn-memory yearly stats\n  user=0\n    packages\n{lignes}In-memory monthly stats\n  user=0\n"
                ))
            }
            ["dumpsys", "wifi"] => sortie(match e.wifi {
                WifiFake::Vide => "Wi-Fi is disabled\n\nLatest scan results:\n".to_string(),
                WifiFake::Libre => "Configured networks\nID: 0 SSID: \"RESEAU-TEST\"\ncuid=1000 cname=android.uid.system:1000\n".to_string(),
                WifiFake::LieSetupwraith => format!(
                    "Configured networks\nID: 0 SSID: \"RESEAU-TEST\"\ncreatorName={SETUPWRAITH}\ncuid=10012 cname={SETUPWRAITH}\n"
                ),
            }),
            ["dumpsys", "activity", "activities"] => sortie(match &e.premier_plan {
                Some(c) => format!("  mResumedActivity: ActivityRecord{{2c1d9e0 u0 {c} t2}}\n"),
                None => "  mResumedActivity: null\n".into(),
            }),
            ["dumpsys", "package", "preferred-xml"] => sortie(match &e.accueil_prefere {
                Some(c) => format!(
                    "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\" ?>\n<preferred-activities>\n\
                     <item name=\"{c}\" match=\"100000\" always=\"true\" set=\"2\">\n<set name=\"{c}\" />\n<filter>\n\
                     <action name=\"android.intent.action.MAIN\" />\n<cat name=\"android.intent.category.HOME\" />\n\
                     </filter>\n</item>\n</preferred-activities>\n"
                ),
                None => "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\" ?>\n<preferred-activities>\n</preferred-activities>\n".into(),
            }),
            ["dumpsys", "package", p] => sortie(if e.paquets.contains_key(*p) {
                format!("Packages:\n  Package [{p}] (abc123):\n    versionName=1.0\n")
            } else {
                String::new()
            }),
            ["cmd", "package", "resolve-activity", ..] => sortie(format!(
                "priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=true\n{}\n",
                e.accueil_affiche().unwrap_or_else(|| ACCUEIL_ORIGINE.into())
            )),
            ["cmd", "package", "set-home-activity", c] => {
                let p = c.split('/').next().unwrap_or_default();
                if !e.paquets.contains_key(p) {
                    return Err(echec(1, &format!("Error: Unknown package: {p}")));
                }
                e.accueil_prefere = Some((*c).into());
                sortie("Success\n")
            }
            ["cmd", "appops", "get", p, op] => {
                let defaut = if *op == "RUN_ANY_IN_BACKGROUND" { "allow" } else { "default" };
                let m = e.appops.get(&((*p).into(), (*op).into())).map_or(defaut, String::as_str);
                sortie(format!("{op}: {m}\n"))
            }
            ["cmd", "appops", "set", p, op, mode] => {
                e.appops.insert(((*p).into(), (*op).into()), (*mode).into());
                sortie("")
            }
            ["input", "keyevent", "KEYCODE_HOME"] => {
                e.premier_plan = e.accueil_affiche();
                sortie("")
            }
            ["am", "broadcast", "-a", "tv.zenhome.CONFIG", "-n", "tv.zenhome/.ConfigReceiver", args @ ..] => {
                if !e.actif(ZEN_HOME) {
                    return sortie("Broadcasting: Intent { act=tv.zenhome.CONFIG }\nBroadcast completed: result=0\n");
                }
                if args == ["--ez", "labels", "true"] {
                    let data: String = e
                        .paquets
                        .iter()
                        .filter(|(_, p)| !p.systeme)
                        .map(|(n, p)| format!("{n}\t{}\n", p.libelle.as_deref().unwrap_or(n)))
                        .collect();
                    return sortie(format!("Broadcasting: Intent {{ act=tv.zenhome.CONFIG }}\nBroadcast completed: result=1, data=\"{data}\"\n"));
                }
                for kv in args.chunks(3) {
                    if let ["--es", k, v] = kv {
                        let v = if *k == "tmdb_token" { "***" } else { v };
                        e.config.insert((*k).into(), v.into());
                    }
                }
                sortie("Broadcasting: Intent { act=tv.zenhome.CONFIG }\nBroadcast completed: result=0\n")
            }
            _ => Err(echec(127, &format!("/system/bin/sh: {}: not found", a.first().unwrap_or(&"")))),
        }
    }
}

fn espace<'a>(e: &'a mut EtatBox, ns: &str) -> Result<&'a mut BTreeMap<String, String>, ErreurTransport> {
    match ns {
        "global" => Ok(&mut e.global),
        "secure" => Ok(&mut e.secure),
        "system" => Ok(&mut e.system),
        _ => Err(echec(255, "Error: unknown namespace")),
    }
}

impl FakeBox {
    pub fn mibox4() -> Self {
        Self(Mutex::new(Interne { etat: EtatBox::mibox4(), scenario: Scenario::default(), journal: Vec::new() }))
    }

    fn interne(&self) -> MutexGuard<'_, Interne> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Commandes reçues, dans l'ordre (les commandes sensibles y figurent masquées).
    pub fn journal(&self) -> Vec<String> {
        self.interne().journal.clone()
    }

    /// Copie de l'état courant de la box.
    pub fn instantane(&self) -> EtatBox {
        self.interne().etat.clone()
    }

    /// Modification directe de l'état pour préparer un scénario.
    pub fn modifier(&self, f: impl FnOnce(&mut EtatBox)) {
        f(&mut self.interne().etat);
    }

    /// Les `n` prochaines commandes échouent par une coupure (« device offline »).
    pub fn couper(&self, n: u32) {
        self.interne().scenario.coupures = n;
    }

    /// Après `k` commandes qui passent, les `n` suivantes échouent par une coupure.
    pub fn couper_apres(&self, k: u32, n: u32) {
        let mut i = self.interne();
        i.scenario.sauter = k;
        i.scenario.coupures = n;
    }

    /// Les `n` prochaines désinstallations sont exécutées mais leur réponse est perdue (coupure).
    pub fn perdre_reponse_desinstallation(&self, n: u32) {
        self.interne().scenario.reponse_perdue_desinstallation = n;
    }

    /// Les `n` prochaines commandes répondent vide (lien instable).
    pub fn lectures_vides(&self, n: u32) {
        self.interne().scenario.lectures_vides = n;
    }

    /// Le lien est connecté mais « offline » jusqu'à `k` reconnexions (`connecter`).
    pub fn perdre_lien(&self, k: u32) {
        let mut i = self.interne();
        i.scenario.connecte = true;
        i.scenario.lien_perdu = k;
    }

    /// L'autorisation s'affiche sur la TV et n'est pas acceptée.
    pub fn autorisation_en_attente(&self) {
        self.interne().scenario.autorisation_en_attente = true;
    }

    /// Les commandes sont refusées (« unauthorized ») alors que le lien reste « device ».
    pub fn shell_non_autorise(&self, oui: bool) {
        self.interne().scenario.shell_non_autorise = oui;
    }

    /// `ro.serialno` répond vide (numéro de série illisible).
    pub fn serie_illisible(&self, oui: bool) {
        self.interne().scenario.serie_illisible = oui;
    }

    pub fn accepter_autorisation(&self) {
        self.interne().scenario.autorisation_en_attente = false;
    }

    /// Box éteinte ou hors réseau : plus rien ne passe.
    pub fn injoignable(&self, oui: bool) {
        self.interne().scenario.injoignable = oui;
    }

    /// `install` échoue avec INSTALL_FAILED_UPDATE_INCOMPATIBLE.
    pub fn signature_incompatible(&self, oui: bool) {
        self.interne().scenario.signature_incompatible = oui;
    }

    pub fn modele(&self, produit: &str) {
        self.modifier(|e| e.produit = produit.into());
    }

    pub fn wifi(&self, w: WifiFake) {
        self.modifier(|e| e.wifi = w);
    }

    pub fn zen_home_visible(&self, oui: bool) {
        self.modifier(|e| e.zen_home_visible = oui);
    }

    fn commande(&self, cmd: &Commande, stdin: bool) -> Result<Sortie, ErreurTransport> {
        let mut i = self.interne();
        i.journal.push(format!("{}{}", if stdin { "stdin: " } else { "shell: " }, cmd.afficher()));
        if cmd.sensible() && !stdin {
            return Err(ErreurTransport::Refusee("commande sensible : utiliser shell_stdin".into()));
        }
        if i.pannes()? {
            if cmd.texte() != "getprop ro.serialno" {
                return Ok(Sortie::default());
            }
            // les lectures vides du scénario ne visent pas le contrôle d'identité (voir `serie_illisible`)
            i.scenario.lectures_vides += 1;
        }
        i.executer(cmd.texte())
    }
}

impl Transport for FakeBox {
    fn shell(&self, cmd: &Commande) -> Result<Sortie, ErreurTransport> {
        self.commande(cmd, false)
    }

    fn shell_stdin(&self, cmd: &Commande) -> Result<Sortie, ErreurTransport> {
        self.commande(cmd, true)
    }

    fn install(&self, apk: &Path) -> Result<Sortie, ErreurTransport> {
        let mut i = self.interne();
        i.journal.push(format!("install: {}", apk.display()));
        i.pannes()?;
        if i.scenario.signature_incompatible && i.etat.paquets.contains_key(ZEN_HOME) {
            return Err(echec(1, "Failure [INSTALL_FAILED_UPDATE_INCOMPATIBLE: signatures do not match]"));
        }
        i.etat.paquets.insert(ZEN_HOME.into(), paquet_tiers(ZEN_HOME, 5, "2026-10-28 22:30:00", "Zen Home"));
        sortie("Performing Streamed Install\nSuccess\n")
    }

    fn uninstall(&self, paquet: &str) -> Result<Sortie, ErreurTransport> {
        let mut i = self.interne();
        i.journal.push(format!("uninstall: {paquet}"));
        if !paquet_valide(paquet) {
            return Err(ErreurTransport::Refusee(format!("paquet invalide : {paquet}")));
        }
        i.pannes()?;
        match i.etat.paquets.get(paquet) {
            Some(p) if !p.systeme => {
                i.etat.paquets.remove(paquet);
                if i.etat.accueil_prefere.as_deref().is_some_and(|c| c.starts_with(&format!("{paquet}/"))) {
                    i.etat.accueil_prefere = None;
                }
                if i.scenario.reponse_perdue_desinstallation > 0 {
                    i.scenario.reponse_perdue_desinstallation -= 1;
                    return Err(coupure());
                }
                sortie("Success\n")
            }
            Some(_) => Err(echec(1, "Failure [DELETE_FAILED_INTERNAL_ERROR]")),
            None => Err(echec(1, "Failure [DELETE_FAILED_INTERNAL_ERROR]")),
        }
    }

    fn etat(&self) -> Result<EtatLien, ErreurTransport> {
        let mut i = self.interne();
        i.journal.push("etat".into());
        let s = &i.scenario;
        Ok(if !s.connecte || s.injoignable {
            EtatLien::Absent
        } else if s.autorisation_en_attente {
            EtatLien::NonAutorise
        } else if s.lien_perdu > 0 {
            EtatLien::Offline
        } else {
            EtatLien::Device
        })
    }

    fn connecter(&self) -> Result<(), ErreurTransport> {
        let mut i = self.interne();
        i.journal.push("connect".into());
        if i.scenario.injoignable {
            return Err(ErreurTransport::Coupure("failed to connect to '192.168.1.50:5555': Connection refused".into()));
        }
        i.scenario.connecte = true;
        i.scenario.lien_perdu = i.scenario.lien_perdu.saturating_sub(1);
        Ok(())
    }

    fn deconnecter(&self) {
        let mut i = self.interne();
        i.journal.push("disconnect".into());
        i.scenario.connecte = false;
    }

    fn attendre(&self, _duree: std::time::Duration) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moteur::transport::{ArgConfig, Dumpsys, Espace, OptionPm};

    fn lire(f: &FakeBox, c: Commande) -> String {
        f.shell(&c).unwrap().stdout
    }

    fn connectee() -> FakeBox {
        let f = FakeBox::mibox4();
        f.connecter().unwrap();
        f
    }

    #[test]
    fn rien_ne_passe_sans_connexion() {
        let f = FakeBox::mibox4();
        assert_eq!(f.etat(), Ok(EtatLien::Absent));
        assert!(matches!(f.shell(&Commande::getprop_tout()), Err(ErreurTransport::Coupure(_))));
        f.connecter().unwrap();
        assert_eq!(f.etat(), Ok(EtatLien::Device));
        f.deconnecter();
        assert_eq!(f.etat(), Ok(EtatLien::Absent));
    }

    #[test]
    fn paquets_desactives_et_listes() {
        let f = connectee();
        let tous = lire(&f, Commande::pm_list(&[OptionPm::Chemins]));
        assert!(tous.contains("package:/data/app/com.spotify.tv.android-1/base.apk=com.spotify.tv.android\n"));
        assert!(lire(&f, Commande::pm_list(&[OptionPm::Desactives])).is_empty());
        lire(&f, Commande::pm_disable("com.mitv.tvhome.atv").unwrap());
        assert_eq!(lire(&f, Commande::pm_list(&[OptionPm::Desactives])), "package:com.mitv.tvhome.atv\n");
        // désactivé reste dans la liste complète
        assert!(lire(&f, Commande::pm_list(&[])).contains("package:com.mitv.tvhome.atv\n"));
        lire(&f, Commande::pm_enable("com.mitv.tvhome.atv").unwrap());
        assert!(lire(&f, Commande::pm_list(&[OptionPm::Desactives])).is_empty());
        let tiers = lire(&f, Commande::pm_list(&[OptionPm::Tiers]));
        assert_eq!(tiers.lines().count(), 4);
        assert!(!lire(&f, Commande::pm_list(&[OptionPm::Systeme])).contains("spotify"));
        assert!(matches!(
            f.shell(&Commande::pm_disable("pas.installe").unwrap()),
            Err(ErreurTransport::Commande { code: 1, .. })
        ));
    }

    #[test]
    fn reglages_lecture_ecriture_suppression() {
        let f = connectee();
        assert_eq!(lire(&f, Commande::settings_get(Espace::Global, "zenhome_garder").unwrap()), "null\n");
        lire(&f, Commande::settings_put(Espace::Global, "zenhome_garder", "a.b,c.d").unwrap());
        assert_eq!(lire(&f, Commande::settings_get(Espace::Global, "zenhome_garder").unwrap()), "a.b,c.d\n");
        assert!(lire(&f, Commande::settings_list(Espace::Global)).contains("zenhome_garder=a.b,c.d\n"));
        lire(&f, Commande::settings_put(Espace::Secure, "enabled_accessibility_services", "x/y:z/w").unwrap());
        assert_eq!(f.instantane().secure["enabled_accessibility_services"], "x/y:z/w");
        lire(&f, Commande::settings_delete(Espace::Global, "zenhome_garder").unwrap());
        assert_eq!(lire(&f, Commande::settings_get(Espace::Global, "zenhome_garder").unwrap()), "null\n");
        // une valeur avec apostrophe traverse le quotage sans altération
        lire(&f, Commande::settings_put(Espace::Global, "k", "L'Aigle $(id)").unwrap());
        assert_eq!(f.instantane().global["k"], "L'Aigle $(id)");
    }

    #[test]
    fn accueil_prefere_et_premier_plan() {
        let f = connectee();
        assert!(!lire(&f, Commande::dumpsys(Dumpsys::PreferredXml)).contains("tv.zenhome"));
        assert!(f.shell(&Commande::set_home_activity("tv.zenhome/.Home").unwrap()).is_err(), "Zen Home pas installé");
        f.install(Path::new("ZenHome.apk")).unwrap();
        lire(&f, Commande::set_home_activity("tv.zenhome/.Home").unwrap());
        assert!(lire(&f, Commande::dumpsys(Dumpsys::PreferredXml)).contains("<item name=\"tv.zenhome/.Home\""));
        // l'accueil d'origine reste devant tant qu'il est actif
        lire(&f, Commande::touche_accueil());
        assert!(lire(&f, Commande::dumpsys(Dumpsys::Activities)).contains("com.google.android.tvlauncher/.MainActivity"));
        lire(&f, Commande::pm_disable("com.google.android.tvlauncher").unwrap());
        lire(&f, Commande::touche_accueil());
        assert!(lire(&f, Commande::dumpsys(Dumpsys::Activities)).contains("tv.zenhome/.Home"));
        f.zen_home_visible(false);
        lire(&f, Commande::touche_accueil());
        assert!(lire(&f, Commande::dumpsys(Dumpsys::Activities)).contains("mResumedActivity: null"));
    }

    #[test]
    fn appops_et_wifi() {
        let f = connectee();
        assert_eq!(lire(&f, Commande::appops_get("tv.zenhome", "GET_USAGE_STATS").unwrap()), "GET_USAGE_STATS: default\n");
        lire(&f, Commande::appops_set("tv.zenhome", "GET_USAGE_STATS", "allow").unwrap());
        assert_eq!(lire(&f, Commande::appops_get("tv.zenhome", "GET_USAGE_STATS").unwrap()), "GET_USAGE_STATS: allow\n");
        assert!(lire(&f, Commande::dumpsys(Dumpsys::Wifi)).contains("cname=android.uid.system"));
        f.wifi(WifiFake::LieSetupwraith);
        assert!(lire(&f, Commande::dumpsys(Dumpsys::Wifi)).contains("creatorName=com.google.android.tungsten.setupwraith"));
        f.wifi(WifiFake::Vide);
        assert!(!lire(&f, Commande::dumpsys(Dumpsys::Wifi)).contains("Configured networks"));
    }

    #[test]
    fn installation_desinstallation_et_signature() {
        let f = connectee();
        f.install(Path::new("ZenHome.apk")).unwrap();
        assert!(f.instantane().paquets.contains_key(ZEN_HOME));
        f.signature_incompatible(true);
        let Err(ErreurTransport::Commande { stderr, .. }) = f.install(Path::new("ZenHome.apk")) else { panic!("attendu : échec") };
        assert!(stderr.contains("INSTALL_FAILED_UPDATE_INCOMPATIBLE"));
        assert!(f.uninstall("com.google.android.tvlauncher").is_err(), "un paquet système ne se désinstalle pas");
        f.uninstall("fr.tf1.mytf1").unwrap();
        assert!(!f.instantane().paquets.contains_key("fr.tf1.mytf1"));
        assert!(matches!(f.uninstall("x; id"), Err(ErreurTransport::Refusee(_))));
    }

    #[test]
    fn jeton_via_stdin_jamais_dans_le_journal() {
        let f = connectee();
        f.install(Path::new("ZenHome.apk")).unwrap();
        let c = Commande::broadcast_config(&[ArgConfig::Lat("48.86".into()), ArgConfig::Jeton("SECRET.jeton".into())]).unwrap();
        assert!(matches!(f.shell(&c), Err(ErreurTransport::Refusee(_))));
        assert!(f.shell_stdin(&c).unwrap().stdout.contains("Broadcast completed"));
        let e = f.instantane();
        assert_eq!((e.config["lat"].as_str(), e.config["tmdb_token"].as_str()), ("48.86", "***"));
        let journal = f.journal().join("\n");
        assert!(!journal.contains("SECRET"), "{journal}");
        assert!(journal.contains("stdin: am broadcast") && journal.contains("--es tmdb_token ***"));
    }

    #[test]
    fn libelles_par_broadcast() {
        let f = connectee();
        let c = Commande::broadcast_config(&[ArgConfig::Libelles]).unwrap();
        assert!(f.shell(&c).unwrap().stdout.contains("result=0"), "sans Zen Home, pas de libellés");
        f.install(Path::new("ZenHome.apk")).unwrap();
        let s = f.shell(&c).unwrap().stdout;
        assert!(s.contains("Broadcast completed: result=1, data=\"") && s.contains("com.spotify.tv.android\tSpotify\n"));
    }

    #[test]
    fn pannes_simulees() {
        let f = connectee();
        f.couper(2);
        assert!(matches!(f.shell(&Commande::getprop_tout()), Err(ErreurTransport::Coupure(_))));
        assert!(matches!(f.shell(&Commande::getprop_tout()), Err(ErreurTransport::Coupure(_))));
        assert!(f.shell(&Commande::getprop_tout()).is_ok());
        f.lectures_vides(1);
        assert_eq!(f.shell(&Commande::getprop_tout()).unwrap().non_vide(), Err(ErreurTransport::Vide));
        assert!(f.shell(&Commande::getprop_tout()).unwrap().non_vide().is_ok());
        f.autorisation_en_attente();
        assert_eq!(f.etat(), Ok(EtatLien::NonAutorise));
        assert_eq!(f.shell(&Commande::getprop_tout()), Err(ErreurTransport::NonAutorise));
        f.accepter_autorisation();
        assert!(f.shell(&Commande::getprop_tout()).is_ok());
        f.perdre_lien(2);
        assert_eq!(f.etat(), Ok(EtatLien::Offline));
        f.connecter().unwrap();
        assert_eq!(f.etat(), Ok(EtatLien::Offline));
        f.connecter().unwrap();
        assert_eq!(f.etat(), Ok(EtatLien::Device));
        f.injoignable(true);
        assert!(f.connecter().is_err());
    }

    #[test]
    fn commande_inconnue_et_journal() {
        let f = connectee();
        let c = Commande::dumpsys_paquet("tv.zenhome").unwrap();
        assert_eq!(lire(&f, c), "");
        f.deconnecter();
        assert_eq!(f.journal(), ["connect", "shell: dumpsys package tv.zenhome", "disconnect"]);
    }
}
