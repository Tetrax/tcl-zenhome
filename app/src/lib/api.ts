// Seul point d'entrée vers le cœur Rust. Dans Tauri : invoke + Channel. Dans un navigateur : simulation (sim.ts).
import type {
  BoxConnectee, BoxTrouvee, Choix, Cible, Decision, ErreurZen, EtatInitial, Evenement, Inventaire,
  RapportApplication, RapportRestauration, SurEvenement, Verification, Ville,
} from "./types";

export interface Api {
  etatInitial(): Promise<EtatInitial>;
  installerAdb(accord: boolean, sur: SurEvenement): Promise<void>;
  chercherBox(): Promise<BoxTrouvee[]>;
  connecter(cible: Cible, sur: SurEvenement): Promise<BoxConnectee>;
  chercherVilles(saisie: string): Promise<Ville[]>;
  inventaire(sur: SurEvenement): Promise<Inventaire>;
  installer(villeId: number | null, jeton: string | null, sur: SurEvenement): Promise<void>;
  appliquer(choix: Choix, sur: SurEvenement): Promise<RapportApplication>;
  verifier(reparer: boolean, sur: SurEvenement): Promise<Verification>;
  restaurer(sur: SurEvenement): Promise<RapportRestauration>;
  decider(demande: number, decision: Decision): Promise<void>;
  annuler(): Promise<void>;
  couperDebogage(): Promise<void>;
  ouvrirSauvegardes(): Promise<void>;
  desinstallerMac(garderSauvegardes: boolean): Promise<void>;
}

export const dansTauri = (): boolean => "__TAURI_INTERNALS__" in window;

/** Normalise n'importe quelle erreur en { code, message } (le cœur renvoie déjà ce format). */
export function normaliserErreur(e: unknown): ErreurZen {
  if (e && typeof e === "object" && "code" in e && "message" in e) return e as ErreurZen;
  if (typeof e === "string" && e) return { code: "interne", message: e };
  return { code: "interne", message: "Une erreur inattendue est survenue. Relancez Zen Home." };
}

// ---- Implémentation Tauri ----
async function tauri(): Promise<Api> {
  const { invoke, Channel } = await import("@tauri-apps/api/core");
  const canal = (sur: SurEvenement) => { const c = new Channel<Evenement>(); c.onmessage = sur; return c; };
  // Tauri v2 convertit les noms d'arguments en camelCase côté JS (ville_id -> villeId).
  return {
    etatInitial: () => invoke("etat_initial"),
    installerAdb: (accord, sur) => invoke("installer_adb", { accord, canal: canal(sur) }),
    chercherBox: () => invoke("chercher_box"),
    connecter: (cible, sur) => invoke("connecter", { cible, canal: canal(sur) }),
    chercherVilles: (saisie) => invoke("chercher_villes", { saisie }),
    inventaire: (sur) => invoke("inventaire", { canal: canal(sur) }),
    installer: (villeId, jeton, sur) => invoke("installer", { villeId, jeton, canal: canal(sur) }),
    appliquer: (choix, sur) => invoke("appliquer", { choix, canal: canal(sur) }),
    verifier: (reparer, sur) => invoke("verifier", { reparer, canal: canal(sur) }),
    restaurer: (sur) => invoke("restaurer", { canal: canal(sur) }),
    decider: (demande, decision) => invoke("decider", { demande, decision }),
    annuler: () => invoke("annuler"),
    couperDebogage: () => invoke("couper_debogage"),
    ouvrirSauvegardes: () => invoke("ouvrir_sauvegardes"),
    desinstallerMac: (garderSauvegardes) => invoke("desinstaller_mac", { garderSauvegardes }),
  };
}

let cache: Promise<Api> | null = null;
const charger = (): Promise<Api> => (cache ??= dansTauri() ? tauri() : import("./sim").then((m) => m.simulation));

/** Façade : charge paresseusement l'implémentation (Tauri ou simulation). Les rejets sont des ErreurZen. */
async function appeler<K extends keyof Api>(k: K, ...a: Parameters<Api[K]>): Promise<Awaited<ReturnType<Api[K]>>> {
  try {
    const impl = await charger();
    return await (impl[k] as (...x: unknown[]) => Promise<Awaited<ReturnType<Api[K]>>>)(...a);
  } catch (e) {
    throw normaliserErreur(e);
  }
}
export const api: Api = {
  etatInitial: () => appeler("etatInitial"),
  installerAdb: (a, s) => appeler("installerAdb", a, s),
  chercherBox: () => appeler("chercherBox"),
  connecter: (c, s) => appeler("connecter", c, s),
  chercherVilles: (q) => appeler("chercherVilles", q),
  inventaire: (s) => appeler("inventaire", s),
  installer: (v, j, s) => appeler("installer", v, j, s),
  appliquer: (c, s) => appeler("appliquer", c, s),
  verifier: (r, s) => appeler("verifier", r, s),
  restaurer: (s) => appeler("restaurer", s),
  decider: (d, x) => appeler("decider", d, x),
  annuler: () => appeler("annuler"),
  couperDebogage: () => appeler("couperDebogage"),
  ouvrirSauvegardes: () => appeler("ouvrirSauvegardes"),
  desinstallerMac: (g) => appeler("desinstallerMac", g),
};
