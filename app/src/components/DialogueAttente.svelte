<!-- Dialogue global sur l'événement attente_utilisateur : réveiller la box, ou accepter l'autorisation sur la TV. -->
<script lang="ts">
  import { zen } from "../lib/zen.svelte";
  import IllustrationTv from "./IllustrationTv.svelte";

  let dlg: HTMLDialogElement;
  let reessayer = $state<HTMLButtonElement>();
  const a = $derived(zen.attente);

  $effect(() => {
    if (a && !dlg.open) { dlg.showModal(); setTimeout(() => reessayer?.focus(), 30); }
    else if (!a && dlg.open) dlg.close();
  });
</script>

<!-- Échap ne ferme pas : il faut choisir Réessayer ou Arrêter. -->
<dialog class="dlg" bind:this={dlg} aria-labelledby="dlg-titre" oncancel={(e) => e.preventDefault()}>
  {#if a}
    {#if a.raison === "autorisation"}
      <h2 id="dlg-titre" tabindex="-1">Acceptez l'autorisation sur la TV</h2>
      <p class="lead">Un message s'affiche sur l'écran de la TV : acceptez-le, en cochant « Toujours autoriser cet ordinateur ».</p>
      <IllustrationTv />
      <p class="petit">Puis cliquez sur « Réessayer ».</p>
    {:else}
      <h2 id="dlg-titre" tabindex="-1">La box ne répond plus</h2>
      <p class="lead">La connexion avec la Mi Box{zen.box ? ` (${zen.box.ip})` : ""} a été perdue.</p>
      <p class="petit">Vérifiez qu'elle est allumée et pas en veille (appuyez sur une touche de la télécommande), et toujours sur le même Wi-Fi que ce Mac. Puis cliquez sur « Réessayer ».</p>
      <p class="petit">Ce qui a déjà été fait est conservé : Zen Home reprend là où il s'est arrêté.</p>
    {/if}
    <div class="pied" style="animation:none">
      <button class="btn primary" type="button" bind:this={reessayer} onclick={() => zen.decider("reessayer")}>Réessayer</button>
      <button class="btn" type="button" onclick={() => zen.decider("arreter")}>Arrêter</button>
    </div>
  {/if}
</dialog>
