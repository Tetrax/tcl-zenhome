// Les consignes d'activation du débogage, affichées à l'écran « Préparez votre TV ou votre box » et dans les aides de la recherche et des erreurs.
// Elles ne servent QU'À guider : la sécurité repose sur le contrôle du modèle fait par le cœur à la connexion.
// TOUS les textes de consignes sont ici, à corriger au même endroit. À CONFIRMER sur une vraie TV : les menus varient selon le fabricant.

/** Chemins propres à un appareil vérifié. */
export interface PrecisionAppareil {
  titre: string;
  /** Où se trouve le « Build » à toucher 7 fois : chemin puis nom de la ligne. */
  debloquer: string; build: string;
  /** Où activer le débogage USB. */
  debogage: string;
}

export const CONSIGNES = {
  /** Chemin commun à tous les appareils : chemin puis nom(s) de la ligne « Build ». */
  debloquer: "Paramètres > Système (ou Préférences de l'appareil) > À propos", build: "« Build » ou « Build de l'OS Android TV »",
  debogage: "Options pour les développeurs > Débogage USB", variante: "« Options pour les concepteurs » sur certaines Mi Box",
  /** Où lire l'adresse IP. */
  adresseIp: "Paramètres > Réseau et Internet",
  /** Précisions affichées dans le bloc repliable. */
  precisions: [
    {
      titre: "Xiaomi Mi Box (4 ou S)",
      debloquer: "Paramètres > Préférences de l'appareil > À propos", build: "Build",
      debogage: "Préférences de l'appareil > Options pour les concepteurs > Débogage USB",
    },
    {
      titre: "TV TCL Google TV",
      debloquer: "Paramètres > Système > À propos", build: "Build de l'OS Android TV",
      debogage: "Paramètres > Système > Options pour les développeurs > Débogage USB",
    },
  ] satisfies PrecisionAppareil[],
  shield: "Sur une Nvidia Shield, activez « Débogage réseau » à la place du débogage USB.",
};

/** Mot pour l'appareil connecté (`BoxConnectee.appareil`) dans les textes : « TV » pour une TV, « appareil » pour un générique, « box » sinon. */
export const motConnecte = (appareil?: string | null): string => (appareil === "tcl-g08" ? "TV" : appareil === "generique" ? "appareil" : "box");

/** Mot avec son article défini : « la box », « la TV », « l'appareil ». */
export const defini = (mot: string): string => (mot === "appareil" ? "l'appareil" : `la ${mot}`);
