<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Travail from "./Travail.svelte";
  import BlocErreur from "./BlocErreur.svelte";
  import Icone from "./Icone.svelte";
  import { zen } from "../lib/zen.svelte";
</script>

{#if zen.erreur}
  <BlocErreur />
{:else if zen.ecran === "confirm_restauration"}
  <!-- Le bouton par défaut (focus) est « Annuler ». -->
  <Panneau titre="Tout annuler ?">
    <p class="lead">Cela va :</p>
    <ul class="lead" style="margin:6px 0 0;padding-left:22px;display:grid;gap:4px">
      <li>réactiver toutes les applis désactivées ;</li>
      <li>remettre l'accueil Xiaomi/Google d'origine ;</li>
      <li>désinstaller Zen Home.</li>
    </ul>
    <div class="note alerte"><Icone nom="alerte" /><p>Les applis que vous avez désinstallées ne reviennent pas.</p></div>
    {#snippet pied()}
      <button class="btn primary" type="button" data-defaut onclick={() => zen.retourAccueil()}>Annuler</button>
      <button class="btn danger" type="button" onclick={() => zen.restaurer()}>Tout annuler</button>
    {/snippet}
  </Panneau>
{:else if zen.ecran === "restauration"}
  <Travail titre="Retour à l'état d'origine" sous="Ne fermez pas cette fenêtre et laissez la TV et la box allumées." fraction={zen.travail.fraction} />
{:else if zen.restauration}
  {@const r = zen.restauration}
  <Panneau titre={r.incomplete ? "Presque terminé" : "Votre box est revenue à son état d'origine"}>
    {#if r.incomplete}
      <div class="note alerte"><Icone nom="alerte" /><p><b>Zen Home a été conservé sur la box, par sécurité.</b> Le retour à l'état d'origine n'a pas pu être terminé entièrement : pour ne pas laisser la TV sans accueil, Zen Home n'a pas été désinstallé. Rien n'est cassé. Vous pouvez relancer « Tout annuler » une fois la box rallumée et reconnectée.</p></div>
    {:else}
      <p class="lead">Les applis ont été réactivées, l'accueil d'origine est revenu et Zen Home a été retiré.</p>
    {/if}
    {#if r.avertissements.length}
      <h3 style="margin-top:18px">À noter</h3>
      <ul class="petit" style="margin:6px 0 0;padding-left:20px;display:grid;gap:4px">{#each r.avertissements as a}<li>{a}</li>{/each}</ul>
    {/if}
    {#snippet pied()}
      {#if r.incomplete}<button class="btn primary" type="button" onclick={() => zen.demarrer("restaurer")}>Réessayer</button>{/if}
      <button class="btn" class:primary={!r.incomplete} type="button" onclick={() => zen.aller("adieu")}>Terminer</button>
      <button class="btn" type="button" onclick={() => zen.ouvrirSauvegardes()}>Ouvrir les sauvegardes</button>
    {/snippet}
  </Panneau>
{/if}
