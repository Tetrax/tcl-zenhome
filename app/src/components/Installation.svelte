<script lang="ts">
  import Travail from "./Travail.svelte";
  import BlocErreur from "./BlocErreur.svelte";
  import { zen } from "../lib/zen.svelte";
  import { motConnecte } from "../lib/appareils";

  const mot = $derived(motConnecte(zen.box?.appareil));
</script>

{#if zen.erreur}
  <BlocErreur />
{:else}
  <Travail titre="Installation de Zen Home" sous={`Zen Home s’installe sur votre ${mot}. Rien n’est encore désactivé : vous choisirez juste après.`} fraction={zen.travail.fraction} />
{/if}
