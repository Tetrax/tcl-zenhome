// Génération procédurale des silhouettes découpées (une canvas 2D par couche).
// Canaux : R = ombrage des facettes (0.5 neutre), G = 255 pour la végétation (sert aux ombres
// longues), A = silhouette. Aucune image externe.

export const TEX_W = 2048;
export const TEX_H = 512;
export const BASE = 0.62; // fraction de la hauteur de canvas où se trouve la ligne de sol

export type Kind = "peaks" | "ridge" | "conifer" | "broadleaf" | "hills" | "frame" | "grass";

export interface LayerDef {
  id: string;
  z: number;
  kind: Kind;
  seed: number;
  t: number;          // 0 lointain … 1 proche (mélange far -> near)
  acc: number;        // part de teinte d'accent
  gy: number;         // altitude du sol (monde)
  amp: number;        // amplitude du relief (unités monde)
  tree?: [number, number]; // hauteurs min/max des arbres
  valley?: number;    // creux central (unités) qui laisse voir le lac
  valleyW?: number;
  cabin?: number;     // abscisse monde de la cabane (couche portant la lumière)
  receive?: boolean;  // reçoit les ombres longues
  w?: number;         // largeur monde imposée
}

// Largeur monde d'une couche : couvre le champ de vision depuis la caméra de départ, avec marge.
export const layerWidth = (d: LayerDef) => d.w ?? 1.3 * 1.12 * (14.5 - d.z) + 10;

function rng(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function makeNoise(seed: number) {
  const r = rng(seed);
  const P = new Float32Array(512);
  for (let i = 0; i < 512; i++) P[i] = r();
  const n = (x: number) => {
    const i = Math.floor(x), f = x - i, s = f * f * (3 - 2 * f);
    return P[i & 511] * (1 - s) + P[(i + 1) & 511] * s;
  };
  const fbm = (x: number, o = 4) => {
    let a = 0.5, s = 0, fr = 1;
    for (let k = 0; k < o; k++) { s += a * n(x * fr + k * 17.3); fr *= 2.03; a *= 0.5; }
    return s / (1 - Math.pow(0.5, o));
  };
  return { n, fbm };
}

export interface Painted {
  canvas: HTMLCanvasElement;
  window?: { px: number; py: number }; // fenêtre de la cabane (px canvas)
}

export function paintLayer(def: LayerDef, W: number): Painted {
  const cv = document.createElement("canvas");
  cv.width = TEX_W; cv.height = TEX_H;
  const ctx = cv.getContext("2d")!;
  const ppu = TEX_W / W;             // pixels par unité monde
  const B = TEX_H * BASE;
  const R = rng(def.seed);
  const N = makeNoise(def.seed * 7 + 1);
  const ux = (px: number) => px / ppu - W / 2;

  // Creux central (lac) : enveloppe 0..1
  const env = (u: number) => (def.valley ? Math.exp(-Math.pow(u / (def.valleyW ?? 14), 2)) : 0);

  // Relief en unités monde
  const height = (u: number): number => {
    let h: number;
    if (def.kind === "peaks") {
      let s = 0, a = 0.5, f = 0.045, tot = 0;
      for (let k = 0; k < 5; k++) { s += a * (1 - Math.abs(2 * N.n(u * f + k * 9.1) - 1)); tot += a; f *= 2.1; a *= 0.5; }
      h = def.amp * (0.18 + 0.82 * Math.pow(s / tot, 1.35));
    } else if (def.kind === "grass") {
      h = def.amp * (N.fbm(u * 0.06, 3) - 0.3);
    } else {
      h = def.amp * (0.2 + 0.8 * N.fbm(u * 0.04, 4)) + 0.25 * def.amp * (N.n(u * 0.4 + 5) - 0.5);
    }
    h -= (def.valley ?? 0) * env(u);
    return Math.max(h, -(1 - BASE) * 0.95 * (W / 4));
  };
  const groundY = (px: number) => B - height(ux(px)) * ppu;

  // --- sol / relief ---
  ctx.beginPath();
  ctx.moveTo(0, TEX_H);
  for (let px = 0; px <= TEX_W; px += 2) ctx.lineTo(px, groundY(px));
  ctx.lineTo(TEX_W, TEX_H);
  ctx.closePath();
  ctx.fillStyle = "rgb(128,0,0)";
  ctx.fill();

  if (def.kind === "peaks") {
    // Facettes : arêtes qui descendent en diagonale depuis les crêtes ; face à gauche = claire, à droite = sombre.
    const hs = new Float32Array(TEX_W);
    for (let px = 0; px < TEX_W; px++) hs[px] = height(ux(px));
    const at = (x: number) => hs[Math.max(0, Math.min(TEX_W - 1, Math.round(x)))];
    const img = ctx.getImageData(0, 0, TEX_W, TEX_H);
    const D = 230;
    for (let px = 0; px < TEX_W; px++) {
      const top = Math.max(0, Math.floor(groundY(px)));
      for (let y = top; y < Math.min(TEX_H, top + D); y++) {
        const d = y - top, sx = px - d * 0.55;
        const slope = at(sx + 9) - at(sx - 9);
        const f = 1 - d / D;
        const s = 0.5 + Math.max(-0.42, Math.min(0.42, slope * 0.55)) * f * f;
        const i = (y * TEX_W + px) * 4;
        if (img.data[i + 3] > 0) img.data[i] = Math.round(s * 255);
      }
    }
    ctx.putImageData(img, 0, 0);
  }

  const trees = (draw: (x: number, by: number, ht: number, shade: number) => void, rows: number, gap: number) => {
    const [hmin, hmax] = def.tree ?? [3, 6];
    for (let row = 0; row < rows; row++) {
      const shade = Math.round(150 - row * (rows > 1 ? 52 / (rows - 1) : 0));
      let x = R() * 12;
      while (x < TEX_W) {
        const u = ux(x);
        const ht = (hmin + R() * (hmax - hmin)) * (0.8 + 0.4 * N.n(u * 0.2 + row * 5));
        const w = ht * 0.42 * ppu;
        const skip =
          env(u) > 0.42 || (def.cabin !== undefined && Math.abs(u - def.cabin) < 2.6);
        if (!skip) draw(x, groundY(x) + row * 0.3 * ppu, ht * ppu, shade + Math.round((R() - 0.5) * 22));
        x += (w * (0.3 + R() * 0.5)) * gap + 2;
      }
    }
  };

  // Sapin stylisé : étages qui s'élargissent, base à y = by
  const fir = (cx: number, by: number, h: number, w: number, shade: number) => {
    const tiers = 4 + Math.floor(R() * 3);
    const top = by - h;
    ctx.fillStyle = `rgb(${shade},255,0)`;
    ctx.beginPath();
    ctx.moveTo(cx, top);
    const pts: [number, number][] = [];
    for (let i = 1; i <= tiers; i++) {
      const f = i / tiers;
      const y = top + h * 0.93 * f;
      const ho = (w / 2) * (0.25 + 0.75 * f) * (0.9 + R() * 0.2);
      pts.push([ho, y]);
      pts.push([ho * 0.5, y - 0.03 * h]);
    }
    for (const [ho, y] of pts) ctx.lineTo(cx + ho, y);
    ctx.lineTo(cx + w * 0.05, by - 0.05 * h);
    ctx.lineTo(cx + w * 0.05, by + 4);
    ctx.lineTo(cx - w * 0.05, by + 4);
    ctx.lineTo(cx - w * 0.05, by - 0.05 * h);
    for (let i = pts.length - 1; i >= 0; i--) ctx.lineTo(cx - pts[i][0], pts[i][1]);
    ctx.closePath();
    ctx.fill();
  };

  // Feuillu : tronc fin + houppier fait de disques
  const leafy = (cx: number, by: number, h: number, shade: number) => {
    ctx.fillStyle = `rgb(${shade},255,0)`;
    ctx.fillRect(cx - h * 0.035, by - h * 0.5, h * 0.07, h * 0.5 + 4);
    const n = 5 + Math.floor(R() * 4);
    for (let i = 0; i < n; i++) {
      const a = (i / n) * Math.PI * 2 + R();
      const rr = h * (0.17 + R() * 0.1);
      ctx.beginPath();
      ctx.arc(cx + Math.cos(a) * h * 0.14, by - h * 0.68 + Math.sin(a) * h * 0.13, rr, 0, Math.PI * 2);
      ctx.fill();
    }
  };

  let win: Painted["window"];

  switch (def.kind) {
    case "conifer":
      trees((x, by, ht, s) => fir(x, by, ht, ht * 0.42, s), 3, 1);
      break;
    case "broadleaf":
      trees((x, by, ht, s) => leafy(x, by, ht, s), 2, 1.5);
      break;
    case "frame": {
      // Grands arbres de cadrage sur les côtés, centre dégagé
      const open = def.valleyW ?? 12;
      for (let row = 0; row < 3; row++) {
        let x = R() * 20;
        while (x < TEX_W) {
          const u = ux(x);
          const edge = Math.max(0, Math.abs(u) - open) / open;
          if (edge > 0) {
            const [hmin, hmax] = def.tree ?? [8, 13];
            const ht = (hmin + R() * (hmax - hmin)) * Math.min(1, 0.45 + edge * 0.6) * ppu;
            fir(x, groundY(x) + row * 0.4 * ppu, ht, ht * 0.4, 70 + row * 14 + Math.round(R() * 16));
          }
          x += (40 + R() * 70) * (row === 0 ? 0.9 : 1.2);
        }
      }
      break;
    }
    case "hills":
      // quelques touffes et buissons sur la crête
      ctx.fillStyle = "rgb(120,255,0)";
      for (let px = 0; px < TEX_W; px += 14 + R() * 40) {
        if (env(ux(px)) > 0.5) continue;
        const r = (0.2 + R() * 0.45) * ppu;
        ctx.beginPath(); ctx.ellipse(px, groundY(px), r * 1.6, r, 0, Math.PI, 0); ctx.fill();
      }
      break;
    case "grass": {
      // Herbes hautes : lames courbes, quelques tiges à fleur
      const [hmin, hmax] = def.tree ?? [0.8, 2.2];
      for (let i = 0; i < 1500; i++) {
        const px = R() * TEX_W;
        const u = ux(px);
        const clump = 0.35 + 0.65 * N.fbm(u * 0.5 + 40, 2);
        const open = 1 - 0.75 * env(u);
        const hh = (hmin + R() * (hmax - hmin)) * clump * open * ppu;
        const by = groundY(px) + 3;
        const bw = (0.05 + R() * 0.07) * ppu;
        const bend = (R() - 0.5) * hh * 0.45;
        const sh = 70 + Math.round(R() * 50);
        ctx.fillStyle = `rgb(${sh},255,0)`;
        ctx.beginPath();
        ctx.moveTo(px - bw, by);
        ctx.quadraticCurveTo(px - bw * 0.3 + bend * 0.4, by - hh * 0.6, px + bend, by - hh);
        ctx.quadraticCurveTo(px + bw * 0.3 + bend * 0.6, by - hh * 0.6, px + bw, by);
        ctx.fill();
        if (R() < 0.05) { // fleur / graine
          ctx.beginPath(); ctx.arc(px + bend, by - hh, bw * 1.5, 0, Math.PI * 2); ctx.fill();
        }
      }
      break;
    }
  }

  // Cabane au loin (couche portant la lumière)
  if (def.cabin !== undefined) {
    const cx = (def.cabin + W / 2) * ppu;
    const by = groundY(cx) + 0.15 * ppu;
    const w = 3.0 * ppu, h = 1.35 * ppu;
    ctx.fillStyle = "rgb(108,0,0)";
    ctx.fillRect(cx - w / 2, by - h, w, h + 3);
    ctx.fillStyle = "rgb(150,0,0)"; // toit
    ctx.beginPath();
    ctx.moveTo(cx - w * 0.62, by - h); ctx.lineTo(cx, by - h - 1.2 * ppu); ctx.lineTo(cx + w * 0.62, by - h);
    ctx.closePath(); ctx.fill();
    ctx.fillRect(cx + w * 0.18, by - h - 1.2 * ppu, 0.3 * ppu, 0.8 * ppu); // cheminée
    win = { px: cx - w * 0.12, py: by - h * 0.5 };
  }

  return { canvas: cv, window: win };
}
