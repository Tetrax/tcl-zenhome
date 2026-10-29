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
const lot = [{ nom: "SALON", inv: salon }, { nom: "CHAMBRE", inv: chambre }];

test("union des applis système et utilisateur", () => {
  const f = fusionner(lot);
  assert.deepEqual(f.applis_systeme.map((a) => a.paquet), ["a", "b", "c", "d"]);
  assert.deepEqual(f.applis_utilisateur.map((a) => a.paquet), ["n", "s", "t"]);
  assert.equal(f.resume.nb_applis, 10);
});

test("état commun conservé, état divergent ramené à actif", () => {
  const etat = Object.fromEntries(fusionner(lot).applis_systeme.map((a) => [a.paquet, a.etat]));
  assert.deepEqual(etat, { a: "actif", b: "actif", c: "garde", d: "actif" });
  const memes = fusionner([{ nom: "1", inv: inv([sys("x", "desactive")], []) }, { nom: "2", inv: inv([sys("x", "desactive")], []) }]);
  assert.equal(memes.applis_systeme[0].etat, "desactive");
});

test("libellé réel préféré au nom de paquet, quelle que soit la box", () => {
  const l = Object.fromEntries(fusionner(lot).applis_utilisateur.map((a) => [a.paquet, a.libelle]));
  assert.deepEqual(l, { n: "Netflix", s: "Spotify", t: "t" });
});

test("sur liste les box où l'appli est installée", () => {
  const s = Object.fromEntries(fusionner(lot).applis_utilisateur.map((a) => [a.paquet, a.sur]));
  assert.deepEqual(s, { n: ["SALON", "CHAMBRE"], s: ["SALON", "CHAMBRE"], t: ["CHAMBRE"] });
});

test("fusionner ne modifie pas les inventaires d'origine", () => {
  fusionner(lot);
  assert.equal(salon.applis_systeme[1].etat, "desactive");
  assert.equal(salon.applis_utilisateur[0].sur, undefined);
});

test("une appli absente de la box n'est jamais envoyée", () => {
  const r = reduire({ garder: ["c", "d"], desactiver: ["a", "b"], desinstaller: ["n", "t"] }, salon);
  assert.deepEqual(r, { garder: ["c"], desactiver: ["a", "b"], desinstaller: ["n"] });
});

test("aucune appli système présente n'est oubliée", () => {
  for (const box of [salon, chambre]) {
    const r = reduire({ garder: ["c"], desactiver: ["a"], desinstaller: [] }, box); // b et d absents du choix
    const envoyees = new Set([...r.garder, ...r.desactiver]);
    for (const a of box.applis_systeme) assert.ok(envoyees.has(a.paquet), a.paquet);
    for (const p of envoyees) assert.ok(box.applis_systeme.some((a) => a.paquet === p), p);
  }
});
