//! Mode démo : une Mi Box simulée (FakeBox) avec de petits délais pour que les animations se voient.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::moteur::fake_box::{FakeBox, PaquetFake};
use crate::moteur::transport::{Commande, ErreurTransport, EtatLien, Sortie, Transport};

/// Les deux Mi Box simulées : nom, IP et numéro de série (SALON est la première, celle qui a une sauvegarde d'origine).
pub const BOXES: [(&str, &str, &str); 2] = [("SALON", "192.168.50.14", "ZH-DEMO-0001"), ("CHAMBRE", "192.168.50.15", "ZH-DEMO-0002")];
const SERIE: &str = BOXES[0].2;

/// Délai artificiel par commande (le moteur lui-même n'attend jamais avec une FakeBox).
pub const RYTHME: Duration = Duration::from_millis(70);

/// Vrai en build debug ou si `ZENHOME_DEMO=1` ; `ZENHOME_DEMO=0` force le mode réel même en debug.
pub fn demo_demandee() -> bool {
    match std::env::var("ZENHOME_DEMO").as_deref() {
        Ok("1") => true,
        Ok("0") => false,
        _ => cfg!(debug_assertions),
    }
}

/// Sauvegarde d'origine factice (une lecture « d'avant Zen Home » de la box de démo), pour que le parcours démo montre
/// « Revenir à avant Zen Home ». Rien n'est écrit si elle existe déjà.
pub fn semer_origine(donnees: &Path) {
    let dossier = donnees.join("sauvegardes").join(format!("20260915-210000-{SERIE}"));
    if dossier.exists() {
        return;
    }
    let _ = std::fs::create_dir_all(&dossier).and_then(|()| {
        [
            ("paquets-tous.txt", "package:/system/priv-app/TvLauncher/TvLauncher.apk=com.google.android.tvlauncher\n"),
            ("paquets-desactives.txt", "\n"),
            ("accueil.txt", "priority=2 preferredOrder=0\ncom.google.android.tvlauncher/.MainActivity\n"),
            ("reglages-secure.txt", "screensaver_components=com.google.android.backdrop/.Backdrop\n"),
            ("reglages-global.txt", "window_animation_scale=1.0\ntransition_animation_scale=1.0\n"),
        ]
        .iter()
        .try_for_each(|(f, c)| std::fs::write(dossier.join(f), c))
    });
}

pub struct DemoBox {
    box_: FakeBox,
    rythme: Duration,
}

fn tiers(p: &str, mo: u64, derniere: &str, libelle: &str) -> PaquetFake {
    PaquetFake {
        systeme: false,
        desactive: false,
        chemin: format!("/data/app/{p}-1/base.apk"),
        octets: (mo * 1_048_576 / 2, mo * 1_048_576 / 4, mo * 1_048_576 / 4),
        derniere_utilisation: Some(derniere.to_string()),
        libelle: Some(libelle.to_string()),
    }
}

impl DemoBox {
    pub fn nouvelle(rythme: Duration, serie: &str) -> Arc<Self> {
        let box_ = FakeBox::mibox4();
        box_.modifier(|e| {
            e.serie = serie.into();
            // Netflix est commune aux deux box ; les autres applis sont propres à chacune.
            let propres: &[(&str, u64, &str, &str)] = if serie == SERIE {
                &[("org.videolan.vlc", 42, "2026-09-02 18:20:00", "VLC"), ("com.disney.disneyplus", 160, "2026-10-01 21:10:45", "Disney+")]
            } else {
                &[("org.xbmc.kodi", 120, "2026-08-14 22:05:30", "Kodi"), ("com.amazon.amazonvideo.livingroom", 75, "2026-10-20 19:30:12", "Prime Video")]
            };
            for (p, mo, d, l) in [("com.netflix.ninja", 98, "2026-10-26 20:41:07", "Netflix")].iter().chain(propres) {
                e.paquets.insert((*p).into(), tiers(p, *mo, d, l));
            }
        });
        Arc::new(Self { box_, rythme })
    }

    fn pause(&self, facteur: u32) {
        std::thread::sleep(self.rythme * facteur);
    }
}

impl Transport for DemoBox {
    fn shell(&self, cmd: &Commande) -> Result<Sortie, ErreurTransport> {
        self.pause(1);
        self.box_.shell(cmd)
    }
    fn shell_stdin(&self, cmd: &Commande) -> Result<Sortie, ErreurTransport> {
        self.pause(1);
        self.box_.shell_stdin(cmd)
    }
    fn install(&self, apk: &Path) -> Result<Sortie, ErreurTransport> {
        self.pause(15);
        self.box_.install(apk)
    }
    fn uninstall(&self, paquet: &str) -> Result<Sortie, ErreurTransport> {
        self.pause(6);
        self.box_.uninstall(paquet)
    }
    fn etat(&self) -> Result<EtatLien, ErreurTransport> {
        self.box_.etat()
    }
    fn connecter(&self) -> Result<(), ErreurTransport> {
        self.pause(8);
        self.box_.connecter()
    }
    fn deconnecter(&self) {
        self.box_.deconnecter();
    }
    fn attendre(&self, _duree: Duration) {}
}
