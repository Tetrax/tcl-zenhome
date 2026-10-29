// État global et déroulé du parcours. L'interface n'envoie que des intentions au cœur (voir api.ts).
import { api, normaliserErreur } from "./api";
import type { Moment } from "../scene";
import type {
  AppliSysteme, BoxConnectee, BoxTrouvee, Choix, Cible, ErreurZen, EtatInitial, Evenement, Inventaire,
  RapportApplication, RapportRestauration, Verification, Ville,
} from "./types";

export type Mode = "installer" | "reparer" | "restaurer";
export type Ecran =
  | "accueil" | "adb" | "recherche" | "connexion" | "etat" | "resume" | "meteo" | "films"
  | "installation" | "systeme" | "utilisateur" | "recap" | "application" | "reparation"
  | "confirm_restauration" | "restauration" | "fin_restauration" | "resultat" | "adieu";

/** Moment de la journée affiché derrière chaque écran. */
export const MOMENT: Record<Ecran, Moment> = {
  accueil: "aube", adb: "matin", recherche: "matin", connexion: "jour", etat: "jour", resume: "jour",
  meteo: "doree", films: "doree", installation: "doree", systeme: "doree", utilisateur: "doree", recap: "doree",
  application: "doree", reparation: "doree", confirm_restauration: "doree", restauration: "doree",
  fin_restauration: "aube", resultat: "nuit", adieu: "nuit",
};
/** Écrans où une action longue pilote la lumière (jusqu'au crépuscule). */
export const ECRANS_TRAVAIL: Ecran[] = ["application", "reparation", "restauration"];

export const ETAPES = ["Accueil", "Box", "Préparation", "Vos choix", "Installation", "Terminé"] as const;
export const ETAPE_DE: Record<Ecran, number> = {
  accueil: 0, adb: 0, recherche: 1, connexion: 1, etat: 2, resume: 2, meteo: 2, films: 2, installation: 2,
  systeme: 3, utilisateur: 3, recap: 3, application: 4, reparation: 4, confirm_restauration: 4, restauration: 4,
  fin_restauration: 5, resultat: 5, adieu: 5,
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
  /** Après un échec de connexion : rouvre directement la saisie de l'adresse. */
  forcerManuel = $state(false);
  cible = $state<Cible | null>(null);
  box = $state<BoxConnectee | null>(null);

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

  aller(e: Ecran) { this.erreur = null; this.reprise = null; this.attente = null; this.ecran = e; }

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
  private fin() { this.travail.actif = false; this.travail.annulation = false; this.attente = null; }
  private echec(e: unknown, reprise: (() => void) | null) {
    this.fin();
    this.erreur = normaliserErreur(e);
    this.reprise = reprise;
  }

  // ---- Accueil ----
  demarrer(mode: Mode) {
    this.mode = mode;
    this.box = null; this.cible = null; this.boxes = []; this.rechercheFaite = false; this.forcerManuel = false;
    this.aller(this.init && !this.init.adb_installe ? "adb" : "recherche");
  }
  retourAccueil() {
    if (this.travail.actif) void api.annuler().catch(() => {});
    this.fin(); this.aller("accueil");
  }

  async annuler() {
    if (!this.travail.actif || this.travail.annulation) return;
    this.travail.annulation = true;
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
  async chercher(essai = 0): Promise<void> {
    this.rechercheEnCours = true; this.rechercheFaite = false; this.boxes = []; this.erreur = null;
    try {
      this.boxes = await api.chercherBox();
      this.rechercheFaite = true;
    } catch (e) {
      const err = normaliserErreur(e);
      if (err.code === "occupe" && essai < 6) { // la recherche précédente se termine : on réessaie sans bruit
        await new Promise((r) => setTimeout(r, 800));
        if (this.ecran === "recherche") return this.chercher(essai + 1);
        return;
      }
      if (err.code === "annule") { this.rechercheFaite = true; } else this.echec(err, () => void this.chercher());
    } finally { this.rechercheEnCours = false; }
  }

  // ---- Connexion ----
  async connecter(cible: Cible) {
    this.cible = cible; this.box = null;
    this.aller("connexion");
    this.debut("Connexion à la box");
    try {
      this.box = await api.connecter(cible, this.sur);
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

  async appliquer(depuis = 1) { // Zen Home est déjà installé (installerZenHome) : on part de l'application des choix
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

  // ---- Fin ----
  async couperDebogage() {
    this.debogageCoupe = "en_cours";
    try { await api.couperDebogage(); this.debogageCoupe = "oui"; } catch (e) { this.debogageCoupe = "echec"; this.erreur = normaliserErreur(e); }
  }
  async ouvrirSauvegardes() {
    try { await api.ouvrirSauvegardes(); } catch (e) { this.erreur = normaliserErreur(e); }
  }
  recommencer() {
    this.box = null; this.cible = null; this.inventaire = null; this.ville = null; this.jeton = ""; this.rapport = null;
    this.verification = null; this.restauration = null; this.boxes = []; this.rechercheFaite = false; this.debogageCoupe = "non";
    this.erreur = null;
    this.aller("accueil");
  }
}

export const zen = new Zen();
