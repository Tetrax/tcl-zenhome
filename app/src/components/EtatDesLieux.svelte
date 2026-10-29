<script lang="ts">
  import Travail from "./Travail.svelte";
  import BlocErreur from "./BlocErreur.svelte";
  import { zen } from "../lib/zen.svelte";
</script>

{#if zen.erreur}
  <BlocErreur />
{:else}
  <Travail titre="État des lieux" sous="Zen Home regarde votre box et enregistre une sauvegarde de son état actuel, sans rien modifier." fraction={zen.travail.fraction} />
{/if}
