//! Tests de la couche commandes sans Tauri : `Coeur` piloté sur une FakeBox, événements enregistrés.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use super::coeur::{Config, Coeur, Fabrique, TransportPartage};
use super::erreur::ErreurCommande;
use super::evenements::{EvenementUi, Raison};
use super::persistance;
use super::types::{Cible, ChoixUi, InventaireUi};
use crate::moteur::connexion::Politique;
use crate::moteur::fake_box::FakeBox;
use crate::moteur::session::Reprise;

const SERIE: &str = "TESTSERIAL0001";
const IP: &str = "192.168.1.20";

fn dossier(nom: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let d = std::env::temp_dir().join(format!("zenhome-cmd-{nom}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn config(donnees: PathBuf, f: &Arc<FakeBox>) -> Config {
    let f = f.clone();
    let fabrique: Fabrique = Box::new(move |_| Ok(f.clone() as TransportPartage));
    Config {
        version: "0.0.0-test".into(),
        donnees,
        apk_ressource: PathBuf::new(),
        apk_sha256: String::new(),
        demo: false,
        fabrique,
        politique: Politique::default(),
        delai_action: Duration::from_secs(60),
        delai_decision: Duration::from_secs(5),
        rythme_demo: Duration::ZERO,
    }
}

/// APK de test : octets quelconques dont on connaît l'empreinte.
fn avec_apk(cfg: &mut Config, contenu: &[u8], empreinte: &str) {
    let chemin = cfg.donnees.join("ZenHome-test.apk");
    std::fs::create_dir_all(&cfg.donnees).unwrap();
    std::fs::write(&chemin, contenu).unwrap();
    cfg.apk_ressource = chemin;
    cfg.apk_sha256 = empreinte.into();
}

fn sha(contenu: &[u8]) -> String {
    Sha256::digest(contenu).iter().map(|b| format!("{b:02x}")).collect()
}

fn pret(nom: &str) -> (Coeur, Arc<FakeBox>, PathBuf) {
    let (f, d) = (Arc::new(FakeBox::mibox4()), dossier(nom));
    let mut cfg = config(d.clone(), &f);
    avec_apk(&mut cfg, b"apk de test", &sha(b"apk de test"));
    (Coeur::nouveau(cfg), f, d)
}

type Recu = Arc<Mutex<Vec<EvenementUi>>>;

fn enregistreur() -> (Recu, impl Fn(EvenementUi) + Send + Sync) {
    let recu: Recu = Arc::default();
    let r = recu.clone();
    (recu, move |e| r.lock().unwrap().push(e))
}

fn rien(_: EvenementUi) {}

fn code<T>(r: Result<T, ErreurCommande>) -> &'static str {
    r.err().expect("une erreur était attendue").code
}

fn connecter(c: &Coeur) {
    c.connecter(&Cible::Ip { ip: IP.into() }, &rien).unwrap();
}

fn inventaire(c: &Coeur) -> InventaireUi {
    c.inventaire(&rien).unwrap_or_else(|e| panic!("inventaire : {e:?}"))
}

/// Choix qui suit les recommandations de la liste.
fn choix_recommande(inv: &InventaireUi) -> ChoixUi {
    let par = |r: &str| inv.applis_systeme.iter().filter(|a| a.recommande == r).map(|a| a.paquet.clone()).collect();
    ChoixUi { garder: par("garder"), desactiver: par("desactiver"), desinstaller: vec![] }
}

#[test]
fn une_seule_action_a_la_fois() {
    let (c, _f, d) = pret("occupe");
    let garde = c.prendre().unwrap();
    assert_eq!(code(c.chercher_villes("Paris")), "occupe");
    assert_eq!(code(c.chercher_box()), "occupe");
    assert_eq!(code(c.connecter(&Cible::Ip { ip: IP.into() }, &rien)), "occupe");
    assert_eq!(code(c.inventaire(&rien)), "occupe");
    assert_eq!(code(c.installer(None, None, &rien)), "occupe");
    assert_eq!(code(c.verifier(false, &rien)), "occupe");
    assert_eq!(code(c.restaurer(&rien)), "occupe");
    assert_eq!(code(c.installer_adb(true, &rien)), "occupe");
    assert_eq!(code(c.desinstaller_mac(true, &d, &d, None)), "occupe");
    drop(garde);
    assert_eq!(code(c.inventaire(&rien)), "injoignable", "libre de nouveau : l'erreur est celle de l'absence de box");
}

#[test]
fn annuler_arrete_l_action_entre_deux_operations() {
    let (c, f, _d) = pret("annuler");
    connecter(&c);
    c.annuler(); // rien en cours : sans effet
    inventaire(&c);

    // lecture hors Session (inventaire) : coupée par le transport surveillé
    let avant = f.journal().len();
    let r = c.inventaire(&|e| {
        if matches!(e, EvenementUi::Etape { .. }) {
            c.annuler();
        }
    });
    assert_eq!(code(r), "annule");
    assert_eq!(f.journal().len(), avant, "aucune commande après l'annulation");

    // action du moteur (Session) : mise en pause propre avant l'opération suivante
    let avant = f.journal().len();
    let r = c.verifier(false, &|e| {
        if matches!(e, EvenementUi::Etape { .. }) {
            c.annuler();
        }
    });
    assert_eq!(code(r), "annule");
    assert_eq!(f.journal().len(), avant);

    // l'annulation ne reste pas armée pour l'action suivante
    assert!(c.verifier(false, &rien).is_ok());
}

#[test]
fn delai_global_depasse() {
    let (f, d) = (Arc::new(FakeBox::mibox4()), dossier("delai"));
    let mut cfg = config(d, &f);
    cfg.delai_action = Duration::ZERO;
    let c = Coeur::nouveau(cfg);
    let r = c.connecter(&Cible::Ip { ip: IP.into() }, &rien);
    assert_eq!(code(r), "interne");
}

#[test]
fn attente_utilisateur_puis_reessayer() {
    let (c, f, _d) = pret("reessayer");
    f.autorisation_en_attente();
    let (recu, enregistre) = enregistreur();
    let r = c.connecter(&Cible::Ip { ip: IP.into() }, &|e| {
        if let EvenementUi::AttenteUtilisateur { demande, raison } = &e {
            assert_eq!(*raison, Raison::Autorisation);
            f.accepter_autorisation();
            c.decider(*demande, "reessayer").unwrap();
            // usage unique : la même demande ne se rejoue pas
            assert_eq!(code(c.decider(*demande, "reessayer")), "choix_invalide");
        }
        enregistre(e);
    });
    assert_eq!(r.unwrap().modele, "Xiaomi Mi Box 4");
    assert!(recu.lock().unwrap().iter().any(|e| matches!(e, EvenementUi::AttenteUtilisateur { .. })));
}

#[test]
fn attente_utilisateur_puis_arreter() {
    let (c, f, _d) = pret("arreter");
    f.autorisation_en_attente();
    let r = c.connecter(&Cible::Ip { ip: IP.into() }, &|e| {
        if let EvenementUi::AttenteUtilisateur { demande, .. } = e {
            c.decider(demande, "arreter").unwrap();
        }
    });
    assert_eq!(code(r), "non_autorise");
    assert_eq!(code(c.decider(999, "arreter")), "choix_invalide", "demande inconnue");
    assert_eq!(code(c.decider(1, "peut-etre")), "choix_invalide");
    assert_eq!(code(c.inventaire(&rien)), "injoignable", "pas de session après un échec de connexion");
}

#[test]
fn sans_reponse_dans_le_delai_l_action_s_arrete() {
    let (f, d) = (Arc::new(FakeBox::mibox4()), dossier("silence"));
    f.injoignable(true);
    let mut cfg = config(d, &f);
    cfg.delai_decision = Duration::from_millis(150);
    let c = Coeur::nouveau(cfg);
    let (recu, enregistre) = enregistreur();
    let debut = Instant::now();
    let r = c.connecter(&Cible::Ip { ip: IP.into() }, &enregistre);
    assert_eq!(code(r), "injoignable");
    assert!(debut.elapsed() >= Duration::from_millis(150));
    assert!(recu.lock().unwrap().iter().any(|e| matches!(e, EvenementUi::AttenteUtilisateur { raison: Raison::Injoignable, .. })));
    // la demande expirée n'est plus valide
    assert_eq!(code(c.decider(1, "reessayer")), "choix_invalide");
}

#[test]
fn ip_publique_ou_inconnue_refusee_sans_toucher_au_reseau() {
    let (c, f, _d) = pret("ip");
    for ip in ["8.8.8.8", "127.0.0.1", "0.0.0.0", "192.168.1.20; id", "box.local", "::1", ""] {
        assert_eq!(code(c.connecter(&Cible::Ip { ip: ip.into() }, &rien)), "injoignable", "{ip}");
    }
    assert_eq!(code(c.connecter(&Cible::Id { id: 3 }, &rien)), "choix_invalide", "id absent de la dernière recherche");
    assert!(f.journal().is_empty(), "{:?}", f.journal());
    assert!(c.connecter(&Cible::Ip { ip: "169.254.10.2".into() }, &rien).is_ok(), "link-local accepté");
}

#[test]
fn appliquer_refuse_les_paquets_non_autorises() {
    let (c, f, _d) = pret("choix");
    assert_eq!(code(c.appliquer(&ChoixUi { garder: vec![], desactiver: vec![], desinstaller: vec![] }, &rien)), "injoignable");
    connecter(&c);
    let vide = ChoixUi { garder: vec![], desactiver: vec![], desinstaller: vec![] };
    assert_eq!(code(c.appliquer(&vide, &rien)), "lecture", "pas d'inventaire lu : rien n'est appliqué");
    let inv = inventaire(&c);
    let avant = f.journal().len();
    let un = |p: &str| vec![p.to_string()];
    for choix in [
        ChoixUi { desinstaller: un("com.google.android.tvlauncher"), ..ChoixUi { garder: vec![], desactiver: vec![], desinstaller: vec![] } },
        ChoixUi { desinstaller: un("tv.zenhome"), ..ChoixUi { garder: vec![], desactiver: vec![], desinstaller: vec![] } },
        ChoixUi { desactiver: un("com.android.vending"), ..ChoixUi { garder: vec![], desactiver: vec![], desinstaller: vec![] } },
        ChoixUi { garder: un("com.exemple.inconnu"), ..ChoixUi { garder: vec![], desactiver: vec![], desinstaller: vec![] } },
        ChoixUi { desinstaller: un("fr.tf1.mytf1; reboot"), ..ChoixUi { garder: vec![], desactiver: vec![], desinstaller: vec![] } },
        ChoixUi { desinstaller: un("com.android.tv.settings"), ..ChoixUi { garder: vec![], desactiver: vec![], desinstaller: vec![] } },
    ] {
        assert_eq!(code(c.appliquer(&choix, &rien)), "choix_invalide", "{choix:?}");
    }
    assert_eq!(f.journal().len(), avant, "aucune commande envoyée pour un choix refusé");
    // un choix en règle passe
    let ok = ChoixUi { desinstaller: un("fr.tf1.mytf1"), ..choix_recommande(&inv) };
    let rapport = c.appliquer(&ok, &rien).unwrap();
    assert_eq!(rapport.desinstallees, ["fr.tf1.mytf1"]);
    // l'inventaire de la session a suivi : la même désinstallation est maintenant refusée
    assert_eq!(code(c.appliquer(&ok, &rien)), "choix_invalide");
}

#[test]
fn le_jeton_n_apparait_ni_dans_les_evenements_ni_dans_les_erreurs_ni_sur_la_box() {
    const JETON: &str = "SECRET.jeton-ABC123";
    let (c, f, _d) = pret("jeton");
    connecter(&c);
    let (recu, enregistre) = enregistreur();
    c.installer(None, Some(JETON.into()), &enregistre).unwrap();
    // Zen Home est installée : une signature incompatible fait échouer la seconde installation
    f.signature_incompatible(true);
    let err = c.installer(None, Some(JETON.into()), &enregistre).err().unwrap();
    assert_eq!(err.code, "interne");
    let tout = format!("{:?}{:?}{}{}", recu.lock().unwrap(), err, serde_json::to_string(&err).unwrap(), f.journal().join("\n"));
    assert!(!tout.contains("SECRET"), "{tout}");
    // jeton au format refusé : l'erreur ne le répète pas
    let err = c.installer(None, Some("mauvais jeton\n; id".into()), &enregistre).err().unwrap();
    assert_eq!(err.code, "choix_invalide");
    assert!(!err.message.contains("mauvais jeton"));
    // jeton vide : traité comme absent
    f.signature_incompatible(false);
    assert!(c.installer(None, Some("   ".into()), &enregistre).is_ok());
}

#[test]
fn apk_a_l_empreinte_fausse_refuse_avant_toute_installation() {
    let (f, d) = (Arc::new(FakeBox::mibox4()), dossier("apk"));
    let mut cfg = config(d.clone(), &f);
    avec_apk(&mut cfg, b"apk modifie", &sha(b"apk d'origine"));
    let c = Coeur::nouveau(cfg);
    connecter(&c);
    let avant = f.journal().len();
    let err = c.installer(None, None, &rien).err().unwrap();
    assert_eq!(err.code, "interne");
    assert!(err.message.contains("empreinte"), "{}", err.message);
    assert_eq!(f.journal().len(), avant, "rien n'a été envoyé à la box");
    assert!(!d.join("apk-verifie").exists() || std::fs::read_dir(d.join("apk-verifie")).unwrap().next().is_none());
    // la vérification ne bloque pas le contrôle : l'APK refusé devient un avertissement
    let (recu, enregistre) = enregistreur();
    c.verifier(true, &enregistre).unwrap();
    assert!(recu.lock().unwrap().iter().any(|e| matches!(e, EvenementUi::Avertissement { texte } if texte.contains("empreinte"))));
    assert!(f.journal().iter().all(|l| !l.starts_with("install:")), "{:?}", f.journal());
}

#[test]
fn apk_absent_au_build_ou_correct() {
    let (f, d) = (Arc::new(FakeBox::mibox4()), dossier("apk-vide"));
    let mut cfg = config(d, &f);
    avec_apk(&mut cfg, b"", "");
    let c = Coeur::nouveau(cfg);
    connecter(&c);
    let err = c.installer(None, None, &rien).err().unwrap();
    assert!(err.message.contains("ne contient pas Zen Home"), "{}", err.message);

    let (c, f, d) = pret("apk-ok");
    connecter(&c);
    c.installer(None, None, &rien).unwrap();
    let installs: Vec<_> = f.journal().into_iter().filter(|l| l.starts_with("install:")).collect();
    assert_eq!(installs.len(), 1);
    assert!(installs[0].contains("apk-verifie"), "installé depuis la copie vérifiée : {installs:?}");
    assert!(std::fs::read_dir(d.join("apk-verifie")).unwrap().next().is_none(), "copie supprimée à la fin");
}

#[test]
fn reprise_persistee_pendant_la_verification_puis_effacee() {
    let (c, _f, d) = pret("reprise");
    connecter(&c);
    let inv = inventaire(&c);
    c.installer(None, None, &rien).unwrap();
    c.appliquer(&choix_recommande(&inv), &rien).unwrap();
    assert!(!c.etat_initial().reprise_en_attente);

    // au cours de la réparation, les paquets désactivés sont déjà sur disque
    let vue = Arc::new(Mutex::new(None));
    let v = vue.clone();
    let dd = d.clone();
    let rapport = c
        .verifier(true, &move |e| {
            if matches!(&e, EvenementUi::Etape { nom } if nom == "Nouveau contrôle") {
                *v.lock().unwrap() = persistance::lire(&dd);
            }
        })
        .unwrap();
    assert_eq!(rapport.ko, 0);
    let en_cours = vue.lock().unwrap().clone().expect("reprise enregistrée pendant la réparation");
    assert_eq!(en_cours.serie.as_deref(), Some(SERIE));
    assert!(!en_cours.desactives.is_empty());
    // vérification réussie : plus de retour arrière à garder
    assert_eq!(persistance::lire(&d), None);
    assert!(c.reprise_session().unwrap().desactives.is_empty());
}

#[test]
fn reprise_relue_au_demarrage_et_reprise_a_la_connexion() {
    let (f, d) = (Arc::new(FakeBox::mibox4()), dossier("relue"));
    let reprise = Reprise { etape: 0, desactives: vec!["com.mitv.tvhome.atv".into()], serie: Some(SERIE.into()) };
    persistance::ecrire(&d, &reprise).unwrap();
    let c = Coeur::nouveau(config(d.clone(), &f));
    assert!(c.etat_initial().reprise_en_attente);
    connecter(&c);
    assert_eq!(c.reprise_session().unwrap().desactives, ["com.mitv.tvhome.atv"]);

    // une autre box ne reprend pas les paquets de celle-ci
    let (autre, d2) = (Arc::new(FakeBox::mibox4()), dossier("relue-autre"));
    autre.modifier(|e| e.serie = "AUTRESERIE".into());
    persistance::ecrire(&d2, &reprise).unwrap();
    let c2 = Coeur::nouveau(config(d2, &autre));
    connecter(&c2);
    assert!(c2.reprise_session().unwrap().desactives.is_empty());
}

#[test]
fn textes_de_la_box_assainis() {
    let (c, f, _d) = pret("assainir");
    f.modifier(|e| e.modele = "MIBOX\u{202e}\u{7}\nfaux".into());
    connecter(&c);
    let inv = inventaire(&c);
    assert!(inv.resume.modele.chars().all(|ch| !ch.is_control() && ch != '\u{202e}'), "{:?}", inv.resume.modele);
}

#[test]
fn parcours_demo_complet() {
    let d = dossier("demo");
    let c = Coeur::nouveau(Config::demo("0.0.0-test".into(), d.clone(), Duration::ZERO));
    let etat = c.etat_initial();
    assert!(etat.demo && !etat.adb_installe && !etat.reprise_en_attente);
    assert_eq!(etat.origine.map(|o| o.date).as_deref(), Some("15/09/2026 21:00"));

    assert_eq!(code(c.installer_adb(false, &rien)), "refus_licence");
    let (recu, enregistre) = enregistreur();
    c.installer_adb(true, &enregistre).unwrap();
    assert!(matches!(recu.lock().unwrap().last(), Some(EvenementUi::Progression { fraction }) if *fraction == 1.0));
    assert!(c.etat_initial().adb_installe);

    let boxes = c.chercher_box().unwrap();
    assert_eq!((boxes[0].nom.as_str(), boxes[0].ip.as_str()), ("SALON", "192.168.50.14"));
    let connexion = c.connecter(&Cible::Id { id: boxes[0].id }, &rien).unwrap();
    assert_eq!((connexion.nom.as_str(), connexion.ip.as_str()), ("SALON", "192.168.50.14"));

    let villes = c.chercher_villes("Montpel").unwrap();
    assert_eq!(villes[0].nom, "Montpellier");
    assert_eq!(code(c.chercher_villes("  ")), "choix_invalide");

    let inv = inventaire(&c);
    assert!(!inv.applis_systeme.is_empty());
    assert!(inv.applis_utilisateur.iter().any(|a| a.paquet == "com.netflix.ninja"));
    assert!(inv.applis_systeme.iter().all(|a| a.etat == "actif"));
    assert!(d.join("demo/sauvegardes").is_dir());

    c.installer(Some(villes[0].id), Some("jeton-demo.ABC".into()), &rien).unwrap();
    let inv = inventaire(&c);
    assert!(inv.applis_utilisateur.iter().any(|a| a.libelle == "Netflix"), "libellés fournis par Zen Home une fois installée");

    let choix = ChoixUi { desinstaller: vec!["org.videolan.vlc".into()], ..choix_recommande(&inv) };
    let rapport = c.appliquer(&choix, &rien).unwrap();
    assert_eq!(rapport.desinstallees, ["org.videolan.vlc"]);
    assert!(rapport.echecs.is_empty());

    let verif = c.verifier(true, &rien).unwrap();
    assert_eq!(verif.ko, 0, "{:?}", verif.lignes.iter().map(|l| (&l.controle, &l.detail)).collect::<Vec<_>>());
    assert!(verif.lignes.iter().all(|l| ["ok", "ko", "avert"].contains(&l.etat)));

    let inv = inventaire(&c);
    assert!(inv.resume.nb_desactivees > 0);
    assert!(c.ouvrir_sauvegardes().is_ok());
    assert!(c.etat_initial().sauvegardes_dir_existe);

    let restauration = c.restaurer(&rien).unwrap();
    assert!(!restauration.incomplete, "{:?}", restauration.avertissements);
    assert_eq!(restauration.depuis_sauvegarde.as_deref(), Some("15/09/2026 21:00"), "la démo propose une origine");
    c.couper_debogage().unwrap();
    assert_eq!(code(c.inventaire(&rien)), "injoignable", "session fermée");
}

/// Sauvegarde « d'avant Zen Home » de la box de test, à la date donnée.
fn sauvegarde_origine(donnees: &std::path::Path, date_serie: &str, desactives: &str) {
    let d = donnees.join("sauvegardes").join(date_serie);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("paquets-tous.txt"), "package:/system/app/A.apk=com.google.android.tvlauncher\n").unwrap();
    std::fs::write(d.join("paquets-desactives.txt"), desactives).unwrap();
}

#[test]
fn origine_annoncee_au_demarrage_et_utilisee_par_la_restauration() {
    let (c, f, d) = pret("origine");
    assert!(c.etat_initial().origine.is_none());
    sauvegarde_origine(&d, "20261001-120000-AUTRESERIE", "");
    assert_eq!(c.etat_initial().origine.map(|o| o.date).as_deref(), Some("01/10/2026 12:00"), "toutes séries confondues");

    connecter(&c);
    assert!(c.etat_initial().origine.is_none(), "avec une session : origine de CETTE box seulement");
    // Zen Home a désactivé deux applis ; l'une l'était déjà avant lui
    f.modifier(|e| {
        for p in ["com.mitv.tvhome.atv", "com.xm.webcontent"] {
            e.paquets.get_mut(p).unwrap().desactive = true;
        }
    });
    let sans = c.restaurer(&rien).unwrap();
    assert_eq!(sans.depuis_sauvegarde, None, "l'origine d'une autre box ne sert pas");
    assert!(f.instantane().paquets.values().all(|p| !p.desactive));

    f.modifier(|e| e.paquets.get_mut("com.xm.webcontent").unwrap().desactive = true);
    sauvegarde_origine(&d, &format!("20261002-130000-{SERIE}"), "package:com.xm.webcontent\n");
    sauvegarde_origine(&d, &format!("20261003-130000-{SERIE}"), "");
    let avec = c.restaurer(&rien).unwrap();
    assert_eq!(avec.depuis_sauvegarde.as_deref(), Some("02/10/2026 13:00"), "la plus ancienne de la série");
    assert!(f.instantane().paquets["com.xm.webcontent"].desactive, "désactivée avant Zen Home : elle le reste");
    assert_eq!(c.etat_initial().origine.map(|o| o.date).as_deref(), Some("02/10/2026 13:00"), "même origine que celle de la restauration");
}

fn lieux(nom: &str) -> (PathBuf, PathBuf, PathBuf) {
    let parent = dossier(nom);
    let documents = parent.join("Documents");
    std::fs::create_dir_all(&documents).unwrap();
    (parent.join("fr.tetrax.zenhome"), documents, parent)
}

#[test]
fn desinstaller_mac_copie_puis_supprime_les_donnees_et_ferme_la_session() {
    let (donnees, documents, parent) = lieux("desinstaller");
    let f = Arc::new(FakeBox::mibox4());
    let c = Coeur::nouveau(config(donnees.clone(), &f));
    connecter(&c);
    inventaire(&c);
    let nb = std::fs::read_dir(donnees.join("sauvegardes")).unwrap().count();
    assert_eq!(nb, 1);

    c.desinstaller_mac(true, &documents, &parent, None).unwrap();
    assert!(!donnees.exists());
    let copies: Vec<_> = std::fs::read_dir(&documents).unwrap().flatten().collect();
    assert_eq!(copies.len(), 1);
    assert!(copies[0].file_name().to_string_lossy().starts_with("Zen Home – sauvegardes "));
    assert_eq!(std::fs::read_dir(copies[0].path()).unwrap().count(), 1, "la sauvegarde est copiée");
    assert_eq!(code(c.inventaire(&rien)), "injoignable", "session fermée");
    assert!(!f.journal().is_empty());
}

#[test]
fn desinstaller_mac_sans_garder_ne_copie_rien() {
    let (donnees, documents, parent) = lieux("desinstaller-sans");
    let f = Arc::new(FakeBox::mibox4());
    sauvegarde_origine(&donnees, "20261001-120000-X", "");
    let c = Coeur::nouveau(config(donnees.clone(), &f));
    c.desinstaller_mac(false, &documents, &parent, None).unwrap();
    assert!(!donnees.exists() && std::fs::read_dir(&documents).unwrap().next().is_none());
}

#[test]
fn desinstaller_mac_copie_impossible_ne_supprime_rien() {
    let (donnees, documents, parent) = lieux("desinstaller-echec");
    let f = Arc::new(FakeBox::mibox4());
    sauvegarde_origine(&donnees, "20261001-120000-X", "");
    let c = Coeur::nouveau(config(donnees.clone(), &f));
    let err = c.desinstaller_mac(true, &documents.join("absent"), &parent, None).err().unwrap();
    assert_eq!(err.code, "interne");
    assert!(donnees.join("sauvegardes").is_dir(), "rien n'est supprimé si la copie demandée échoue");
    // Corbeille impossible (bundle inexistant) : les données restent intactes
    let (donnees2, documents2, parent2) = lieux("desinstaller-corbeille");
    sauvegarde_origine(&donnees2, "20261001-120000-X", "");
    let c3 = Coeur::nouveau(config(donnees2.clone(), &f));
    let err = c3.desinstaller_mac(false, &documents2, &parent2, Some(&parent2.join("Absente.app"))).err().unwrap();
    assert_eq!(err.code, "interne");
    assert!(err.message.contains("Glissez"), "{}", err.message);
    assert!(donnees2.join("sauvegardes").is_dir(), "données intactes");
    // dossier de données qui n'est pas celui de l'appli : refusé, sans rien supprimer
    let c2 = Coeur::nouveau(config(parent.join("ailleurs"), &f));
    std::fs::create_dir_all(parent.join("ailleurs")).unwrap();
    assert_eq!(code(c2.desinstaller_mac(false, &documents, &parent, None)), "interne");
    assert!(parent.join("ailleurs").is_dir());
}

#[test]
fn desinstaller_mac_en_demo_ne_supprime_rien() {
    let (donnees, documents, parent) = lieux("desinstaller-demo");
    // en démo les données sont dans le sous-dossier `demo` de celles de l'appli
    let c = Coeur::nouveau(Config::demo("0.0.0-test".into(), donnees.clone(), Duration::ZERO));
    assert!(c.en_demo());
    c.desinstaller_mac(true, &documents, &parent, Some(&parent)).unwrap();
    assert!(donnees.join("demo/sauvegardes").is_dir());
    assert!(std::fs::read_dir(&documents).unwrap().next().is_none(), "aucune copie en démo");
    assert!(parent.is_dir());
}
