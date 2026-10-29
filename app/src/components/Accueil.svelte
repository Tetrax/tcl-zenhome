<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import { zen } from "../lib/zen.svelte";
</script>

<Panneau titre="Zen Home" grand large>
  {#snippet marque()}
    <div class="mark"><svg viewBox="0 0 24 24" width="30" height="30" aria-hidden="true"><circle cx="12" cy="12" r="10.5" fill="none" stroke="currentColor" stroke-width="1" /><path d="M7 16c0-5 3.5-8 10-8 0 5-3 8-8 8-.8 0-1.4-.1-2 0Z" fill="currentColor" /></svg></div>
  {/snippet}
  {#if zen.init?.reprise_en_attente}
    <div class="note alerte"><Icone nom="alerte" /><p><b>Une opération précédente a été interrompue.</b> Branchez la box, puis choisissez « Réparer » : Zen Home vérifiera son état et corrigera ce qui reste à faire.</p></div>
  {/if}
  <p class="lead">Un accueil apaisé pour votre Xiaomi Mi Box 4, sans publicité ni pistage. Il suffit d'une dizaine de minutes.</p>
  <h3 style="margin-top:22px">Avant de commencer</h3>
  <ol class="consignes">
    <li>Allumez la TV et la Mi Box, et laissez-les allumées pendant toute l'installation (environ 10 minutes).</li>
    <li>La Mi Box et ce Mac doivent être sur le même réseau Wi-Fi.</li>
    <li>
      Sur la box, débloquez les options cachées : <span class="chemin">Paramètres &gt; Préférences de l'appareil &gt; À propos</span>,
      descendez jusqu'à « Build » et appuyez 7 fois sur OK, jusqu'au message « Vous êtes maintenant développeur ».
    </li>
    <li>
      Revenez en arrière, puis activez le débogage :
      <span class="chemin">Préférences de l'appareil &gt; Options pour les concepteurs &gt; Débogage USB</span>.
    </li>
    <li>Gardez la télécommande à portée de main : une demande d'autorisation va s'afficher sur la TV, il faudra l'accepter.</li>
  </ol>
  <p class="petit">macOS peut demander si Zen Home peut trouver des appareils sur votre réseau local : acceptez, c'est ce qui permet de retrouver la box.</p>
  {#snippet pied()}
    <button class="btn primary" type="button" onclick={() => zen.demarrer("installer")}><span>Commencer</span><Icone nom="fleche" /></button>
    <span class="espace"></span>
    <button class="lien" type="button" onclick={() => zen.demarrer("reparer")} title="Après une mise à jour de la box">Réparer</button>
    <button class="lien" type="button" onclick={() => zen.demarrer("restaurer")}>Tout annuler</button>
  {/snippet}
</Panneau>
