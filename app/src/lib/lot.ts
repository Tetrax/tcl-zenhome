// Ajout de plusieurs box d'affilée : fusion des inventaires pour poser les questions une seule fois, puis choix ramené à chaque box.
import type { AppliSysteme, AppliUtilisateur, Choix, Inventaire } from "./types";

/** Inventaire commun : union des applis. Une appli système dont l'état diffère d'une box à l'autre repasse à « actif » (recommandation). */
export function fusionner(inventaires: { nom: string; inv: Inventaire }[]): Inventaire {
  const sys = new Map<string, AppliSysteme>();
  const util = new Map<string, AppliUtilisateur>();
  for (const { nom, inv } of inventaires) {
    for (const a of inv.applis_systeme) {
      const vue = sys.get(a.paquet);
      if (!vue) sys.set(a.paquet, { ...a });
      else if (vue.etat !== a.etat) vue.etat = "actif";
    }
    for (const a of inv.applis_utilisateur) {
      const vue = util.get(a.paquet);
      if (!vue) util.set(a.paquet, { ...a, sur: [nom] });
      else {
        vue.sur!.push(nom);
        if (vue.libelle === vue.paquet && a.libelle !== a.paquet) vue.libelle = a.libelle;
      }
    }
  }
  return {
    applis_systeme: [...sys.values()],
    applis_utilisateur: [...util.values()],
    resume: inventaires[0].inv.resume, // modèle et Android identiques (même modèle de box) ; les compteurs sont ceux de la 1re box
  };
}

/** Choix limité à ce que cette box contient. Une appli système présente mais absente du choix est gardée, jamais ignorée. */
export function reduire(choix: Choix, inv: Inventaire): Choix {
  const sys = new Set(inv.applis_systeme.map((a) => a.paquet));
  const util = new Set(inv.applis_utilisateur.map((a) => a.paquet));
  const desactiver = choix.desactiver.filter((p) => sys.has(p));
  const garder = choix.garder.filter((p) => sys.has(p));
  const cites = new Set([...desactiver, ...garder]);
  for (const p of sys) if (!cites.has(p)) garder.push(p);
  return { garder, desactiver, desinstaller: choix.desinstaller.filter((p) => util.has(p)) };
}
