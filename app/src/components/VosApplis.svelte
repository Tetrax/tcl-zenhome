<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import { zen } from "../lib/zen.svelte";

  const liste = $derived(zen.inventaire?.applis_utilisateur ?? []);
  const choisies = $derived(liste.filter((a) => zen.desinstaller[a.paquet]));
  let confirmer = $state(false);

  function suite() {
    if (choisies.length === 0) zen.aller("recap");
    else confirmer = true;
  }
</script>

{#if confirmer}
  <!-- Confirmation explicite : le bouton par défaut (focus, Entrée) est « Annuler ». -->
  <Panneau titre="Supprimer ces applis ?">
    <div class="note alerte"><Icone nom="alerte" /><p><b>C'est IRRÉVERSIBLE.</b> Leurs données (comptes, réglages, historique) seront perdues. Vous pourrez réinstaller ces applis, pas retrouver ces données.</p></div>
    <p class="lead">Vous allez désinstaller :</p>
    <ul class="liste" style="margin-top:6px">
      {#each choisies as a (a.paquet)}
        <li class="ligne" style="cursor:default"><span class="texte"><span class="nom">• {a.libelle}</span><span class="expl">{a.taille} · dernière utilisation : {a.derniere_utilisation}{a.sur ? ` · présente sur : ${a.sur.join(", ")}` : ""}</span></span></li>
      {/each}
    </ul>
    {#snippet pied()}
      <button class="btn primary" type="button" data-defaut onclick={() => (confirmer = false)}>Annuler</button>
      <button class="btn danger" type="button" onclick={() => zen.aller("recap")}>Désinstaller {choisies.length === 1 ? "cette appli" : `ces ${choisies.length} applis`}</button>
    {/snippet}
  </Panneau>
{:else}
  <Panneau titre="Vos applis" large>
    <p class="lead">Voici les applis que vous avez installées vous-même, avec leur taille et leur dernière utilisation.</p>
    <p class="lead">Cochez <b>celles que vous voulez supprimer</b>. Rien n'est coché d'avance : pour tout garder, ne cochez rien et continuez.</p>
    <div class="barre-liste">
      <span class="petit">Désinstaller une appli est définitif.</span>
      <p class="compteur" aria-live="polite">{#if choisies.length === 0}Aucune appli ne sera supprimée{:else}<b>{choisies.length}</b> appli{choisies.length > 1 ? "s" : ""} à supprimer{/if}</p>
    </div>
    <ul class="liste" style="margin-top:6px">
      {#each liste as a (a.paquet)}
        <li>
          <label class="ligne danger">
            <input type="checkbox" bind:checked={zen.desinstaller[a.paquet]} />
            <span class="case"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3.5 8.5 3 3 6-7" /></svg></span>
            <span class="texte"><span class="nom">{a.libelle}</span><span class="expl">{a.taille} · dernière utilisation : {a.derniere_utilisation}{a.sur ? ` · présente sur : ${a.sur.join(", ")}` : ""}</span></span>
          </label>
        </li>
      {/each}
    </ul>
    {#snippet pied()}
      <button class="btn primary" type="button" onclick={suite}><span>{choisies.length ? "Continuer" : "Tout garder"}</span><Icone nom="fleche" /></button>
      <button class="lien" type="button" onclick={() => zen.aller("systeme")}>Retour</button>
    {/snippet}
  </Panneau>
{/if}
