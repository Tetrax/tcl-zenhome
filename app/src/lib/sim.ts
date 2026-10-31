// Simulation du cœur Rust pour un navigateur (hors Tauri) : mêmes réponses, mêmes événements, avec des délais.
// Réglages par l'URL :
//   ?aucune          aucune box trouvée (saisie manuelle de l'adresse)
//   ?plusieurs       (par défaut il y a déjà deux box : SALON 192.168.50.14 et CHAMBRE 192.168.50.15)
//   ?coupure         coupure simulée : reconnexion pendant l'application, puis box injoignable pendant la vérification
//   ?autorisation    la connexion attend l'autorisation sur la TV (Réessayer)
//   ?refus           la première connexion échoue (non_autorise)
//   ?refus2          la première connexion à la 2e box (CHAMBRE) seulement échoue (non_autorise)
//   ?autremodele     la connexion échoue (autre_modele)
//   ?sansadb         adb n'est pas installé (écran d'installation d'adb)
//   ?reprise         une vérification précédente a été interrompue (reprise_en_attente)
//   ?sansorigine     aucune sauvegarde d'origine (restauration générique ; par défaut seule SALON en a une)
//   ?sansmemoire     aucune box mémorisée (par défaut SALON l'est : elle s'affiche dès la recherche)
//   ?rapide          délais divisés par 5 (tests)
import type {
  AppliSysteme, AppliUtilisateur, BoxConnectee, BoxTrouvee, Choix, Cible, Decision, ErreurZen,
  Inventaire, LigneVerif, RapportApplication, RapportRestauration, SurEvenement, Verification, Ville,
} from "./types";
import type { Api } from "./api";

const q = new URLSearchParams(location.search);
const FACTEUR = q.has("rapide") ? 0.2 : 1;
// Même date que la sauvegarde d'origine semée par le mode démo du cœur.
const DATE_ORIGINE = "15/09/2026 21:00";
const erreur = (code: ErreurZen["code"], message: string): ErreurZen => ({ code, message });

let enCours = false;
let annule = false;
let coupureFaite = false;
let coupureVerifFaite = false;
let refusFait = false;
let refus2Fait = false;
let demandeSuiv = 1;
const attentes = new Map<number, (d: Decision) => void>();

/** Pause qui réagit à « Annuler ». */
async function pause(ms: number): Promise<void> {
  const fin = performance.now() + ms * FACTEUR;
  while (performance.now() < fin) {
    if (annule) throw erreur("annule", "Opération annulée. Ce qui a déjà été fait est conservé.");
    await new Promise((r) => setTimeout(r, 40));
  }
  if (annule) throw erreur("annule", "Opération annulée. Ce qui a déjà été fait est conservé.");
}

/** Une action à la fois : sinon `occupe`. Remet le drapeau d'annulation à zéro. */
async function exclusive<T>(f: () => Promise<T>): Promise<T> {
  if (enCours) throw erreur("occupe", "Une autre opération est déjà en cours. Patientez un instant.");
  enCours = true; annule = false;
  try { return await f(); } finally { enCours = false; annule = false; for (const [, r] of attentes) r("arreter"); attentes.clear(); }
}

async function progresser(sur: SurEvenement, etape: string, ms: number, de = 0, a = 1): Promise<void> {
  sur({ type: "etape", nom: etape });
  const pas = 8;
  for (let i = 1; i <= pas; i++) { await pause(ms / pas); sur({ type: "progression", fraction: de + ((a - de) * i) / pas }); }
}

async function demander(sur: SurEvenement, raison: "injoignable" | "autorisation"): Promise<Decision> {
  const demande = demandeSuiv++;
  const p = new Promise<Decision>((r) => attentes.set(demande, r));
  sur({ type: "attente_utilisateur", demande, raison });
  const d = await p;
  if (annule) throw erreur("annule", "Opération annulée.");
  return d;
}

// ---- Données ----
const SYS: Omit<AppliSysteme, "etat">[] = [
  { paquet: "tv.alphonso.alphonso_eula", libelle: "Alphonso", explication: "Reconnaît ce que vous regardez pour cibler la publicité.", recommande: "desactiver", groupe: "pistage" },
  { paquet: "com.miui.tv.analytics", libelle: "Statistiques Xiaomi", explication: "Envoie votre usage de la box à Xiaomi.", recommande: "desactiver", groupe: "pistage" },
  { paquet: "com.google.android.feedback", libelle: "Rapports Google", explication: "Envoie des rapports d'utilisation à Google.", recommande: "desactiver", groupe: "pistage" },
  { paquet: "com.google.android.tv.bugreportsender", libelle: "Rapports de bug", explication: "Envoie des rapports de plantage à Google.", recommande: "desactiver", groupe: "pistage" },
  { paquet: "com.mitv.tvhome.atv", libelle: "PatchWall", explication: "L'accueil Xiaomi et ses contenus promotionnels.", recommande: "desactiver", groupe: "pistage" },
  { paquet: "com.mitv.tvhome.michannel", libelle: "Chaînes Mi", explication: "Contenus poussés par Xiaomi.", recommande: "desactiver", groupe: "pistage" },
  { paquet: "com.xm.webcontent", libelle: "Contenus web Xiaomi", explication: "Promotions affichées par Xiaomi.", recommande: "desactiver", groupe: "pistage" },
  { paquet: "android.autoinstalls.config.xioami.mibox3", libelle: "Installations automatiques", explication: "Installe des applis partenaires sans vous demander.", recommande: "desactiver", groupe: "pistage" },
  { paquet: "com.xiaomi.android.tvsetup.partnercustomizer", libelle: "Rangées partenaires", explication: "Ajoute des publicités de partenaires à l'accueil.", recommande: "desactiver", groupe: "pistage" },
  { paquet: "com.google.android.tvrecommendations", libelle: "Recommandations Google", explication: "Suggestions de l'ancien accueil, inutiles avec Zen Home.", recommande: "desactiver", groupe: "pistage" },
  { paquet: "com.google.android.backdrop", libelle: "Écran de veille Google", explication: "Remplacé par celui de Zen Home.", recommande: "desactiver", groupe: "pistage" },
  { paquet: "com.google.android.katniss", libelle: "Recherche vocale", explication: "Le bouton micro de la télécommande. Si vous la désactivez, le micro ne fera plus rien, mais vous libérez environ 115 Mo de mémoire.", recommande: "garder", groupe: "option" },
  { paquet: "com.google.android.tv", libelle: "Chaînes en direct", explication: "Sert aux tuners TV. La Mi Box n'en a pas, l'appli tourne pour rien.", recommande: "desactiver", groupe: "option" },
  { paquet: "com.google.android.tts", libelle: "Synthèse vocale", explication: "Lit les textes à voix haute pour le lecteur d'écran. Inutile si personne n'en a besoin, libère environ 300 Mo.", recommande: "garder", groupe: "option" },
  { paquet: "com.google.android.marvin.talkback", libelle: "Lecteur d'écran (TalkBack)", explication: "Aide pour les personnes malvoyantes. Inutile si personne n'en a besoin.", recommande: "garder", groupe: "option" },
  { paquet: "com.mitv.milinkservice", libelle: "Mi Link", explication: "Diffuser depuis un téléphone Xiaomi. Le Chromecast intégré reste disponible pour tous les téléphones.", recommande: "desactiver", groupe: "option" },
  { paquet: "com.mitv.videoplayer", libelle: "Lecteur vidéo Xiaomi", explication: "Lit les vidéos d'une clé USB. Gardez-le si vous regardez des fichiers sur clé USB.", recommande: "garder", groupe: "option" },
  { paquet: "com.google.android.play.games", libelle: "Play Jeux", explication: "L'appli de jeux Google. Gardez-la si vous jouez sur la box.", recommande: "garder", groupe: "option" },
  { paquet: "com.google.android.youtube.tvmusic", libelle: "YouTube Music", explication: "Gardez-la si vous écoutez de la musique avec.", recommande: "garder", groupe: "option" },
  { paquet: "com.google.android.videos", libelle: "Google TV (films Google)", explication: "Location et achat de films chez Google. Gardez-la si vous en avez acheté.", recommande: "garder", groupe: "option" },
  { paquet: "com.android.camera2", libelle: "Appareil photo", explication: "La box n'a pas de caméra.", recommande: "desactiver", groupe: "option" },
  { paquet: "com.android.printspooler", libelle: "Impression", explication: "La box ne sert pas à imprimer.", recommande: "desactiver", groupe: "option" },
  { paquet: "com.xiaomi.mitv.updateservice", libelle: "Mises à jour Xiaomi", explication: "Installe les mises à jour de sécurité de la box. Recommandé de garder.", recommande: "garder", groupe: "option" },
  { paquet: "com.mitv.download.service", libelle: "Téléchargements Xiaomi", explication: "Utilisé par les mises à jour Xiaomi. Recommandé de garder.", recommande: "garder", groupe: "option" },
];
const UTIL: AppliUtilisateur[] = [
  { paquet: "com.netflix.ninja", libelle: "Netflix", taille: "182 Mo", derniere_utilisation: "hier" },
  { paquet: "com.google.android.youtube.tv", libelle: "YouTube", taille: "96 Mo", derniere_utilisation: "aujourd'hui" },
  { paquet: "com.spotify.tv.android", libelle: "Spotify", taille: "74 Mo", derniere_utilisation: "il y a 5 jours" },
  { paquet: "tv.twitch.android.app", libelle: "Twitch", taille: "63 Mo", derniere_utilisation: "il y a 7 mois" },
  { paquet: "com.king.candycrushsaga", libelle: "Candy Crush Saga", taille: "148 Mo", derniere_utilisation: "jamais" },
  { paquet: "com.example.vieille.appli", libelle: "Appli à nom très très long pour voir comment la ligne se comporte quand le texte déborde", taille: "12 Mo", derniere_utilisation: "il y a 2 ans" },
  { paquet: "com.disney.disneyplus", libelle: "Disney+", taille: "120 Mo", derniere_utilisation: "il y a 3 semaines" },
];

/** Une box simulée : son propre état, ses applis et sa sauvegarde d'origine (SALON seulement). */
interface BoxSim {
  id: number; nom: string; ip: string; origine: boolean;
  sansSys: string[]; util: string[]; etats: Map<string, AppliSysteme["etat"]>; applis: number; desactivees: number;
}
const BOXES: BoxSim[] = [
  { id: 1, nom: "SALON", ip: "192.168.50.14", origine: true, sansSys: ["com.android.camera2"],
    util: ["com.netflix.ninja", "com.google.android.youtube.tv", "com.spotify.tv.android", "tv.twitch.android.app", "com.example.vieille.appli"],
    etats: new Map([["com.google.android.tv", "desactive"], ["com.google.android.katniss", "garde"]]), applis: 61, desactivees: 2 },
  { id: 2, nom: "CHAMBRE", ip: "192.168.50.15", origine: false, sansSys: ["com.mitv.videoplayer", "com.google.android.play.games"],
    util: ["com.netflix.ninja", "com.google.android.youtube.tv", "com.spotify.tv.android", "com.king.candycrushsaga", "com.disney.disneyplus"],
    etats: new Map([["com.google.android.katniss", "desactive"], ["com.google.android.backdrop", "desactive"]]), applis: 58, desactivees: 2 },
];
// La box simulée suit la box connectée.
let box = BOXES[0];
const sysBox = () => SYS.filter((a) => !box.sansSys.includes(a.paquet));
const utilBox = () => UTIL.filter((u) => box.util.includes(u.paquet));
let choixFait: Choix | null = null;

const delaiReseau = () => pause(350);

export const simulation: Api = {
  async etatInitial() {
    await pause(250);
    return { version: "0.0.2-sim", adb_installe: !q.has("sansadb"), demo: true, sauvegardes_dir_existe: true, reprise_en_attente: q.has("reprise"), origine: q.has("sansorigine") ? null : { date: DATE_ORIGINE },
      boxes_memorisees: q.has("sansmemoire") ? [] : [{ nom: BOXES[0].nom, ip: BOXES[0].ip }],
      films_inclus: !q.has("sansfilms") };
  },

  installerAdb: (accord, sur) => exclusive(async () => {
    if (!accord) throw erreur("refus_licence", "L'outil adb ne peut pas être installé sans votre accord.");
    await progresser(sur, "Téléchargement de l'outil adb (environ 18 Mo)", 2600, 0, 0.8);
    await progresser(sur, "Vérification de l'empreinte numérique", 700, 0.8, 0.92);
    await progresser(sur, "Installation de l'outil", 600, 0.92, 1);
  }),

  async chercherBox(): Promise<BoxTrouvee[]> {
    return exclusive(async () => {
      await pause(3400);
      if (q.has("aucune")) return [];
      return BOXES.map(({ id, nom, ip }) => ({ id, nom, ip }));
    });
  },

  connecter: (cible: Cible, sur) => exclusive(async (): Promise<BoxConnectee> => {
    const choisie = BOXES.find((b) => ("ip" in cible ? b.ip === cible.ip : b.id === cible.id));
    const ip = "ip" in cible ? cible.ip : choisie?.ip ?? BOXES[0].ip;
    await progresser(sur, `Connexion à la box (${ip})`, 1200, 0, 0.4);
    if (ip === "192.168.50.99") throw erreur("injoignable", "La box ne répond pas. Vérifiez qu'elle est allumée (pas en veille) et sur le même Wi-Fi que ce Mac.");
    if (q.has("refus") && !refusFait) { refusFait = true; throw erreur("non_autorise", "La box n'a pas accepté la connexion. Acceptez l'autorisation affichée sur la TV (cochez « Toujours autoriser »), puis réessayez."); }
    if (q.has("refus2") && choisie === BOXES[1] && !refus2Fait) { refus2Fait = true; throw erreur("non_autorise", "La box n'a pas accepté la connexion. Acceptez l'autorisation affichée sur la TV (cochez « Toujours autoriser »), puis réessayez."); }
    if (q.has("autorisation")) {
      if ((await demander(sur, "autorisation")) === "arreter") throw erreur("annule", "Opération arrêtée. Rien n'a été modifié.");
      sur({ type: "reconnectee" });
    }
    if (q.has("autremodele")) throw erreur("autre_modele", "Cet appareil n'est pas une Xiaomi Mi Box 4 (modèle détecté : « SHIELD »). Par sécurité, Zen Home s'arrête ici, sans rien modifier.");
    await progresser(sur, "Lecture du modèle de la box", 700, 0.4, 1);
    box = choisie ?? BOXES[0];
    return { nom: box.nom, ip, modele: "Xiaomi Mi Box 4 (MIBOX4)" };
  }),

  async chercherVilles(saisie): Promise<Ville[]> {
    await delaiReseau();
    const norme = (s: string) => s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
    const k = norme(saisie.trim());
    if (k.length < 2) return [];
    const toutes: Ville[] = [
      { id: 2988507, nom: "Paris", region: "Île-de-France", pays: "France" },
      { id: 4717560, nom: "Paris", region: "Texas", pays: "États-Unis" },
      { id: 2996944, nom: "Lyon", region: "Auvergne-Rhône-Alpes", pays: "France" },
      { id: 2998324, nom: "Lille", region: "Hauts-de-France", pays: "France" },
      { id: 2995469, nom: "Marseille", region: "Provence-Alpes-Côte d'Azur", pays: "France" },
      { id: 2972315, nom: "Toulouse", region: "Occitanie", pays: "France" },
      { id: 2990969, nom: "Nantes", region: "Pays de la Loire", pays: "France" },
      { id: 3031582, nom: "Bordeaux", region: "Nouvelle-Aquitaine", pays: "France" },
      { id: 2973783, nom: "Strasbourg", region: "Grand Est", pays: "France" },
      { id: 2992166, nom: "Montpellier", region: "Occitanie", pays: "France" },
      { id: 2983990, nom: "Rennes", region: "Bretagne", pays: "France" },
      { id: 2980291, nom: "Saint-Étienne", region: "Auvergne-Rhône-Alpes", pays: "France" },
      { id: 2802361, nom: "Bruxelles", region: "Région de Bruxelles-Capitale", pays: "Belgique" },
      { id: 2660646, nom: "Genève", region: "Genève", pays: "Suisse" },
    ];
    return toutes.filter((v) => norme(v.nom).includes(k));
  },

  inventaire: (sur) => exclusive(async (): Promise<Inventaire> => {
    await progresser(sur, "Sauvegarde de l'état actuel de la box", 1600, 0, 0.35);
    await progresser(sur, "Lecture des applis installées", 1500, 0.35, 0.8);
    await progresser(sur, "Recherche des noms lisibles", 900, 0.8, 1);
    return {
      applis_systeme: sysBox().map((a) => ({ ...a, etat: box.etats.get(a.paquet) ?? "actif" })),
      applis_utilisateur: utilBox(),
      resume: { modele: "Xiaomi Mi Box 4", android: "Android TV 9", nb_applis: box.applis, nb_desactivees: box.desactivees },
    };
  }),

  installer: (villeId, jeton, sur) => exclusive(async () => {
    if (jeton && !/^[A-Za-z0-9._~+/=-]+$/.test(jeton)) throw erreur("choix_invalide", "Ce texte ne ressemble pas à un jeton TMDB.");
    void villeId;
    await progresser(sur, "Envoi de Zen Home sur la box", 3200, 0, 0.7);
    await progresser(sur, "Installation de Zen Home", 1800, 0.7, 0.9);
    await progresser(sur, villeId !== null || jeton ? "Réglage de la météo et des films" : "Réglages de Zen Home", 900, 0.9, 1);
  }),

  appliquer: (choix, sur) => exclusive(async (): Promise<RapportApplication> => {
    const connus = new Set(sysBox().map((s) => s.paquet)), users = new Set(utilBox().map((u) => u.paquet));
    if (choix.garder.concat(choix.desactiver).some((p) => !connus.has(p)) || choix.desinstaller.some((p) => !users.has(p)))
      throw erreur("choix_invalide", "Un des choix ne correspond à aucune appli de la box. Recommencez depuis le début.");
    choixFait = choix;
    sur({ type: "etape", nom: "Mémorisation de vos choix" });
    await pause(700); sur({ type: "progression", fraction: 0.1 });
    const total = choix.desactiver.length + choix.desinstaller.length || 1;
    let fait = 0;
    for (const p of choix.desactiver) {
      const a = SYS.find((s) => s.paquet === p)!;
      sur({ type: "etape", nom: `Désactivation : ${a.libelle}` });
      await pause(380);
      if (q.has("coupure") && !coupureFaite && fait === 3) {
        coupureFaite = true;
        sur({ type: "connexion_perdue", essai: 1 }); await pause(1500);
        sur({ type: "connexion_perdue", essai: 2 }); await pause(1500);
        sur({ type: "reconnectee" });
      }
      fait++; sur({ type: "progression", fraction: 0.1 + 0.9 * (fait / total) });
    }
    const echecs: string[] = [];
    for (const p of choix.desinstaller) {
      const u = UTIL.find((x) => x.paquet === p)!;
      sur({ type: "etape", nom: `Désinstallation : ${u.libelle}` });
      await pause(700);
      fait++; sur({ type: "progression", fraction: 0.1 + 0.9 * (fait / total) });
    }
    box.desactivees += choix.desactiver.length;
    box.applis -= choix.desinstaller.length - echecs.length;
    return { desactivees: choix.desactiver.length, gardees: choix.garder.length, desinstallees: choix.desinstaller.filter((p) => !echecs.includes(p)), echecs };
  }),

  verifier: (reparer, sur) => exclusive(async (): Promise<Verification> => {
    await progresser(sur, "Contrôle de la box", 1300, 0, 0.35);
    if (q.has("coupure") && !coupureVerifFaite) {
      coupureVerifFaite = true;
      // Box injoignable : la reconnexion automatique échoue, l'utilisateur doit la réveiller.
      for (let i = 1; i <= 3; i++) { sur({ type: "connexion_perdue", essai: i }); await pause(900); }
      const d = await demander(sur, "injoignable");
      if (d === "arreter") throw erreur("annule", "Opération arrêtée. Ce qui a déjà été fait est conservé ; relancez « Réparer » pour terminer.");
      sur({ type: "reconnectee" });
    }
    if (reparer) await progresser(sur, "Réglage final de la box", 2600, 0.35, 0.9);
    await progresser(sur, "Nouveau contrôle", 1000, reparer ? 0.9 : 0.35, 1);
    const lignes: LigneVerif[] = [
      { etat: "ok", controle: "Zen Home installé", detail: "version 1.4.2" },
      { etat: "ok", controle: "Rôle HOME = tv.zenhome", detail: "" },
      { etat: "ok", controle: "Désactivé : com.google.android.tvlauncher", detail: "" },
      { etat: "ok", controle: "Désactivé : com.google.android.tungsten.setupwraith", detail: "" },
    ];
    const gardes = new Set(choixFait?.garder ?? ["com.google.android.katniss", "com.xiaomi.mitv.updateservice", "com.mitv.download.service"]);
    for (const a of sysBox()) {
      if (gardes.has(a.paquet)) lignes.push({ etat: "avert", controle: `Désactivé : ${a.paquet}`, detail: "gardé actif (votre choix)" });
      else lignes.push({ etat: "ok", controle: `Désactivé : ${a.paquet}`, detail: "" });
    }
    lignes.push(
      { etat: "ok", controle: "Accessibilité : KeyService", detail: "" },
      { etat: "ok", controle: "Écran de veille = ZenDream", detail: "" },
      { etat: "ok", controle: "Stats d'usage (Applis ouvertes)", detail: "" },
      { etat: "ok", controle: "Spotify : arrière-plan restreint", detail: "" },
      { etat: "ok", controle: "Wi-Fi non lié à setupwraith", detail: "" },
    );
    return { lignes, ko: lignes.filter((l) => l.etat === "ko").length };
  }),

  restaurer: (sur) => exclusive(async (): Promise<RapportRestauration> => {
    await progresser(sur, "Réactivation des applis désactivées", 2200, 0, 0.5);
    await progresser(sur, "Retour à l'accueil d'origine", 1500, 0.5, 0.8);
    await progresser(sur, "Désinstallation de Zen Home", 1300, 0.8, 1);
    return { incomplete: false, avertissements: [], depuis_sauvegarde: q.has("sansorigine") || !box.origine ? null : DATE_ORIGINE };
  }),

  async decider(demande, decision) {
    attentes.get(demande)?.(decision);
    attentes.delete(demande);
  },
  async annuler() { annule = true; },
  async couperDebogage() { await pause(900); },
  async ouvrirSauvegardes() { await pause(150); },
  async rapportTexte(erreur, resume) { return `Zen Home 0.0.2-sim\nSystème : simulation\n${erreur ? `Erreur : ${erreur}\n` : ""}\n${resume}\n`; },
  async envoyerRapport() { await pause(150); },
  desinstallerMac: () => exclusive(async () => { await pause(1500); }),
};
