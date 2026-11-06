<!-- Association par code d'une TV qui n'accepte pas adb sur le port 5555 (Chromecast avec Google TV, Google TV Streamer, quelques Sony ou Philips) : débogage sans fil d'Android 11+. -->
<script lang="ts">
  import { onMount } from "svelte";
  import Panneau from "./Panneau.svelte";
  import { zen } from "../lib/zen.svelte";
  import { CONSIGNES as c } from "../lib/appareils";
  import { ipLocale, ipValide } from "../lib/validation";

  let code = $state("");
  let ip = $state("");
  let ipOuverte = $state(false);
  let message = $state("");
  let champ = $state<HTMLInputElement>();
  const enCours = $derived(zen.travail.actif);

  onMount(() => {
    const t = setTimeout(() => champ?.focus(), 450);
    return () => clearTimeout(t);
  });

  async function envoyer(e: SubmitEvent) {
    e.preventDefault();
    const chiffres = code.replace(/\s+/g, ""); // le collage « 745 385 » est accepté
    if (!/^\d{6}$/.test(chiffres)) {
      message = "Le code comporte 6 chiffres. Recopiez ceux qui s'affichent sur la TV, dans la fenêtre « Code d'association via le Wi-Fi ».";
      return;
    }
    const adresse = ip.replace(/\s+/g, "").replace(/:\d+$/, ""); // « 192.168.1.23:45509 » : le port n'est pas utile
    if (adresse && !(ipValide(adresse) && ipLocale(adresse))) {
      message = "Cette adresse n'a pas l'air valide. Elle ressemble à 192.168.1.23 : c'est l'« Adresse IP et port » de l'écran Débogage sans fil, sans le port.";
      ipOuverte = true;
      return;
    }
    message = "";
    const erreur = await zen.appairer(chiffres, adresse);
    if (erreur) {
      message = erreur.message;
      if (erreur.code === "plusieurs_fenetres") ipOuverte = true;
      if (erreur.code === "code_refuse") { code = ""; setTimeout(() => champ?.focus(), 80); }
    }
  }
</script>

<Panneau titre="Ma TV demande un code d'association" large>
  <p class="lead">Certaines TV (Chromecast avec Google TV, Google TV Streamer, quelques Sony ou Philips) n'acceptent pas la connexion habituelle. Elles demandent le « Débogage sans fil » et une association avec un code, une seule fois.</p>
  <ol class="consignes">
    <li>Sur la TV, ouvrez <span class="chemin">{c.sansFil}</span> et passez sur « Activé ». Si la TV demande « Autoriser le débogage sans fil sur ce réseau ? », acceptez, en cochant « Toujours autoriser sur ce réseau ».</li>
    <li>Toujours sur cet écran de la TV, choisissez « Associer l'appareil avec un code d'association ». Une fenêtre affiche un code à 6 chiffres : laissez-la ouverte.</li>
    <li>Recopiez le code ici, puis cliquez sur « Associer ».</li>
  </ol>

  <form class="champ" onsubmit={envoyer} novalidate>
    <label for="champ-code">Code d'association (6 chiffres)</label>
    <div class="rang">
      <input id="champ-code" bind:this={champ} bind:value={code} inputmode="numeric" autocomplete="off" spellcheck="false" placeholder="123 456" maxlength="12"
        disabled={enCours} aria-invalid={message ? "true" : undefined} aria-describedby="erreur-code" />
      <button class="btn primary small" type="submit" disabled={enCours}>Associer</button>
    </div>
    <p class="msg" id="erreur-code" role="alert">{message}</p>
    {#if enCours}
      <p class="petit" style="margin-top:0" aria-live="polite">Association en cours<span class="dots" aria-hidden="true"><i>.</i><i>.</i><i>.</i></span> Gardez la fenêtre ouverte sur la TV.</p>
    {/if}
    <details bind:open={ipOuverte}>
      <summary>Plusieurs TV affichent un code ?</summary>
      <label for="champ-ip-appairage">Adresse IP de la TV à associer (facultatif)</label>
      <input id="champ-ip-appairage" bind:value={ip} inputmode="decimal" autocomplete="off" spellcheck="false" placeholder="192.168.1.23" disabled={enCours} />
      <p class="petit">Elle est indiquée sur la TV, à « Adresse IP et port » : gardez seulement les 4 nombres du début.</p>
    </details>
  </form>
  {#snippet pied()}
    {#if enCours}
      <button class="btn small" type="button" onclick={() => zen.annuler()} disabled={zen.travail.annulation}>{zen.travail.annulation ? "Arrêt en cours…" : "Annuler"}</button>
    {:else}
      <button class="lien" type="button" onclick={() => zen.aller("recherche")}>Retour</button>
    {/if}
  {/snippet}
</Panneau>
