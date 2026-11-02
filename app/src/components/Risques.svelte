<!-- Case obligatoire pour un appareil non vérifié : le cœur refuse d'installer sans cet accord (risques_acceptes). -->
<script lang="ts">
  import { zen } from "../lib/zen.svelte";
</script>

{#if zen.risquesRequis}
  <label class="ligne" style="margin-top:16px">
    <input type="checkbox" bind:checked={zen.risquesAcceptes} />
    <span class="case"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3.5 8.5 3 3 6-7" /></svg></span>
    <span class="texte"><span class="nom">J'ai compris : {zen.lot ? "au moins un de ces modèles n'a pas été testé" : "ce modèle n'a pas été testé"}. Zen Home garde une sauvegarde et « Revenir à avant Zen Home » remet {zen.lot ? "les appareils" : "l'appareil"} comme avant.</span></span>
  </label>
{/if}
