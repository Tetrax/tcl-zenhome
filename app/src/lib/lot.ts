// Ajout de plusieurs box d'affilée : fusion des inventaires pour poser les questions une seule fois, puis choix ramené à chaque box.
import type { AppliSysteme, AppliUtilisateur, Choix, Inventaire } from "./types";

/** Inventaire commun : union des applis. Une appli système dont l'état diffère d'une box à l'autre repasse à « actif » (recommandation). */
/** `id` identifie la box de façon unique (son IP : plusieurs box peuvent porter le même nom). */
export function fusionner(inventaires: { id: string; inv: Inventaire }[]): Inventaire {
  const sys = new Map<string, AppliSysteme>();
  const util = new Map<string, AppliUtilisateur>();
  for (const { id, inv } of inventaires) {
    for (const a of inv.applis_systeme) {
      const vue = sys.get(a.paquet);
      if (!vue) sys.set(a.paquet, { ...a });
      else if (vue.etat !== a.etat) vue.etat = "actif";
    }
    for (const a of inv.applis_utilisateur) {
      const vue = util.get(a.paquet);
      if (!vue) util.set(a.paquet, { ...a, sur: [id] });
      else {
        vue.sur!.push(id);
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

/** Choix limité à ce que cette box contient. Une appli système présente mais absente du choix est gardée, jamais ignorée.
 *  Une désinstallation n'est gardée que si la box `id` figurait dans `sur` de l'inventaire `commun` au moment du choix :
 *  l'utilisateur ne l'a vue que pour ces box (une box absente des questions n'en reçoit aucune). */
export function reduire(choix: Choix, inv: Inventaire, id: string, commun: Inventaire): Choix {
  const sys = new Set(inv.applis_systeme.map((a) => a.paquet));
  const util = new Set(inv.applis_utilisateur.map((a) => a.paquet));
  const montrees = new Set(commun.applis_utilisateur.filter((a) => a.sur?.includes(id)).map((a) => a.paquet));
  const desactiver = choix.desactiver.filter((p) => sys.has(p));
  const garder = choix.garder.filter((p) => sys.has(p));
  const cites = new Set([...desactiver, ...garder]);
  for (const p of sys) if (!cites.has(p)) garder.push(p);
  return { garder, desactiver, desinstaller: choix.desinstaller.filter((p) => util.has(p) && montrees.has(p)) };
}
