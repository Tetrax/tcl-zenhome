// Point d'entrée de la scène 3D. La 3D n'est qu'un décor : si WebGL2 manque, est trop lent
// (failIfMajorPerformanceCaveat) ou se perd, on renvoie null / on prévient, et l'interface passe au repli CSS.
import type { Clairiere } from "./clairiere";

export type { Clairiere, Moment, Signal } from "./clairiere";

/** ?gl=off force le repli CSS ; ?gl=force ignore le test de performance (utile en machine virtuelle ou en test). */
const modeGl = () => new URLSearchParams(location.search).get("gl");

export async function creerFond(canvas: HTMLCanvasElement, reduced: boolean, onPerdu: (perdu: boolean) => void): Promise<Clairiere | null> {
  const mode = modeGl();
  if (mode === "off") return null;
  const gl = canvas.getContext("webgl2", {
    failIfMajorPerformanceCaveat: mode !== "force", antialias: true, alpha: false, powerPreference: "high-performance",
  }) as WebGL2RenderingContext | null;
  if (!gl) return null;
  try {
    const { creerScene } = await import("./clairiere"); // three.js n'est chargé que si la 3D est possible
    return creerScene(canvas, gl, reduced, onPerdu);
  } catch (e) {
    console.warn("Scène 3D indisponible, repli CSS :", e);
    return null;
  }
}
