// Scène 3D « Clairière vivante » : herbe instanciée, lisières d'arbres, ciel, rayons,
// particules. Le parcours est une succession de « looks » (heure du jour) entre lesquels
// on fond : caméra, lumière, ciel et brume sont interpolés ensemble.
import * as THREE from "three";
import { mergeGeometries } from "three/examples/jsm/utils/BufferGeometryUtils.js";
import * as S from "./shaders";

// ---- Réglages de performance (tout est ici) ----
export const CFG = {
  GRASS_COUNT: 110_000, // brins d'herbe instanciés (le plus coûteux ; 60 000 pour un Mac modeste)
  CONIFERS: 380, // sapins en lisière
  ROUND_TREES: 140, // arbres feuillus
  RAY_COUNT: 12, // plans de rayons de lumière
  POLLEN_COUNT: 900,
  FIREFLY_COUNT: 140,
  GROUND_SEGMENTS: 200,
  MAX_PIXEL_RATIO: 2, // plafond du pixel ratio
  MIN_PIXEL_RATIO: 1, // plancher de la qualité dynamique
  ADAPTIVE: true, // baisse le pixel ratio si < 45 i/s pendant 2 s
};

THREE.ColorManagement.enabled = false; // on travaille en sRGB brut, couleurs choisies à l'œil

export type Heure = "dawn" | "day" | "golden" | "dusk" | "night";
export type Balise = "off" | "search" | "found" | "rest";

// ---- Looks par heure ----
interface Look {
  top: THREE.Color; mid: THREE.Color; horizon: THREE.Color; glow: THREE.Color;
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
const MOON = V(-0.3, 0.43, -0.85).normalize();

const LOOKS: Record<Heure, Look> = {
  dawn: {
    top: C(0.2, 0.3, 0.56), mid: C(0.55, 0.58, 0.82), horizon: C(0.82, 0.76, 0.9), glow: C(1.0, 0.6, 0.58),
    key: C(1.0, 0.7, 0.64, 0.85), ambSky: C(0.4, 0.48, 0.74, 0.95), ambGround: C(0.24, 0.27, 0.38),
    grassBase: C(0.1, 0.22, 0.24), grassTip: C(0.42, 0.64, 0.56),
    sunDir: dirAE(-28, 3), keyDir: dirAE(-28, 3), camPos: V(0, 3.4, 27), camTarget: V(1.5, 2.9, -12), fov: 58,
    fogDensity: 0.022, mist: 0.6, sunDisc: 1, moon: 0, stars: 0.06, clouds: 0.5, rays: 0.4, pollen: 0.2, fireflies: 0,
  },
  day: {
    top: C(0.24, 0.55, 0.9), mid: C(0.52, 0.76, 0.95), horizon: C(0.8, 0.9, 0.95), glow: C(1.0, 0.95, 0.8, 0.8),
    key: C(1.0, 0.95, 0.82, 1.0), ambSky: C(0.55, 0.7, 0.9, 0.8), ambGround: C(0.3, 0.38, 0.26),
    grassBase: C(0.1, 0.32, 0.14), grassTip: C(0.5, 0.76, 0.28),
    sunDir: dirAE(32, 50), keyDir: dirAE(32, 50), camPos: V(0, 2.5, 15), camTarget: V(0, 2.1, -14), fov: 52,
    fogDensity: 0.011, mist: 0.15, sunDisc: 1, moon: 0, stars: 0, clouds: 0.9, rays: 0.25, pollen: 0.5, fireflies: 0,
  },
  golden: {
    top: C(0.26, 0.38, 0.7), mid: C(0.85, 0.66, 0.62), horizon: C(1.0, 0.78, 0.52), glow: C(1.0, 0.55, 0.2),
    key: C(1.0, 0.66, 0.34, 1.3), ambSky: C(0.55, 0.52, 0.72, 0.75), ambGround: C(0.36, 0.26, 0.2),
    grassBase: C(0.15, 0.28, 0.1), grassTip: C(0.82, 0.78, 0.3),
    sunDir: dirAE(24, 8), keyDir: dirAE(24, 8), camPos: V(-2, 1.6, 7), camTarget: V(3, 2.0, -16), fov: 50,
    fogDensity: 0.016, mist: 0.25, sunDisc: 1, moon: 0, stars: 0, clouds: 0.8, rays: 1.0, pollen: 0.95, fireflies: 0.1,
  },
  dusk: {
    top: C(0.1, 0.14, 0.36), mid: C(0.42, 0.3, 0.55), horizon: C(0.64, 0.42, 0.55), glow: C(0.95, 0.42, 0.36, 0.8),
    key: C(0.9, 0.45, 0.5, 0.5), ambSky: C(0.3, 0.3, 0.56, 0.75), ambGround: C(0.16, 0.14, 0.27),
    grassBase: C(0.07, 0.12, 0.15), grassTip: C(0.4, 0.42, 0.48),
    sunDir: dirAE(24, -3), keyDir: dirAE(24, -3), camPos: V(0, 1.4, 4), camTarget: V(2, 3.2, -14), fov: 52,
    fogDensity: 0.02, mist: 0.4, sunDisc: 0.5, moon: 0, stars: 0.55, clouds: 0.5, rays: 0.4, pollen: 0.2, fireflies: 0.55,
  },
  night: {
    top: C(0.02, 0.04, 0.13), mid: C(0.04, 0.08, 0.21), horizon: C(0.09, 0.14, 0.29), glow: C(0.2, 0.3, 0.6, 0.35),
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
function colored(g: THREE.BufferGeometry, c: [number, number, number]): THREE.BufferGeometry {
  const ng = g.index ? g.toNonIndexed() : g;
  ng.deleteAttribute("normal"); ng.deleteAttribute("uv");
  const n = ng.getAttribute("position").count, a = new Float32Array(n * 3);
  for (let i = 0; i < n; i++) a.set(c, i * 3);
  ng.setAttribute("aCol", new THREE.BufferAttribute(a, 3));
  return ng;
}
const TRUNK: [number, number, number] = [0.2, 0.14, 0.1];
function coniferGeometry(): THREE.BufferGeometry {
  const parts = [colored(new THREE.CylinderGeometry(0.02, 0.035, 0.22, 5).translate(0, 0.11, 0), TRUNK)];
  const tiers: [number, number, number][] = [[0.1, 0.42, 0.3], [0.3, 0.4, 0.24], [0.5, 0.36, 0.18], [0.68, 0.32, 0.11]];
  for (const [y, h, r] of tiers) parts.push(colored(new THREE.ConeGeometry(r, h, 8, 1, true).translate(0, y + h / 2, 0), [0.09, 0.24, 0.19]));
  return mergeGeometries(parts)!;
}
function roundTreeGeometry(): THREE.BufferGeometry {
  const r = rng(7), parts = [colored(new THREE.CylinderGeometry(0.02, 0.04, 0.5, 5).translate(0, 0.25, 0), TRUNK)];
  const blobs: [number, number, number, number][] = [[0, 0.62, 0, 0.27], [0.15, 0.5, 0.05, 0.2], [-0.14, 0.52, -0.06, 0.21], [0.02, 0.84, 0.02, 0.2]];
  for (const [x, y, z, s] of blobs) {
    const g = new THREE.IcosahedronGeometry(s, 1), p = g.getAttribute("position");
    for (let i = 0; i < p.count; i += 3) { // jitter par face : silhouette à facettes irrégulières
      const k = 0.85 + r() * 0.3;
      for (let j = 0; j < 3; j++) p.setXYZ(i + j, p.getX(i + j) * k, p.getY(i + j) * k, p.getZ(i + j) * k);
    }
    parts.push(colored(g.translate(x, y, z), [0.16, 0.3, 0.15]));
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
  go(h: Heure, o?: { duration?: number; delay?: number }): void;
  snap(h: Heure): void;
  setBalise(b: Balise): void;
  /** Vrai si le rendu WebGL tourne (faux après perte de contexte). */
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

  // Arbres en lisière (clairsemés par grappes, jamais sur le trajet de la caméra).
  {
    const mat = new THREE.ShaderMaterial({ vertexShader: S.TREE_VERT, fragmentShader: S.TREE_FRAG, side: THREE.DoubleSide, uniforms: { ...U } });
    const r = rng(42), dummy = new THREE.Object3D();
    const place = (geo: THREE.BufferGeometry, count: number, hMin: number, hMax: number, wRatio: number) => {
      const mats: THREE.Matrix4[] = [], hues: number[] = [];
      let guard = 0;
      while (mats.length < count && guard++ < count * 40) {
        const a = r() * Math.PI * 2, rad = 38 + 8 * Math.sin(a * 5) + Math.pow(r(), 1.3) * 80;
        const x = Math.sin(a) * rad, z = -10 - Math.cos(a) * rad;
        const clump = 0.5 + 0.5 * Math.sin(a * 7 + 1) * Math.sin(a * 3.1 + 2);
        if (r() > 0.3 + 0.7 * clump || z > 18 || Math.hypot(x, z - 26) < 16) continue;
        const h = hMin + r() * (hMax - hMin) * (0.7 + rad / 200);
        dummy.position.set(x, groundH(x, z) - 0.3, z);
        dummy.rotation.y = r() * 6.28; dummy.scale.set(h * wRatio * (0.85 + r() * 0.3), h, h * wRatio * (0.85 + r() * 0.3));
        dummy.updateMatrix(); mats.push(dummy.matrix.clone()); hues.push(r());
      }
      const im = new THREE.InstancedMesh(geo, mat, mats.length);
      mats.forEach((m, i) => im.setMatrixAt(i, m));
      geo.setAttribute("iHue", new THREE.InstancedBufferAttribute(new Float32Array(hues), 1));
      im.frustumCulled = false; scene.add(im);
    };
    place(coniferGeometry(), CFG.CONIFERS, 8, 15, 1);
    place(roundTreeGeometry(), CFG.ROUND_TREES, 6, 11, 1);
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
  const waveU = { uSearch: { value: 0 }, uBurst: { value: 0 }, uSpot: { value: 0 }, uColor: { value: new THREE.Color(0.7, 0.95, 1) } };
  const wave = new THREE.Mesh(new THREE.CircleGeometry(17, 64), new THREE.ShaderMaterial({
    vertexShader: S.WAVE_VERT, fragmentShader: S.WAVE_FRAG, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, uniforms: { ...U, ...waveU },
  }));
  wave.rotation.x = -Math.PI / 2; wave.position.set(BEACON.x, beaconY + 0.45, BEACON.z); wave.frustumCulled = false; scene.add(wave);
  const orbG = new THREE.BufferGeometry(); orbG.setAttribute("position", new THREE.BufferAttribute(new Float32Array(3), 3));
  const orbU = { uLevel: { value: 0 }, uSizeM: { value: 1.6 }, uColor: { value: new THREE.Color(0.75, 0.95, 1) } };
  const orb = new THREE.Points(orbG, new THREE.ShaderMaterial({
    vertexShader: S.ORB_VERT, fragmentShader: S.ORB_FRAG, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, uniforms: { uTime: U.uTime, uScale: U.uScale, ...orbU },
  }));
  orb.frustumCulled = false; orb.visible = false; scene.add(orb);

  // ---- État : transitions entre looks ----
  let cur = cloneLook(LOOKS.dawn), from = cloneLook(cur), to = LOOKS.dawn;
  let t0 = 0, dur = 0.001, arcOn = false, time = 0;
  const camLook = new THREE.Vector3();
  const balise = { mode: "off" as Balise, level: 0, search: 0, burst: 0, spot: 0, age: 99, foundAt: -1 };
  const orbPos = new THREE.Vector3(BEACON.x + 7, beaconY + 1.4, BEACON.z);

  const go: Clairiere["go"] = (h, o = {}) => {
    from = cloneLook(cur); to = LOOKS[h];
    dur = (o.duration ?? 2.8) * (reduced ? 0.6 : 1); t0 = time + (o.delay ?? 0); arcOn = !reduced;
  };
  const snap: Clairiere["snap"] = (h) => { cur = cloneLook(LOOKS[h]); from = cloneLook(cur); to = LOOKS[h]; t0 = -1e3; dur = 0.001; };
  const setBalise = (b: Balise) => {
    balise.mode = b;
    if (b === "found") { balise.burst = 0.001; balise.age = 0; balise.foundAt = time; }
  };

  // ---- Interaction ----
  const mouse = { x: 0, y: 0, sx: 0, sy: 0, inside: false };
  const ray = new THREE.Raycaster(), ndc = new THREE.Vector2(), hit = new THREE.Vector3(0, -999, 0);
  window.addEventListener("pointermove", (e) => { mouse.x = (e.clientX / innerWidth) * 2 - 1; mouse.y = -((e.clientY / innerHeight) * 2 - 1); mouse.inside = true; });
  document.addEventListener("pointerleave", () => { mouse.inside = false; });

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
  let raf = 0, last = performance.now(), perdu = false, ema = 1 / 60, lowSince = 0, calls = 0, tris = 0;
  const tmpDir = new THREE.Vector3(), tmpQ = new THREE.Quaternion(), up = V(0, 1, 0);

  function update(dt: number) {
    time += dt; U.uTime.value = time;
    // Fondu entre looks.
    const p = clamp01((time - t0) / dur), e = ease(p);
    blend(cur, from, to, e);
    if (reduced) { cur.camPos.copy(LOOKS.day.camPos); cur.camTarget.copy(LOOKS.day.camTarget); cur.fov = LOOKS.day.fov; }

    U.uFogColor.value.copy(cur.horizon); U.uGlow.value.copy(cur.glow); U.uKeyDir.value.copy(cur.keyDir); U.uKey.value.copy(cur.key);
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
    const lv = { off: 0, search: 0.8, found: 1.3, rest: 0.6 }[b.mode];
    b.level += (lv - b.level) * k(2.5);
    b.search += ((b.mode === "search" ? 1 : 0) - b.search) * k(2);
    b.spot += ((b.mode === "found" || b.mode === "rest" ? (b.mode === "found" ? 0.55 : 0.28) : 0) - b.spot) * k(1.5);
    if (b.burst > 0 && b.burst < 1) b.burst = Math.min(1, b.burst + dt / 2.2);
    b.age += dt;
    const tx = b.mode === "search" ? BEACON.x + Math.sin(time * 0.55) * 7 * (0.6 + 0.4 * Math.sin(time * 0.2)) : BEACON.x;
    const tz = b.mode === "search" ? BEACON.z + Math.cos(time * 0.4) * 5 : BEACON.z;
    const ty = beaconY + 1.4 + Math.sin(time * (b.mode === "search" ? 1.3 : 1.5)) * (b.mode === "search" ? 0.35 : 0.06);
    orbPos.x += (tx - orbPos.x) * k(b.mode === "search" ? 3 : 1.8); orbPos.y += (ty - orbPos.y) * k(3); orbPos.z += (tz - orbPos.z) * k(b.mode === "search" ? 3 : 1.8);
    orb.position.copy(orbPos); orb.visible = b.level > 0.01;
    orbU.uLevel.value = b.level; orbU.uSizeM.value = b.mode === "found" ? 2.4 : b.mode === "rest" ? 1.5 : 1.6;
    waveU.uSearch.value = b.search; waveU.uBurst.value = b.burst; waveU.uSpot.value = b.spot;
    wave.visible = b.search > 0.01 || b.burst > 0 || b.spot > 0.01;
    grassU.uPulse.value.set(BEACON.x, BEACON.z, b.age, b.age < 8 ? 1 : 0);
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
  const start = () => { if (!raf && !perdu && !document.hidden) { last = performance.now(); raf = requestAnimationFrame(frame); } };
  const stop = () => { cancelAnimationFrame(raf); raf = 0; };
  document.addEventListener("visibilitychange", () => (document.hidden ? stop() : start())); // rien n'est rendu onglet caché
  canvas.addEventListener("webglcontextlost", (e) => { e.preventDefault(); perdu = true; stop(); onLost(true); });
  canvas.addEventListener("webglcontextrestored", () => { perdu = false; onLost(false); start(); });

  snap("dawn"); update(0.016); start();
  return { go, snap, setBalise, stats: () => `${(1 / ema).toFixed(0)} i/s · pr ${pr} · ${calls} appels · ${(tris / 1000).toFixed(0)}k tri` };
}
