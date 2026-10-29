// Palettes (une par écran / heure du jour) et cadrages de caméra.
// Les couleurs sont en sRGB brut (la gestion des couleurs de three est coupée dans scene.ts).

export type Step = "welcome" | "search" | "choose" | "done";
export const STEPS: Step[] = ["welcome", "search", "choose", "done"];

export interface Palette {
  // Couleurs
  skyTop: string; skyMid: string; skyHor: string; // dégradé du ciel
  haze: string;     // brume / perspective aérienne
  far: string;      // couleur d'une couche lointaine, avant brume
  near: string;     // couleur d'une couche proche
  accent: string;   // teinte de la végétation
  rim: string;      // liseré de lumière sur les crêtes
  sunCol: string;   // soleil (ou halo de lune)
  waterA: string; waterB: string; // eau proche de la rive / en profondeur
  // Scalaires (positions en coordonnées d'écran 0..1)
  sunX: number; sunY: number; sunSize: number; sunPow: number; halo: number;
  moonX: number; moonY: number; moonPow: number;
  stars: number; fog: number; mist: number; dust: number; fire: number;
  shadow: number; rimAmt: number; glitter: number; birds: number;
  focus: number;   // distance du plan net (unités monde)
}

export const COLOR_KEYS = [
  "skyTop", "skyMid", "skyHor", "haze", "far", "near", "accent", "rim", "sunCol", "waterA", "waterB",
] as const;
export const NUM_KEYS = [
  "sunX", "sunY", "sunSize", "sunPow", "halo", "moonX", "moonY", "moonPow",
  "stars", "fog", "mist", "dust", "fire", "shadow", "rimAmt", "glitter", "birds", "focus",
] as const;

export const PALETTES: Record<Step, Palette> = {
  // 1. Aube : lavande et rose pâle, brume dense
  welcome: {
    skyTop: "#8d84bf", skyMid: "#c7a8cf", skyHor: "#f7d6d0", haze: "#e6cfdf",
    far: "#b9a8d0", near: "#3a3262", accent: "#6b5b8c", rim: "#fff0e0", sunCol: "#fff3e6",
    waterA: "#ecd3df", waterB: "#8f80b6",
    sunX: 0.69, sunY: 0.72, sunSize: 0.04, sunPow: 1, halo: 0.2,
    moonX: 0.3, moonY: -0.3, moonPow: 0,
    stars: 0, fog: 0.019, mist: 1, dust: 0.5, fire: 0, shadow: 0, rimAmt: 0.45, glitter: 0.5, birds: 0.8,
    focus: 55,
  },
  // 2. Matin clair
  search: {
    skyTop: "#6aa5d8", skyMid: "#a8d1ea", skyHor: "#f4f1de", haze: "#d9e8ec",
    far: "#98bccf", near: "#1d4650", accent: "#3e7b4d", rim: "#fffbe0", sunCol: "#fffbe6",
    waterA: "#c4e4ee", waterB: "#4b8aa4",
    sunX: 0.66, sunY: 0.58, sunSize: 0.038, sunPow: 1, halo: 0.16,
    moonX: 0.3, moonY: -0.3, moonPow: 0,
    stars: 0, fog: 0.0095, mist: 0.45, dust: 0.85, fire: 0, shadow: 0.25, rimAmt: 0.6, glitter: 0.9, birds: 1,
    focus: 52,
  },
  // 3. Fin d'après-midi dorée, ombres longues
  choose: {
    skyTop: "#6d74ad", skyMid: "#e2a27c", skyHor: "#ffd28c", haze: "#f0b982",
    far: "#c98b7c", near: "#37223a", accent: "#7a5b2c", rim: "#ffcb8c", sunCol: "#ffe5b4",
    waterA: "#ffbb7a", waterB: "#7b4d5e",
    sunX: 0.7, sunY: 0.66, sunSize: 0.06, sunPow: 1, halo: 0.26,
    moonX: 0.3, moonY: -0.3, moonPow: 0,
    stars: 0, fog: 0.011, mist: 0.4, dust: 1, fire: 0, shadow: 1, rimAmt: 1, glitter: 1, birds: 0.7,
    focus: 36,
  },
  // 4. Nuit bleue
  done: {
    skyTop: "#04081c", skyMid: "#0d1a42", skyHor: "#2b4580", haze: "#1c3162",
    far: "#233b6c", near: "#050a1a", accent: "#0f2748", rim: "#a5bcff", sunCol: "#f1f4ff",
    waterA: "#12275a", waterB: "#050c28",
    sunX: 0.8, sunY: -0.25, sunSize: 0.04, sunPow: 0, halo: 0.2,
    moonX: 0.54, moonY: 0.87, moonPow: 1,
    stars: 1, fog: 0.012, mist: 0.4, dust: 0.3, fire: 1, shadow: 0, rimAmt: 0.5, glitter: 1, birds: 0,
    focus: 34,
  },
};

export interface Cam { pos: [number, number, number]; look: [number, number, number]; fov: number }

// Cadrage de chaque écran : le travelling avance dans le décor (z décroissant).
export const CAMS: Record<Step, Cam> = {
  welcome: { pos: [0, 3.4, 14], look: [0, 2.4, -40], fov: 36 },
  search: { pos: [-1.6, 2.9, 3], look: [1.5, 2.5, -45], fov: 33 },
  choose: { pos: [2.6, 2.4, -5], look: [3, 2.7, -40], fov: 31 },
  done: { pos: [0.5, 1.8, -13], look: [-1.5, 3.2, -50], fov: 34 },
};

// Mélange de deux couleurs hex (utilisé par le repli SVG).
export function mixHex(a: string, b: string, t: number): string {
  const p = (h: string, i: number) => parseInt(h.slice(1 + i * 2, 3 + i * 2), 16);
  const c = [0, 1, 2].map((i) => Math.round(p(a, i) + (p(b, i) - p(a, i)) * t));
  return "#" + c.map((v) => v.toString(16).padStart(2, "0")).join("");
}
