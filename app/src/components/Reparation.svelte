<script lang="ts">
  import Travail from "./Travail.svelte";
  import BlocErreur from "./BlocErreur.svelte";
  import { zen } from "../lib/zen.svelte";
</script>

{#if zen.erreur}
  <BlocErreur />
{:else}
  <Travail titre="Réparation en cours" sous="Zen Home contrôle la box et corrige ce que la dernière mise à jour a défait. Vos choix précédents sont conservés." fraction={zen.travail.fraction} />
{/if}
