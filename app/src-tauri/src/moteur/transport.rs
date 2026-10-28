//! Transport vers la box : trait `Transport`, commandes validées et implémentation `AdbProcess` (sous-processus adb).
//!
//! Sécurité : `adb shell` recolle ses arguments en une seule ligne que le shell de la BOX interprète.
//! Une commande n'existe donc que sous la forme `Commande`, construite par nos constructeurs à partir de valeurs
//! validées (paquets, clés, nombres) ou quotées (`shq`). Aucun transport n'accepte de chaîne brute.

use std::fmt;
use std::io::{Read, Write};
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const PORT_SERVEUR_ADB: &str = "5038";
pub const PORT_BOX: u16 = 5555;
/// Au-delà, la sortie d'une commande est tronquée (dumpsys usagestats peut peser plusieurs Mo).
pub const SORTIE_MAX: usize = 16 * 1024 * 1024;
const DELAI_COMMANDE: Duration = Duration::from_secs(30);
const DELAI_CONNEXION: Duration = Duration::from_secs(10);
const DELAI_INSTALLATION: Duration = Duration::from_secs(180);

// ---------------------------------------------------------------- validations et quotage

/// Nom de paquet : `^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$`, 255 caractères au plus (RE_PKG du kit).
pub fn paquet_valide(s: &str) -> bool {
    if s.is_empty() || s.len() > 255 {
        return false;
    }
    let mut segments = 0;
    for seg in s.split('.') {
        segments += 1;
        let mut c = seg.chars();
        if !c.next().is_some_and(|f| f.is_ascii_alphabetic()) || !c.all(|x| x.is_ascii_alphanumeric() || x == '_') {
            return false;
        }
    }
    segments >= 2
}

/// Quote une valeur pour le shell de la box (équivalent de `shq` de lib.sh) : entre apostrophes, `'` devient `'\''`.
pub fn shq(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Découpe une ligne de commande comme un shell POSIX (apostrophes, antislash). Sert au `FakeBox` et aux tests de `shq`.
pub fn decouper_shell(ligne: &str) -> Vec<String> {
    let (mut mots, mut cour, mut en_mot, mut guillemet) = (Vec::new(), String::new(), false, false);
    let mut it = ligne.chars();
    while let Some(c) = it.next() {
        match c {
            '\'' => {
                guillemet = !guillemet;
                en_mot = true;
            }
            _ if guillemet => cour.push(c),
            '\\' => {
                en_mot = true;
                cour.extend(it.next());
            }
            c if c.is_whitespace() => {
                if en_mot {
                    mots.push(std::mem::take(&mut cour));
                    en_mot = false;
                }
            }
            c => {
                cour.push(c);
                en_mot = true;
            }
        }
    }
    if en_mot {
        mots.push(cour);
    }
    mots
}

fn refus<T>(msg: impl Into<String>) -> Result<T, ErreurTransport> {
    Err(ErreurTransport::Refusee(msg.into()))
}

/// Clé de réglage ou de propriété : lettres, chiffres, `_`, `.`, `-`.
fn cle_valide(s: &str) -> bool {
    (1..=128).contains(&s.len()) && !s.starts_with('-') && s.chars().all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
}

/// Caractères de format Unicode (catégorie Cf) : bidi, largeur nulle, etc.
pub(super) fn est_format(c: char) -> bool {
    matches!(u32::from(c),
        0xAD | 0x600..=0x605 | 0x61C | 0x6DD | 0x70F | 0x8E2 | 0x180E | 0x200B..=0x200F | 0x202A..=0x202E
        | 0x2060..=0x2064 | 0x2066..=0x206F | 0xFEFF | 0xFFF9..=0xFFFB | 0x110BD | 0x1D173..=0x1D17A
        | 0xE0001 | 0xE0020..=0xE007F)
}

/// Longueur maximale d'un texte venant de la box dans une erreur, un avertissement ou un événement.
const TEXTE_BOX_MAX: usize = 300;

/// Texte venant de la box, avant tout affichage : sans contrôles C0/C1 (`\r` compris), formats Unicode (bidi,
/// largeur nulle) ni séparateurs de ligne ; sauts de ligne et tabulations deviennent des espaces ; 300 caractères au plus.
pub(super) fn assainir(brut: &str) -> String {
    let propre: String = brut
        .chars()
        .filter_map(|c| match c {
            '\n' | '\t' => Some(' '),
            c if c.is_control() || est_format(c) || c == '\u{2028}' || c == '\u{2029}' => None,
            c => Some(c),
        })
        .take(TEXTE_BOX_MAX)
        .collect();
    propre.trim().to_string()
}

/// Nombre décimal : `^-?[0-9]+(\.[0-9]+)?$` (LAT/LON du kit).
fn nombre_valide(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    let (ent, dec) = s.split_once('.').map_or((s, None), |(e, d)| (e, Some(d)));
    let chiffres = |t: &str| !t.is_empty() && t.chars().all(|c| c.is_ascii_digit());
    chiffres(ent) && dec.is_none_or(chiffres)
}

/// Jeton TMDB : `^[A-Za-z0-9._~+/=-]+$`.
fn jeton_valide(s: &str) -> bool {
    !s.is_empty() && s.len() <= 2048 && s.chars().all(|c| c.is_ascii_alphanumeric() || "._~+/=-".contains(c))
}

/// Texte libre destiné à `shq` : ni contrôle (dont saut de ligne et NUL), ni plus de `max` caractères.
fn texte_libre_valide(s: &str, max: usize) -> bool {
    s.chars().count() <= max && !s.chars().any(char::is_control)
}

/// `paquet/.Classe` ou `paquet/pkg.Classe` (pas de `$` : le shell de la box l'interpréterait).
pub fn composant_valide(s: &str) -> bool {
    s.split_once('/').is_some_and(|(p, c)| {
        paquet_valide(p) && !c.is_empty() && c.chars().all(|x| x.is_ascii_alphanumeric() || x == '_' || x == '.')
    })
}

fn operation_valide(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

// ---------------------------------------------------------------- erreurs et sorties

#[derive(Debug, Clone, PartialEq)]
pub enum ErreurTransport {
    /// Le lien adb a décroché (offline, introuvable, refusé, délai...) : une reconnexion peut suffire.
    Coupure(String),
    /// La box attend que l'utilisateur accepte l'autorisation à l'écran.
    NonAutorise,
    /// La commande a tourné sur la box mais a échoué (`stderr`, à défaut la sortie standard).
    Commande { code: i32, stderr: String },
    /// Réponse vide là où une réponse était attendue (voir `Sortie::non_vide`) : lien instable probable.
    Vide,
    /// Valeur ou commande refusée avant tout envoi (validation).
    Refusee(String),
    /// adb n'a pas pu être lancé.
    Lancement(String),
}

impl fmt::Display for ErreurTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coupure(m) => write!(f, "liaison coupée : {m}"),
            Self::NonAutorise => write!(f, "autorisation à accepter sur la TV"),
            Self::Commande { code, stderr } => write!(f, "commande en échec (code {code}) : {stderr}"),
            Self::Vide => write!(f, "réponse vide"),
            Self::Refusee(m) => write!(f, "valeur refusée : {m}"),
            Self::Lancement(m) => write!(f, "adb impossible à lancer : {m}"),
        }
    }
}

impl std::error::Error for ErreurTransport {}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sortie {
    pub stdout: String,
    pub stderr: String,
}

impl Sortie {
    /// Erreur `Vide` si la sortie standard ne contient que des blancs.
    pub fn non_vide(self) -> Result<Self, ErreurTransport> {
        if self.stdout.trim().is_empty() { Err(ErreurTransport::Vide) } else { Ok(self) }
    }
}

/// État du lien adb avec la box (`adb get-state`).
#[derive(Debug, Clone, PartialEq)]
pub enum EtatLien {
    Device,
    Offline,
    NonAutorise,
    /// Pas connectée (« device not found », « no devices »).
    Absent,
    Autre(String),
}

/// Classe un échec d'adb d'après son texte (stderr + stdout). Mêmes familles que `lancer_script` du kit.
pub fn classer_erreur(code: i32, texte: &str) -> ErreurTransport {
    let t = texte.to_lowercase();
    let a = |m: &str| t.contains(m);
    if a("unauthorized") || a("failed to authenticate") || a("still authorizing") {
        ErreurTransport::NonAutorise
    } else if a("device offline")
        || (a("device") && a("not found"))
        || a("no devices")
        || a("error: closed")
        || a("connection refused")
        || a("no route to host")
        || a("timed out")
        || a("connection reset")
        || a("failed to connect")
        || a("cannot connect")
        || a("broken pipe")
    {
        ErreurTransport::Coupure(assainir(texte))
    } else {
        ErreurTransport::Commande { code, stderr: assainir(texte) }
    }
}

fn etat_depuis(code: i32, texte: &str) -> EtatLien {
    let t = texte.trim().to_lowercase();
    if code == 0 && t == "device" {
        EtatLien::Device
    } else if t.contains("unauthorized") || t.contains("authorizing") {
        EtatLien::NonAutorise
    } else if t.contains("offline") {
        EtatLien::Offline
    } else if t.contains("not found") || t.contains("no devices") {
        EtatLien::Absent
    } else {
        EtatLien::Autre(assainir(&t.chars().take(80).collect::<String>()))
    }
}

// ---------------------------------------------------------------- commandes

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Espace {
    Global,
    Secure,
    System,
}

impl Espace {
    pub fn nom(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Secure => "secure",
            Self::System => "system",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptionPm {
    Desactives,
    Tiers,
    Systeme,
    Chemins,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dumpsys {
    Diskstats,
    Usagestats,
    Wifi,
    Activities,
    PreferredXml,
}

/// Arguments du broadcast `tv.zenhome.CONFIG` (ConfigReceiver de Zen Home).
/// `Debug` masque la position et le jeton.
#[derive(Clone, PartialEq)]
pub enum ArgConfig {
    Lat(String),
    Lon(String),
    Ville(String),
    Jeton(String),
    Libelles,
}

impl fmt::Debug for ArgConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lat(_) => f.write_str("Lat(***)"),
            Self::Lon(_) => f.write_str("Lon(***)"),
            Self::Ville(_) => f.write_str("Ville(***)"),
            Self::Jeton(_) => f.write_str("Jeton(***)"),
            Self::Libelles => f.write_str("Libelles"),
        }
    }
}

/// Commande exécutée par le shell de la box. Seuls les constructeurs ci-dessous la fabriquent.
/// Le texte réel d'une commande sensible (jeton, position) n'est jamais affiché : `Debug`/`Display` montrent la version masquée.
#[derive(Clone, PartialEq)]
pub struct Commande {
    texte: String,
    masque: Option<String>,
}

impl Commande {
    fn simple(texte: impl Into<String>) -> Self {
        Self { texte: texte.into(), masque: None }
    }

    /// Texte réellement envoyé (jamais à journaliser ni afficher).
    pub(super) fn texte(&self) -> &str {
        &self.texte
    }

    /// Texte sûr à afficher ou à journaliser.
    pub fn afficher(&self) -> &str {
        self.masque.as_deref().unwrap_or(&self.texte)
    }

    /// Vrai si la commande contient un secret ou la position : elle ne passe que par `shell_stdin`.
    pub fn sensible(&self) -> bool {
        self.masque.is_some()
    }

    pub fn getprop_tout() -> Self {
        Self::simple("getprop")
    }

    pub fn getprop(cle: &str) -> Result<Self, ErreurTransport> {
        if !cle_valide(cle) {
            return refus(format!("propriété invalide : {cle}"));
        }
        Ok(Self::simple(format!("getprop {cle}")))
    }

    pub fn pm_list(options: &[OptionPm]) -> Self {
        let mut t = String::from("pm list packages");
        for o in options {
            t.push_str(match o {
                OptionPm::Desactives => " -d",
                OptionPm::Tiers => " -3",
                OptionPm::Systeme => " -s",
                OptionPm::Chemins => " -f",
            });
        }
        Self::simple(t)
    }

    pub fn settings_list(espace: Espace) -> Self {
        Self::simple(format!("settings list {}", espace.nom()))
    }

    pub fn settings_get(espace: Espace, cle: &str) -> Result<Self, ErreurTransport> {
        if !cle_valide(cle) {
            return refus(format!("clé de réglage invalide : {cle}"));
        }
        Ok(Self::simple(format!("settings get {} {cle}", espace.nom())))
    }

    pub fn settings_put(espace: Espace, cle: &str, valeur: &str) -> Result<Self, ErreurTransport> {
        if !cle_valide(cle) || !texte_libre_valide(valeur, 1024) {
            return refus(format!("réglage invalide : {cle}"));
        }
        Ok(Self::simple(format!("settings put {} {cle} {}", espace.nom(), shq(valeur))))
    }

    pub fn settings_delete(espace: Espace, cle: &str) -> Result<Self, ErreurTransport> {
        if !cle_valide(cle) {
            return refus(format!("clé de réglage invalide : {cle}"));
        }
        Ok(Self::simple(format!("settings delete {} {cle}", espace.nom())))
    }

    pub fn dumpsys(quoi: Dumpsys) -> Self {
        Self::simple(match quoi {
            Dumpsys::Diskstats => "dumpsys diskstats",
            Dumpsys::Usagestats => "dumpsys usagestats",
            Dumpsys::Wifi => "dumpsys wifi",
            Dumpsys::Activities => "dumpsys activity activities",
            Dumpsys::PreferredXml => "dumpsys package preferred-xml",
        })
    }

    pub fn dumpsys_paquet(paquet: &str) -> Result<Self, ErreurTransport> {
        if !paquet_valide(paquet) {
            return refus(format!("paquet invalide : {paquet}"));
        }
        Ok(Self::simple(format!("dumpsys package {paquet}")))
    }

    /// Activité d'accueil résolue (accueil actuel du kit, `accueil.txt`).
    pub fn resoudre_accueil() -> Self {
        Self::simple(
            "cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.HOME",
        )
    }

    pub fn set_home_activity(composant: &str) -> Result<Self, ErreurTransport> {
        if !composant_valide(composant) {
            return refus(format!("composant invalide : {composant}"));
        }
        Ok(Self::simple(format!("cmd package set-home-activity {composant}")))
    }

    pub fn pm_disable(paquet: &str) -> Result<Self, ErreurTransport> {
        if !paquet_valide(paquet) {
            return refus(format!("paquet invalide : {paquet}"));
        }
        Ok(Self::simple(format!("pm disable-user --user 0 {paquet}")))
    }

    pub fn pm_enable(paquet: &str) -> Result<Self, ErreurTransport> {
        if !paquet_valide(paquet) {
            return refus(format!("paquet invalide : {paquet}"));
        }
        Ok(Self::simple(format!("pm enable {paquet}")))
    }

    pub fn appops_get(paquet: &str, operation: &str) -> Result<Self, ErreurTransport> {
        if !paquet_valide(paquet) || !operation_valide(operation) {
            return refus(format!("appops invalide : {paquet} {operation}"));
        }
        Ok(Self::simple(format!("cmd appops get {paquet} {operation}")))
    }

    pub fn appops_set(paquet: &str, operation: &str, mode: &str) -> Result<Self, ErreurTransport> {
        if !paquet_valide(paquet) || !operation_valide(operation) || !["allow", "ignore", "deny", "default"].contains(&mode) {
            return refus(format!("appops invalide : {paquet} {operation} {mode}"));
        }
        Ok(Self::simple(format!("cmd appops set {paquet} {operation} {mode}")))
    }

    /// `pm grant` d'une permission runtime (nom de permission pointé, comme un nom de paquet).
    pub fn pm_grant(paquet: &str, permission: &str) -> Result<Self, ErreurTransport> {
        if !paquet_valide(paquet) || !paquet_valide(permission) {
            return refus(format!("permission invalide : {paquet} {permission}"));
        }
        Ok(Self::simple(format!("pm grant {paquet} {permission}")))
    }

    pub fn standby_bucket(paquet: &str, bucket: &str) -> Result<Self, ErreurTransport> {
        if !paquet_valide(paquet) || !["active", "working_set", "frequent", "rare"].contains(&bucket) {
            return refus(format!("standby bucket invalide : {paquet} {bucket}"));
        }
        Ok(Self::simple(format!("am set-standby-bucket {paquet} {bucket}")))
    }

    pub fn touche_accueil() -> Self {
        Self::simple("input keyevent KEYCODE_HOME")
    }

    /// Broadcast de configuration de Zen Home. Avec un jeton ou une position, la commande est sensible (version masquée `***`).
    pub fn broadcast_config(args: &[ArgConfig]) -> Result<Self, ErreurTransport> {
        if args.is_empty() {
            return refus("broadcast sans argument");
        }
        let base = "am broadcast -a tv.zenhome.CONFIG -n tv.zenhome/.ConfigReceiver";
        let (mut texte, mut masque, mut sensible) = (base.to_string(), base.to_string(), false);
        for a in args {
            let (reel, cache) = match a {
                ArgConfig::Lat(v) | ArgConfig::Lon(v) if !nombre_valide(v) => return refus("coordonnée invalide"),
                ArgConfig::Lat(v) => {
                    sensible = true;
                    (format!(" --es lat {v}"), " --es lat ***".to_string())
                }
                ArgConfig::Lon(v) => {
                    sensible = true;
                    (format!(" --es lon {v}"), " --es lon ***".to_string())
                }
                ArgConfig::Ville(v) if v.is_empty() || !texte_libre_valide(v, 60) => return refus("ville invalide"),
                ArgConfig::Ville(v) => {
                    sensible = true;
                    (format!(" --es city {}", shq(v)), " --es city ***".to_string())
                }
                ArgConfig::Jeton(v) if !jeton_valide(v) => return refus("jeton au format inattendu"),
                ArgConfig::Jeton(v) => {
                    sensible = true;
                    (format!(" --es tmdb_token {}", shq(v)), " --es tmdb_token ***".to_string())
                }
                ArgConfig::Libelles => (" --ez labels true".to_string(), " --ez labels true".to_string()),
            };
            texte.push_str(&reel);
            masque.push_str(&cache);
        }
        Ok(Self { texte, masque: sensible.then_some(masque) })
    }
}

impl fmt::Debug for Commande {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Commande({:?})", self.afficher())
    }
}

impl fmt::Display for Commande {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.afficher())
    }
}

// ---------------------------------------------------------------- trait

pub trait Transport {
    /// Exécute une commande non sensible via `adb shell`. Les commandes sensibles sont refusées (voir `shell_stdin`).
    fn shell(&self, cmd: &Commande) -> Result<Sortie, ErreurTransport>;
    /// Comme `deployer.sh` pour le jeton : la commande est envoyée sur l'entrée standard d'un `adb shell` nu,
    /// donc absente de la liste des processus du Mac. Utiliser pour toute commande `sensible()`.
    fn shell_stdin(&self, cmd: &Commande) -> Result<Sortie, ErreurTransport>;
    /// `adb install -r` (ne désinstalle jamais rien).
    fn install(&self, apk: &Path) -> Result<Sortie, ErreurTransport>;
    fn uninstall(&self, paquet: &str) -> Result<Sortie, ErreurTransport>;
    fn etat(&self) -> Result<EtatLien, ErreurTransport>;
    /// `adb connect` : `Ok` ne garantit pas que le lien est `Device` (voir `etat`).
    fn connecter(&self) -> Result<(), ErreurTransport>;
    fn deconnecter(&self);
    /// Pause entre deux tentatives ; les faux transports la rendent instantanée.
    fn attendre(&self, duree: Duration) {
        std::thread::sleep(duree);
    }
}

// ---------------------------------------------------------------- AdbProcess

/// Transport réel : un sous-processus adb par commande, serveur adb dédié sur le port 5038.
pub struct AdbProcess {
    adb: PathBuf,
    serie: String,
    delai: Duration,
}

struct Brut {
    code: i32,
    stdout: String,
    stderr: String,
}

impl Brut {
    /// Texte d'erreur : stderr, à défaut la sortie standard (adb install/uninstall y écrit « Failure [...] »).
    fn texte_erreur(&self) -> String {
        if self.stderr.trim().is_empty() { self.stdout.clone() } else { format!("{}\n{}", self.stderr, self.stdout) }
    }
}

fn lire_borne(flux: Option<impl Read + Send + 'static>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut f) = flux {
            let _ = f.by_ref().take(SORTIE_MAX as u64).read_to_end(&mut buf);
            // Le reste est lu puis jeté : adb ne doit pas recevoir de SIGPIPE ni rester bloqué sur un tube plein.
            let _ = std::io::copy(&mut f, &mut std::io::sink());
        }
        buf
    })
}

fn texte(octets: &[u8]) -> String {
    String::from_utf8_lossy(octets).replace('\r', "")
}

impl AdbProcess {
    pub fn nouveau(adb: impl Into<PathBuf>, ip: Ipv4Addr) -> Self {
        Self { adb: adb.into(), serie: format!("{ip}:{PORT_BOX}"), delai: DELAI_COMMANDE }
    }

    pub fn avec_delai(mut self, delai: Duration) -> Self {
        self.delai = delai;
        self
    }

    pub fn serie(&self) -> &str {
        &self.serie
    }

    fn lancer(&self, args: &[&str], entree: Option<&str>, delai: Duration) -> Result<Brut, ErreurTransport> {
        let mut child = Command::new(&self.adb)
            .args(args)
            .env("ANDROID_ADB_SERVER_PORT", PORT_SERVEUR_ADB)
            .stdin(if entree.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ErreurTransport::Lancement(e.to_string()))?;
        if let (Some(mut stdin), Some(e)) = (child.stdin.take(), entree) {
            // Écriture dans un fil à part : un adb qui ne lit pas ne doit pas bloquer l'appelant.
            let e = e.to_string();
            std::thread::spawn(move || {
                let _ = stdin.write_all(e.as_bytes());
            });
        }
        let out = lire_borne(child.stdout.take());
        let err = lire_borne(child.stderr.take());
        let fin = Instant::now() + delai;
        let code = loop {
            match child.try_wait() {
                Ok(Some(st)) => break st.code().unwrap_or(-1),
                Ok(None) if Instant::now() >= fin => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ErreurTransport::Coupure("délai dépassé".into()));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(e) => return Err(ErreurTransport::Lancement(e.to_string())),
            }
        };
        Ok(Brut { code, stdout: texte(&out.join().unwrap_or_default()), stderr: texte(&err.join().unwrap_or_default()) })
    }

    fn avec_serie(&self, reste: &[&str], entree: Option<&str>, delai: Duration) -> Result<Brut, ErreurTransport> {
        let mut args = vec!["-s", self.serie.as_str()];
        args.extend_from_slice(reste);
        self.lancer(&args, entree, delai)
    }

    fn en_sortie(brut: Brut) -> Result<Sortie, ErreurTransport> {
        if brut.code == 0 {
            Ok(Sortie { stdout: brut.stdout, stderr: brut.stderr })
        } else {
            Err(classer_erreur(brut.code, &brut.texte_erreur()))
        }
    }

    /// install/uninstall : adb peut répondre « Failure [...] » avec le code 0 sur d'anciennes versions.
    fn en_succes(brut: Brut) -> Result<Sortie, ErreurTransport> {
        if brut.code == 0 && brut.stdout.contains("Success") {
            Ok(Sortie { stdout: brut.stdout, stderr: brut.stderr })
        } else if brut.code == 0 {
            Err(ErreurTransport::Commande { code: 0, stderr: assainir(&brut.stdout) })
        } else {
            Err(classer_erreur(brut.code, &brut.texte_erreur()))
        }
    }
}

impl Transport for AdbProcess {
    fn shell(&self, cmd: &Commande) -> Result<Sortie, ErreurTransport> {
        if cmd.sensible() {
            return refus("commande sensible : utiliser shell_stdin");
        }
        Self::en_sortie(self.avec_serie(&["shell", cmd.texte()], None, self.delai)?)
    }

    fn shell_stdin(&self, cmd: &Commande) -> Result<Sortie, ErreurTransport> {
        let entree = format!("{}\n", cmd.texte());
        Self::en_sortie(self.avec_serie(&["shell"], Some(&entree), self.delai)?)
    }

    fn install(&self, apk: &Path) -> Result<Sortie, ErreurTransport> {
        let chemin = apk.to_str().filter(|p| !p.is_empty() && !p.starts_with('-'));
        let Some(chemin) = chemin else { return refus("chemin d'APK invalide") };
        Self::en_succes(self.avec_serie(&["install", "-r", chemin], None, DELAI_INSTALLATION)?)
    }

    fn uninstall(&self, paquet: &str) -> Result<Sortie, ErreurTransport> {
        if !paquet_valide(paquet) {
            return refus(format!("paquet invalide : {paquet}"));
        }
        Self::en_succes(self.avec_serie(&["uninstall", paquet], None, self.delai)?)
    }

    fn etat(&self) -> Result<EtatLien, ErreurTransport> {
        let b = self.avec_serie(&["get-state"], None, self.delai)?;
        Ok(etat_depuis(b.code, &b.texte_erreur()))
    }

    fn connecter(&self) -> Result<(), ErreurTransport> {
        let b = self.lancer(&["connect", &self.serie], None, DELAI_CONNEXION)?;
        let t = b.texte_erreur();
        if t.to_lowercase().contains("connected to") {
            return Ok(());
        }
        match classer_erreur(b.code, &t) {
            // « failed to connect » ou sortie inattendue : le lien n'est pas établi, une nouvelle tentative est possible.
            ErreurTransport::Commande { stderr, .. } => Err(ErreurTransport::Coupure(stderr)),
            e => Err(e),
        }
    }

    fn deconnecter(&self) {
        let _ = self.lancer(&["disconnect", &self.serie], None, DELAI_CONNEXION);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paquet_regex_stricte() {
        for ok in ["tv.zenhome", "com.google.android.tvlauncher", "a.b", "tv.alphonso.alphonso_eula", "com.A_b.c9"] {
            assert!(paquet_valide(ok), "{ok}");
        }
        let long = format!("a.{}", "b".repeat(253));
        let trop_long = format!("a.{}", "b".repeat(254));
        assert!(paquet_valide(&long) && long.len() == 255);
        assert!(!paquet_valide(&trop_long));
        for ko in [
            "", "tv", "tv.", ".tv", "tv..zenhome", "1tv.zenhome", "tv.1zenhome", "_tv.zenhome", "tv.zen-home",
            "tv.zenhome\n", " tv.zenhome", "tv.zenhome;id", "tv.zen home", "$(id).x", "tv.zénhome", "a.b/c", "a.b'c",
        ] {
            assert!(!paquet_valide(ko), "{ko:?}");
        }
    }

    #[test]
    fn shq_cas_simples() {
        assert_eq!(shq("Paris"), "'Paris'");
        assert_eq!(shq(""), "''");
        assert_eq!(shq("L'Haÿ-les-Roses"), "'L'\\''Haÿ-les-Roses'");
        assert_eq!(shq("''"), "''\\'''\\'''");
        assert_eq!(shq("$(rm -rf /); `id` \"x\" \\ *"), "'$(rm -rf /); `id` \"x\" \\ *'");
    }

    #[test]
    fn shq_aller_retour_par_le_decoupeur_shell() {
        let cas = ["", "a", "a b", "L'Haÿ", "'", "'''", "\\", "\\'", "$(id)", "`id`", "a;b|c&d", "*?[x]", "\"", "é€😀", "a\tb"];
        for c in cas {
            let ligne = format!("echo {}", shq(c));
            assert_eq!(decouper_shell(&ligne), vec!["echo".to_string(), c.to_string()], "{c:?}");
        }
        // plusieurs valeurs quotées sur une même ligne restent des mots distincts
        assert_eq!(decouper_shell(&format!("x {} {}", shq("a b"), shq("c'd"))), ["x", "a b", "c'd"]);
    }

    #[test]
    fn decoupeur_comprend_les_mots_nus() {
        assert_eq!(decouper_shell("pm  list packages -d"), ["pm", "list", "packages", "-d"]);
        assert_eq!(decouper_shell("a\\ b"), ["a b"]);
    }

    #[test]
    fn constructeurs_refusent_les_valeurs_dangereuses() {
        assert!(Commande::pm_disable("tv.zenhome; reboot").is_err());
        assert!(Commande::pm_enable("$(id).x").is_err());
        assert!(Commande::dumpsys_paquet("x y.z").is_err());
        assert!(Commande::getprop("ro.x; id").is_err());
        assert!(Commande::settings_get(Espace::Global, "a b").is_err());
        assert!(Commande::settings_put(Espace::Global, "k", "ligne1\nligne2").is_err());
        assert!(Commande::settings_put(Espace::Global, "k;id", "v").is_err());
        assert!(Commande::set_home_activity("tv.zenhome/.Home$1").is_err());
        assert!(Commande::set_home_activity("tv.zenhome").is_err());
        assert!(Commande::appops_set("tv.zenhome", "get_usage_stats", "allow").is_err());
        assert!(Commande::appops_set("tv.zenhome", "GET_USAGE_STATS", "allow; id").is_err());
        assert!(Commande::broadcast_config(&[]).is_err());
        assert!(Commande::broadcast_config(&[ArgConfig::Lat("48.8; id".into())]).is_err());
        assert!(Commande::broadcast_config(&[ArgConfig::Lat("48.".into())]).is_err());
        assert!(Commande::broadcast_config(&[ArgConfig::Ville("a\nb".into())]).is_err());
        assert!(Commande::broadcast_config(&[ArgConfig::Ville("x".repeat(61))]).is_err());
        assert!(Commande::broadcast_config(&[ArgConfig::Jeton("a b".into())]).is_err());
    }

    #[test]
    fn constructeurs_produisent_les_commandes_du_kit() {
        assert_eq!(Commande::pm_list(&[OptionPm::Desactives]).texte(), "pm list packages -d");
        assert_eq!(Commande::pm_list(&[OptionPm::Chemins]).texte(), "pm list packages -f");
        assert_eq!(Commande::pm_disable("com.mitv.tvhome.atv").unwrap().texte(), "pm disable-user --user 0 com.mitv.tvhome.atv");
        assert_eq!(
            Commande::settings_put(Espace::Global, "zenhome_garder", "a.b,c.d").unwrap().texte(),
            "settings put global zenhome_garder 'a.b,c.d'"
        );
        assert_eq!(
            Commande::set_home_activity("tv.zenhome/.Home").unwrap().texte(),
            "cmd package set-home-activity tv.zenhome/.Home"
        );
        let b = Commande::broadcast_config(&[ArgConfig::Lat("48.86".into()), ArgConfig::Lon("-2.35".into()), ArgConfig::Ville("L'Aigle".into())])
            .unwrap();
        assert_eq!(
            b.texte(),
            "am broadcast -a tv.zenhome.CONFIG -n tv.zenhome/.ConfigReceiver --es lat 48.86 --es lon -2.35 --es city 'L'\\''Aigle'"
        );
        assert!(b.sensible(), "la position est masquée comme le jeton");
    }

    #[test]
    fn jeton_sensible_masque_partout() {
        let c = Commande::broadcast_config(&[ArgConfig::Jeton("abc.DEF-123".into())]).unwrap();
        assert!(c.sensible());
        assert!(c.texte().contains("abc.DEF-123"));
        for vue in [c.afficher().to_string(), format!("{c}"), format!("{c:?}")] {
            assert!(!vue.contains("abc.DEF"), "{vue}");
            assert!(vue.contains("***"));
        }
    }

    #[test]
    fn adb_process_refuse_une_commande_sensible_en_argument() {
        let t = AdbProcess::nouveau("/inexistant/adb", Ipv4Addr::new(192, 168, 1, 50));
        let c = Commande::broadcast_config(&[ArgConfig::Jeton("abc".into())]).unwrap();
        assert!(matches!(t.shell(&c), Err(ErreurTransport::Refusee(_))));
        assert!(matches!(t.uninstall("pas un paquet"), Err(ErreurTransport::Refusee(_))));
        assert!(matches!(t.install(Path::new("-s")), Err(ErreurTransport::Refusee(_))));
        // adb absent : erreur de lancement, jamais de panique
        assert!(matches!(t.shell(&Commande::getprop_tout()), Err(ErreurTransport::Lancement(_))));
        assert_eq!(t.serie(), "192.168.1.50:5555");
    }

    #[test]
    fn classement_des_erreurs_adb() {
        use ErreurTransport::*;
        for t in [
            "error: device offline",
            "error: device '192.168.1.50:5555' not found",
            "error: no devices/emulators found",
            "error: closed",
            "failed to connect to '192.168.1.50:5555': Connection refused",
            "failed to connect to 192.168.1.50:5555: No route to host",
            "cannot connect to 192.168.1.50:5555: Operation timed out",
        ] {
            assert!(matches!(classer_erreur(1, t), Coupure(_)), "{t}");
        }
        for t in ["error: device unauthorized.", "failed to authenticate to 192.168.1.50:5555", "error: device still authorizing"] {
            assert_eq!(classer_erreur(1, t), NonAutorise, "{t}");
        }
        // une erreur de la commande distante n'est pas une coupure, même si elle contient « not found »
        assert_eq!(
            classer_erreur(127, "/system/bin/sh: foo: not found"),
            Commande { code: 127, stderr: "/system/bin/sh: foo: not found".into() }
        );
        assert!(matches!(classer_erreur(1, "Failure [INSTALL_FAILED_UPDATE_INCOMPATIBLE]"), Commande { code: 1, .. }));
    }

    #[test]
    fn etats_du_lien() {
        assert_eq!(etat_depuis(0, "device\n"), EtatLien::Device);
        assert_eq!(etat_depuis(1, "error: device offline"), EtatLien::Offline);
        assert_eq!(etat_depuis(1, "error: device unauthorized."), EtatLien::NonAutorise);
        assert_eq!(etat_depuis(1, "error: device '1.2.3.4:5555' not found"), EtatLien::Absent);
        assert!(matches!(etat_depuis(0, "bootloader"), EtatLien::Autre(_)));
    }

    #[test]
    fn reponse_vide_distincte() {
        assert_eq!(Sortie::default().non_vide(), Err(ErreurTransport::Vide));
        assert_eq!(Sortie { stdout: " \n".into(), stderr: String::new() }.non_vide(), Err(ErreurTransport::Vide));
        assert!(Sortie { stdout: "x".into(), stderr: String::new() }.non_vide().is_ok());
    }

    #[test]
    fn texte_de_la_box_nettoye_et_borne_dans_les_erreurs() {
        let sale = format!("Failure \u{1b}[31m\u{85}\u{202e}x\r\nligne2 {}", "a".repeat(1000));
        let ErreurTransport::Commande { stderr, .. } = classer_erreur(1, &sale) else { panic!("attendu : Commande") };
        assert!(stderr.chars().count() <= 300, "{}", stderr.len());
        assert!(stderr.starts_with("Failure [31mx"), "{stderr:?}");
        assert!(!stderr.chars().any(|c| c.is_control() || c == '\u{202e}'), "{stderr:?}");
        let ErreurTransport::Coupure(m) = classer_erreur(1, &format!("error: device offline\u{7}{}", "b".repeat(1000))) else {
            panic!("attendu : Coupure")
        };
        assert!(m.chars().count() <= 300 && !m.contains('\u{7}'));
        let brut = Brut { code: 0, stdout: format!("pas de succès \u{1b}[0m\u{202e}{}", "c".repeat(1000)), stderr: String::new() };
        let Err(ErreurTransport::Commande { stderr, .. }) = AdbProcess::en_succes(brut) else { panic!("attendu : Commande") };
        assert!(stderr.chars().count() <= 300 && !stderr.chars().any(|c| c.is_control() || c == '\u{202e}'), "{stderr:?}");
        // la classification se fait sur le texte complet : un marqueur loin dans le texte reste reconnu
        assert_eq!(classer_erreur(1, &format!("{} unauthorized", "x".repeat(1000))), ErreurTransport::NonAutorise);
    }

    #[test]
    fn lecture_bornee_continue_de_lire_et_jette() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Flot(usize, Arc<AtomicUsize>);
        impl Read for Flot {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let n = buf.len().min(self.0);
                buf[..n].fill(b'x');
                self.0 -= n;
                self.1.fetch_add(n, Ordering::SeqCst);
                Ok(n)
            }
        }
        let lus = Arc::new(AtomicUsize::new(0));
        let garde = lire_borne(Some(Flot(SORTIE_MAX + 5000, lus.clone()))).join().unwrap();
        assert_eq!(garde.len(), SORTIE_MAX);
        assert_eq!(lus.load(Ordering::SeqCst), SORTIE_MAX + 5000, "le reste est lu puis jeté : pas de SIGPIPE côté adb");
    }

    #[test]
    fn cle_sans_tiret_en_tete_et_position_masquee() {
        assert!(Commande::settings_get(Espace::Global, "-x").is_err());
        assert!(Commande::getprop("--help").is_err());
        assert!(Commande::settings_delete(Espace::Global, "-k").is_err());
        assert!(Commande::settings_put(Espace::Global, "a-b.c_d", "v").is_ok());
        for a in [ArgConfig::Jeton("SECRET".into()), ArgConfig::Lat("48.8566".into()), ArgConfig::Lon("2.3522".into()), ArgConfig::Ville("Paris".into())] {
            let d = format!("{a:?}");
            assert!(!d.contains("SECRET") && !d.contains("48.8566") && !d.contains("2.3522") && !d.contains("Paris"), "{d}");
        }
        let c = Commande::broadcast_config(&[ArgConfig::Lat("48.8566".into()), ArgConfig::Lon("2.3522".into()), ArgConfig::Ville("Paris".into())])
            .unwrap();
        assert!(c.sensible(), "la position ne passe que par shell_stdin");
        for vue in [c.afficher().to_string(), format!("{c}"), format!("{c:?}")] {
            assert!(!vue.contains("48.8566") && !vue.contains("2.3522") && !vue.contains("Paris"), "{vue}");
        }
        assert!(c.texte().contains("--es lat 48.8566 --es lon 2.3522 --es city 'Paris'"));
        assert!(!Commande::broadcast_config(&[ArgConfig::Libelles]).unwrap().sensible());
    }

    #[test]
    fn nombres_et_jetons() {
        for ok in ["48", "-2.35", "0.5"] {
            assert!(nombre_valide(ok), "{ok}");
        }
        for ko in ["", "-", ".5", "5.", "1e5", "1,5", "--1", "1.2.3"] {
            assert!(!nombre_valide(ko), "{ko}");
        }
        assert!(jeton_valide("eyJ.abc_def-1=="));
        assert!(!jeton_valide("") && !jeton_valide("a b") && !jeton_valide("a'b"));
    }
}
