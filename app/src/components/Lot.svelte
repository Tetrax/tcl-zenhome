<!-- Plusieurs box : la liste des box avec leur état, et l'étape de la box en cours. Après le passage 1, choix si des box ont échoué. -->
<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Travail from "./Travail.svelte";
  import { zen } from "../lib/zen.svelte";
  import { libelleBox } from "../lib/libelles";

  const classe = { attente: "attente", "non-traitee": "attente", "en-cours": "en-cours", ok: "fait", echec: "echec" } as const;
  const reussies = $derived((zen.lot ?? []).some((b) => b.etat === "ok"));
  const [titre, sous] = $derived.by(() => {
    if (zen.mode === "reparer") return ["Réparation en cours", "Zen Home contrôle chaque box, l'une après l'autre, et corrige ce que la dernière mise à jour a défait."];
    if (zen.mode === "restaurer") return ["Retour à l'état d'origine", "Ne fermez pas cette fenêtre et laissez les TV et les box allumées."];
    return zen.lotPasse === 1
      ? ["Installation de Zen Home", "Zen Home s’installe sur vos box, l’une après l’autre. Rien n’est encore désactivé : vous choisirez juste après."]
      : ["Application en cours", "Ne fermez pas cette fenêtre et laissez les TV et les box allumées."];
  });
</script>

{#snippet liste()}
  <ol class="jalons" style="width:min(440px,100%)">
    {#each zen.lot ?? [] as b (b.ip)}
      <li class={classe[b.etat]}>
        <i></i>
        <span>« {b.nom} » <span class="petit">· {libelleBox[b.etat]}</span>
          {#if b.message}<span class="petit" style="display:block">{b.message}</span>{/if}
        </span>
      </li>
    {/each}
  </ol>
{/snippet}

{#if zen.lotPhase === "choix"}
  <Panneau titre="Certaines box n'ont pas pu être préparées">
    <p class="lead">Les autres sont prêtes. Vous pouvez réessayer celles qui ont échoué, ou continuer sans elles : elles ne seront pas modifiées.</p>
    {@render liste()}
    {#snippet pied()}
      <button class="btn primary" type="button" onclick={() => zen.reessayerPassage1()}>Réessayer les box en échec</button>
      {#if reussies}<button class="btn" type="button" onclick={() => zen.questions()}>Continuer sans elles</button>{/if}
      <button class="lien" type="button" onclick={() => zen.retourAccueil()}>Revenir à l'accueil</button>
    {/snippet}
  </Panneau>
{:else}
  <Travail {titre} {sous} fraction={zen.fractionLot()} extra={liste} />
{/if}
