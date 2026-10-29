// Scène 3D « Clairière vivante » : herbe instanciée, lisières d'arbres, ciel, rayons, particules.
// Module isolé : il ne connaît rien de l'interface. On le pilote par un « moment » de la journée
// et par des signaux (recherche, box trouvée, progression, succès). Tout est procédural, sans ressource externe.
import * as THREE from "three";
import { mergeGeometries, mergeVertices } from "three/examples/jsm/utils/BufferGeometryUtils.js";
import * as S from "./shaders";

// ---- Réglages de performance (tout est ici) ----
export const CFG = {
  GRASS_COUNT: 110_000, // brins d'herbe instanciés (le plus coûteux)
  CONIFERS: 380, // sapins en lisière
  ROUND_TREES: 120, // arbres feuillus
  RAY_COUNT: 12, // plans de rayons de lumière
  POLLEN_COUNT: 900,
  FIREFLY_COUNT: 140,
  GROUND_SEGMENTS: 200,
  MAX_PIXEL_RATIO: 2, // plafond du pixel ratio
  MIN_PIXEL_RATIO: 1, // plancher de la qualité dynamique
  ADAPTIVE: true, // baisse le pixel ratio si < 45 i/s pendant 2 s
};

THREE.ColorManagement.enabled = false; // sRGB brut : couleurs choisies à l'œil

export type Moment = "aube" | "matin" | "jour" | "doree" | "crepuscule" | "nuit";
export type Signal = "aucun" | "recherche" | "trouvee" | "repos";

// ---- Looks par moment ----
interface Look {
  top: THREE.Color; mid: THREE.Color; horizon: THREE.Color; fog: THREE.Color; glow: THREE.Color;
  key: THREE.Color; ambSky: THREE.Color; ambGround: THREE.Color;
  grassBase: THREE.Color; grassTip: THREE.Color;
  sunDir: THREE.Vector3; keyDir: THREE.Vector3;
  camPos: THREE.Vector3; camTarget: THREE.Vector3;
  fov: number; fogDensity: number; mist: number; sunDisc: number; moon: number;
  stars: number; clouds: number; rays: number; pollen: number; fireflies: number;
}
const C = (r: number, g: number, b: number, k = 1) => new THREE.Color(r * k, g * k, b * k);
const V = (x: number, y: number, z: number) => new THREE.Vector3(x, y, z);
const dirAE = (az: number, el: number) => {
  const a = (az * Math.PI) / 180, e = (el * Math.PI) / 180;
  return V(Math.sin(a) * Math.cos(e), Math.sin(e), -Math.cos(a) * Math.cos(e)).normalize();
};
const MOON = V(0.26, 0.42, -0.87).normalize(); // à droite, du côté libre de l'écran (le panneau est à gauche)

const LOOKS: Record<Moment, Look> = {
  aube: {
    top: C(0.2, 0.3, 0.56), mid: C(0.55, 0.58, 0.82), horizon: C(0.82, 0.76, 0.9), fog: C(0.82, 0.76, 0.9), glow: C(1.0, 0.6, 0.58),
    key: C(1.0, 0.7, 0.64, 0.85), ambSky: C(0.4, 0.48, 0.74, 0.95), ambGround: C(0.24, 0.27, 0.38),
    grassBase: C(0.1, 0.22, 0.24), grassTip: C(0.42, 0.64, 0.56),
    sunDir: dirAE(-28, 3), keyDir: dirAE(-28, 3), camPos: V(0, 3.4, 27), camTarget: V(1.5, 2.9, -12), fov: 58,
    fogDensity: 0.022, mist: 0.6, sunDisc: 1, moon: 0, stars: 0.06, clouds: 0.5, rays: 0.4, pollen: 0.2, fireflies: 0,
  },
  matin: {
    top: C(0.26, 0.5, 0.84), mid: C(0.6, 0.76, 0.92), horizon: C(0.92, 0.88, 0.82), fog: C(0.92, 0.88, 0.82), glow: C(1.0, 0.82, 0.58, 0.8),
    key: C(1.0, 0.88, 0.7, 0.95), ambSky: C(0.5, 0.62, 0.85, 0.85), ambGround: C(0.27, 0.33, 0.27),
    grassBase: C(0.1, 0.28, 0.17), grassTip: C(0.46, 0.7, 0.34),
    sunDir: dirAE(-14, 20), keyDir: dirAE(-14, 20), camPos: V(0, 2.9, 17), camTarget: V(0.8, 2.4, -13), fov: 55,
    fogDensity: 0.016, mist: 0.4, sunDisc: 1, moon: 0, stars: 0, clouds: 0.7, rays: 0.65, pollen: 0.35, fireflies: 0,
  },
  jour: {
    top: C(0.24, 0.55, 0.9), mid: C(0.52, 0.76, 0.95), horizon: C(0.8, 0.9, 0.95), fog: C(0.8, 0.9, 0.95), glow: C(1.0, 0.95, 0.8, 0.8),
    key: C(1.0, 0.95, 0.82, 1.0), ambSky: C(0.55, 0.7, 0.9, 0.8), ambGround: C(0.3, 0.38, 0.26),
    grassBase: C(0.1, 0.32, 0.14), grassTip: C(0.5, 0.76, 0.28),
    sunDir: dirAE(32, 50), keyDir: dirAE(32, 50), camPos: V(0, 2.5, 15), camTarget: V(0, 2.1, -14), fov: 52,
    fogDensity: 0.011, mist: 0.15, sunDisc: 1, moon: 0, stars: 0, clouds: 0.9, rays: 0.25, pollen: 0.5, fireflies: 0,
  },
  // Heure dorée : lumière chaude mais ombres froides ; l'herbe et les arbres gardent leur vert.
  doree: {
    top: C(0.28, 0.4, 0.7), mid: C(0.82, 0.68, 0.64), horizon: C(0.98, 0.76, 0.56), fog: C(0.9, 0.72, 0.6), glow: C(1.0, 0.58, 0.24),
    key: C(1.0, 0.73, 0.46, 1.0), ambSky: C(0.5, 0.56, 0.78, 0.8), ambGround: C(0.3, 0.27, 0.24),
    grassBase: C(0.1, 0.27, 0.12), grassTip: C(0.6, 0.7, 0.26),
    sunDir: dirAE(24, 8), keyDir: dirAE(24, 8), camPos: V(-2, 1.6, 7), camTarget: V(3, 2.0, -16), fov: 50,
    fogDensity: 0.014, mist: 0.25, sunDisc: 1, moon: 0, stars: 0, clouds: 0.8, rays: 1.0, pollen: 0.95, fireflies: 0.1,
  },
  // Crépuscule : bande orangée à l'horizon qui s'éteint en bleu-violet profond, sans rose bonbon.
  crepuscule: {
    top: C(0.07, 0.1, 0.28), mid: C(0.27, 0.27, 0.48), horizon: C(0.74, 0.46, 0.34), fog: C(0.4, 0.35, 0.47), glow: C(0.98, 0.5, 0.22, 0.8),
    key: C(0.86, 0.5, 0.36, 0.55), ambSky: C(0.26, 0.3, 0.52, 0.8), ambGround: C(0.14, 0.14, 0.24),
    grassBase: C(0.07, 0.13, 0.15), grassTip: C(0.34, 0.42, 0.42),
    sunDir: dirAE(24, -3), keyDir: dirAE(24, -3), camPos: V(0, 1.4, 4), camTarget: V(2, 3.2, -14), fov: 52,
    fogDensity: 0.019, mist: 0.4, sunDisc: 0.5, moon: 0.2, stars: 0.55, clouds: 0.5, rays: 0.35, pollen: 0.2, fireflies: 0.55,
  },
  nuit: {
    top: C(0.02, 0.04, 0.13), mid: C(0.04, 0.08, 0.21), horizon: C(0.09, 0.14, 0.29), fog: C(0.09, 0.14, 0.29), glow: C(0.2, 0.3, 0.6, 0.35),
    key: C(0.55, 0.68, 1.0, 0.55), ambSky: C(0.15, 0.21, 0.42), ambGround: C(0.05, 0.07, 0.14),
    grassBase: C(0.03, 0.07, 0.11), grassTip: C(0.18, 0.3, 0.4),
    sunDir: dirAE(24, -14), keyDir: MOON.clone(), camPos: V(0, 1.3, 3), camTarget: V(0, 5.4, -14), fov: 60,
    fogDensity: 0.015, mist: 0.35, sunDisc: 0, moon: 1, stars: 1, clouds: 0.2, rays: 0.28, pollen: 0.15, fireflies: 1,
  },
};

const cloneLook = (l: Look): Look => {
  const o: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(l)) o[k] = typeof v === "number" ? v : (v as THREE.Color).clone();
  return o as unknown as Look;
};
function blend(out: Look, a: Look, b: Look, t: number): void {
  const o = out as unknown as Record<string, unknown>, A = a as unknown as Record<string, unknown>, B = b as unknown as Record<string, unknown>;
  for (const k of Object.keys(o)) {
    const x = A[k], y = B[k];
    if (typeof x === "number") o[k] = x + ((y as number) - x) * t;
    else (o[k] as THREE.Color).copy(x as THREE.Color).lerp(y as THREE.Color, t);
  }
  out.sunDir.normalize(); out.keyDir.normalize();
}
const ease = (p: number) => (p < 0.5 ? 4 * p * p * p : 1 - Math.pow(-2 * p + 2, 3) / 2);
const clamp01 = (x: number) => Math.min(1, Math.max(0, x));
const smooth = (a: number, b: number, x: number) => { const t = clamp01((x - a) / (b - a)); return t * t * (3 - 2 * t); };

// ---- Terrain ----
const BEACON = { x: 0, z: -6 };
export function groundH(x: number, z: number): number {
  const r = Math.hypot(x, z + 10);
  const bowl = smooth(34, 72, r) * 10; // cuvette qui enserre la clairière
  const rolling = Math.sin(x * 0.09 + 1.3) * Math.cos(z * 0.07) * 0.55 + Math.sin(x * 0.21 - z * 0.17) * 0.18 + Math.sin(z * 0.045 + x * 0.03) * 0.9;
  const flat = smooth(2, 14, Math.hypot(x - BEACON.x, z - BEACON.z));
  return rolling * (0.35 + 0.65 * flat) + bowl;
}
function rng(seed: number) { // mulberry32 : décor identique à chaque lancement
  return () => { seed |= 0; seed = (seed + 0x6d2b79f5) | 0; let t = Math.imul(seed ^ (seed >>> 15), 1 | seed); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}

// ---- Géométries ----
function bladeGeometry(): THREE.BufferGeometry {
  const rows = [0, 0.25, 0.5, 0.75], pos: number[] = [], idx: number[] = [];
  for (const y of rows) { const w = 0.5 * (1 - Math.pow(y, 1.3) * 0.9); pos.push(-w, y, 0, w, y, 0); }
  pos.push(0, 1, 0);
  for (let i = 0; i < 3; i++) { const a = i * 2; idx.push(a, a + 1, a + 2, a + 1, a + 3, a + 2); }
  idx.push(6, 7, 8);
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.Float32BufferAttribute(pos, 3));
  g.setIndex(idx);
  return g;
}

type RGB = [number, number, number];
/** Prépare une pièce d'arbre : indexée, normales lissées, couleur de base (aCol) et drapeau feuillage (aLeaf). */
function piece(g: THREE.BufferGeometry, col: RGB, leaf: number, shade?: (x: number, y: number, z: number) => number): THREE.BufferGeometry {
  g.deleteAttribute("normal"); g.deleteAttribute("uv");
  const m = g.index ? g : mergeVertices(g);
  m.computeVertexNormals();
  const p = m.getAttribute("position"), n = p.count, c = new Float32Array(n * 3), l = new Float32Array(n).fill(leaf);
  for (let i = 0; i < n; i++) {
    const k = shade ? shade(p.getX(i), p.getY(i), p.getZ(i)) : 1;
    c[i * 3] = col[0] * k; c[i * 3 + 1] = col[1] * k; c[i * 3 + 2] = col[2] * k;
  }
  m.setAttribute("aCol", new THREE.BufferAttribute(c, 3));
  m.setAttribute("aLeaf", new THREE.BufferAttribute(l, 1));
  return m;
}
const TRUNK: RGB = [0.2, 0.14, 0.1];
const bump = (x: number, y: number, z: number, a: number) =>
  Math.sin(x * 3.1 + a) * Math.sin(y * 2.7 + a * 1.7) * Math.sin(z * 3.3 - a * 0.6);

/** Jupe de branches : cône à bord déchiqueté qui retombe, avec un anneau intermédiaire pour la courbure. */
function skirt(y: number, h: number, r: number, r0: () => number, droop: number): THREE.BufferGeometry {
  const N = 13, pos: number[] = [0, y + h, 0], idx: number[] = [];
  const a0 = r0() * 6.28;
  for (let i = 0; i < N; i++) { // anneau intermédiaire
    const a = a0 + (i / N) * 6.2832, k = 0.5 * (0.9 + r0() * 0.2);
    pos.push(Math.cos(a) * r * k, y + h * 0.5 - droop * 0.3, Math.sin(a) * r * k);
  }
  for (let i = 0; i < N; i++) { // pointes basses : une sur deux plus longue, longueur au hasard
    const a = a0 + ((i + 0.0) / N) * 6.2832, k = (i % 2 ? 0.66 : 1.05) * (0.85 + r0() * 0.3);
    pos.push(Math.cos(a) * r * k, y - droop * (i % 2 ? 0.2 : 0.7) - r0() * droop * 0.3, Math.sin(a) * r * k);
  }
  for (let i = 0; i < N; i++) {
    const j = (i + 1) % N;
    idx.push(0, 1 + j, 1 + i);                          // haut -> milieu
    idx.push(1 + i, 1 + j, 1 + N + i, 1 + j, 1 + N + j, 1 + N + i); // milieu -> pointes
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.Float32BufferAttribute(pos, 3));
  g.setIndex(idx);
  return g;
}
/** Sapin : tronc, 8 jupes superposées (rayon décroissant), chacune tournée, inclinée et teintée un peu différemment. */
function coniferGeometry(seed: number, slender: number): THREE.BufferGeometry {
  const r = rng(seed), parts = [piece(new THREE.CylinderGeometry(0.018, 0.04, 0.3, 6).translate(0, 0.15, 0), TRUNK, 0)];
  const tiers = 8;
  for (let i = 0; i < tiers; i++) {
    const t = i / (tiers - 1), y = 0.1 + t * 0.78, h = 0.2 - t * 0.05 + r() * 0.02;
    const rad = (0.34 * Math.pow(1 - t, 0.9) + 0.035) * slender * (0.92 + r() * 0.16);
    const g = skirt(y, h, rad, r, 0.06 * (1 - t * 0.6));
    g.rotateY(r() * 6.28); g.rotateZ((r() - 0.5) * 0.08); g.rotateX((r() - 0.5) * 0.08);
    const g1 = 0.9 + r() * 0.2;
    parts.push(piece(g, [0.085 * g1, 0.24 * g1, 0.19 * g1], 1, (x, yy, z) => {
      const d = Math.hypot(x, z) / Math.max(rad, 0.001); // pointes claires, cœur sombre
      return 0.6 + 0.5 * Math.min(d, 1.2) + 0.15 * t;
    }));
  }
  return mergeGeometries(parts)!;
}
/** Feuillu : tronc fourchu, canopée faite de lobes bosselés (rayon modulé par un bruit lisse), dessous plus sombre. */
function roundTreeGeometry(seed: number, wide: number): THREE.BufferGeometry {
  const r = rng(seed), parts = [piece(new THREE.CylinderGeometry(0.02, 0.045, 0.55, 6).translate(0, 0.27, 0).rotateZ((r() - 0.5) * 0.12), TRUNK, 0)];
  for (let i = 0; i < 2; i++) { // deux charpentières
    const a = r() * 6.28;
    parts.push(piece(new THREE.CylinderGeometry(0.008, 0.016, 0.3, 5).translate(0, 0.15, 0).rotateZ(0.7).rotateY(a).translate(0, 0.42, 0), TRUNK, 0));
  }
  const lobes = 8 + Math.floor(r() * 3), g0 = 0.85 + r() * 0.3;
  for (let i = 0; i < lobes; i++) {
    const a = r() * 6.28, rr = Math.sqrt(r()) * 0.24 * wide, cy = 0.62 + r() * 0.3 - rr * 0.35, s = 0.15 + r() * 0.1;
    const cx = Math.cos(a) * rr, cz = Math.sin(a) * rr, ph = r() * 20, amp = 0.2 + r() * 0.12;
    const g = new THREE.IcosahedronGeometry(s, 2), p = g.getAttribute("position");
    for (let k = 0; k < p.count; k++) { // les sommets confondus reçoivent le même déplacement : pas de couture
      const dx = p.getX(k) / s, dy = p.getY(k) / s, dz = p.getZ(k) / s;
      const f = 1 + amp * bump(dx, dy, dz, ph) + 0.08 * bump(dx * 2.3, dy * 2.3, dz * 2.3, ph + 5);
      p.setXYZ(k, p.getX(k) * f * wide * 0.9, p.getY(k) * f * 0.85, p.getZ(k) * f * wide * 0.9);
    }
    const tint = 0.9 + r() * 0.2;
    parts.push(piece(g.translate(cx, cy, cz), [0.16 * tint, 0.31 * tint, 0.14 * tint * g0], 1, (_x, y) => 0.55 + 0.7 * smooth(cy - s, cy + s, y)));
  }
  return mergeGeometries(parts)!;
}

// ---- Particules (pollen / lucioles) ----
function makeSprites(count: number, box: THREE.Vector3, min: THREE.Vector3, o: { size: number; wander: number; blink: number; drift: THREE.Vector3; color: THREE.Color }, U: Record<string, THREE.IUniform>) {
  const r = rng(count + 5), seed = new Float32Array(count * 4);
  for (let i = 0; i < seed.length; i++) seed[i] = r();
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.BufferAttribute(new Float32Array(count * 3), 3));
  g.setAttribute("aSeed", new THREE.BufferAttribute(seed, 4));
  const amount = { value: 0 };
  const m = new THREE.ShaderMaterial({
    vertexShader: S.SPRITE_VERT, fragmentShader: S.SPRITE_FRAG, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending,
    uniforms: { ...U, uBoxMin: { value: min }, uBoxSize: { value: box }, uDrift: { value: o.drift }, uSize: { value: o.size }, uWander: { value: o.wander }, uBlink: { value: o.blink }, uColor: { value: o.color }, uAmount: amount },
  });
  const pts = new THREE.Points(g, m); pts.frustumCulled = false;
  return { pts, amount };
}

export interface Clairiere {
  /** Fond vers un moment (durée et délai en secondes). */
  allerA(m: Moment, o?: { duree?: number; delai?: number }): void;
  /** Saute sans transition. */
  poser(m: Moment): void;
  setSignal(s: Signal): void;
  /** Pilote la lumière par une progression réelle : de `de` vers `vers`, f entre 0 et 1 (null = retour aux transitions minutées). */
  setProgression(f: number | null, de?: Moment, vers?: Moment): void;
  /** Onde de lumière (succès). */
  impulsion(): void;
  dispose(): void;
  stats(): string;
}

/** Crée la scène sur un canvas dont le contexte WebGL2 est déjà obtenu. */
export function creerScene(canvas: HTMLCanvasElement, gl: WebGL2RenderingContext, reduced: boolean, onLost: (perdu: boolean) => void): Clairiere {
  const renderer = new THREE.WebGLRenderer({ canvas, context: gl, antialias: true, powerPreference: "high-performance" });
  let pr = Math.min(window.devicePixelRatio || 1, CFG.MAX_PIXEL_RATIO);
  renderer.setPixelRatio(pr);
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(55, 1, 0.1, 600);

  // Uniforms partagés par tous les matériaux (une seule mise à jour par image).
  const U = {
    uTime: { value: 0 }, uWind: { value: reduced ? 0.3 : 1 }, uClouds: { value: 0 }, uMist: { value: 0 }, uFogDensity: { value: 0.02 },
    uFogColor: { value: new THREE.Color() }, uGlow: { value: new THREE.Color() }, uKeyDir: { value: V(0, 1, 0) }, uKey: { value: new THREE.Color() },
    uAmbSky: { value: new THREE.Color() }, uAmbGround: { value: new THREE.Color() }, uGrassBase: { value: new THREE.Color() }, uGrassTip: { value: new THREE.Color() },
    uScale: { value: 800 },
  };

  // Ciel
  const skyMat = new THREE.ShaderMaterial({
    vertexShader: S.SKY_VERT, fragmentShader: S.SKY_FRAG, side: THREE.BackSide, depthTest: false, depthWrite: false,
    uniforms: { ...U, uTop: { value: new THREE.Color() }, uMid: { value: new THREE.Color() }, uHorizon: { value: new THREE.Color() }, uSunDir: { value: V(0, 0, -1) }, uSunDisc: { value: 1 }, uMoon: { value: 0 }, uStars: { value: 0 } },
  });
  const sky = new THREE.Mesh(new THREE.SphereGeometry(300, 32, 16), skyMat);
  sky.renderOrder = -1000; sky.frustumCulled = false; scene.add(sky);

  // Sol : maillage vallonné, normales calculées une fois.
  const SZ = 280, seg = CFG.GROUND_SEGMENTS;
  const gg = new THREE.PlaneGeometry(SZ, SZ, seg, seg); gg.rotateX(-Math.PI / 2);
  const gp = gg.getAttribute("position");
  for (let i = 0; i < gp.count; i++) gp.setY(i, groundH(gp.getX(i), gp.getZ(i)));
  gg.computeVertexNormals();
  scene.add(new THREE.Mesh(gg, new THREE.ShaderMaterial({ vertexShader: S.GROUND_VERT, fragmentShader: S.GROUND_FRAG, uniforms: { ...U } })));

  // Herbe : une lame de 9 sommets, CFG.GRASS_COUNT instances.
  const grassU = { uMouse: { value: V(0, -999, 0) }, uMouseK: { value: 0 }, uPulse: { value: new THREE.Vector4(BEACON.x, BEACON.z, 99, 0) } };
  {
    const base = bladeGeometry(), geo = new THREE.InstancedBufferGeometry();
    geo.index = base.index; geo.setAttribute("position", base.getAttribute("position"));
    const n = CFG.GRASS_COUNT, off = new Float32Array(n * 3), par = new Float32Array(n * 4), r = rng(1);
    for (let i = 0; i < n; i++) {
      const x = (r() * 2 - 1) * 36, z = -48 + r() * 78, y = groundH(x, z);
      const dB = Math.hypot(x - BEACON.x, z - BEACON.z);
      const hf = 0.3 + 0.7 * smooth(1.5, 11, dB); // herbe rase autour du point de la box
      off.set([x, y - 0.02, z], i * 3);
      par.set([(0.32 + r() * 0.5) * hf * (0.8 + 0.5 * Math.sin(x * 0.12) * Math.sin(z * 0.1)), 0.028 + r() * 0.03, r() * 100, r() * Math.PI * 2], i * 4);
    }
    geo.setAttribute("aOffset", new THREE.InstancedBufferAttribute(off, 3));
    geo.setAttribute("aParams", new THREE.InstancedBufferAttribute(par, 4));
    geo.instanceCount = n;
    const mesh = new THREE.Mesh(geo, new THREE.ShaderMaterial({ vertexShader: S.GRASS_VERT, fragmentShader: S.GRASS_FRAG, side: THREE.DoubleSide, uniforms: { ...U, ...grassU } }));
    mesh.frustumCulled = false; scene.add(mesh);
  }

  // Arbres en lisière : plusieurs variantes de silhouette, clairsemés par grappes, jamais sur le trajet de la caméra.
  {
    const mat = new THREE.ShaderMaterial({ vertexShader: S.TREE_VERT, fragmentShader: S.TREE_FRAG, side: THREE.DoubleSide, uniforms: { ...U } });
    const r = rng(42), dummy = new THREE.Object3D();
    const place = (variants: THREE.BufferGeometry[], count: number, hMin: number, hMax: number, wRatio: number) => {
      const lots: { mats: THREE.Matrix4[]; hues: number[] }[] = variants.map(() => ({ mats: [], hues: [] }));
      let total = 0, guard = 0;
      while (total < count && guard++ < count * 40) {
        const a = r() * Math.PI * 2, rad = 38 + 8 * Math.sin(a * 5) + Math.pow(r(), 1.3) * 80;
        const x = Math.sin(a) * rad, z = -10 - Math.cos(a) * rad;
        const clump = 0.5 + 0.5 * Math.sin(a * 7 + 1) * Math.sin(a * 3.1 + 2);
        if (r() > 0.3 + 0.7 * clump || z > 18 || Math.hypot(x, z - 26) < 16) continue;
        const h = hMin + r() * (hMax - hMin) * (0.7 + rad / 200);
        dummy.position.set(x, groundH(x, z) - 0.3, z);
        dummy.rotation.set((r() - 0.5) * 0.06, r() * 6.28, (r() - 0.5) * 0.06);
        dummy.scale.set(h * wRatio * (0.8 + r() * 0.4), h, h * wRatio * (0.8 + r() * 0.4));
        dummy.updateMatrix();
        const lot = lots[total++ % variants.length];
        lot.mats.push(dummy.matrix.clone()); lot.hues.push(r());
      }
      variants.forEach((geo, v) => {
        const { mats, hues } = lots[v];
        const g = geo.clone(); // un attribut d'instance par variante
        g.setAttribute("iHue", new THREE.InstancedBufferAttribute(new Float32Array(hues), 1));
        const im = new THREE.InstancedMesh(g, mat, mats.length);
        mats.forEach((m, i) => im.setMatrixAt(i, m));
        im.frustumCulled = false; scene.add(im);
      });
    };
    place([coniferGeometry(11, 1), coniferGeometry(23, 0.78), coniferGeometry(37, 1.15)], CFG.CONIFERS, 8, 15, 1);
    place([roundTreeGeometry(5, 1), roundTreeGeometry(17, 1.25), roundTreeGeometry(29, 0.85)], CFG.ROUND_TREES, 6, 11, 1);
  }

  // Rayons de lumière : plans de base 1x1 (y de 0 à 1), orientés vers la source chaque image.
  const rayMat = new THREE.ShaderMaterial({
    vertexShader: S.RAY_VERT, fragmentShader: S.RAY_FRAG, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, side: THREE.DoubleSide,
    uniforms: { ...U, uRays: { value: 0 } },
  });
  const rayGeo = new THREE.PlaneGeometry(1, 1).translate(0, 0.5, 0);
  const rays: { m: THREE.Mesh; len: number }[] = [];
  {
    const r = rng(9);
    for (let i = 0; i < CFG.RAY_COUNT; i++) {
      const m = new THREE.Mesh(rayGeo, rayMat); m.frustumCulled = false;
      const x = (r() * 2 - 1) * 28, z = -34 + r() * 40, len = 28 + r() * 22, w = 4 + r() * 7;
      m.position.set(x, groundH(x, z), z);
      rays.push({ m, len }); m.userData.w = w; scene.add(m);
    }
  }

  // Particules : pollen de jour, lucioles de nuit.
  const pollen = makeSprites(CFG.POLLEN_COUNT, V(46, 5.5, 62), V(-23, 0.3, -38), { size: 0.045, wander: 0.6, blink: 0, drift: V(0.5, 0.05, 0.2), color: new THREE.Color(1, 0.93, 0.7) }, U);
  const fire = makeSprites(CFG.FIREFLY_COUNT, V(34, 3, 38), V(-17, 0.5, -30), { size: 0.2, wander: 2.2, blink: 1, drift: V(0, 0, 0), color: new THREE.Color(0.8, 1, 0.45) }, U);
  scene.add(pollen.pts, fire.pts);

  // Signal : disque d'ondes au sol + orbe (luciole-guide, puis point de la box).
  const beaconY = groundH(BEACON.x, BEACON.z);
  const waveU = { uSearch: { value: 0 }, uBurst: { value: 0 }, uSpot: { value: 0 }, uColor: { value: new THREE.Color(0.62, 0.92, 1) } };
  // Disque d'ondes qui épouse le relief (sinon il flotte au-dessus des creux et forme une nappe pâle).
  const waveGeo = new THREE.RingGeometry(0.001, 24, 96, 36);
  {
    const p = waveGeo.getAttribute("position");
    for (let i = 0; i < p.count; i++) p.setZ(i, groundH(BEACON.x + p.getX(i), BEACON.z - p.getY(i)) - beaconY);
  }
  const wave = new THREE.Mesh(waveGeo, new THREE.ShaderMaterial({
    vertexShader: S.WAVE_VERT, fragmentShader: S.WAVE_FRAG, transparent: true, depthWrite: false, uniforms: { ...U, ...waveU },
  }));
  wave.rotation.x = -Math.PI / 2; wave.position.set(BEACON.x, beaconY + 0.3, BEACON.z); wave.frustumCulled = false; scene.add(wave);
  const orbG = new THREE.BufferGeometry(); orbG.setAttribute("position", new THREE.BufferAttribute(new Float32Array(3), 3));
  const orbU = { uLevel: { value: 0 }, uSizeM: { value: 1.6 }, uColor: { value: new THREE.Color(0.75, 0.95, 1) } };
  const orb = new THREE.Points(orbG, new THREE.ShaderMaterial({
    vertexShader: S.ORB_VERT, fragmentShader: S.ORB_FRAG, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, uniforms: { uTime: U.uTime, uScale: U.uScale, ...orbU },
  }));
  orb.frustumCulled = false; orb.visible = false; scene.add(orb);

  // ---- État : transitions entre looks ----
  let cur = cloneLook(LOOKS.aube), from = cloneLook(cur), to = LOOKS.aube;
  let t0 = 0, dur = 0.001, arcOn = false, time = 0;
  // Mode « progression » : la lumière suit un avancement réel (installation) au lieu d'un minuteur.
  const prog = { on: false, f: 0, smooth: 0, from: cloneLook(cur), to: LOOKS.crepuscule };
  const camLook = new THREE.Vector3();
  const balise = { mode: "aucun" as Signal, level: 0, search: 0, burst: 0, spot: 0, age: 99 };
  const orbPos = new THREE.Vector3(BEACON.x + 7, beaconY + 1.4, BEACON.z);

  const allerA: Clairiere["allerA"] = (m, o = {}) => {
    prog.on = false;
    from = cloneLook(cur); to = LOOKS[m];
    dur = (o.duree ?? 2.8) * (reduced ? 0.6 : 1); t0 = time + (o.delai ?? 0); arcOn = !reduced;
  };
  const poser: Clairiere["poser"] = (m) => { prog.on = false; cur = cloneLook(LOOKS[m]); from = cloneLook(cur); to = LOOKS[m]; t0 = -1e3; dur = 0.001; };
  const setSignal: Clairiere["setSignal"] = (s) => { if (s === balise.mode) return; balise.mode = s; if (s === "trouvee") impulsion(); };
  const impulsion = () => { balise.burst = 0.001; balise.age = 0; };
  const setProgression: Clairiere["setProgression"] = (f, de, vers) => {
    if (f === null) { prog.on = false; return; }
    if (!prog.on || de || vers) { // début : on repart de la lumière actuelle
      prog.on = true; prog.from = cloneLook(cur); prog.to = LOOKS[vers ?? "crepuscule"]; prog.smooth = 0;
      if (de) { prog.from = cloneLook(LOOKS[de]); prog.smooth = 0; }
    }
    prog.f = clamp01(f);
  };

  // ---- Interaction ----
  const mouse = { x: 0, y: 0, sx: 0, sy: 0, inside: false };
  const ray = new THREE.Raycaster(), ndc = new THREE.Vector2(), hit = new THREE.Vector3(0, -999, 0);
  const onMove = (e: PointerEvent) => { mouse.x = (e.clientX / innerWidth) * 2 - 1; mouse.y = -((e.clientY / innerHeight) * 2 - 1); mouse.inside = true; };
  const onLeave = () => { mouse.inside = false; };
  window.addEventListener("pointermove", onMove);
  document.addEventListener("pointerleave", onLeave);

  function resize() {
    const w = innerWidth, h = innerHeight;
    renderer.setPixelRatio(pr); renderer.setSize(w, h, false);
    camera.aspect = w / h;
    // Décale le cadrage vers la droite pour laisser la place au panneau (grands écrans).
    if (w > 900) camera.setViewOffset(w, h, -w * 0.13, 0, w, h); else camera.clearViewOffset();
    camera.updateProjectionMatrix();
  }
  resize(); addEventListener("resize", resize);

  // ---- Boucle ----
  let raf = 0, last = performance.now(), perdu = false, ema = 1 / 60, lowSince = 0, calls = 0, tris = 0, mort = false;
  const tmpDir = new THREE.Vector3(), tmpQ = new THREE.Quaternion(), up = V(0, 1, 0);

  function update(dt: number) {
    time += dt; U.uTime.value = time;
    // Fondu entre looks (minuté, ou piloté par la progression).
    let p: number;
    if (prog.on) {
      prog.smooth += (prog.f - prog.smooth) * (1 - Math.exp(-dt * 1.6));
      p = prog.smooth; blend(cur, prog.from, prog.to, ease(p));
    } else {
      p = clamp01((time - t0) / dur); blend(cur, from, to, ease(p));
    }
    const e = ease(p);
    if (reduced) { cur.camPos.copy(LOOKS.jour.camPos); cur.camTarget.copy(LOOKS.jour.camTarget); cur.fov = LOOKS.jour.fov; }

    U.uFogColor.value.copy(cur.fog); U.uGlow.value.copy(cur.glow); U.uKeyDir.value.copy(cur.keyDir); U.uKey.value.copy(cur.key);
    U.uAmbSky.value.copy(cur.ambSky); U.uAmbGround.value.copy(cur.ambGround); U.uGrassBase.value.copy(cur.grassBase); U.uGrassTip.value.copy(cur.grassTip);
    U.uFogDensity.value = cur.fogDensity; U.uMist.value = cur.mist; U.uClouds.value = cur.clouds;
    const su = skyMat.uniforms;
    su.uTop.value.copy(cur.top); su.uMid.value.copy(cur.mid); su.uHorizon.value.copy(cur.horizon); su.uSunDir.value.copy(cur.sunDir);
    su.uSunDisc.value = cur.sunDisc; su.uMoon.value = cur.moon; su.uStars.value = cur.stars;
    rayMat.uniforms.uRays.value = cur.rays; pollen.amount.value = cur.pollen; fire.amount.value = cur.fireflies;

    // Caméra : trajet en arc léger pendant la transition + parallaxe amortie + respiration.
    mouse.sx += (mouse.x - mouse.sx) * (1 - Math.exp(-dt * 2.5)); mouse.sy += (mouse.y - mouse.sy) * (1 - Math.exp(-dt * 2.5));
    const arc = arcOn && p > 0 && p < 1 ? Math.sin(Math.PI * e) * 1.8 : 0;
    const px = reduced ? 0 : mouse.sx, py = reduced ? 0 : mouse.sy, br = reduced ? 0 : 1;
    camera.position.set(cur.camPos.x + arc + px * 0.9 + Math.sin(time * 0.15) * 0.25 * br, cur.camPos.y + py * 0.35 + Math.sin(time * 0.21) * 0.08 * br, cur.camPos.z);
    camLook.set(cur.camTarget.x - px * 1.2, cur.camTarget.y + py * 0.5, cur.camTarget.z);
    camera.lookAt(camLook);
    if (Math.abs(camera.fov - cur.fov) > 0.01) { camera.fov = cur.fov; camera.updateProjectionMatrix(); }
    camera.updateMatrixWorld();
    sky.position.copy(camera.position);
    U.uScale.value = renderer.domElement.height / (2 * Math.tan((camera.fov * Math.PI) / 360));

    // Herbe : point survolé sur un plan à y = 0.5.
    ndc.set(mouse.x, mouse.y); ray.setFromCamera(ndc, camera);
    const ro = ray.ray.origin, rd = ray.ray.direction;
    if (mouse.inside && rd.y < -0.01) { const t = (0.5 - ro.y) / rd.y; if (t < 90) hit.set(ro.x + rd.x * t, 0.5, ro.z + rd.z * t); }
    const gm = grassU.uMouse.value; gm.lerp(hit, 1 - Math.exp(-dt * 10));
    grassU.uMouseK.value += ((mouse.inside && !reduced ? 1 : 0) - grassU.uMouseK.value) * (1 - Math.exp(-dt * 4));

    // Rayons : base au sol, axe vers la source de lumière.
    tmpDir.copy(cur.keyDir); tmpQ.setFromUnitVectors(up, tmpDir);
    for (const { m, len } of rays) {
      m.quaternion.copy(tmpQ); m.scale.set(m.userData.w as number, len, 1);
    }

    // Balise : guide qui rôde pendant la recherche, puis se pose.
    const b = balise, k = (r: number) => 1 - Math.exp(-dt * r);
    const lv = { aucun: 0, recherche: 1.0, trouvee: 1.3, repos: 0.6 }[b.mode];
    b.level += (lv - b.level) * k(2.5);
    b.search += ((b.mode === "recherche" ? 1 : 0) - b.search) * k(2);
    b.spot += ((b.mode === "trouvee" ? 0.55 : b.mode === "repos" ? 0.28 : 0) - b.spot) * k(1.5);
    if (b.burst > 0 && b.burst < 1) b.burst = Math.min(1, b.burst + dt / 2.2);
    b.age += dt;
    const sr = b.mode === "recherche";
    const tx = sr ? BEACON.x + Math.sin(time * 0.55) * 7 * (0.6 + 0.4 * Math.sin(time * 0.2)) : BEACON.x;
    const tz = sr ? BEACON.z + Math.cos(time * 0.4) * 5 : BEACON.z;
    const ty = beaconY + 1.4 + Math.sin(time * (sr ? 1.3 : 1.5)) * (sr ? 0.35 : 0.06);
    orbPos.x += (tx - orbPos.x) * k(sr ? 3 : 1.8); orbPos.y += (ty - orbPos.y) * k(3); orbPos.z += (tz - orbPos.z) * k(sr ? 3 : 1.8);
    orb.position.copy(orbPos); orb.visible = b.level > 0.01;
    orbU.uLevel.value = b.level; orbU.uSizeM.value = b.mode === "trouvee" ? 2.4 : b.mode === "repos" ? 1.5 : 2.0;
    waveU.uSearch.value = b.search; waveU.uBurst.value = b.burst; waveU.uSpot.value = b.spot;
    wave.visible = b.search > 0.01 || b.burst > 0 || b.spot > 0.01;
    // L'herbe porte aussi l'onde : anneaux répétés pendant la recherche (lisibles sur toute la clairière), onde unique sinon.
    if (b.search > 0.05) grassU.uPulse.value.set(BEACON.x, BEACON.z, time % 3.4, b.search);
    else grassU.uPulse.value.set(BEACON.x, BEACON.z, b.age, b.age < 8 ? 1 : 0);
  }

  function frame(now: number) {
    raf = requestAnimationFrame(frame);
    const dt = Math.min((now - last) / 1000, 0.05); last = now;
    update(dt);
    renderer.render(scene, camera);
    calls = renderer.info.render.calls; tris = renderer.info.render.triangles;
    // Qualité dynamique : baisse le pixel ratio si la fluidité manque pendant 2 s.
    ema += (dt - ema) * 0.05;
    if (CFG.ADAPTIVE && time > 4) {
      if (ema > 1 / 45) { lowSince += dt; if (lowSince > 2 && pr > CFG.MIN_PIXEL_RATIO) { pr = Math.max(CFG.MIN_PIXEL_RATIO, pr - 0.25); resize(); lowSince = 0; } } else lowSince = 0;
    }
  }
  const start = () => { if (!raf && !perdu && !mort && !document.hidden) { last = performance.now(); raf = requestAnimationFrame(frame); } };
  const stop = () => { cancelAnimationFrame(raf); raf = 0; };
  const onVis = () => (document.hidden ? stop() : start()); // rien n'est rendu fenêtre cachée
  const onCtxLost = (e: Event) => { e.preventDefault(); perdu = true; stop(); onLost(true); };
  const onCtxBack = () => { perdu = false; onLost(false); start(); };
  document.addEventListener("visibilitychange", onVis);
  canvas.addEventListener("webglcontextlost", onCtxLost);
  canvas.addEventListener("webglcontextrestored", onCtxBack);

  poser("aube"); update(0.016); start();
  return {
    allerA, poser, setSignal, setProgression, impulsion,
    dispose() {
      mort = true; stop();
      window.removeEventListener("pointermove", onMove); document.removeEventListener("pointerleave", onLeave);
      removeEventListener("resize", resize); document.removeEventListener("visibilitychange", onVis);
      canvas.removeEventListener("webglcontextlost", onCtxLost); canvas.removeEventListener("webglcontextrestored", onCtxBack);
      renderer.dispose();
    },
    stats: () => `${(1 / ema).toFixed(0)} i/s · pr ${pr} · ${calls} appels · ${(tris / 1000).toFixed(0)}k tri`,
  };
}
