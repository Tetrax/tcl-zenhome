// Libellés humains du tableau final : les noms techniques de la vérification ne s'affichent qu'en détail.
import type { EtatLigne, LigneVerif } from "./types";

const FIXES: Record<string, string> = {
  "Zen Home installé": "Zen Home est installé sur la box",
  "Accessibilité : KeyService": "Les raccourcis de la télécommande fonctionnent",
  "Écran de veille = ZenDream": "L'écran de veille de Zen Home est actif",
  "Stats d'usage (Applis ouvertes)": "Zen Home peut voir vos applis récentes",
  "Spotify : arrière-plan restreint": "Spotify est limité en arrière-plan",
  "Wi-Fi non lié à setupwraith": "Le Wi-Fi ne dépend pas de l'assistant de configuration",
};
const PAQUETS: Record<string, string> = {
  "com.google.android.tvlauncher": "L'ancien accueil Google est désactivé",
  "com.google.android.tungsten.setupwraith": "L'assistant de configuration Google est désactivé",
};

export const libelleEtat: Record<EtatLigne, string> = { ok: "En ordre", avert: "À noter", ko: "À regarder" };

export interface LigneHumaine { etat: EtatLigne; libelle: string; detail: string; brut: string; appli: boolean }

/** `noms` : paquet -> libellé lisible (fourni par l'inventaire). */
export function humaniser(l: LigneVerif, noms: Map<string, string>): LigneHumaine {
  const brut = `${l.controle}${l.detail ? " — " + l.detail : ""}`;
  let libelle = FIXES[l.controle];
  let appli = false;
  if (!libelle && l.controle.startsWith("Rôle HOME")) libelle = "Zen Home est l'accueil de la TV";
  if (!libelle && l.controle.startsWith("Désactivé : ")) {
    const p = l.controle.slice("Désactivé : ".length);
    libelle = PAQUETS[p];
    if (!libelle) { libelle = `${noms.get(p) ?? p} est désactivée`; appli = true; }
  }
  let detail = l.detail;
  if (detail.startsWith("gardé actif (votre choix)")) detail = "Gardée active, comme vous l'avez choisi";
  else if (detail === "absent") detail = "Absent de la box";
  else if (detail.startsWith("version ")) detail = "Version " + detail.slice(8);
  return { etat: l.etat, libelle: libelle ?? l.controle, detail, brut, appli };
}

/** État d'une box dans un lot de plusieurs box. */
export const libelleBox = { attente: "En attente", "en-cours": "En cours", ok: "OK", echec: "Échec", "non-traitee": "Non traitée" } as const;
