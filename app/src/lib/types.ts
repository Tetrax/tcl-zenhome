// Types du contrat entre l'interface et le cœur Rust (voir app/CONTRAT.md).

export interface EtatInitial { version: string; adb_installe: boolean; demo: boolean; sauvegardes_dir_existe: boolean; reprise_en_attente: boolean;
  /** Sauvegarde d'origine (faite avant Zen Home) la plus récente, ou null. `date` : « JJ/MM/AAAA HH:MM ». */
  origine: { date: string } | null;
  /** Box déjà configurées (la plus récente d'abord), à proposer tout de suite pendant la recherche. */
  boxes_memorisees: BoxMemorisee[];
  /** Vrai si l'appli embarque un jeton TMDB : les films marchent sans jeton personnel. */
  films_inclus: boolean }
export interface BoxMemorisee { nom: string; ip: string }
export interface BoxTrouvee { id: number; nom: string; ip: string }
export type Cible = { id: number } | { ip: string };
export interface BoxConnectee { nom: string; ip: string; modele: string }
export interface Ville { id: number; nom: string; region?: string; pays?: string }

export interface AppliSysteme {
  paquet: string;
  libelle: string;
  explication: string;
  recommande: "desactiver" | "garder";
  etat: "actif" | "desactive" | "garde";
  /** Facultatif (extension demandée au cœur) : sépare « pistage et publicité » des applis « au choix ». */
  groupe?: "pistage" | "option";
}
export interface AppliUtilisateur { paquet: string; libelle: string; taille: string; derniere_utilisation: string;
  /** Facultatif, ajout de plusieurs box : identifiants (IP) des box où l'appli est installée. */
  sur?: string[] }
export interface Inventaire {
  applis_systeme: AppliSysteme[];
  applis_utilisateur: AppliUtilisateur[];
  resume: { modele: string; android: string; nb_applis: number; nb_desactivees: number };
}
export interface Choix { garder: string[]; desactiver: string[]; desinstaller: string[] }
export interface RapportApplication { desactivees: number; gardees: number; desinstallees: string[]; echecs: string[] }
export type EtatLigne = "ok" | "ko" | "avert";
export interface LigneVerif { etat: EtatLigne; controle: string; detail: string }
export interface Verification { lignes: LigneVerif[]; ko: number }
export interface RapportRestauration { incomplete: boolean; avertissements: string[]; depuis_sauvegarde: string | null }

export type CodeErreur =
  | "occupe" | "annule" | "adb_absent" | "refus_licence" | "injoignable" | "non_autorise"
  | "autre_modele" | "autre_box" | "lecture" | "choix_invalide" | "reseau" | "interne";
export interface ErreurZen { code: CodeErreur; message: string }

export type Evenement =
  | { type: "etape"; nom: string }
  | { type: "progression"; fraction: number }
  | { type: "connexion_perdue"; essai: number }
  | { type: "reconnectee" }
  | { type: "attente_utilisateur"; demande: number; raison: "injoignable" | "autorisation" }
  | { type: "avertissement"; texte: string };
export type SurEvenement = (e: Evenement) => void;
export type Decision = "reessayer" | "arreter";
