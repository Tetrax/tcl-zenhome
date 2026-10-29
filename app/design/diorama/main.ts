// Prototype « Diorama » : machine à 4 écrans (accueil, recherche, choix, terminé)
// pilotant le décor three.js (ou son repli SVG). Données et moteur de recherche simulés.
import { STEPS, type Step } from "./palettes";
import { Diorama, type Backdrop } from "./scene";
import { FallbackBackdrop } from "./fallback";

// ---------- Données : extraites de tv/debloat-mibox4.txt ----------
interface App { pkg: string; name: string; why: string; tag?: "option" | "garder"; group: string }
const APPS: App[] = [
  { pkg: "tv.alphonso.alphonso_eula", name: "Alphonso", why: "Reconnaît ce que vous regardez pour cibler la publicité.", group: "Pistage et pub" },
  { pkg: "com.miui.tv.analytics", name: "Statistiques Xiaomi", why: "Envoie votre usage de la box à Xiaomi.", group: "Pistage et pub" },
  { pkg: "com.google.android.feedback", name: "Rapports Google", why: "Envoie des rapports d’utilisation à Google.", group: "Pistage et pub" },
  { pkg: "com.mitv.tvhome.atv", name: "PatchWall", why: "L’accueil Xiaomi et ses contenus promotionnels.", group: "Pistage et pub" },
  { pkg: "com.mitv.tvhome.michannel", name: "Chaînes Mi", why: "Contenus poussés par Xiaomi.", group: "Pistage et pub" },
  { pkg: "android.autoinstalls.config.xioami.mibox3", name: "Installations automatiques", why: "Installe des applis partenaires sans vous demander.", group: "Pistage et pub" },
  { pkg: "com.google.android.katniss", name: "Recherche vocale", why: "Le bouton micro de la télécommande. Désactivée, elle libère environ 115 Mo de mémoire.", tag: "garder", group: "Au choix" },
  { pkg: "com.google.android.tv", name: "Chaînes en direct", why: "Sert aux tuners TV. La Mi Box n’en a pas, l’appli tourne pour rien.", tag: "option", group: "Au choix" },
  { pkg: "com.google.android.tts", name: "Synthèse vocale", why: "Lit les textes à voix haute. Inutile si personne n’en a besoin, libère environ 300 Mo.", tag: "option", group: "Au choix" },
  { pkg: "com.google.android.marvin.talkback", name: "Lecteur d’écran (TalkBack)", why: "Aide pour les personnes malvoyantes. Inutile si personne n’en a besoin.", tag: "option", group: "Au choix" },
  { pkg: "com.mitv.videoplayer", name: "Lecteur vidéo Xiaomi", why: "Lit les vidéos d’une clé USB. Gardez-le si vous regardez des fichiers sur clé USB.", tag: "option", group: "Au choix" },
  { pkg: "com.xiaomi.mitv.updateservice", name: "Mises à jour Xiaomi", why: "Installe les mises à jour de sécurité de la box. Recommandé de garder.", tag: "garder", group: "Au choix" },
];

// ---------- Utilitaires DOM ----------
const $ = <T extends HTMLElement>(sel: string) => document.querySelector(sel) as T;
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
const wait = (ms: number) => new Promise<void>((r) => setTimeout(r, reduced ? Math.min(ms, 250) : ms));
const params = new URLSearchParams(location.search);

// ---------- Décor : WebGL2 si possible, sinon repli SVG ----------
let backdrop!: Backdrop;
let currentStep: Step = "welcome";

function useFallback() {
  backdrop?.dispose();
  $("#scene").hidden = true;
  backdrop = new FallbackBackdrop($("#fallback"));
  backdrop.goto(currentStep, true);
  if (found) backdrop.setBox(true);
}
function initBackdrop() {
  const canvas = $<HTMLCanvasElement>("#scene");
  const forced = params.get("renderer");
  let gl: WebGL2RenderingContext | null = null;
  if (forced !== "svg") {
    try {
      gl = canvas.getContext("webgl2", {
        failIfMajorPerformanceCaveat: forced !== "gl", // ?renderer=gl force le test sur machine sans GPU
        antialias: false, alpha: false, powerPreference: "high-performance",
      }) as WebGL2RenderingContext | null;
    } catch { gl = null; }
  }
  if (!gl) { useFallback(); return; }
  try {
    backdrop = new Diorama(canvas, gl, useFallback);
  } catch (e) {
    console.warn("Décor WebGL indisponible, repli SVG", e);
    useFallback();
  }
}

// ---------- Écrans ----------
const screens: Record<Step, HTMLElement> = {
  welcome: $("#s-welcome"), search: $("#s-search"), choose: $("#s-choose"), done: $("#s-done"),
};
Object.values(screens).forEach((s) => { s.inert = true; });
document.querySelectorAll<HTMLElement>(".panel, .phase").forEach((p) =>
  Array.from(p.children).forEach((c, i) => (c as HTMLElement).style.setProperty("--i", String(i))));

let busy = false;
let found = false;
let timers: number[] = [];
const later = (fn: () => void, ms: number) => { timers.push(window.setTimeout(fn, reduced ? Math.min(ms, 300) : ms)); };
const clearTimers = () => { timers.forEach(clearTimeout); timers = []; };

function paintChrome(step: Step) {
  document.body.dataset.step = step;
  document.body.dataset.theme = step === "done" ? "night" : "day";
  const idx = STEPS.indexOf(step);
  document.querySelectorAll<HTMLElement>(".steps li").forEach((li, i) => {
    if (i === idx) li.setAttribute("aria-current", "step"); else li.removeAttribute("aria-current");
    li.classList.toggle("past", i < idx);
  });
}

function focusMain(step: Step) {
  const target = screens[step].querySelector<HTMLElement>(
    step === "welcome" ? '[data-action="start"]' : step === "done" ? '[data-action="close"]' : step === "choose" ? "#apps input" : "h2");
  if (step === "search") { const h = target!; h.tabIndex = -1; h.focus({ preventScroll: true }); return; }
  target?.focus({ preventScroll: true });
}

async function go(next: Step, instant = false) {
  if (busy) return;
  busy = true;
  clearTimers();
  const prev = screens[currentStep];
  const first = instant;
  if (!first && next !== currentStep) {
    prev.inert = true; prev.classList.add("leaving");
    await wait(450);
    prev.classList.remove("show", "leaving");
  }
  currentStep = next;
  paintChrome(next);
  backdrop.goto(next, instant);
  if (!instant) await wait(900); // la caméra est déjà en route : le panneau arrive en cours de travelling
  const s = screens[next];
  s.classList.add("show"); s.inert = false;
  focusMain(next);
  busy = false;
  if (next === "search") startSearch();
  if (next === "done") $("#s-done").setAttribute("aria-busy", "false");
}

// ---------- Écran 2 : recherche (faux moteur, ~3 s) ----------
function startSearch() {
  found = false;
  $("#s-search").classList.remove("is-found");
  $(".searching").hidden = false;
  $("#found").hidden = true; $("#manual").hidden = true;
  $("#search-status").textContent = "Nous écoutons le réseau de la maison.";
  backdrop.setBox(false); backdrop.setSignal(1);
  later(() => { $("#search-status").textContent = "Un appareil répond…"; }, 1800);
  later(() => onFound("192.168.50.14"), 3300);
}
function onFound(ip: string) {
  found = true;
  backdrop.setSignal(0); backdrop.setBox(true);
  $("#found-ip").textContent = ip;
  $("#search-status").textContent = "Appareil trouvé.";
  $("#s-search").classList.add("is-found");
  $(".searching").hidden = true;
  $("#found").hidden = false; $("#manual").hidden = true;
  $<HTMLElement>('[data-action="confirm"]').focus({ preventScroll: true });
}

const ipRe = /^(25[0-5]|2[0-4]\d|1?\d?\d)(\.(25[0-5]|2[0-4]\d|1?\d?\d)){3}$/;
$("#manual").addEventListener("submit", (e) => {
  e.preventDefault();
  const v = $<HTMLInputElement>("#ip").value.trim();
  if (!ipRe.test(v)) {
    $("#ip-err").textContent = "Adresse non valide. Exemple : 192.168.1.20";
    $("#ip").setAttribute("aria-invalid", "true");
    return;
  }
  $("#ip-err").textContent = ""; $("#ip").removeAttribute("aria-invalid");
  $("#found-ip").textContent = v;
  go("choose");
});
$("#manual").addEventListener("keydown", (e) => {
  if ((e as KeyboardEvent).key === "Escape") { e.stopPropagation(); backToFound(); }
});
function backToFound() {
  $("#manual").hidden = true; $("#found").hidden = false;
  $<HTMLElement>('[data-action="other"]').focus({ preventScroll: true });
}

// ---------- Écran 3 : liste à cocher ----------
const list = $("#apps");
const checkSvg = '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5l3.2 3L13 4.5"/></svg>';
let lastGroup = "";
for (const a of APPS) {
  if (a.group !== lastGroup) {
    lastGroup = a.group;
    const g = document.createElement("li"); g.className = "group"; g.setAttribute("aria-hidden", "true"); g.textContent = a.group;
    list.append(g);
  }
  const li = document.createElement("li");
  const label = document.createElement("label"); label.className = "row-app";
  const input = document.createElement("input");
  input.type = "checkbox"; input.value = a.pkg;
  input.checked = a.tag !== "garder"; // sans balise et [option] : cochées ; [option:garder] : décochées
  label.append(input);
  label.insertAdjacentHTML("beforeend",
    `<span class="box">${checkSvg}</span><span class="txt"><b>${a.name}${a.tag === "garder" ? '<i class="tag">Recommandé : garder</i>' : ""}</b><span>${a.why}</span></span>`);
  li.append(label); list.append(li);
}
const inputs = () => Array.from(list.querySelectorAll<HTMLInputElement>("input"));
const countEl = $("#count"), toggleAll = $<HTMLButtonElement>("#toggle-all"), applyBtn = $<HTMLButtonElement>("#apply");
const checkedN = () => inputs().filter((i) => i.checked).length;
function refresh() {
  const n = checkedN();
  countEl.textContent = n === 0 ? "Aucune appli ne sera désactivée" : n === 1 ? "1 appli sera désactivée" : `${n} applis seront désactivées`;
  countEl.classList.remove("tick"); void countEl.offsetWidth; countEl.classList.add("tick");
  toggleAll.textContent = n === inputs().length ? "Tout désélectionner" : "Tout sélectionner";
  applyBtn.disabled = n === 0;
}
list.addEventListener("change", refresh);
toggleAll.addEventListener("click", () => {
  const all = checkedN() !== inputs().length;
  inputs().forEach((i) => (i.checked = all));
  refresh();
});
refresh();

async function apply() {
  const chosen = inputs().filter((i) => i.checked).map((i) => APPS.find((a) => a.pkg === i.value)!);
  $("#n-off").textContent = String(chosen.length);
  $("#n-keep").textContent = String(APPS.length - chosen.length);
  $("#phase-pick").hidden = true; $("#phase-run").hidden = false;
  const bar = $("#bar"), fill = bar.firstElementChild as HTMLElement, label = $("#run-label"), pct = $("#percent");
  busy = true;
  backdrop.setProgress(1);
  const total = reduced ? 800 : 4200, t0 = performance.now();
  let idx = -1;
  await new Promise<void>((res) => {
    const tick = () => {
      const p = Math.min(1, (performance.now() - t0) / total);
      const e = p < 0.5 ? 2 * p * p : 1 - Math.pow(-2 * p + 2, 2) / 2; // départ et fin doux
      fill.style.transform = `scaleX(${e})`;
      pct.textContent = `${Math.round(e * 100)} %`;
      bar.setAttribute("aria-valuenow", String(Math.round(e * 100)));
      const i = Math.min(chosen.length - 1, Math.floor(e * chosen.length));
      if (i !== idx && chosen[i]) { idx = i; label.textContent = `Désactivation de ${chosen[i].name}…`; }
      if (p < 1) requestAnimationFrame(tick); else res();
    };
    requestAnimationFrame(tick);
  });
  label.textContent = "Terminé.";
  await wait(500);
  busy = false;
  backdrop.setProgress(0);
  await go("done");
  $("#phase-pick").hidden = false; $("#phase-run").hidden = true; // remis à zéro pour un nouveau passage
  fill.style.transform = "scaleX(0)";
}

// ---------- Actions ----------
document.addEventListener("click", (ev) => {
  const t = (ev.target as HTMLElement).closest<HTMLElement>("[data-action]");
  if (!t) return;
  switch (t.dataset.action) {
    case "start": go("search"); break;
    case "confirm": go("choose"); break;
    case "other":
      $("#found").hidden = true; $("#manual").hidden = false;
      $<HTMLInputElement>("#ip").value = ""; $("#ip-err").textContent = "";
      $<HTMLInputElement>("#ip").focus({ preventScroll: true });
      break;
    case "back": backToFound(); break;
    case "apply": apply(); break;
    case "close":
      // En application Tauri : fermer la fenêtre. Ici (prototype web) : on rejoue le parcours.
      go("welcome");
      break;
  }
});

// ---------- Démarrage ----------
initBackdrop();
const start = (params.get("step") as Step) || "welcome";
if (STEPS.includes(start)) {
  currentStep = start;
  paintChrome(start);
  backdrop.goto(start, true);
  if (start === "choose" || start === "done") { found = true; backdrop.setBox(true); } // cabane allumée (lien de debug)
  screens[start].classList.add("show"); screens[start].inert = false;
  focusMain(start);
  if (start === "search") startSearch();
  if (start === "done") { $("#n-off").textContent = "9"; $("#n-keep").textContent = "3"; }
} else {
  screens.welcome.classList.add("show"); screens.welcome.inert = false;
  focusMain("welcome");
}
