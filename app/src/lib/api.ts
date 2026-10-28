// Seul point d'entrée vers le backend Rust. Hors Tauri (navigateur), renvoie des données de démo.
export interface PingInfo {
  version: string;
  os: string;
  arch: string;
}

const inTauri = (): boolean => "__TAURI_INTERNALS__" in window;

export const isDemo = (): boolean => !inTauri();

async function call<T>(cmd: string, demo: T, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri()) return demo;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export const ping = (): Promise<PingInfo> =>
  call("ping", { version: "0.0.0-demo", os: "navigateur", arch: "n/a" });

export interface MiBox {
  nom: string;
  ip: string;
}

export interface ResultatAdb {
  commande: string;
  code: number | null;
  stdout: string;
  stderr: string;
  delai_depasse: boolean;
}

export interface Rapport {
  annonces_googlecast: number;
  mi_box: MiBox[];
  ip_ciblee: string | null;
  adb_present: boolean;
  adb: ResultatAdb[];
  erreurs: string[];
}

export const testReseau = (ipManuelle: string): Promise<Rapport> =>
  call(
    "test_reseau",
    { annonces_googlecast: 0, mi_box: [], ip_ciblee: null, adb_present: false, adb: [], erreurs: ["mode démo"] },
    { ipManuelle: ipManuelle.trim() || null },
  );
