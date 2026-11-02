<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import { zen } from "../lib/zen.svelte";
  import Risques from "./Risques.svelte";
  import { motConnecte, defini } from "../lib/appareils";
  const r = $derived(zen.inventaire?.resume);
  const mot = $derived(motConnecte(zen.box?.appareil));
  const bloque = $derived(zen.risquesRequis && !zen.risquesAcceptes);
</script>

{#if r && zen.box}
  <Panneau titre={`Voici votre ${mot}`}>
    {#if zen.box.verifie}
      <p><span class="badge"><Icone nom="check" />Vérifié</span></p>
    {:else}
      <p><span class="badge avert"><Icone nom="info" />Non vérifié</span></p>
      <p class="petit">Zen Home ne connaît pas encore ce modèle. L'installation devrait fonctionner, mais elle n'a pas été testée sur cet appareil.</p>
    {/if}
    <p class="lead">Une sauvegarde de l'état actuel est enregistrée sur ce Mac. Rien n'a été modifié.</p>
    <dl class="grille">
      <div><dt>Modèle</dt><dd>{r.modele}</dd></div>
      <div><dt>Système</dt><dd>{r.android}</dd></div>
      <div><dt>Applis installées</dt><dd class="grand">{r.nb_applis}</dd></div>
      <div><dt>Déjà désactivées</dt><dd class="grand">{r.nb_desactivees}</dd></div>
    </dl>
    <p class="petit">Nom de {defini(mot)} : « {zen.box.nom} » · {zen.box.ip}</p>
    <Risques />
    {#snippet pied()}
      <button class="btn primary" type="button" disabled={bloque} onclick={() => zen.aller("meteo")}><span>Continuer</span><Icone nom="fleche" /></button>
    {/snippet}
  </Panneau>
{/if}
