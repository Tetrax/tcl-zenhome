<!-- « Quel appareil ? » : ne sert qu'à afficher les bonnes consignes d'activation du débogage (le cœur vérifie le modèle). -->
<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import { zen } from "../lib/zen.svelte";
  import { FICHES, fiche, type Famille } from "../lib/appareils";

  let choix = $state<Famille | null>(null);
  const f = $derived(choix ? fiche(choix) : null);
  const cartes: { cle: Famille; titre: string; phrase: string }[] = [
    { cle: "mibox", titre: FICHES.mibox.titre, phrase: "Android TV 9" },
    { cle: "googletv", titre: FICHES.googletv.titre, phrase: "Google TV, Android 14" },
    { cle: "autre", titre: "Autre appareil", phrase: "Une autre box ou TV Android / Google TV" },
  ];
</script>

<Panneau titre="Quel appareil voulez-vous équiper ?" large>
  <p class="lead">Ce choix sert seulement à vous montrer où activer le débogage. Zen Home vérifie lui-même le modèle à la connexion.</p>
  <div class="cartes" role="radiogroup" aria-label="Appareil">
    {#each cartes as c (c.cle)}
      <button class="choix" class:choisi={choix === c.cle} type="button" role="radio" aria-checked={choix === c.cle} onclick={() => (choix = c.cle)}>
        <span class="nom">{c.titre}</span><span class="phrase">{c.phrase}</span>
      </button>
    {/each}
  </div>
  {#if f}
    <ol class="consignes">
      <li>Sur la {f.mot}, débloquez les options cachées : <span class="chemin">{f.debloquer}</span>, descendez jusqu'à « {f.build} » et appuyez 7 fois sur OK, jusqu'au message « Vous êtes maintenant développeur ».</li>
      <li>Revenez en arrière, puis activez le débogage : <span class="chemin">{f.debogage}</span>.</li>
    </ol>
  {:else if choix === "autre"}
    <div class="note"><Icone nom="info" /><p>Cet appareil n'est peut-être pas encore pris en charge : par sécurité, Zen Home ne modifie que les appareils qu'il connaît. Il peut en revanche lire l'appareil et préparer un <b>rapport d'appareil</b> (sans aucune modification), que vous pouvez m'envoyer pour qu'il soit ajouté. Activez d'abord le débogage USB dans les options pour les développeurs de l'appareil, puis indiquez son adresse ou lancez la recherche.</p></div>
  {/if}
  {#snippet pied()}
    <button class="btn primary" type="button" disabled={!choix} onclick={() => choix && zen.choisirAppareil(choix)}>
      <span>{choix === "autre" ? "Créer un rapport d'appareil" : "Continuer"}</span><Icone nom="fleche" />
    </button>
    <button class="lien" type="button" onclick={() => zen.retourAccueil()}>Retour</button>
  {/snippet}
</Panneau>

<style>
  .cartes { display: grid; gap: 10px; margin-top: 14px; }
  .choix { display: flex; flex-direction: column; gap: 2px; text-align: left; padding: 14px 18px; border-radius: 20px; color: inherit; font: inherit; text-shadow: inherit; cursor: pointer;
    background: rgba(255, 255, 255, .1); border: 1px solid rgba(255, 255, 255, .24); transition: background .2s, border-color .2s; }
  .choix:hover { background: rgba(255, 255, 255, .16); }
  .choix.choisi { border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }
  .nom { font-size: 19px; font-weight: 500; }
  .phrase { font-size: 14px; opacity: .85; }
</style>
