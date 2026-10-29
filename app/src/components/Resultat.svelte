<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import Rapport from "./Rapport.svelte";
  import { zen } from "../lib/zen.svelte";
  import { humaniser, libelleEtat } from "../lib/libelles";

  const v = $derived(zen.verification);
  const noms = $derived(new Map([...(zen.inventaire?.applis_systeme ?? []), ...(zen.inventaire?.applis_utilisateur ?? [])].map((a) => [a.paquet, a.libelle])));
  const lignes = $derived((v?.lignes ?? []).map((l) => humaniser(l, noms)));
  const principales = $derived(lignes.filter((l) => !l.appli));
  const applis = $derived(lignes.filter((l) => l.appli));
  const nb = (e: string) => lignes.filter((l) => l.etat === e).length;
  const parfait = $derived((v?.ko ?? 0) === 0);
  const reparation = $derived(zen.mode === "reparer");
  const titre = $derived(parfait ? (reparation ? "Votre box est réparée" : "Zen Home est prêt") : "Presque terminé");
</script>

{#snippet tableau(legende: string, rows: typeof lignes)}
  <table class="tableau">
    <caption>{legende}</caption>
    <thead class="sr"><tr><th scope="col">Contrôle</th><th scope="col">État</th></tr></thead>
    <tbody>
      {#each rows as l}
        <tr>
          <th scope="row">{l.libelle}{#if l.detail}<span class="detail">{l.detail}</span>{/if}</th>
          <td class="etat"><span class="badge {l.etat}"><Icone nom={l.etat === "ok" ? "check" : l.etat === "avert" ? "info" : "alerte"} />{libelleEtat[l.etat]}</span></td>
        </tr>
      {/each}
    </tbody>
  </table>
{/snippet}

<Panneau {titre} large>
  {#snippet marque()}
    <div class="grand-check" aria-hidden="true">
      <svg viewBox="0 0 80 80" width="72" height="72"><circle cx="40" cy="40" r="36" class="cercle" /><path d="m24 41 11 11 22-24" class="coche" /></svg>
    </div>
  {/snippet}
  {#if parfait}
    <p class="lead">Rallumez la TV : votre accueil apaisé vous attend.</p>
  {:else}
    <div class="note alerte"><Icone nom="alerte" /><p>Quelques points sont à regarder ci-dessous. Vous pouvez relancer « Réparer » depuis l'accueil, ou consulter le rapport dans les sauvegardes.</p></div>
  {/if}
  {#if zen.rapport}
    <dl class="grille">
      <div><dt>Applis désactivées</dt><dd class="grand">{zen.rapport.desactivees}</dd></div>
      <div><dt>Applis gardées</dt><dd class="grand">{zen.rapport.gardees}</dd></div>
      <div><dt>Applis désinstallées</dt><dd class="grand">{zen.rapport.desinstallees.length}</dd></div>
      {#if zen.rapport.echecs.length}<div><dt>Désinstallations échouées</dt><dd class="grand">{zen.rapport.echecs.length}</dd></div>{/if}
    </dl>
  {/if}
  {#if v}
    <p class="petit" style="margin-top:16px" aria-live="polite">Vérification finale : {nb("ok")} en ordre{nb("avert") ? `, ${nb("avert")} à noter` : ""}{nb("ko") ? `, ${nb("ko")} à regarder` : ""}.</p>
    {@render tableau("Zen Home sur votre box", principales)}
    {#if applis.length}
      <details>
        <summary>Détail des {applis.length} applis allégées</summary>
        {@render tableau("Applis allégées", applis)}
      </details>
    {/if}
    <details>
      <summary>Détails techniques</summary>
      <ul class="petit" style="margin:4px 0 0;padding-left:18px">{#each lignes as l}<li>{l.etat.toUpperCase()} · {l.brut}</li>{/each}</ul>
    </details>
  {/if}
  <div class="note" style="margin-top:20px">
    <Icone nom="info" />
    <div>
      <p><b>Pour votre sécurité</b>, vous pouvez couper le « débogage USB » de la box : sinon n'importe quel appareil de votre réseau Wi-Fi pourrait la piloter. Il faudra le réactiver pour utiliser « Réparer ».</p>
      <p style="margin-top:10px">
        {#if zen.debogageCoupe === "oui"}<b>Débogage coupé.</b> La box ne peut plus être pilotée depuis le réseau.
        {:else}<button class="btn small" type="button" disabled={zen.debogageCoupe === "en_cours"} class:busy={zen.debogageCoupe === "en_cours"} onclick={() => zen.couperDebogage()}>Couper le débogage</button>
          <span class="petit"> Sinon, il reste activé.</span>{/if}
      </p>
    </div>
  </div>
  {#if !parfait}<Rapport />{/if}
  {#if zen.erreur}<p class="msg" role="alert" style="margin-top:12px">{zen.erreur.message}</p>{/if}
  {#snippet pied()}
    <button class="btn primary" type="button" onclick={() => zen.aller("adieu")}>Terminer</button>
    <button class="btn" type="button" onclick={() => zen.ouvrirSauvegardes()}>Ouvrir les sauvegardes</button>
  {/snippet}
</Panneau>
