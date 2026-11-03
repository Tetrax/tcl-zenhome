<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import { zen } from "../lib/zen.svelte";
</script>

<Panneau titre="À bientôt">
  <p class="lead">{zen.mode === "restaurer" ? "Tout est de nouveau comme avant Zen Home." : "Profitez de votre nouvel accueil."} Vous pouvez fermer cette fenêtre.</p>
  {#snippet pied()}
    <button class="btn" type="button" onclick={() => zen.recommencer()}>Revenir à l'accueil</button>
  {/snippet}
</Panneau>
