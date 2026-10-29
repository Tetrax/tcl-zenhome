// @ts-nocheck : exécuté par Node (imports « .ts », pas de @types/node), hors de la vérification svelte-check.
// node --test : npm run test:lot
import { test } from "node:test";
import assert from "node:assert/strict";
import { fusionner, reduire } from "./lot.ts";
import type { AppliSysteme, Inventaire } from "./types.ts";

const sys = (paquet: string, etat: AppliSysteme["etat"] = "actif"): AppliSysteme => ({ paquet, libelle: paquet, explication: "", recommande: "desactiver", etat });
const util = (paquet: string, libelle = paquet) => ({ paquet, libelle, taille: "1 Mo", derniere_utilisation: "hier" });
const inv = (s: AppliSysteme[], u: ReturnType<typeof util>[], nb = 10): Inventaire =>
  ({ applis_systeme: s, applis_utilisateur: u, resume: { modele: "Mi Box 4", android: "9", nb_applis: nb, nb_desactivees: 0 } });

const salon = inv([sys("a"), sys("b", "desactive"), sys("c", "garde")], [util("n", "Netflix"), util("s", "s")], 10);
const chambre = inv([sys("a"), sys("b", "actif"), sys("d")], [util("n", "n"), util("s", "Spotify"), util("t")], 20);
const lot = [{ id: "10.0.0.1", inv: salon }, { id: "10.0.0.2", inv: chambre }];

test("union des applis système et utilisateur", () => {
  const f = fusionner(lot);
  assert.deepEqual(f.applis_systeme.map((a) => a.paquet), ["a", "b", "c", "d"]);
  assert.deepEqual(f.applis_utilisateur.map((a) => a.paquet), ["n", "s", "t"]);
  assert.equal(f.resume.nb_applis, 10);
});

test("état commun conservé, état divergent ramené à actif", () => {
  const etat = Object.fromEntries(fusionner(lot).applis_systeme.map((a) => [a.paquet, a.etat]));
  assert.deepEqual(etat, { a: "actif", b: "actif", c: "garde", d: "actif" });
  const memes = fusionner([{ id: "1", inv: inv([sys("x", "desactive")], []) }, { id: "2", inv: inv([sys("x", "desactive")], []) }]);
  assert.equal(memes.applis_systeme[0].etat, "desactive");
});

test("libellé réel préféré au nom de paquet, quelle que soit la box", () => {
  const l = Object.fromEntries(fusionner(lot).applis_utilisateur.map((a) => [a.paquet, a.libelle]));
  assert.deepEqual(l, { n: "Netflix", s: "Spotify", t: "t" });
});

test("sur liste les box où l'appli est installée", () => {
  const s = Object.fromEntries(fusionner(lot).applis_utilisateur.map((a) => [a.paquet, a.sur]));
  assert.deepEqual(s, { n: ["10.0.0.1", "10.0.0.2"], s: ["10.0.0.1", "10.0.0.2"], t: ["10.0.0.2"] });
});

test("fusionner ne modifie pas les inventaires d'origine", () => {
  fusionner(lot);
  assert.equal(salon.applis_systeme[1].etat, "desactive");
  assert.equal(salon.applis_utilisateur[0].sur, undefined);
});

const commun = fusionner(lot);

test("une appli absente de la box n'est jamais envoyée", () => {
  const r = reduire({ garder: ["c", "d"], desactiver: ["a", "b"], desinstaller: ["n", "t"] }, salon, "10.0.0.1", commun);
  assert.deepEqual(r, { garder: ["c"], desactiver: ["a", "b"], desinstaller: ["n"] });
});

test("aucune appli système présente n'est oubliée", () => {
  for (const box of [salon, chambre]) {
    const r = reduire({ garder: ["c"], desactiver: ["a"], desinstaller: [] }, box, "10.0.0.1", commun); // b et d absents du choix
    const envoyees = new Set([...r.garder, ...r.desactiver]);
    for (const a of box.applis_systeme) assert.ok(envoyees.has(a.paquet), a.paquet);
    for (const p of envoyees) assert.ok(box.applis_systeme.some((a) => a.paquet === p), p);
  }
});

test("des box de même nom restent distinguées par leur identifiant", () => {
  const deux = fusionner([{ id: "10.0.0.7", inv: inv([], [util("x")]) }, { id: "10.0.0.8", inv: inv([], [util("x"), util("y")]) }]);
  assert.deepEqual(deux.applis_utilisateur.map((a) => a.sur), [["10.0.0.7", "10.0.0.8"], ["10.0.0.8"]]);
});

test("une désinstallation n'est envoyée qu'aux box pour lesquelles elle a été montrée", () => {
  const choix = { garder: [], desactiver: [], desinstaller: ["t", "n"] }; // t : vue pour CHAMBRE seulement, n : pour les deux
  assert.deepEqual(reduire(choix, chambre, "10.0.0.2", commun).desinstaller, ["t", "n"]);
  // SALON ne possède pas t ; si elle l'installe plus tard, t ne lui a jamais été montrée pour elle
  const salonPlus = inv(salon.applis_systeme, [...salon.applis_utilisateur, util("t")]);
  assert.deepEqual(reduire(choix, salonPlus, "10.0.0.1", commun).desinstaller, ["n"]);
});

test("une box absente des questions ne reçoit aucune désinstallation", () => {
  const choix = { garder: [], desactiver: ["a"], desinstaller: ["n", "s", "t"] };
  const r = reduire(choix, chambre, "10.0.0.9", commun); // jamais passée par fusionner
  assert.deepEqual(r.desinstaller, []);
  assert.ok(r.desactiver.includes("a"));
});
