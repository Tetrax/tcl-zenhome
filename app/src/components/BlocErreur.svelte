<!-- Erreur du cœur : message déjà en français, titre et conseils selon le code. -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import IllustrationTv from "./IllustrationTv.svelte";
  import Rapport from "./Rapport.svelte";
  import { zen } from "../lib/zen.svelte";
  import type { CodeErreur } from "../lib/types";

  let { extra }: { extra?: { libelle: string; action: () => void } } = $props();
  const e = $derived(zen.erreur);

  const TITRES: Record<CodeErreur, string> = {
    occupe: "Une opération est déjà en cours", annule: "Opération arrêtée", adb_absent: "L'outil adb est manquant",
    refus_licence: "Votre accord est nécessaire", injoignable: "La box ne répond pas", non_autorise: "La box attend votre autorisation",
    autre_modele: "Ce n'est pas une Mi Box 4", autre_box: "Ce n'est pas la même box", lecture: "La box n'a pas pu être lue",
    choix_invalide: "Un choix n'a pas été accepté", reseau: "Problème de connexion Internet", interne: "Un problème est survenu",
  };
  const sansReprise: CodeErreur[] = ["autre_modele", "autre_box", "refus_licence"];
  const peutReessayer = $derived(!!zen.reprise && !!e && !sansReprise.includes(e.code));
</script>

{#if e}
  <Panneau titre={TITRES[e.code]}>
    {#snippet marque()}<div class="mark" style="color:#ffd0c4"><Icone nom={e.code === "annule" ? "info" : "alerte"} taille={30} /></div>{/snippet}
    <p class="lead" role="alert">{e.message}</p>
    {#if e.code === "injoignable"}
      <ul class="petit" style="margin:14px 0 0;padding-left:20px;display:grid;gap:6px">
        <li>Le débogage USB est activé (Paramètres &gt; Préférences de l'appareil &gt; Options pour les concepteurs).</li>
        <li>L'adresse IP est la bonne.</li>
        <li>La box est allumée (pas en veille) et sur le même Wi-Fi que ce Mac.</li>
        <li>Vous avez accepté l'autorisation affichée sur la TV.</li>
      </ul>
    {:else if e.code === "non_autorise"}
      <p class="petit">Sur la TV, une fenêtre doit s'afficher. Cochez « Toujours autoriser cet ordinateur », puis validez.</p>
      <IllustrationTv />
    {:else if e.code === "annule" || e.code === "reseau" || e.code === "lecture"}
      <p class="petit">Ce qui a déjà été fait est conservé : vous pouvez reprendre là où l'opération s'est arrêtée.</p>
    {:else if e.code === "autre_modele"}
      <p class="petit">Par sécurité, Zen Home n'a rien modifié sur cet appareil.</p>
    {/if}
    {#if !["annule", "occupe", "refus_licence"].includes(e.code)}<Rapport />{/if}
    {#snippet pied()}
      {#if e.code === "adb_absent"}
        <button class="btn primary" type="button" onclick={() => zen.aller("adb")}>Installer l'outil adb</button>
      {:else if peutReessayer}
        <button class="btn primary" type="button" onclick={() => zen.reprise?.()}>{e.code === "annule" ? "Reprendre" : "Réessayer"}</button>
      {/if}
      {#if extra}<button class="btn" type="button" onclick={extra.action}>{extra.libelle}</button>{/if}
      <button class="lien" type="button" onclick={() => zen.retourAccueil()}>Revenir à l'accueil</button>
    {/snippet}
  </Panneau>
{/if}
