<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import BlocErreur from "./BlocErreur.svelte";
  import IllustrationTv from "./IllustrationTv.svelte";
  import { zen } from "../lib/zen.svelte";

  const cible = $derived(zen.cible && "ip" in zen.cible ? zen.cible.ip : zen.boxes.find((b) => zen.cible && "id" in zen.cible && b.id === zen.cible.id)?.ip ?? "");
  function autreAdresse() { zen.forcerManuel = true; zen.aller("recherche"); }
</script>

{#if zen.erreur}
  <BlocErreur extra={{ libelle: "Changer d'adresse", action: autreAdresse }} />
{:else}
  <Panneau titre="Connexion à votre box">
    <p class="lead" aria-live="polite">{zen.travail.etape || "Connexion à la box"}…</p>
    {#if cible}<p class="petit">Adresse : <span class="ip">{cible}</span></p>{/if}
    <p class="petit">Un message va s'afficher sur l'écran de la TV : acceptez-le, en cochant « Toujours autoriser cet ordinateur ». Gardez la télécommande à portée de main.</p>
    <IllustrationTv />
    {#snippet pied()}
      <button class="btn small" type="button" onclick={() => zen.annuler()} disabled={zen.travail.annulation}>{zen.travail.annulation ? "Arrêt en cours…" : "Annuler"}</button>
    {/snippet}
  </Panneau>
{/if}
