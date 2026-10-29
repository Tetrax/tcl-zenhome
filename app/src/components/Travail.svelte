<!-- Écran d'une action longue : anneau de progression, étape en cours (annoncée), jalons, bouton Annuler. -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import Panneau from "./Panneau.svelte";
  import { zen } from "../lib/zen.svelte";

  let { titre, sous, fraction, jalons, annulable = true, extra }: {
    titre: string; sous?: string; fraction: number | null;
    jalons?: { libelle: string; etat: "attente" | "en-cours" | "fait" }[]; annulable?: boolean; extra?: Snippet;
  } = $props();
  const pc = $derived(fraction === null ? null : Math.round(fraction * 100));
</script>

<Panneau {titre}>
  <div class="travail">
    {#if sous}<p class="lead" style="margin-top:0">{sous}</p>{/if}
    <div class="anneau" class:indet={pc === null} style="margin-top:18px" role="progressbar" aria-label="Progression" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pc ?? undefined}>
      <div class="rotor"><svg viewBox="0 0 120 120" width="140" height="140"><circle cx="60" cy="60" r="52" class="piste" /><circle cx="60" cy="60" r="52" class="arc" style:stroke-dashoffset={pc === null ? null : 326.7 * (1 - (fraction ?? 0))} /></svg></div>
      <span class="pourcent" aria-hidden="true">{pc === null ? "" : `${pc} %`}</span>
    </div>
    <p class="etape-courante" aria-live="polite" aria-atomic="true">{zen.travail.etape}</p>
    {#if jalons}
      <ol class="jalons">
        {#each jalons as j (j.libelle)}
          <li class={j.etat}><i></i><span>{j.libelle}{#if j.etat === "fait"}<span class="sr"> : terminé</span>{:else if j.etat === "en-cours"}<span class="sr"> : en cours</span>{/if}</span></li>
        {/each}
      </ol>
    {/if}
    {#if extra}{@render extra()}{/if}
    {#if zen.travail.avertissements.length}
      <ul class="avertissements">{#each zen.travail.avertissements as a}<li>{a}</li>{/each}</ul>
    {/if}
  </div>
  {#snippet pied()}
    {#if annulable}
      <button class="btn small" type="button" onclick={() => zen.annuler()} disabled={zen.travail.annulation || !zen.travail.actif}>
        {zen.travail.annulation ? "Arrêt en cours…" : "Annuler"}
      </button>
      <span class="petit">Ce qui est déjà fait est conservé.</span>
    {/if}
  {/snippet}
</Panneau>
