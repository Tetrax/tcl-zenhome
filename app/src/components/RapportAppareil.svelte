<!-- Rapport d'appareil : lecture seule de l'appareil, texte à copier ou à envoyer. -->
<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Travail from "./Travail.svelte";
  import BlocErreur from "./BlocErreur.svelte";
  import Icone from "./Icone.svelte";
  import { api, normaliserErreur } from "../lib/api";
  import { zen } from "../lib/zen.svelte";

  let message = $state("");
  const texte = $derived(zen.texteRapportAppareil);

  async function copier() {
    try {
      await navigator.clipboard.write([new ClipboardItem({ "text/plain": new Blob([texte], { type: "text/plain" }) })]);
      message = "Rapport copié. Collez-le dans un message à hebertval34@gmail.com.";
    } catch { message = "La copie n'a pas fonctionné : sélectionnez le texte ci-dessus et copiez-le à la main."; }
  }
  // Le lien de messagerie est limité en taille : l'en-tête ne garde que les premières lignes (modèle, Android), le reste se colle.
  async function envoyer() {
    const entete = texte.split("\n").filter((l) => l.startsWith("ro.")).slice(0, 9).join("\n");
    try {
      await api.envoyerRapport("", `${entete}\n\nCollez ici le rapport copié (bouton « Copier le rapport ») : ce message est trop court pour le contenir.`);
      message = "Votre messagerie s'ouvre : pensez à coller le rapport copié dans le message avant de l'envoyer.";
    } catch (e) { message = normaliserErreur(e).message; }
  }
</script>

{#if zen.erreur}
  <BlocErreur />
{:else if zen.travail.actif}
  <Travail titre="Lecture de l'appareil" sous="Zen Home lit quelques informations sur l'appareil, sans rien modifier." fraction={zen.travail.fraction} />
{:else}
  <Panneau titre="Rapport d'appareil" large>
    <p class="lead">Voici ce que Zen Home a lu. Rien n'a été modifié sur l'appareil. Le rapport ne contient ni numéro de série, ni adresse, ni nom de réseau, ni le nom de vos applis.</p>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex : zone défilante, atteignable au clavier -->
    <pre class="rapport" role="region" tabindex="0" aria-label="Texte du rapport">{texte}</pre>
    <div class="note"><Icone nom="info" /><p>Cliquez sur « Copier le rapport », puis sur « Envoyer » et collez-le dans le message.</p></div>
    <p class="petit" role="status">{message}</p>
    {#snippet pied()}
      <button class="btn primary" type="button" onclick={copier}>Copier le rapport</button>
      <button class="btn" type="button" onclick={envoyer}>Envoyer</button>
      <button class="lien" type="button" onclick={() => zen.retourAccueil()}>Revenir à l'accueil</button>
    {/snippet}
  </Panneau>
{/if}

<style>
  .rapport { margin: 14px 0 0; padding: 12px 14px; max-height: 260px; overflow: auto; border-radius: 14px; font: 12px/1.45 ui-monospace, Menlo, monospace;
    white-space: pre-wrap; overflow-wrap: anywhere; background: rgba(0, 0, 0, .28); border: 1px solid rgba(255, 255, 255, .22); user-select: text; }
</style>
