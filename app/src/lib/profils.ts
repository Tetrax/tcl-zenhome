// Profils prêts à l'emploi pour les applis système : calcul pur des choix (pas de désinstallation, jamais).
import type { AppliSysteme } from "./types";

export type Profil = "recommande" | "simple" | "leger" | "perso";

/** Gardées même par le profil « Simple » : recherche vocale (micro de la télécommande), mises à jour Xiaomi et téléchargements Xiaomi (utilisés par les mises à jour), service système TCL (mises à jour de la TV).
 *  Un paquet absent de la box est simplement ignoré. */
export const GARDEES_SIMPLE = ["com.google.android.katniss", "com.xiaomi.mitv.updateservice", "com.mitv.download.service", "com.tcl.ttvs"];

/** Valeur par défaut de la case d'une appli système : l'état actuel d'abord, puis la recommandation (comme le kit). */
export function desactiverParDefaut(a: AppliSysteme): boolean {
  if (a.etat === "garde") return false;
  if (a.etat === "desactive") return true;
  return a.recommande === "desactiver";
}

/** Case « désactiver » de chaque appli (par paquet) pour un profil. Une appli absente de la box n'est pas dans `applis`, donc ignorée.
 *  Une appli déjà désactivée le reste : un profil ne réactive jamais rien sans que l'utilisateur le décide en détail. */
export function choixDuProfil(profil: Exclude<Profil, "perso">, applis: AppliSysteme[]): Record<string, boolean> {
  const desactiver = (a: AppliSysteme) =>
    profil === "recommande" ? desactiverParDefaut(a)
    : a.etat === "desactive" || (profil === "leger" ? a.groupe === "pistage" : !GARDEES_SIMPLE.includes(a.paquet));
  return Object.fromEntries(applis.map((a) => [a.paquet, desactiver(a)]));
}
