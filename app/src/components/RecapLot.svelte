<!-- Plusieurs box : récapitulatif final, box par box. -->
<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import { zen, type BoxLot } from "../lib/zen.svelte";

  const lot = $derived(zen.lot ?? []);
  const nbOk = $derived(lot.filter((b) => b.etat === "ok").length);
  const reprises = $derived(lot.filter((b) => b.etat === "echec" || b.etat === "non-traitee").length);
  const aRegarder = (b: BoxLot) => b.etat === "ok" && ((b.verification?.ko ?? 0) > 0 || !!b.restauration?.incomplete);
  const parfait = $derived(reprises === 0 && !lot.some(aRegarder));
  const peutReessayer = $derived(reprises > 0 && (zen.mode !== "installer" || zen.lotPasse === 2));
  const restauration = $derived(zen.mode === "restaurer");
  const titre = $derived(!parfait ? "Presque terminé" : restauration ? "Vos box sont revenues à leur état d'origine" : zen.mode === "reparer" ? "Vos box sont réparées" : "Zen Home est prêt");

  function detail(b: BoxLot): string {
    if (b.etat === "non-traitee") return "Cette box n'a pas été traitée.";
    if (b.etat !== "ok") return b.message;
    const ko = b.verification?.ko ?? 0;
    const verif = ko ? `${ko} point${ko > 1 ? "s" : ""} à regarder` : "tout est en ordre";
    const maj = (t: string) => t.charAt(0).toUpperCase() + t.slice(1);
    if (b.restauration) {
      const r = b.restauration;
      const d = r.depuis_sauvegarde ? `Remise comme dans la sauvegarde du ${r.depuis_sauvegarde}` : "Restauration générale";
      return r.incomplete ? `${d} ; Zen Home a été conservé par sécurité, relancez le retour une fois la box reconnectée` : d;
    }
    if (b.rapport) return `Applis : ${b.rapport.desactivees} désactivées, ${b.rapport.gardees} gardées, ${b.rapport.desinstallees.length} désinstallées ; ${verif}${b.horsChoix ? ". Cette box n'a pas pris part à vos choix : aucune appli n'y a été désinstallée." : ""}`;
    return maj(verif);
  }
</script>

<Panneau {titre} large>
  {#snippet marque()}
    <div class="grand-check" aria-hidden="true">
      <svg viewBox="0 0 80 80" width="72" height="72"><circle cx="40" cy="40" r="36" class="cercle" /><path d="m24 41 11 11 22-24" class="coche" /></svg>
    </div>
  {/snippet}
  <p class="lead" aria-live="polite">
    {#if parfait}{restauration ? "Toutes vos box sont comme avant Zen Home." : "Rallumez les TV : votre accueil apaisé vous attend."}
    {:else}{nbOk} box sur {lot.length} {nbOk > 1 ? "sont terminées" : "est terminée"}. Les autres sont signalées ci-dessous.{/if}
  </p>
  <table class="tableau">
    <caption>Vos box</caption>
    <thead class="sr"><tr><th scope="col">Box</th><th scope="col">État</th></tr></thead>
    <tbody>
      {#each lot as b (b.ip)}
        <tr>
          <th scope="row">« {b.nom} » <span class="petit">{b.ip}</span>
            <span class="detail">{detail(b)}</span>
            {#each b.restauration?.avertissements ?? [] as a}<span class="detail">{a}</span>{/each}
            {#if b.debogage}<span class="detail">{b.debogage === "oui" ? "Débogage coupé." : "Le débogage n'a pas pu être coupé."}</span>{/if}
          </th>
          <td class="etat">
            <span class="badge {b.etat === 'ok' ? (aRegarder(b) ? 'avert' : 'ok') : 'ko'}">
              <Icone nom={b.etat === "ok" ? (aRegarder(b) ? "info" : "check") : "alerte"} />{b.etat === "ok" ? (aRegarder(b) ? "À regarder" : "OK") : b.etat === "echec" ? "Échec" : "Non traitée"}
            </span>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if !restauration && nbOk > 0}
    <div class="note" style="margin-top:20px">
      <Icone nom="info" />
      <div>
        <p><b>Pour votre sécurité</b>, vous pouvez couper le « débogage USB » des box : sinon n'importe quel appareil de votre réseau Wi-Fi pourrait les piloter. Il faudra le réactiver pour utiliser « Réparer ».</p>
        <p style="margin-top:10px">
          {#if zen.debogageCoupe === "oui"}<b>Débogage coupé</b> sur toutes les box.
          {:else}<button class="btn small" type="button" disabled={zen.debogageCoupe === "en_cours"} class:busy={zen.debogageCoupe === "en_cours"} onclick={() => zen.couperTout()}>Couper le débogage sur toutes les box</button>
            <span class="petit"> {reprises ? "Les box en échec ne sont pas concernées. " : ""}Sinon, il reste activé.</span>{/if}
        </p>
      </div>
    </div>
  {/if}
  {#snippet pied()}
    {#if peutReessayer}<button class="btn primary" type="button" onclick={() => zen.reessayerLot()}>Réessayer les box en échec</button>{/if}
    <button class="btn" class:primary={!peutReessayer} type="button" onclick={() => zen.aller("adieu")}>Terminer</button>
    <button class="btn" type="button" onclick={() => zen.ouvrirSauvegardes()}>Ouvrir les sauvegardes</button>
  {/snippet}
</Panneau>
