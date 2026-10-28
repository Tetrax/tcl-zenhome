// Décor three.js : couches de papier découpé, ciel, lac, brume, poussières, oiseaux, lueur de la cabane.
// Tout est procédural. La caméra traverse réellement le décor d'un écran à l'autre.
import * as THREE from "three";
import {
  BASE, TEX_H, TEX_W, layerWidth, paintLayer, type LayerDef,
} from "./layers";
import {
  CAMS, COLOR_KEYS, NUM_KEYS, PALETTES, type Palette, type Step,
} from "./palettes";
import * as SH from "./shaders";

// ---------- Réglages de qualité / performance ----------
export const QUALITY = {
  maxPixelRatio: 2,       // plafond du devicePixelRatio
  dofSamples: 8,          // prises du flou de profondeur (1 = flou par mipmap seul, 0 interdit)
  adaptive: true,         // baisse la résolution si les images dépassent le budget
  budgetMs: 21,           // temps moyen par image toléré avant de baisser la résolution
  minPixelRatio: 1,
  transitionSec: 2.8,     // durée du travelling + changement de palette
  parallaxX: 1.5,         // amplitude de la parallaxe souris (unités monde)
  parallaxY: 0.55,
};

THREE.ColorManagement.enabled = false; // palettes en sRGB brut, pas de conversion

const SKIRT = 90;         // débord de la silhouette vers le bas (le sol remplit l'écran)
const WATER_Y = -0.1;

// Couches du plus lointain au plus proche (z décroissant vers le fond)
const LAYERS: LayerDef[] = [
  { id: "m0", z: -100, kind: "peaks", seed: 11, t: 0.0, acc: 0.0, gy: 0, amp: 28 },
  { id: "m1", z: -86, kind: "peaks", seed: 23, t: 0.1, acc: 0.0, gy: 0, amp: 18 },
  { id: "r2", z: -72, kind: "conifer", seed: 37, t: 0.22, acc: 0.1, gy: 0, amp: 9, tree: [1.8, 3.6] },
  { id: "f3", z: -60, kind: "conifer", seed: 41, t: 0.34, acc: 0.25, gy: 0, amp: 3, tree: [3.2, 6], cabin: 9 },
  { id: "h5", z: -38, kind: "broadleaf", seed: 53, t: 0.5, acc: 0.5, gy: 0, amp: 4, tree: [3.5, 6.5], valley: 7.5, valleyW: 20 },
  { id: "f6", z: -28, kind: "conifer", seed: 67, t: 0.62, acc: 0.35, gy: -0.2, amp: 4, tree: [5, 9], valley: 6.2, valleyW: 17, receive: true },
  { id: "h7", z: -18, kind: "hills", seed: 71, t: 0.74, acc: 0.6, gy: -0.3, amp: 3.5, valley: 5, valleyW: 13, receive: true },
  { id: "fr8", z: -9, kind: "frame", seed: 83, t: 0.86, acc: 0.4, gy: -0.6, amp: 3, tree: [7, 11], valley: 3.2, valleyW: 12, receive: true, w: 80 },
  { id: "g9", z: -1, kind: "grass", seed: 97, t: 0.94, acc: 0.5, gy: -0.9, amp: 1.2, tree: [0.8, 2.1], valley: 2, valleyW: 9 },
  { id: "g10", z: 7, kind: "grass", seed: 101, t: 1.0, acc: 0.4, gy: 0.1, amp: 1.0, tree: [1.0, 2.3], valley: 0.5, valleyW: 7 },
];
const WATER_Z = -48;
const MIST_Z = [-92, -80, -66, -54, -43, -33, -23, -13, 0];

type Uni = Record<string, THREE.IUniform>;
const cap = (k: string) => "u" + k[0].toUpperCase() + k.slice(1);
const smoother = (t: number) => t * t * t * (t * (t * 6 - 15) + 10);
const clamp01 = (x: number) => Math.min(1, Math.max(0, x));

// Rectangle dont le bas déborde (SKIRT) ; v dépasse 0 -> la dernière ligne de la texture (sol plein) se répète.
function makeQuad(w: number, h: number, skirt: number): THREE.BufferGeometry {
  const g = new THREE.BufferGeometry();
  const x = w / 2, v0 = -skirt / h;
  g.setAttribute("position", new THREE.Float32BufferAttribute([-x, -skirt, 0, x, -skirt, 0, x, h, 0, -x, h, 0], 3));
  g.setAttribute("uv", new THREE.Float32BufferAttribute([0, v0, 1, v0, 1, 1, 0, 1], 2));
  g.setIndex([0, 1, 2, 0, 2, 3]);
  return g;
}

function pointsGroup(n: number, seed: number): THREE.BufferGeometry {
  let a = seed;
  const r = () => ((a = (Math.imul(a, 1664525) + 1013904223) >>> 0) / 4294967296);
  const g = new THREE.BufferGeometry();
  const seeds = new Float32Array(n * 4);
  for (let i = 0; i < seeds.length; i++) seeds[i] = r();
  g.setAttribute("position", new THREE.BufferAttribute(new Float32Array(n * 3), 3)); // factice : compte des sommets
  g.setAttribute("aSeed", new THREE.BufferAttribute(seeds, 4));
  return g;
}

export interface Backdrop {
  goto(step: Step, instant?: boolean): void;
  setSignal(v: number): void;
  setBox(on: boolean): void;
  setProgress(p: number): void;
  dispose(): void;
}

export class Diorama implements Backdrop {
  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera = new THREE.PerspectiveCamera(34, 1, 0.5, 400);
  private U: Uni = {};
  private colors = {} as Record<string, THREE.Color>;
  private nums = {} as Record<string, number>;
  private fromColors = {} as Record<string, THREE.Color>;
  private toColors = {} as Record<string, THREE.Color>;
  private fromNums = {} as Record<string, number>;
  private toNums = {} as Record<string, number>;
  private cam = { pos: new THREE.Vector3(), look: new THREE.Vector3(), fov: 34 };
  private fromCam = { pos: new THREE.Vector3(), look: new THREE.Vector3(), fov: 34 };
  private toCam = { pos: new THREE.Vector3(), look: new THREE.Vector3(), fov: 34 };
  private t = 0;            // temps de l'animation d'ambiance
  private tr = 1;           // progression de la transition 0..1
  private sway = 1;
  private reduced: boolean;
  private raf = 0;
  private last = 0;
  private running = false;
  private dirty = true;
  private mouse = new THREE.Vector2();
  private mouseS = new THREE.Vector2();
  private pr = 1;
  private frameAcc = 0;
  private frameN = 0;
  private lost = false;
  private lostTimer = 0;
  private signalT = 0; private signalV = 0;
  private boxT = 0; private boxV = 0; private pulse = 0; private pulseOn = false;
  private progT = 0; private progV = 0;
  private dustMats: THREE.ShaderMaterial[] = [];
  private fireMat!: THREE.ShaderMaterial;
  private birdMat!: THREE.ShaderMaterial;
  private textures: THREE.Texture[] = [];
  private disposers: (() => void)[] = [];

  constructor(private canvas: HTMLCanvasElement, gl: WebGL2RenderingContext, private onFail: () => void) {
    this.reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
    this.renderer = new THREE.WebGLRenderer({ canvas, context: gl, antialias: false });
    this.renderer.setClearColor(0x000000, 1);
    this.pr = Math.min(window.devicePixelRatio || 1, QUALITY.maxPixelRatio);
    this.initUniforms();
    this.build();
    this.snapTo("welcome");
    this.resize();

    const on = <K extends keyof WindowEventMap>(t: K, f: (e: WindowEventMap[K]) => void) => {
      window.addEventListener(t, f as EventListener);
      this.disposers.push(() => window.removeEventListener(t, f as EventListener));
    };
    on("resize", () => this.resize());
    on("pointermove", (e) => {
      this.mouse.set((e.clientX / innerWidth - 0.5) * 2, (e.clientY / innerHeight - 0.5) * 2);
    });
    const vis = () => (document.hidden ? this.stop() : this.start());
    document.addEventListener("visibilitychange", vis);
    this.disposers.push(() => document.removeEventListener("visibilitychange", vis));

    // Perte du contexte GL : on met en pause ; si rien ne revient, on bascule sur le repli.
    canvas.addEventListener("webglcontextlost", (e) => {
      e.preventDefault();
      this.lost = true; this.stop();
      this.lostTimer = window.setTimeout(() => this.onFail(), 2500);
    });
    canvas.addEventListener("webglcontextrestored", () => {
      clearTimeout(this.lostTimer);
      this.lost = false; this.dirty = true; this.start();
    });
    this.start();
  }

  // ---------- Uniformes partagés ----------
  private initUniforms() {
    const U = this.U;
    for (const k of COLOR_KEYS) { this.colors[k] = new THREE.Color(); U[cap(k)] = { value: this.colors[k] }; }
    for (const k of NUM_KEYS) { this.nums[k] = 0; U[cap(k)] = { value: 0 }; }
    Object.assign(U, {
      uTime: { value: 0 }, uRes: { value: new THREE.Vector2(1, 1) }, uResY: { value: 1 },
      uAspect: { value: 1.7 }, uTanHalf: { value: 0.3 },
      uSun: { value: new THREE.Vector2() }, uMoonP: { value: new THREE.Vector2() },
      uSunDir: { value: new THREE.Vector2(-0.5, 0.8) }, uSunSign: { value: 0 },
      uShift: { value: new THREE.Vector2() },
      uSignal: { value: 0 }, uBox: { value: 0 }, uPulse: { value: 0 },
      uRipple: { value: new THREE.Vector2(0.5, WATER_Y - 2.6) },
    });
  }

  private build() {
    const items: { obj: THREE.Object3D; z: number }[] = [];
    const shared = this.U;
    const dofN = Math.max(1, QUALITY.dofSamples | 0);

    // Ciel
    const sky = new THREE.Mesh(
      new THREE.PlaneGeometry(2, 2),
      new THREE.ShaderMaterial({
        uniforms: shared, vertexShader: SH.skyVert, fragmentShader: SH.skyFrag, depthTest: false, depthWrite: false,
      }),
    );
    sky.frustumCulled = false; sky.renderOrder = -1;
    this.scene.add(sky);

    const tx = (cv: HTMLCanvasElement) => {
      const t = new THREE.CanvasTexture(cv);
      t.minFilter = THREE.LinearMipmapLinearFilter; t.generateMipmaps = true;
      t.wrapS = t.wrapT = THREE.ClampToEdgeWrapping; t.colorSpace = THREE.NoColorSpace;
      this.textures.push(t);
      return t;
    };
    const flat = (m: THREE.ShaderMaterial) => { m.transparent = true; m.depthTest = false; m.depthWrite = false; return m; };

    let lamp: THREE.Vector3 | null = null;
    for (const def of LAYERS) {
      const W = layerWidth(def), H = W / 4;
      const painted = paintLayer(def, W);
      const dNom = 14.5 - def.z;
      const mat = flat(new THREE.ShaderMaterial({
        uniforms: {
          ...shared,
          uTex: { value: tx(painted.canvas) }, uSize: { value: new THREE.Vector2(W, H) },
          uT: { value: def.t }, uAcc: { value: def.acc }, uReceive: { value: def.receive ? 1 : 0 },
          uGrain: { value: Math.max(1.5, 380 / dNom) },
        },
        vertexShader: SH.layerVert, fragmentShader: SH.layerFrag, defines: { DOF_N: dofN },
      }));
      const mesh = new THREE.Mesh(makeQuad(W, H, SKIRT), mat);
      mesh.position.set(0, def.gy - (1 - BASE) * H, def.z);
      items.push({ obj: mesh, z: def.z });
      if (painted.window) {
        const ppu = TEX_W / W;
        lamp = new THREE.Vector3(painted.window.px / ppu - W / 2, mesh.position.y + (TEX_H - painted.window.py) / ppu, def.z + 0.05);
      }
    }

    // Lac
    {
      const W = layerWidth({ z: WATER_Z } as LayerDef);
      const mat = flat(new THREE.ShaderMaterial({
        uniforms: { ...shared, uWaterY: { value: WATER_Y }, uSpeed: { value: this.reduced ? 0 : 1 } },
        vertexShader: SH.worldVert, fragmentShader: SH.waterFrag,
      }));
      const m = new THREE.Mesh(makeQuad(W, 4, SKIRT), mat);
      m.position.set(0, WATER_Y + 0.15 - 4, WATER_Z);
      items.push({ obj: m, z: WATER_Z });
    }

    // Nappes de brume
    MIST_Z.forEach((z, i) => {
      const W = layerWidth({ z } as LayerDef), Hm = 10;
      const mat = flat(new THREE.ShaderMaterial({
        uniforms: { ...shared, uSeed: { value: i * 1.37 }, uSpeed: { value: (i % 2 ? 1 : -1) * (0.004 + 0.002 * (i % 3)) * (this.reduced ? 0 : 1) } },
        vertexShader: SH.worldVert, fragmentShader: SH.mistFrag,
      }));
      const m = new THREE.Mesh(new THREE.PlaneGeometry(W, Hm), mat);
      m.position.set(0, 1.2 + (i % 3) * 0.6, z + 0.02);
      items.push({ obj: m, z: z + 0.02 });
    });

    // Poussières de lumière, intercalées entre les couches
    const dustDefs: [number, number, number, number, number][] = [ // zMin, zMax, demi-largeur, hauteur, nombre
      [-85, -50, 55, 16, 70], [-48, -18, 28, 9, 90], [-17, 12, 12, 7, 70],
    ];
    dustDefs.forEach(([z0, z1, hw, hh, n], i) => {
      const mat = flat(new THREE.ShaderMaterial({
        uniforms: {
          ...shared, uAmount: { value: 0 }, uSize: { value: 0.16 }, uBlink: { value: 0 },
          uMin: { value: new THREE.Vector3(-hw, 0.2, z0) }, uSpan: { value: new THREE.Vector3(hw * 2, hh, z1 - z0) },
          uDrift: { value: new THREE.Vector2(0.03, 0.05) }, uColor: { value: new THREE.Color("#fff1d0") },
        },
        vertexShader: SH.pointsVert, fragmentShader: SH.pointsFrag, blending: THREE.AdditiveBlending,
      }));
      this.dustMats.push(mat);
      const p = new THREE.Points(pointsGroup(n, 100 + i), mat);
      p.frustumCulled = false;
      items.push({ obj: p, z: (z0 + z1) / 2 });
    });

    // Lucioles (nuit)
    {
      this.fireMat = flat(new THREE.ShaderMaterial({
        uniforms: {
          ...shared, uAmount: { value: 0 }, uSize: { value: 0.3 }, uBlink: { value: 1 },
          uMin: { value: new THREE.Vector3(-16, 0.1, -42) }, uSpan: { value: new THREE.Vector3(32, 4.5, 32) },
          uDrift: { value: new THREE.Vector2(0.02, 0.01) }, uColor: { value: new THREE.Color("#d8ff7a") },
        },
        vertexShader: SH.pointsVert, fragmentShader: SH.pointsFrag, blending: THREE.AdditiveBlending,
      }));
      const p = new THREE.Points(pointsGroup(46, 777), this.fireMat);
      p.frustumCulled = false;
      items.push({ obj: p, z: -26 });
    }

    // Oiseaux lointains
    {
      this.birdMat = flat(new THREE.ShaderMaterial({
        uniforms: {
          ...shared, uAmount: { value: 0 }, uSize: { value: 3.2 },
          uMin: { value: new THREE.Vector3(-70, 12, -96) }, uSpan: { value: new THREE.Vector3(140, 8, 2) },
          uColor: { value: new THREE.Color("#4a4468") },
        },
        vertexShader: SH.birdVert, fragmentShader: SH.birdFrag,
      }));
      const p = new THREE.Points(pointsGroup(7, 5), this.birdMat);
      p.frustumCulled = false;
      items.push({ obj: p, z: -96 });
    }

    // Lampe de la cabane : s'allume quand la box est trouvée
    if (lamp) {
      const mat = flat(new THREE.ShaderMaterial({
        uniforms: { ...shared, uLamp: { value: new THREE.Color("#ffb860") } },
        vertexShader: SH.glowVert, fragmentShader: SH.glowFrag, blending: THREE.AdditiveBlending,
      }));
      const m = new THREE.Mesh(new THREE.PlaneGeometry(9, 9), mat);
      m.position.copy(lamp);
      items.push({ obj: m, z: lamp.z });
    }

    // Ordre de peinture : du plus lointain au plus proche
    items.sort((a, b) => a.z - b.z).forEach((it, i) => { it.obj.renderOrder = i; this.scene.add(it.obj); });
  }

  // ---------- Palette / caméra ----------
  private load(p: Palette, c: Record<string, THREE.Color>, n: Record<string, number>) {
    for (const k of COLOR_KEYS) c[k] = (c[k] ?? new THREE.Color()).set(p[k]);
    for (const k of NUM_KEYS) n[k] = p[k];
  }
  private loadCam(step: Step, c: { pos: THREE.Vector3; look: THREE.Vector3; fov: number }) {
    const s = CAMS[step];
    c.pos.set(...s.pos); c.look.set(...s.look); c.fov = s.fov;
  }

  private snapTo(step: Step) {
    this.load(PALETTES[step], this.toColors, this.toNums);
    this.loadCam(step, this.toCam);
    this.tr = 1;
    this.apply(1);
  }

  goto(step: Step, instant = false) {
    if (instant || this.reduced) {
      // Pas de travelling : on garde la caméra, simple fondu de palette (plus court)
      this.copyCurrent();
      this.load(PALETTES[step], this.toColors, this.toNums);
      this.loadCam(step, this.toCam);
      if (instant) { this.tr = 1; this.apply(1); } else { this.tr = 0; this.fromCam.pos.copy(this.toCam.pos); this.fromCam.look.copy(this.toCam.look); this.fromCam.fov = this.toCam.fov; }
      this.dirty = true; return;
    }
    this.copyCurrent();
    this.load(PALETTES[step], this.toColors, this.toNums);
    this.loadCam(step, this.toCam);
    this.sway = this.toCam.pos.x >= this.fromCam.pos.x ? 1 : -1;
    this.tr = 0;
    this.dirty = true;
  }

  private copyCurrent() {
    for (const k of COLOR_KEYS) this.fromColors[k] = (this.fromColors[k] ?? new THREE.Color()).copy(this.colors[k]);
    for (const k of NUM_KEYS) this.fromNums[k] = this.nums[k];
    this.fromCam.pos.copy(this.cam.pos); this.fromCam.look.copy(this.cam.look); this.fromCam.fov = this.cam.fov;
  }

  // Interpolation à l'avancement s (0..1) : palette légèrement décalée pour que la lumière suive le mouvement
  private apply(s: number) {
    const e = smoother(s);
    const pe = smoother(clamp01((s - 0.06) / 0.88));
    const reducedFade = this.reduced ? smoother(s) : pe;
    for (const k of COLOR_KEYS) {
      if (!this.fromColors[k]) this.fromColors[k] = this.toColors[k].clone();
      this.colors[k].lerpColors(this.fromColors[k], this.toColors[k], reducedFade);
    }
    for (const k of NUM_KEYS) {
      const a = this.fromNums[k] ?? this.toNums[k];
      this.nums[k] = a + (this.toNums[k] - a) * reducedFade;
      this.U[cap(k)].value = this.nums[k];
    }
    const sway = Math.sin(Math.PI * e) * 1.8 * this.sway;
    this.cam.pos.lerpVectors(this.fromCam.pos, this.toCam.pos, e);
    this.cam.look.lerpVectors(this.fromCam.look, this.toCam.look, e);
    this.cam.pos.x += sway; this.cam.look.x += sway * 0.4;
    this.cam.fov = this.fromCam.fov + (this.toCam.fov - this.fromCam.fov) * e + Math.sin(Math.PI * e) * 1.6;

    const n = this.nums, U = this.U;
    (U.uSun.value as THREE.Vector2).set(n.sunX, n.sunY);
    (U.uMoonP.value as THREE.Vector2).set(n.moonX, n.moonY);
    const sx = Math.max(-1, Math.min(1, (n.sunX - 0.5) * 4));
    // En nuit (lune), la lumière vient de la lune
    const lx = n.moonPow > 0.5 ? Math.max(-1, Math.min(1, (n.moonX - 0.5) * 4)) : sx;
    U.uSunSign.value = lx;
    (U.uSunDir.value as THREE.Vector2).set(lx * 0.8 || 0.1, 0.6).normalize();
    this.dustMats.forEach((m) => (m.uniforms.uAmount.value = n.dust));
    this.fireMat.uniforms.uAmount.value = n.fire;
    this.birdMat.uniforms.uAmount.value = n.birds;
    const hor = this.colors.skyHor;
    (this.dustMats[0].uniforms.uColor.value as THREE.Color).copy(this.colors.sunCol).lerp(hor, 0.15);
    for (const m of this.dustMats) (m.uniforms.uColor.value as THREE.Color).copy(this.dustMats[0].uniforms.uColor.value);
    (this.birdMat.uniforms.uColor.value as THREE.Color).copy(this.colors.near).lerp(this.colors.far, 0.3);
  }

  // ---------- API pour l'interface ----------
  setSignal(v: number) { this.signalT = v; this.dirty = true; }
  setBox(on: boolean) {
    if (on && this.boxT < 1) { this.pulseOn = true; this.pulse = 0.001; }
    this.boxT = on ? 1 : 0; this.dirty = true;
  }
  setProgress(p: number) { this.progT = p; this.dirty = true; }

  // ---------- Boucle ----------
  private resize() {
    const w = innerWidth, h = innerHeight;
    this.renderer.setPixelRatio(this.pr);
    this.renderer.setSize(w, h, false);
    const U = this.U;
    (U.uRes.value as THREE.Vector2).set(w * this.pr, h * this.pr);
    U.uResY.value = h * this.pr;
    U.uAspect.value = w / h;
    this.camera.aspect = w / h;
    this.dirty = true;
  }

  start() {
    if (this.running || this.lost) return;
    this.running = true; this.last = performance.now();
    this.raf = requestAnimationFrame(this.loop);
  }
  stop() { this.running = false; cancelAnimationFrame(this.raf); }

  private loop = (now: number) => {
    if (!this.running) return;
    this.raf = requestAnimationFrame(this.loop);
    const dt = Math.min(0.1, (now - this.last) / 1000);
    this.last = now;

    // En mouvement réduit : on ne rend que lorsque quelque chose change.
    const animating = this.tr < 1 || this.signalV !== this.signalT || this.boxV !== this.boxT || this.pulseOn
      || this.progV !== this.progT || this.dirty;
    if (this.reduced && !animating) return;

    if (!this.reduced) this.t += dt;
    const U = this.U;
    U.uTime.value = this.t;

    if (this.tr < 1) {
      this.tr = Math.min(1, this.tr + dt / (this.reduced ? 0.9 : QUALITY.transitionSec));
      this.apply(this.tr);
    }
    // lissages
    const k = 1 - Math.exp(-dt * 2.5);
    this.signalV += (this.signalT - this.signalV) * k;
    if (Math.abs(this.signalT - this.signalV) < 0.002) this.signalV = this.signalT;
    this.boxV += (this.boxT - this.boxV) * (1 - Math.exp(-dt * 1.6));
    if (Math.abs(this.boxT - this.boxV) < 0.002) this.boxV = this.boxT;
    if (this.pulseOn) { this.pulse += dt / 1.8; if (this.pulse >= 1) { this.pulse = 0; this.pulseOn = false; } }
    this.progV += (this.progT - this.progV) * (1 - Math.exp(-dt * 2));
    if (Math.abs(this.progT - this.progV) < 0.001) this.progV = this.progT;
    U.uSignal.value = this.signalV; U.uBox.value = this.boxV; U.uPulse.value = this.pulse;

    // parallaxe souris amortie (coupée en mouvement réduit)
    if (!this.reduced) this.mouseS.lerp(this.mouse, 1 - Math.exp(-dt * 3));
    const px = this.mouseS.x * QUALITY.parallaxX, py = -this.mouseS.y * QUALITY.parallaxY;
    (U.uShift.value as THREE.Vector2).set(-this.mouseS.x * 0.004, this.mouseS.y * 0.003);

    // léger flottement de caméra + dolly lent pendant l'application
    const bob = this.reduced ? 0 : 1;
    const bx = Math.sin(this.t * 0.17) * 0.12 * bob, by = Math.sin(this.t * 0.23 + 1) * 0.06 * bob;
    const dz = -this.progV * 3.2;
    const c = this.camera;
    c.position.set(this.cam.pos.x + px + bx, this.cam.pos.y + py + by, this.cam.pos.z + dz);
    const aspect = c.aspect;
    c.fov = this.cam.fov * Math.max(1, 1.55 / aspect) ** 0.8; // écran étroit : champ plus large
    c.updateProjectionMatrix();
    c.lookAt(this.cam.look.x + px + bx, this.cam.look.y + py + by, this.cam.look.z + dz);
    U.uTanHalf.value = Math.tan((c.fov * Math.PI) / 360);

    this.renderer.render(this.scene, c);
    this.dirty = false;

    // Résolution adaptative : si la moyenne dépasse le budget, on baisse le pixel ratio
    if (QUALITY.adaptive && !this.reduced) {
      this.frameAcc += dt * 1000; this.frameN++;
      if (this.frameN >= 90) {
        if (this.frameAcc / this.frameN > QUALITY.budgetMs && this.pr > QUALITY.minPixelRatio) {
          this.pr = Math.max(QUALITY.minPixelRatio, this.pr - 0.25);
          this.resize();
        }
        this.frameAcc = 0; this.frameN = 0;
      }
    }
  };

  dispose() {
    this.stop();
    this.disposers.forEach((f) => f());
    this.textures.forEach((t) => t.dispose());
    this.scene.traverse((o) => {
      const m = o as THREE.Mesh;
      if (m.geometry) m.geometry.dispose();
      if (m.material) (m.material as THREE.Material).dispose();
    });
    this.renderer.dispose();
  }
}
