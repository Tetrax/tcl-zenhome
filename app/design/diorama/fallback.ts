// Repli sans WebGL2 : couches SVG statiques par heure, transitions CSS sur les couleurs.
import { PALETTES, STEPS, mixHex, type Step } from "./palettes";
import type { Backdrop } from "./scene";

const NS = "http://www.w3.org/2000/svg";
const VW = 1600, VH = 900;

const el = <K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string | number> = {}) => {
  const e = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  return e;
};

// Petit bruit déterministe pour les crêtes
function ridge(seed: number, base: number, amp: number, jag: number): string {
  let s = seed;
  const r = () => ((s = (Math.imul(s, 1664525) + 1013904223) >>> 0) / 4294967296);
  const pts: number[] = [];
  for (let i = 0; i < 18; i++) pts.push(r());
  const h = (x: number) => {
    const f = (x / VW) * 16, i = Math.floor(f), t = f - i, u = t * t * (3 - 2 * t);
    return pts[i % 18] * (1 - u) + pts[(i + 1) % 18] * u;
  };
  let d = `M0 ${VH}`;
  for (let x = 0; x <= VW; x += 16) {
    const y = base - amp * h(x) - jag * (((x / 16) | 0) % 2 ? 1 : 0) * r();
    d += ` L${x} ${y.toFixed(1)}`;
  }
  return d + ` L${VW} ${VH} Z`;
}

function grass(): string {
  let s = 9;
  const r = () => ((s = (Math.imul(s, 1664525) + 1013904223) >>> 0) / 4294967296);
  let d = `M0 ${VH}`;
  for (let x = 0; x < VW; x += 14) {
    const h = 40 + r() * 70 * (Math.abs(x - VW / 2) / (VW / 2) + 0.2);
    d += ` L${x} ${VH - 20} L${x + 7 + (r() - 0.5) * 16} ${VH - 20 - h} L${x + 14} ${VH - 20}`;
  }
  return d + ` L${VW} ${VH} Z`;
}

export class FallbackBackdrop implements Backdrop {
  private svg = el("svg", { viewBox: `0 0 ${VW} ${VH}`, preserveAspectRatio: "xMidYMax slice", class: "fb-svg" });
  private stops: SVGStopElement[] = [];
  private paths: SVGPathElement[] = [];
  private groups: SVGGElement[] = [];
  private sun = el("circle", { r: 46, class: "fb-sun" });
  private moon = el("circle", { r: 40, class: "fb-sun" });
  private lamp = el("circle", { cx: 1010, cy: 470, r: 6, class: "fb-lamp" });
  private stars = el("g", { class: "fb-stars" });
  private n = 7;

  constructor(host: HTMLElement) {
    const defs = el("defs");
    const g = el("linearGradient", { id: "fb-sky", x1: 0, y1: 0, x2: 0, y2: 1 });
    [0, 0.55, 1].forEach((o) => { const s = el("stop", { offset: o }); g.append(s); this.stops.push(s); });
    defs.append(g);
    this.svg.append(defs, el("rect", { width: VW, height: VH, fill: "url(#fb-sky)" }), this.stars, this.sun, this.moon);

    let seed = 3;
    for (let i = 0; i < 90; i++) {
      seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
      const a = seed / 4294967296;
      seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
      const b = seed / 4294967296;
      this.stars.append(el("circle", { cx: a * VW, cy: b * VH * 0.5, r: 1 + (a * 7 % 1) * 1.2 }));
    }
    // 7 couches du fond vers l'avant ; la dernière est l'herbe haute
    const defsL = [[560, 240, 90], [610, 170, 60], [660, 120, 40], [700, 90, 30], [760, 70, 20], [820, 60, 20]];
    defsL.forEach(([base, amp, jag], i) => {
      const grp = el("g", { class: "fb-layer" });
      const p = el("path", { d: ridge(11 + i * 13, base, amp, jag), class: "fb-path" });
      grp.append(p); this.groups.push(grp); this.paths.push(p);
      this.svg.append(grp);
      if (i === 2) { // le lac, derrière les collines proches
        this.svg.append(el("rect", { x: 0, y: 640, width: VW, height: 260, class: "fb-water" }));
      }
      if (i === 1) this.svg.append(this.lamp);
    });
    const gp = el("g", { class: "fb-layer" });
    const p = el("path", { d: grass(), class: "fb-path" });
    gp.append(p); this.groups.push(gp); this.paths.push(p); this.svg.append(gp);
    host.append(this.svg);
    this.goto("welcome");
  }

  goto(step: Step) {
    const p = PALETTES[step];
    const idx = STEPS.indexOf(step);
    this.stops[0].style.stopColor = p.skyTop;
    this.stops[1].style.stopColor = p.skyMid;
    this.stops[2].style.stopColor = p.skyHor;
    this.paths.forEach((path, i) => {
      const t = i / (this.paths.length - 1);
      const c = mixHex(mixHex(p.far, p.near, t), p.haze, Math.max(0, 0.75 - t * 0.85) * (0.5 + p.fog * 25));
      path.style.fill = c;
    });
    // « Travelling » : chaque couche grossit d'autant plus qu'elle est proche
    this.groups.forEach((g, i) => {
      const k = i / (this.groups.length - 1);
      g.style.transform = `scale(${1 + idx * 0.05 * (0.3 + k)}) translateY(${idx * k * -6}px)`;
    });
    this.svg.style.setProperty("--fb-water-a", p.waterA);
    this.svg.style.setProperty("--fb-water-b", p.waterB);
    this.sun.style.fill = p.sunCol;
    this.sun.style.opacity = String(p.sunPow);
    this.sun.style.transform = `translate(${p.sunX * VW}px, ${(1 - p.sunY) * VH}px)`;
    this.moon.style.fill = "#eef2ff";
    this.moon.style.opacity = String(p.moonPow);
    this.moon.style.transform = `translate(${p.moonX * VW}px, ${(1 - p.moonY) * VH}px)`;
    this.stars.style.opacity = String(p.stars);
    this.lamp.style.opacity = idx >= 1 && this.on ? "1" : "0";
  }

  private on = false;
  setBox(on: boolean) { this.on = on; this.lamp.style.opacity = on ? "1" : "0"; }
  setSignal() {}
  setProgress() {}
  dispose() { this.svg.remove(); }
}
