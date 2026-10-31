// @ts-nocheck : exécuté par Node (imports « .ts »), hors de la vérification svelte-check.
// node --test : npm run test:profils
import { test } from "node:test";
import assert from "node:assert/strict";
import { choixDuProfil, GARDEES_SIMPLE } from "./profils.ts";
import type { AppliSysteme } from "./types.ts";

const a = (paquet: string, groupe: "pistage" | "option", recommande: "desactiver" | "garder" = "desactiver", etat: AppliSysteme["etat"] = "actif"): AppliSysteme =>
  ({ paquet, libelle: paquet, explication: "", recommande, etat, groupe });
const applis = [
  a("pub", "pistage"), a("stats", "pistage"),
  a("tv", "option"), a("tts", "option", "garder"), a("jeux", "option", "garder"),
  a("com.google.android.katniss", "option", "garder"),
  a("com.xiaomi.mitv.updateservice", "option", "garder"),
  a("com.mitv.download.service", "option", "garder"),
];
const desactivees = (c: Record<string, boolean>) => Object.keys(c).filter((p) => c[p]);

test("Simple : tout désactivé sauf recherche vocale et mises à jour Xiaomi", () => {
  assert.deepEqual(desactivees(choixDuProfil("simple", applis)), ["pub", "stats", "tv", "tts", "jeux"]);
  assert.equal(GARDEES_SIMPLE.length, 3);
});

test("Simple désactive une option que Recommandé garde (jeux, musique…)", () => {
  const c = choixDuProfil("simple", [a("x", "option", "garder")]);
  assert.equal(c.x, true);
});

test("Léger : seul le pistage est désactivé", () => {
  assert.deepEqual(desactivees(choixDuProfil("leger", applis)), ["pub", "stats"]);
});

test("Recommandé : recommandation, état actuel prioritaire", () => {
  const c = choixDuProfil("recommande", [...applis, a("deja", "option", "garder", "desactive"), a("garde", "pistage", "desactiver", "garde")]);
  assert.deepEqual(desactivees(c), ["pub", "stats", "tv", "deja"]); // tts et jeux (option:garder) restent gardées
});

test("appli absente de la box ignorée, aucune clé en trop", () => {
  const c = choixDuProfil("simple", [a("pub", "pistage")]);
  assert.deepEqual(Object.keys(c), ["pub"]);
  assert.equal("com.google.android.katniss" in c, false);
});
