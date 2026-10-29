// Machine à 4 écrans : Accueil -> Recherche -> Choix (puis progression) -> Terminé.
// Le « moteur » est simulé (aucun appel réseau, aucun adb).
import { creerScene, type Clairiere, type Heure } from "./scene";

const $ = <T extends HTMLElement>(s: string) => document.querySelector<T>(s)!;
const params = new URLSearchParams(location.search);
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, reduced ? ms * 0.5 : ms));

// ---- Applis (extraits réels de tv/debloat-mibox4.txt) ----
// balise : "" = à désactiver ; "option" = au choix, désactivée ; "garder" = au choix, gardée.
type Balise = "" | "option" | "garder";
const APPLIS: { pkg: string; nom: string; txt: string; b: Balise }[] = [
  { pkg: "tv.alphonso.alphonso_eula", nom: "Alphonso", txt: "Reconnaît ce que vous regardez pour cibler la publicité.", b: "" },
  { pkg: "com.miui.tv.analytics", nom: "Statistiques Xiaomi", txt: "Envoie votre usage de la box à Xiaomi.", b: "" },
  { pkg: "com.google.android.feedback", nom: "Rapports Google", txt: "Envoie des rapports d'utilisation à Google.", b: "" },
  { pkg: "com.mitv.tvhome.atv", nom: "PatchWall", txt: "L'accueil Xiaomi et ses contenus promotionnels.", b: "" },
  { pkg: "com.xiaomi.android.tvsetup.partnercustomizer", nom: "Rangées partenaires", txt: "Ajoute des publicités de partenaires à l'accueil.", b: "" },
  { pkg: "com.google.android.tvrecommendations", nom: "Recommandations Google", txt: "Suggestions de l'ancien accueil, inutiles avec Zen Home.", b: "" },
  { pkg: "com.google.android.katniss", nom: "Recherche vocale", txt: "Le bouton micro de la télécommande. Si vous la désactivez, le micro ne fera plus rien, mais vous libérez environ 115 Mo de mémoire.", b: "garder" },
  { pkg: "com.google.android.tv", nom: "Chaînes en direct", txt: "Sert aux tuners TV. La Mi Box n'en a pas, l'appli tourne pour rien.", b: "option" },
  { pkg: "com.google.android.tts", nom: "Synthèse vocale", txt: "Lit les textes à voix haute pour le lecteur d'écran. Inutile si personne n'en a besoin, libère environ 300 Mo.", b: "option" },
  { pkg: "com.google.android.marvin.talkback", nom: "Lecteur d'écran (TalkBack)", txt: "Aide pour les personnes malvoyantes. Inutile si personne n'en a besoin.", b: "option" },
  { pkg: "com.mitv.milinkservice", nom: "Mi Link", txt: "Diffuser depuis un téléphone Xiaomi. Le Chromecast intégré reste disponible pour tous les téléphones.", b: "option" },
  { pkg: "com.xiaomi.mitv.updateservice", nom: "Mises à jour Xiaomi", txt: "Installe les mises à jour de sécurité de la box. Recommandé de garder.", b: "garder" },
];

// ---- Scène (avec repli CSS si WebGL2 indisponible) ----
let scene: Clairiere | null = null;
const MODE_GL = params.get("gl"); // ?gl=off force le repli ; ?gl=force ignore le test de performance
function initScene() {
  const canvas = $<HTMLCanvasElement>("#scene");
  const gl = MODE_GL === "off" ? null : (canvas.getContext("webgl2", { failIfMajorPerformanceCaveat: MODE_GL !== "force", antialias: true, alpha: false, powerPreference: "high-performance" }) as WebGL2RenderingContext | null);
  if (!gl) { document.body.classList.add("nogl"); return; }
  try {
    scene = creerScene(canvas, gl, reduced, (perdu) => document.body.classList.toggle("nogl", perdu));
  } catch (e) {
    console.warn("Scène 3D indisponible, repli CSS :", e);
    document.body.classList.add("nogl");
  }
}

function heure(h: Heure, opts?: { duration?: number; delay?: number }) {
  document.body.dataset.heure = h; // pilote l'accent, le verre et le repli CSS
  scene?.go(h, opts);
}

// ---- Navigation entre écrans ----
const timers: number[] = [];
function afficher(n: number) {
  const ancien = document.querySelector<HTMLElement>(".screen.is-active");
  const nouveau = document.querySelector<HTMLElement>(`.screen[data-screen="${n}"]`)!;
  if (ancien) {
    ancien.classList.remove("is-active");
    setTimeout(() => { ancien.hidden = true; ancien.inert = true; }, 700);
  }
  const delai = ancien ? 450 : 0;
  setTimeout(() => {
    nouveau.hidden = false; nouveau.inert = false;
    void nouveau.offsetWidth; // force le recalcul pour lancer les transitions
    nouveau.classList.add("is-active");
    setTimeout(() => nouveau.querySelector<HTMLElement>(".btn.primary:not(:disabled), h1, h2")?.focus({ preventScroll: true }), 1000);
  }, delai);
  document.querySelectorAll<HTMLElement>(".steps li").forEach((li) => {
    const k = Number(li.dataset.step);
    li.toggleAttribute("aria-current", k === n); if (k === n) li.setAttribute("aria-current", "step");
    li.classList.toggle("fait", k < n);
  });
}

// Écran 1 -> 2
function versRecherche() {
  heure("day");
  afficher(2);
  scene?.setBalise("search");
  timers.push(window.setTimeout(boxTrouvee, 3300)); // faux moteur : trouvée après ~3 s
}
function boxTrouvee() {
  $("#panel-recherche").dataset.etat = "trouve";
  scene?.setBalise("found");
  setTimeout(() => $<HTMLButtonElement>("#cest-elle").focus({ preventScroll: true }), 800);
}

// Adresse manuelle
const reIPv4 = /^(25[0-5]|2[0-4]\d|1?\d?\d)(\.(25[0-5]|2[0-4]\d|1?\d?\d)){3}$/;
function initAdresse() {
  const bouton = $<HTMLButtonElement>("#autre-adresse"), zone = $("#autre"), champ = $<HTMLInputElement>("#champ-ip"), err = $("#erreur-ip");
  bouton.addEventListener("click", () => {
    const ouvert = zone.classList.toggle("ouvert");
    bouton.setAttribute("aria-expanded", String(ouvert));
    if (ouvert) setTimeout(() => champ.focus(), 350);
  });
  $("#form-ip").addEventListener("submit", (e) => {
    e.preventDefault();
    const v = champ.value.trim();
    if (!reIPv4.test(v)) {
      err.textContent = "Cette adresse n'a pas l'air valide (exemple : 192.168.1.20).";
      champ.classList.remove("invalide"); void champ.offsetWidth; champ.classList.add("invalide");
      return;
    }
    err.textContent = ""; $("#ip").textContent = v;
    zone.classList.remove("ouvert"); bouton.setAttribute("aria-expanded", "false");
    $<HTMLButtonElement>("#cest-elle").focus();
  });
}

// ---- Écran 3 : liste ----
const coche = '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3.5 8.5 3 3 6-7"/></svg>';
function construireListe() {
  const ul = $("#liste");
  const groupes: [string, (a: (typeof APPLIS)[number]) => boolean][] = [
    ["Pistage et publicité", (a) => a.b === ""],
    ["Au choix", (a) => a.b !== ""],
  ];
  for (const [titre, filtre] of groupes) {
    ul.insertAdjacentHTML("beforeend", `<li class="groupe" role="presentation">${titre}</li>`);
    for (const a of APPLIS.filter(filtre)) {
      const puce = a.b === "garder" ? '<span class="puce garder">Mieux vaut garder</span>' : a.b === "option" ? '<span class="puce">Au choix</span>' : "";
      ul.insertAdjacentHTML("beforeend",
        `<li><label class="ligne"><input type="checkbox" data-pkg="${a.pkg}" ${a.b === "garder" ? "" : "checked"} /><span class="case">${coche}</span><span class="texte"><span class="nom">${a.nom}${puce}</span><span class="expl">${a.txt}</span></span></label></li>`);
    }
  }
  ul.addEventListener("change", majCompteur);
  // Entrée coche aussi (Espace le fait déjà) ; flèches haut/bas pour naviguer.
  ul.addEventListener("keydown", (e) => {
    const cases = [...ul.querySelectorAll<HTMLInputElement>("input")], i = cases.indexOf(e.target as HTMLInputElement);
    if (i < 0) return;
    if (e.key === "Enter") { e.preventDefault(); cases[i].click(); }
    if (e.key === "ArrowDown" || e.key === "ArrowUp") { e.preventDefault(); cases[Math.max(0, Math.min(cases.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)))].focus(); }
  });
  $("#tout").addEventListener("click", () => {
    const cases = ul.querySelectorAll<HTMLInputElement>("input"), tout = [...cases].every((c) => c.checked);
    cases.forEach((c) => (c.checked = !tout));
    majCompteur();
  });
  majCompteur();
}
const cochees = () => [...document.querySelectorAll<HTMLInputElement>("#liste input:checked")];
let dernier = -1;
function majCompteur() {
  const n = cochees().length, total = APPLIS.length;
  const c = $("#compteur");
  const texte = n === 0 ? "Aucune appli ne sera désactivée" : n === 1 ? "<b>1</b> appli sera désactivée" : `<b>${n}</b> applis seront désactivées`;
  c.innerHTML = texte;
  if (n !== dernier && dernier >= 0) { const b = c.querySelector("b"); b?.classList.remove("saut"); void b?.offsetWidth; b?.classList.add("saut"); }
  dernier = n;
  $<HTMLButtonElement>("#appliquer").disabled = n === 0;
  $("#tout").textContent = n === total ? "Tout désélectionner" : "Tout sélectionner";
}

// ---- Progression simulée (~4 s) ----
async function appliquer() {
  const choisies = cochees().map((c) => APPLIS.find((a) => a.pkg === c.dataset.pkg)!);
  const panel = $("#panel-choix"), btn = $<HTMLButtonElement>("#appliquer");
  btn.classList.add("busy");
  panel.style.minHeight = `${panel.offsetHeight}px`;
  await sleep(350);
  panel.classList.add("applique");
  panel.querySelector(".pane-progres")!.setAttribute("aria-hidden", "false");
  heure("dusk", { duration: 4.4 }); // le soleil se couche pendant que ça travaille

  const etapes = ["Connexion à la Mi Box", "Sauvegarde de l'état actuel", "Désactivation des applis", "Vérification"];
  const ol = $("#etapes"); ol.innerHTML = etapes.map((e) => `<li><i></i><span>${e}</span></li>`).join("");
  const lis = [...ol.children] as HTMLElement[];
  const seuils = [0, 0.16, 0.34, 0.86], arc = $("#arc"), pct = $("#pourcent"), titre = $("#progres-titre");
  const DUREE = reduced ? 2200 : 4200, debut = performance.now();
  let eta = -1;
  await new Promise<void>((fin) => {
    const tick = (now: number) => {
      const x = Math.min(1, (now - debut) / DUREE), p = x < 0.5 ? 2 * x * x : 1 - Math.pow(-2 * x + 2, 2) / 2;
      arc.style.strokeDashoffset = String(326.7 * (1 - p));
      pct.textContent = `${Math.round(p * 100)} %`;
      const e = seuils.filter((s) => p >= s).length - 1;
      if (e !== eta) { eta = e; lis.forEach((li, i) => { li.className = i < e ? "fait" : i === e ? "en-cours" : ""; }); }
      if (e === 2) { // défile le nom de l'appli en cours
        const k = Math.min(choisies.length - 1, Math.floor(((p - 0.34) / 0.52) * choisies.length));
        lis[2].querySelector("span")!.textContent = `Désactivation : ${choisies[k].nom}`;
      }
      titre.textContent = p >= 1 ? "Terminé" : "Un instant…";
      x < 1 ? requestAnimationFrame(tick) : fin();
    };
    requestAnimationFrame(tick);
  });
  lis.forEach((li) => (li.className = "fait")); lis[2].querySelector("span")!.textContent = "Applis désactivées";
  await sleep(700);
  terminer(choisies.length);
}

function terminer(off: number) {
  afficher(4);
  $("#r-off").textContent = String(off); $("#r-on").textContent = String(APPLIS.length - off);
  heure("night", { duration: 5.5, delay: 1.2 }); // crépuscule puis nuit étoilée
  scene?.setBalise("rest");
}

// ---- Démarrage ----
function init() {
  initScene();
  construireListe(); initAdresse();
  if (params.has("debug")) {
    const d = $("#debug"); d.hidden = false;
    setInterval(() => (d.textContent = scene ? scene.stats() : "repli CSS"), 500);
  }
  $("#commencer").addEventListener("click", versRecherche);
  $("#cest-elle").addEventListener("click", () => { heure("golden"); afficher(3); scene?.setBalise("rest"); });
  $("#appliquer").addEventListener("click", appliquer);
  $("#fermer").addEventListener("click", async (e) => {
    (e.currentTarget as HTMLElement).classList.add("busy");
    await sleep(500);
    document.body.classList.add("ferme");
    const a = $("#adieu"); a.hidden = false; void a.offsetWidth; a.classList.add("visible");
    $("#rejouer").focus();
  });
  $("#rejouer").addEventListener("click", () => (location.href = location.pathname));

  // ?ecran=N : saute directement à un écran (pratique pour les captures).
  const saut = Number(params.get("ecran"));
  if (saut >= 2 && saut <= 4) {
    const h: Heure = saut === 2 ? "day" : saut === 3 ? "golden" : "night";
    document.body.dataset.heure = h; scene?.snap(h);
    if (saut === 2) { afficher(2); scene?.setBalise("search"); timers.push(window.setTimeout(boxTrouvee, 3300)); }
    else { scene?.setBalise("rest"); if (saut === 4) { afficher(4); $("#r-off").textContent = "9"; $("#r-on").textContent = "3"; } else afficher(3); }
  } else afficher(1);
}
init();
