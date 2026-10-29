// État global et déroulé du parcours. L'interface n'envoie que des intentions au cœur (voir api.ts).
import { api, normaliserErreur } from "./api";
import { fusionner, reduire } from "./lot";
import type { Moment } from "../scene";
import type {
  AppliSysteme, BoxConnectee, BoxMemorisee, BoxTrouvee, Choix, Cible, ErreurZen, EtatInitial, Evenement, Inventaire,
  RapportApplication, RapportRestauration, Verification, Ville,
} from "./types";

export type Mode = "installer" | "reparer" | "restaurer";
export type Ecran =
  | "accueil" | "adb" | "recherche" | "connexion" | "etat" | "resume" | "meteo" | "films"
  | "installation" | "systeme" | "utilisateur" | "recap" | "application" | "reparation"
  | "confirm_restauration" | "restauration" | "fin_restauration" | "resultat" | "adieu" | "lot" | "recap_lot";

/** Une box d'un lot (plusieurs box configurées d'affilée avec les mêmes choix). */
export interface BoxLot {
  cible: Cible; nom: string; ip: string; message: string;
  etat: "attente" | "en-cours" | "ok" | "echec" | "non-traitee";
  /** Inventaire relevé une fois Zen Home installé (passage 1) ; vrai si le passage 1 est terminé pour cette box. */
  inv?: Inventaire; passe1?: boolean;
  rapport?: RapportApplication; verification?: Verification; restauration?: RapportRestauration;
  debogage?: "oui" | "echec";
  /** Vrai si la box n'a pas pris part aux questions communes (échec du passage 1) : aucune désinstallation pour elle. */
  horsChoix?: boolean;
}

/** Une box proposée dans la recherche : trouvée (avec `id`) ou seulement mémorisée (connexion par son adresse). */
export interface Proposition { id?: number; nom: string; ip: string; recente: boolean }

/** Moment de la journée affiché derrière chaque écran. */
export const MOMENT: Record<Ecran, Moment> = {
  accueil: "aube", adb: "matin", recherche: "matin", connexion: "jour", etat: "jour", resume: "jour",
  meteo: "doree", films: "doree", installation: "doree", systeme: "doree", utilisateur: "doree", recap: "doree",
  application: "doree", reparation: "doree", confirm_restauration: "doree", restauration: "doree",
  fin_restauration: "aube", resultat: "nuit", adieu: "nuit", lot: "doree", recap_lot: "nuit",
};
/** Écrans où une action longue pilote la lumière (jusqu'au crépuscule). */
export const ECRANS_TRAVAIL: Ecran[] = ["application", "reparation", "restauration", "lot"];

export const ETAPES = ["Accueil", "Box", "Préparation", "Vos choix", "Installation", "Terminé"] as const;
export const ETAPE_DE: Record<Ecran, number> = {
  accueil: 0, adb: 0, recherche: 1, connexion: 1, etat: 2, resume: 2, meteo: 2, films: 2, installation: 2,
  systeme: 3, utilisateur: 3, recap: 3, application: 4, reparation: 4, confirm_restauration: 4, restauration: 4,
  fin_restauration: 5, resultat: 5, adieu: 5, lot: 4, recap_lot: 5,
};

const TOKEN_OK = /^[A-Za-z0-9._~+/=-]+$/;
export const jetonValide = (j: string) => TOKEN_OK.test(j);

/** Valeur par défaut de la case d'une appli système : l'état actuel d'abord, puis la recommandation (comme le kit). */
export function desactiverParDefaut(a: AppliSysteme): boolean {
  if (a.etat === "garde") return false;
  if (a.etat === "desactive") return true;
  return a.recommande === "desactiver";
}

class Zen {
  init = $state<EtatInitial | null>(null);
  ecran = $state<Ecran>("accueil");
  mode = $state<Mode>("installer");

  // Recherche et connexion
  rechercheEnCours = $state(false);
  boxes = $state<BoxTrouvee[]>([]);
  rechercheFaite = $state(false);
  /** Box déjà configurées, affichées dès le début de la recherche. */
  memorisees = $state<BoxMemorisee[]>([]);
  /** Après un échec de connexion : rouvre directement la saisie de l'adresse. */
  forcerManuel = $state(false);
  cible = $state<Cible | null>(null);
  box = $state<BoxConnectee | null>(null);

  // Plusieurs box : les adresses cochées dans la recherche, puis le lot (null = parcours à une seule box)
  coches = $state<string[]>([]);
  lot = $state<BoxLot[] | null>(null);
  lotPasse = $state(0); // installer : 1 = installation, 2 = application des choix
  lotPhase = $state<"travail" | "choix">("travail");
  lotFait = $state(0);
  lotSur = $state(1);
  lotAnnule = false;
  /** Nom de la box en cours de traitement (pour dire devant quelle TV aller). */
  boxActive = $state("");

  // Préparation
  inventaire = $state<Inventaire | null>(null);
  ville = $state<Ville | null>(null);
  jeton = $state("");
  desactiver = $state<Record<string, boolean>>({});
  desinstaller = $state<Record<string, boolean>>({});

  // Action longue en cours
  travail = $state({ actif: false, etape: "", fraction: null as number | null, avertissements: [] as string[], annulation: false });
  jalon = $state(0); // application : 0 installer, 1 appliquer, 2 vérifier
  reco = $state<{ essai: number } | { revenue: true } | null>(null);
  attente = $state<{ demande: number; raison: "injoignable" | "autorisation" } | null>(null);
  erreur = $state<ErreurZen | null>(null);
  reprise: (() => void) | null = null;

  // Résultats
  rapport = $state<RapportApplication | null>(null);
  verification = $state<Verification | null>(null);
  restauration = $state<RapportRestauration | null>(null);
  debogageCoupe = $state<"non" | "en_cours" | "oui" | "echec">("non");
  /** Compteur incrémenté à chaque succès, pour déclencher l'onde de lumière de la scène. */
  succes = $state(0);

  async charger() {
    try { this.init = await api.etatInitial(); } catch (e) { this.erreur = normaliserErreur(e); }
  }

  aller(e: Ecran) {
    if (e === "accueil" || e === "adieu" || e === "recherche") this.jeton = ""; // le jeton ne survit pas à la sortie du lot
    if (e === "recherche") this.lot = null;
    this.erreur = null; this.reprise = null; this.attente = null; this.ecran = e; }

  private sur = (e: Evenement) => {
    const t = this.travail;
    switch (e.type) {
      case "etape": t.etape = e.nom; break;
      case "progression": t.fraction = Math.min(1, Math.max(0, e.fraction)); break;
      case "connexion_perdue": this.reco = { essai: e.essai }; break;
      case "reconnectee": this.reco = { revenue: true }; setTimeout(() => { if (this.reco && "revenue" in this.reco) this.reco = null; }, 3500); break;
      case "attente_utilisateur": this.attente = { demande: e.demande, raison: e.raison }; break;
      case "avertissement": if (t.avertissements.length < 20) t.avertissements.push(e.texte); break;
    }
  };

  private debut(etape = "") {
    this.erreur = null; this.reprise = null; this.reco = null; this.attente = null;
    this.travail = { actif: true, etape, fraction: null, avertissements: [], annulation: false };
  }
  private fin() { this.travail.actif = false; this.travail.annulation = false; this.attente = null; this.boxActive = ""; }
  private echec(e: unknown, reprise: (() => void) | null) {
    this.fin();
    this.erreur = normaliserErreur(e);
    this.reprise = reprise;
  }

  // ---- Accueil ----
  demarrer(mode: Mode) {
    this.mode = mode;
    this.box = null; this.cible = null; this.boxes = []; this.rechercheFaite = false; this.forcerManuel = false;
    this.lot = null; this.coches = [];
    this.aller(this.init && !this.init.adb_installe ? "adb" : "recherche");
  }
  retourAccueil() {
    if (this.travail.actif || this.rechercheEnCours) void api.annuler().catch(() => {});
    this.lot = null; // la boucle du lot en cours s'arrête d'elle-même
    this.fin(); this.aller("accueil");
  }

  async annuler() {
    if (!this.travail.actif || this.travail.annulation) return;
    this.travail.annulation = true;
    if (this.lot) this.lotAnnule = true; // arrête tout le lot, pas seulement la box en cours
    try { await api.annuler(); } catch { /* l'action en cours rendra son verdict */ }
  }
  async decider(decision: "reessayer" | "arreter") {
    const a = this.attente;
    if (!a) return;
    this.attente = null;
    // Une demande expirée ou déjà traitée est refusée par le cœur : l'action en cours donnera son propre verdict.
    try { await api.decider(a.demande, decision); } catch { /* voir ci-dessus */ }
  }

  // ---- adb ----
  async installerAdb() {
    this.debut("Préparation du téléchargement");
    try {
      await api.installerAdb(true, this.sur);
      if (this.init) this.init.adb_installe = true;
      this.fin();
      this.aller("recherche");
    } catch (e) { this.echec(e, () => void this.installerAdb()); }
  }

  // ---- Recherche ----
  /** Les box trouvées, puis les mémorisées qui n'ont pas (encore) répondu : une seule ligne par adresse. */
  get propositions(): Proposition[] {
    const trouvees = this.boxes.map((b) => ({ ...b, recente: false }));
    return [...trouvees, ...this.memorisees.filter((m) => !this.boxes.some((b) => b.ip === m.ip)).map((m) => ({ ...m, recente: true }))];
  }
  /** Une connexion ou un lot arrête la recherche en cours (le cœur ne fait qu'une chose à la fois) et attend qu'elle ait rendu la main. */
  private async attendreRecherche() {
    if (!this.rechercheEnCours) return;
    await api.annuler().catch(() => {});
    await this.recherche; // la fin réelle de l'appel chercher_box
  }
  private recherche: Promise<void> | null = null;
  /** Connexion à une box ; « occupé » (le cœur finit encore la recherche annulée) : 3 nouveaux essais espacés de 500 ms. */
  private async connecterBox(cible: Cible): Promise<BoxConnectee> {
    for (let essai = 0; ; essai++) {
      try { return await api.connecter(cible, this.sur); }
      catch (e) {
        if (normaliserErreur(e).code !== "occupe" || essai >= 3) throw e;
        await new Promise((r) => setTimeout(r, 500));
      }
    }
  }

  chercher(essai = 0): Promise<void> { return (this.recherche = this.lancerRecherche(essai)); }
  private async lancerRecherche(essai: number): Promise<void> {
    this.rechercheEnCours = true; this.rechercheFaite = false; this.boxes = []; this.coches = []; this.erreur = null;
    api.etatInitial().then((i) => { this.memorisees = i.boxes_memorisees ?? []; }, () => {}); // sans attendre : la recherche continue
    try {
      this.boxes = await api.chercherBox();
      this.rechercheFaite = true;
    } catch (e) {
      const err = normaliserErreur(e);
      if (err.code === "occupe" && essai < 6) { // la recherche précédente se termine : on réessaie sans bruit
        await new Promise((r) => setTimeout(r, 800));
        if (this.ecran === "recherche") return await this.chercher(essai + 1);
        return;
      }
      if (err.code === "annule") { this.rechercheFaite = true; } else this.echec(err, () => void this.chercher());
    } finally { this.rechercheEnCours = false; }
  }

  // ---- Connexion ----
  async connecter(cible: Cible) {
    this.lot = null; this.cible = cible; this.box = null;
    this.aller("connexion");
    this.debut("Connexion à la box");
    try {
      await this.attendreRecherche();
      if (!this.travail.actif) return; // retour à l'accueil pendant l'attente
      this.box = await this.connecterBox(cible);
      this.fin();
      this.succes++;
      if (this.mode === "installer") await this.lireEtat();
      else if (this.mode === "reparer") await this.reparer();
      else {
        // Une sauvegarde d'origine a pu être écrite depuis le démarrage : l'écran de confirmation cite la bonne.
        try { this.init = await api.etatInitial(); } catch { /* on garde l'état connu */ }
        this.aller("confirm_restauration");
      }
    } catch (e) { this.echec(e, () => void this.connecter(cible)); }
  }

  // ---- État des lieux ----
  async lireEtat() {
    this.aller("etat");
    this.debut("Lecture de la box");
    try {
      const inv = await api.inventaire(this.sur);
      this.inventaire = inv;
      this.desactiver = Object.fromEntries(inv.applis_systeme.map((a) => [a.paquet, desactiverParDefaut(a)]));
      this.desinstaller = Object.fromEntries(inv.applis_utilisateur.map((a) => [a.paquet, false]));
      this.fin();
      this.aller("resume");
    } catch (e) { this.echec(e, () => void this.lireEtat()); }
  }

  // ---- Installation de Zen Home, avant les choix (comme le kit) : Zen Home fournit ensuite le vrai nom des applis ----
  async installerZenHome() {
    if (this.lot) return void this.lancerPassage1();
    this.aller("installation");
    this.debut("Installation de Zen Home");
    try {
      await api.installer(this.ville ? this.ville.id : null, this.jeton ? this.jeton : null, this.sur);
      this.jeton = ""; // le jeton ne reste pas en mémoire une fois envoyé
      this.debut("Lecture des noms des applis");
      const inv = await api.inventaire(this.sur);
      this.inventaire = inv;
      this.desactiver = Object.fromEntries(inv.applis_systeme.map((a) => [a.paquet, desactiverParDefaut(a)]));
      this.desinstaller = Object.fromEntries(inv.applis_utilisateur.map((a) => [a.paquet, false]));
      this.fin();
      this.aller("systeme");
    } catch (e) { this.echec(e, () => void this.installerZenHome()); }
  }

  recommande() {
    if (!this.inventaire) return;
    for (const a of this.inventaire.applis_systeme) this.desactiver[a.paquet] = desactiverParDefaut(a);
  }
  toutSelectionner(v: boolean) {
    if (!this.inventaire) return;
    for (const a of this.inventaire.applis_systeme) this.desactiver[a.paquet] = v;
  }

  choix(): Choix {
    const inv = this.inventaire;
    if (!inv) return { garder: [], desactiver: [], desinstaller: [] };
    return {
      desactiver: inv.applis_systeme.filter((a) => this.desactiver[a.paquet]).map((a) => a.paquet),
      garder: inv.applis_systeme.filter((a) => !this.desactiver[a.paquet]).map((a) => a.paquet),
      desinstaller: inv.applis_utilisateur.filter((a) => this.desinstaller[a.paquet]).map((a) => a.paquet),
    };
  }

  // ---- Application : installer -> appliquer -> vérifier ----
  /** Part de la barre de progression occupée par chaque étape. */
  private static readonly PARTS: [number, number][] = [[0, 0.4], [0.4, 0.75], [0.75, 1]];
  fractionGlobale(): number | null {
    const f = this.travail.fraction;
    const [a, b] = Zen.PARTS[this.jalon] ?? [0, 1];
    if (this.ecran !== "application") return f;
    return a + (b - a) * (f ?? 0);
  }

  async appliquer(depuis = 1) {
    if (this.lot) return void this.lancerTraitement(this.lot.filter((b) => b.etat === "ok"), 2);
    // Zen Home est déjà installé (installerZenHome) : on part de l'application des choix
    if (this.ecran !== "application") { this.aller("application"); this.rapport = null; this.verification = null; this.debogageCoupe = "non"; }
    const choix = this.choix();
    try {
      if (depuis <= 0) {
        this.jalon = 0; this.debut("Installation de Zen Home");
        await api.installer(this.ville ? this.ville.id : null, this.jeton ? this.jeton : null, this.sur);
        this.jeton = ""; // le jeton ne reste pas en mémoire une fois envoyé
      }
      if (depuis <= 1) {
        this.jalon = 1; this.debut("Application de vos choix");
        this.rapport = await api.appliquer(choix, this.sur);
      }
      this.jalon = 2; this.debut("Vérification et réglage final");
      this.verification = await api.verifier(true, this.sur);
      try { this.inventaire = await api.inventaire(this.sur); } catch { /* affichage seulement : on garde l'inventaire précédent */ }
      this.fin(); this.succes++;
      this.aller("resultat");
    } catch (e) { const j = this.jalon; this.echec(e, () => void this.appliquer(j)); }
  }

  // ---- Réparer ----
  async reparer() {
    this.aller("reparation"); this.jalon = 0; this.verification = null; this.rapport = null; this.debogageCoupe = "non";
    this.debut("Contrôle de la box");
    try {
      this.verification = await api.verifier(true, this.sur);
      this.fin(); this.succes++;
      this.aller("resultat");
    } catch (e) { this.echec(e, () => void this.reparer()); }
  }

  // ---- Revenir à avant Zen Home ----
  async restaurer() {
    this.aller("restauration"); this.restauration = null;
    this.debut("Retour à l'état d'origine");
    try {
      this.restauration = await api.restaurer(this.sur);
      this.fin(); this.succes++;
      this.aller("fin_restauration");
    } catch (e) { this.echec(e, () => void this.restaurer()); }
  }

  // ---- Plusieurs box : une box après l'autre, avec les mêmes choix (voir CONTRAT.md, « Plusieurs box ») ----
  /** Une seule box cochée : parcours habituel ; deux ou plus : lot. */
  async choisirBoxes(ips: string[]) {
    this.coches = ips;
    const choisies = this.propositions.filter((p) => ips.includes(p.ip));
    const cible = (p: Proposition): Cible => (p.id === undefined ? { ip: p.ip } : { id: p.id });
    if (!choisies.length) return;
    if (choisies.length < 2) return void this.connecter(cible(choisies[0]));
    this.lot = choisies.map((p) => ({ cible: cible(p), nom: p.nom, ip: p.ip, message: "", etat: "attente" as const }));
    this.lotPasse = 0;
    await this.attendreRecherche();
    if (this.mode === "installer") this.aller("meteo"); // pas de box connectée : météo et films d'abord
    else if (this.mode === "reparer") void this.lancerTraitement(this.lot, 0);
    else this.aller("confirm_restauration");
  }
  restaurerLot() { if (this.lot) void this.lancerTraitement(this.lot, 0); }

  /** Noms des box pour un `sur` (IP) ; l'IP est ajoutée si plusieurs box portent le même nom. */
  nomsBox(ips: string[]): string {
    const lot = this.lot ?? [];
    return ips.map((ip) => { const b = lot.find((x) => x.ip === ip); return !b ? ip : lot.filter((x) => x.nom === b.nom).length > 1 ? `${b.nom} (${ip})` : b.nom; }).join(", ");
  }

  fractionLot(): number { return (this.lotFait + (this.travail.fraction ?? 0)) / this.lotSur; }

  private pas(nom: string) {
    if (this.lotAnnule) throw { code: "annule", message: "Opération annulée. Ce qui a déjà été fait est conservé." };
    this.debut(nom);
  }
  /** Traite les box d'affilée : une box en échec n'arrête pas les autres, « Annuler » arrête tout. Faux si on a quitté le lot. */
  private async chacune(boxes: BoxLot[], f: (b: BoxLot) => Promise<void>): Promise<boolean> {
    const lot = this.lot;
    this.lotAnnule = false; this.lotPhase = "travail"; this.lotFait = 0; this.lotSur = boxes.length;
    for (const b of boxes) { b.etat = "attente"; b.message = ""; }
    this.aller("lot");
    await this.attendreRecherche();
    for (const b of boxes) {
      if (this.lot !== lot) return false;
      if (this.lotAnnule) { b.etat = "non-traitee"; continue; }
      b.etat = "en-cours"; this.boxActive = b.nom;
      try {
        this.pas("Connexion à la box"); this.box = null;
        this.box = await this.connecterBox(b.cible);
        await f(b);
        b.etat = "ok";
      } catch (e) {
        b.etat = "echec"; b.message = normaliserErreur(e).message;
      }
      this.lotFait++;
    }
    this.fin();
    return this.lot === lot;
  }

  private async passage1(b: BoxLot) {
    this.pas("Lecture de la box"); await api.inventaire(this.sur); // sauvegarde de l'état d'origine
    this.pas("Installation de Zen Home"); await api.installer(this.ville ? this.ville.id : null, this.jeton ? this.jeton : null, this.sur);
    this.pas("Lecture des noms des applis"); b.inv = await api.inventaire(this.sur);
    b.passe1 = true;
  }
  /** Ce qui reste à faire sur une box déjà connectée, selon le mode. */
  private async traiter(b: BoxLot) {
    if (this.mode === "reparer") { this.pas("Contrôle de la box"); b.verification = await api.verifier(true, this.sur); return; }
    if (this.mode === "restaurer") { this.pas("Retour à l'état d'origine"); b.restauration = await api.restaurer(this.sur); return; }
    if (!b.passe1) await this.passage1(b); // reprise d'une box en échec au passage 1
    this.pas("Lecture de la box"); const inv = await api.inventaire(this.sur); // inventaire propre à cette box
    this.pas("Application de vos choix"); b.rapport = await api.appliquer(reduire(this.choix(), inv, b.ip, this.inventaire!), this.sur);
    this.pas("Vérification et réglage final"); b.verification = await api.verifier(true, this.sur);
  }
  async lancerTraitement(boxes: BoxLot[], passe: number) {
    this.lotPasse = passe;
    if (await this.chacune(boxes, (b) => this.traiter(b))) this.finLot();
  }
  private finLot() {
    this.fin();
    if (this.lot?.some((b) => b.etat === "ok")) this.succes++;
    this.aller("recap_lot");
  }
  reessayerLot() { if (this.lot) void this.lancerTraitement(this.lot.filter((b) => b.etat === "echec" || b.etat === "non-traitee"), this.lotPasse); }

  async lancerPassage1(boxes = this.lot ?? []) {
    this.lotPasse = 1;
    if (await this.chacune(boxes, (b) => this.passage1(b))) this.apresPassage1();
  }
  private apresPassage1() {
    if (this.lotAnnule) return this.finLot();
    if (this.lot?.every((b) => b.etat === "ok")) return this.questions();
    this.lotPhase = "choix";
  }
  reessayerPassage1() { void this.lancerPassage1(this.lot?.filter((b) => b.etat !== "ok")); }
  /** Les box qui ont réussi le passage 1 posent les questions, une seule fois pour toutes. */
  questions() {
    for (const b of this.lot ?? []) b.horsChoix = b.etat !== "ok";
    const inv = fusionner((this.lot ?? []).filter((b) => b.etat === "ok").map((b) => ({ id: b.ip, inv: b.inv! })));
    this.inventaire = inv;
    this.desactiver = Object.fromEntries(inv.applis_systeme.map((a) => [a.paquet, desactiverParDefaut(a)]));
    this.desinstaller = Object.fromEntries(inv.applis_utilisateur.map((a) => [a.paquet, false]));
    this.aller("systeme");
  }

  // ---- Fin ----
  async couperDebogage() {
    this.debogageCoupe = "en_cours";
    try { await api.couperDebogage(); this.debogageCoupe = "oui"; } catch (e) { this.debogageCoupe = "echec"; this.erreur = normaliserErreur(e); }
  }
  /** Lot : se reconnecte à chaque box réussie pour couper son débogage. */
  async couperTout() {
    this.debogageCoupe = "en_cours";
    const ok = (this.lot ?? []).filter((b) => b.etat === "ok");
    for (const b of ok) {
      this.boxActive = b.nom;
      try { this.debut("Connexion à la box"); this.box = await this.connecterBox(b.cible); await api.couperDebogage(); b.debogage = "oui"; }
      catch { b.debogage = "echec"; }
    }
    this.fin();
    this.debogageCoupe = ok.every((b) => b.debogage === "oui") ? "oui" : "echec";
  }
  /** Résumé de la dernière action pour le rapport (textes déjà nettoyés par le cœur ; le cœur filtre et borne encore). */
  resumeRapport(): string {
    const l = [`Action : ${{ installer: "installation", reparer: "réparation", restaurer: "retour à l'état d'origine" }[this.mode]}`];
    const applis = (r?: RapportApplication | null) => { if (r) l.push(`Applis : ${r.desactivees} désactivées, ${r.gardees} gardées, ${r.desinstallees.length} désinstallées, ${r.echecs.length} échecs${r.echecs.length ? ` (${r.echecs.join(", ")})` : ""}`); };
    const verif = (v?: Verification | null) => {
      if (!v) return;
      l.push(`Vérification : ${v.lignes.length - v.ko} points sur ${v.lignes.length} en ordre`);
      for (const x of v.lignes) if (x.etat !== "ok") l.push(`${x.etat === "ko" ? "À REGARDER" : "À noter"} : ${x.controle} ${x.detail}`.trim());
    };
    const resto = (r?: RapportRestauration | null) => { if (r) l.push(`Restauration ${r.incomplete ? "incomplète" : "terminée"}`, ...r.avertissements); };
    if (this.lot) {
      this.lot.forEach((b, i) => { l.push(`Box ${i + 1} : ${b.etat}${b.message ? ` : ${b.message}` : ""}`); applis(b.rapport); verif(b.verification); resto(b.restauration); });
    } else { applis(this.rapport); verif(this.verification); resto(this.restauration); }
    return l.join("\n");
  }
  async ouvrirSauvegardes() {
    try { await api.ouvrirSauvegardes(); } catch (e) { this.erreur = normaliserErreur(e); }
  }
  recommencer() {
    this.lot = null; this.coches = [];
    this.box = null; this.cible = null; this.inventaire = null; this.ville = null; this.jeton = ""; this.rapport = null;
    this.verification = null; this.restauration = null; this.boxes = []; this.rechercheFaite = false; this.debogageCoupe = "non";
    this.erreur = null;
    this.aller("accueil");
  }
}

export const zen = new Zen();
