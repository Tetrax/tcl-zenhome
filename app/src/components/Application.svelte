<script lang="ts">
  import Travail from "./Travail.svelte";
  import BlocErreur from "./BlocErreur.svelte";
  import { zen } from "../lib/zen.svelte";

  const NOMS = ["Installation de Zen Home", "Application de vos choix", "Vérification et réglage final"];
  const jalons = $derived(NOMS.map((libelle, i) => ({ libelle, etat: i < zen.jalon ? "fait" as const : i === zen.jalon ? "en-cours" as const : "attente" as const })));
</script>

{#if zen.erreur}
  <BlocErreur />
{:else}
  <Travail titre="Application en cours" sous="Ne fermez pas cette fenêtre et laissez la TV et la box allumées." fraction={zen.fractionGlobale()} {jalons} />
{/if}
