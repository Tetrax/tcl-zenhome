// Seul point d'entrée vers le backend Rust. Hors Tauri (navigateur), renvoie des données de démo.
export interface PingInfo {
  version: string;
  os: string;
  arch: string;
}

const inTauri = (): boolean => "__TAURI_INTERNALS__" in window;

export const isDemo = (): boolean => !inTauri();

async function call<T>(cmd: string, demo: T): Promise<T> {
  if (!inTauri()) return demo;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd);
}

export const ping = (): Promise<PingInfo> =>
  call("ping", { version: "0.0.0-demo", os: "navigateur", arch: "n/a" });
