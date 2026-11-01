<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import { zen } from "../lib/zen.svelte";
  import type { Profil } from "../lib/profils";

  const profils: { cle: Exclude<Profil, "perso">; titre: string; phrase: string; desactive: string; garde: string; conseille?: boolean }[] = [
    { cle: "recommande", titre: "Recommandé", conseille: true, phrase: "Le choix conseillé par Zen Home, qui tient compte de l'état actuel de la box.",
      desactive: "pistage, publicité et applis inutiles sur une box ou une TV", garde: "ce qui sert souvent : jeux, musique, films Google, lecteur USB, aide à la lecture, recherche vocale et mises à jour du constructeur" },
    { cle: "simple", titre: "Simple", phrase: "Une box épurée : tout ce qui n'est pas indispensable est désactivé.",
      desactive: "pistage, publicité et applis facultatives (jeux, musique, lecteur USB…)", garde: "recherche vocale et mises à jour du constructeur" },
    { cle: "leger", titre: "Léger", phrase: "On ne touche qu'au pistage et à la publicité.",
      desactive: "pistage et publicité", garde: "tout le reste (jeux, musique, lecteur USB, recherche vocale…)" },
  ];
</script>

<Panneau titre="Comment voulez-vous votre box ?" large>
  <p class="lead">Choisissez un profil : vous verrez le détail avant que quoi que ce soit change. Les applis sont <b>désactivées, pas supprimées</b> : « Revenir à avant Zen Home » les remet.</p>
  <div class="profils">
    {#each profils as p (p.cle)}
      <button class="profil" class:choisi={zen.profil === p.cle} type="button" data-defaut={p.conseille ? "" : undefined} onclick={() => zen.choisirProfil(p.cle)}>
        <span class="nom">{p.titre}{#if p.conseille}<span class="puce">Conseillé</span>{/if}</span>
        <span class="phrase">{p.phrase}</span>
        <span class="detail"><b>Désactivé :</b> {p.desactive}.</span>
        <span class="detail"><b>Gardé :</b> {p.garde}.</span>
      </button>
    {/each}
  </div>
  <p class="petit"><button class="lien" type="button" onclick={() => zen.choisirProfil("perso")}>Personnaliser</button> : choisir appli par appli.</p>
</Panneau>

<style>
  .profils { display: grid; gap: 12px; margin-top: 14px; }
  .profil { display: flex; flex-direction: column; gap: 4px; text-align: left; padding: 16px 18px; border-radius: 20px; color: inherit; font: inherit; text-shadow: inherit; cursor: pointer;
    background: rgba(255, 255, 255, .1); border: 1px solid rgba(255, 255, 255, .24); transition: background .2s, border-color .2s; }
  .profil:hover { background: rgba(255, 255, 255, .16); }
  .profil.choisi { border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }
  .nom { font-size: 19px; font-weight: 500; }
  .phrase { font-size: 15px; }
  .detail { font-size: 13.5px; opacity: .85; }
  .puce { margin-left: 10px; font-size: 12px; font-weight: 400; padding: 2px 9px; border-radius: 999px; background: rgba(255, 255, 255, .18); }
</style>
