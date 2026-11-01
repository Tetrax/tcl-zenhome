// Les appareils proposés à l'écran « Quel appareil ? ». Ce choix ne sert QU'À afficher les bonnes consignes d'activation
// du débogage : la sécurité repose sur le contrôle du modèle fait par le cœur à la connexion.
// TOUS les textes de consignes sont ici, à corriger au même endroit. À CONFIRMER sur une vraie TV : tout ce qui concerne Google TV.

export type Famille = "mibox" | "googletv" | "autre";

export interface FicheAppareil {
  titre: string;
  /** Mot courant pour le désigner dans les textes. */
  mot: string;
  /** Où se trouve le « Build » à toucher 7 fois : chemin puis nom de la ligne. */
  debloquer: string; build: string;
  /** Où activer le débogage USB. */
  debogage: string;
  /** Où lire l'adresse IP. */
  adresseIp: string;
}

export const FICHES: Record<Exclude<Famille, "autre">, FicheAppareil> = {
  mibox: {
    titre: "Xiaomi Mi Box (4 ou S)", mot: "Mi Box",
    debloquer: "Paramètres > Préférences de l'appareil > À propos", build: "Build",
    debogage: "Préférences de l'appareil > Options pour les concepteurs > Débogage USB",
    adresseIp: "Paramètres > Préférences de l'appareil > À propos > État > Adresse IP",
  },
  googletv: {
    titre: "TV TCL Google TV", mot: "TV",
    debloquer: "Paramètres > Système > À propos", build: "Build de l'OS Android TV",
    debogage: "Paramètres > Système > Options pour les développeurs > Débogage USB",
    adresseIp: "Paramètres > Réseau et Internet, puis votre réseau Wi-Fi",
  },
};

/** Fiche de l'appareil choisi, ou null pour « Autre appareil ». */
export const fiche = (f: Famille): FicheAppareil | null => (f === "autre" ? null : FICHES[f]);
