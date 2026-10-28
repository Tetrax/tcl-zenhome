//! Mode démo : une Mi Box simulée (FakeBox) avec de petits délais pour que les animations se voient.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::moteur::fake_box::{FakeBox, PaquetFake};
use crate::moteur::transport::{Commande, ErreurTransport, EtatLien, Sortie, Transport};

pub const NOM_BOX: &str = "SALON";
pub const IP_BOX: &str = "192.168.50.14";
const SERIE: &str = "ZH-DEMO-0001";

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
    pub fn nouvelle(rythme: Duration) -> Arc<Self> {
        let box_ = FakeBox::mibox4();
        box_.modifier(|e| {
            e.serie = SERIE.into();
            for (p, mo, d, l) in [
                ("com.netflix.ninja", 98, "2026-10-26 20:41:07", "Netflix"),
                ("org.videolan.vlc", 42, "2026-09-02 18:20:00", "VLC"),
                ("com.disney.disneyplus", 160, "2026-10-01 21:10:45", "Disney+"),
            ] {
                e.paquets.insert(p.into(), tiers(p, mo, d, l));
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
