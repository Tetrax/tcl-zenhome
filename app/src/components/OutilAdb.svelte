<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Travail from "./Travail.svelte";
  import BlocErreur from "./BlocErreur.svelte";
  import { zen } from "../lib/zen.svelte";

  const URL_GOOGLE = "https://developer.android.com/studio/releases/platform-tools";
  let copie = $state(false);
  async function copier() {
    try { await navigator.clipboard.writeText(URL_GOOGLE); copie = true; setTimeout(() => (copie = false), 2000); } catch { /* le lien reste sélectionnable */ }
  }
</script>

{#if zen.erreur}
  <BlocErreur extra={{ libelle: "Retour", action: () => zen.aller("adb") }} />
{:else if zen.travail.actif}
  <Travail titre="Installation de l'outil adb" sous="Téléchargement depuis dl.google.com, puis vérification automatique." fraction={zen.travail.fraction} />
{:else}
  <Panneau titre="Un outil est nécessaire">
    <p class="lead">Pour dialoguer avec votre box, Zen Home a besoin d'« adb », un outil officiel et gratuit de Google (Android SDK Platform-Tools).</p>
    <p class="petit">Il va être téléchargé depuis dl.google.com (environ 18 Mo), vérifié par empreinte numérique, puis rangé dans le dossier Application Support/ZenHome de votre Bibliothèque. Rien d'autre n'est installé sur votre Mac.</p>
    <p class="petit">Page officielle et conditions d'utilisation :</p>
    <p style="margin-top:6px"><span class="url">{URL_GOOGLE}</span> <button class="lien" type="button" onclick={copier}>{copie ? "Copié" : "Copier le lien"}</button></p>
    <p class="petit"><b>En cliquant sur « J'accepte », vous acceptez ces conditions.</b></p>
    {#snippet pied()}
      <button class="btn primary" type="button" onclick={() => zen.installerAdb()}>J'accepte</button>
      <button class="btn" type="button" onclick={() => zen.retourAccueil()}>Annuler</button>
    {/snippet}
  </Panneau>
{/if}
