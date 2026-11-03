<!-- « Préparez votre TV ou votre box » : consignes communes d'activation du débogage (le cœur reconnaît lui-même l'appareil à la connexion). -->
<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import { zen } from "../lib/zen.svelte";
  import { CONSIGNES as c } from "../lib/appareils";
</script>

<Panneau titre="Préparez votre TV ou votre box" large>
  <p class="lead">Pour que Zen Home puisse s'y connecter, il faut d'abord activer le débogage sur l'appareil, avec la télécommande.</p>
  <ol class="consignes">
    <li>Débloquez les options cachées : <span class="chemin">{c.debloquer}</span>, descendez jusqu'à {c.build} et appuyez 7 fois sur OK, jusqu'au message « Vous êtes maintenant développeur ».</li>
    <li>Revenez en arrière, puis activez le débogage : <span class="chemin">{c.debogage}</span> ({c.variante}).</li>
  </ol>
  <details>
    <summary>Précisions selon votre appareil</summary>
    {#each c.precisions as p (p.titre)}
      <p class="petit"><b>{p.titre}</b> : <span class="chemin">{p.debloquer}</span>, ligne « {p.build} » (7 fois sur OK) ; puis <span class="chemin">{p.debogage}</span>.</p>
    {/each}
    <p class="petit">{c.shield}</p>
  </details>
  {#snippet pied()}
    <button class="btn primary" type="button" onclick={() => zen.aller("recherche")}>
      <span>Continuer</span><Icone nom="fleche" />
    </button>
    <button class="lien" type="button" onclick={() => zen.retourAccueil()}>Retour</button>
  {/snippet}
</Panneau>
