<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import MiseAJour from "./MiseAJour.svelte";
  import DialogueDesinstallation from "./DialogueDesinstallation.svelte";
  import { zen } from "../lib/zen.svelte";

  let desinstallation = $state(false);
</script>

<Panneau titre="Zen Home" grand large>
  {#snippet marque()}
    <div class="mark"><svg viewBox="0 0 24 24" width="30" height="30" aria-hidden="true"><circle cx="12" cy="12" r="10.5" fill="none" stroke="currentColor" stroke-width="1" /><path d="M7 16c0-5 3.5-8 10-8 0 5-3 8-8 8-.8 0-1.4-.1-2 0Z" fill="currentColor" /></svg></div>
  {/snippet}
  {#if zen.init?.reprise_en_attente}
    <div class="note alerte"><Icone nom="alerte" /><p><b>Une opération précédente a été interrompue.</b> Branchez la box, puis choisissez « Réparer » : Zen Home vérifiera son état et corrigera ce qui reste à faire.</p></div>
  {/if}
  <MiseAJour />
  <p class="lead">Un accueil apaisé pour votre box ou TV Android (Xiaomi Mi Box 4, TV TCL Google TV), sans publicité ni pistage. Il suffit d'une dizaine de minutes.</p>
  <h3 style="margin-top:22px">Avant de commencer</h3>
  <ol class="consignes">
    <li>Allumez la TV et la box (ou la TV Google TV), et laissez-les allumées pendant toute l'installation (environ 10 minutes).</li>
    <li>La box ou la TV et ce Mac doivent être sur le même réseau Wi-Fi.</li>
    <li>À l'étape suivante, vous choisirez votre appareil : Zen Home vous montrera comment y activer le débogage.</li>
    <li>Gardez la télécommande à portée de main : une demande d'autorisation va s'afficher sur la TV, il faudra l'accepter.</li>
  </ol>
  <p class="petit">macOS peut demander si Zen Home peut trouver des appareils sur votre réseau local : acceptez, c'est ce qui permet de retrouver la box.</p>
  <p class="petit"><button class="lien" type="button" onclick={() => (desinstallation = true)}>Désinstaller de ce Mac</button></p>
  {#snippet pied()}
    <button class="btn primary" type="button" onclick={() => zen.demarrer("installer")}><span>Commencer</span><Icone nom="fleche" /></button>
    <span class="espace"></span>
    <button class="lien" type="button" onclick={() => zen.demarrer("reparer")} title="Après une mise à jour de la box">Réparer</button>
    <button class="lien" type="button" onclick={() => zen.demarrer("restaurer")}>Revenir à avant Zen Home</button>
  {/snippet}
</Panneau>
<DialogueDesinstallation bind:ouvert={desinstallation} />
