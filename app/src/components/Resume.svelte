<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import { zen } from "../lib/zen.svelte";
  const r = $derived(zen.inventaire?.resume);
</script>

{#if r && zen.box}
  <Panneau titre="Voici votre box">
    <p class="lead">Une sauvegarde de l'état actuel est enregistrée dans Documents/Zen Home. Rien n'a été modifié.</p>
    <dl class="grille">
      <div><dt>Modèle</dt><dd>{r.modele}</dd></div>
      <div><dt>Système</dt><dd>{r.android}</dd></div>
      <div><dt>Applis installées</dt><dd class="grand">{r.nb_applis}</dd></div>
      <div><dt>Déjà désactivées</dt><dd class="grand">{r.nb_desactivees}</dd></div>
    </dl>
    <p class="petit">Nom de la box : « {zen.box.nom} » · {zen.box.ip}</p>
    {#snippet pied()}
      <button class="btn primary" type="button" onclick={() => zen.aller("meteo")}><span>Continuer</span><Icone nom="fleche" /></button>
    {/snippet}
  </Panneau>
{/if}
