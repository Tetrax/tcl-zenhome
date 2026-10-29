<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import { zen } from "../lib/zen.svelte";
  import type { AppliSysteme } from "../lib/types";

  const liste = $derived(zen.inventaire?.applis_systeme ?? []);
  const total = $derived(liste.length);
  const n = $derived(liste.filter((a) => zen.desactiver[a.paquet]).length);

  // Groupes visuels : « pistage et publicité » / « au choix » (champ `groupe` du cœur) ; à défaut, selon la recommandation.
  const groupes = $derived.by(() => {
    const aGroupe = liste.some((a) => a.groupe);
    const defs: { cle: string; titre: string; test: (a: AppliSysteme) => boolean }[] = aGroupe
      ? [
          { cle: "p", titre: "Pistage et publicité", test: (a) => a.groupe === "pistage" },
          { cle: "o", titre: "Au choix", test: (a) => a.groupe !== "pistage" },
        ]
      : [
          { cle: "d", titre: "Recommandé : désactiver", test: (a) => a.recommande === "desactiver" },
          { cle: "g", titre: "Recommandé : garder", test: (a) => a.recommande === "garder" },
        ];
    return defs.map((g) => ({ ...g, applis: liste.filter(g.test) })).filter((g) => g.applis.length);
  });

  let saut = $state(false);
  let precedent = -1;
  $effect(() => {
    if (precedent >= 0 && n !== precedent) { saut = false; requestAnimationFrame(() => (saut = true)); }
    precedent = n;
  });

  function suite() {
    const suivant = (zen.inventaire?.applis_utilisateur.length ?? 0) > 0 ? "utilisateur" : "recap";
    zen.aller(suivant);
  }
  function precedente() { zen.aller("films"); }
</script>

<Panneau titre="Alléger votre box" large>
  <p class="lead">La box contient {total} applis qui ne servent à rien avec Zen Home (publicité, pistage, rapports envoyés à Xiaomi ou Google) ou dont vous n'avez peut-être pas besoin. Elles sont <b>désactivées, pas supprimées</b> : « Tout annuler » les remet.</p>
  <div class="barre-liste">
    <span>
      <button class="lien" type="button" onclick={() => zen.recommande()}>Recommandé</button>
      <button class="lien" type="button" onclick={() => zen.toutSelectionner(n !== total)}>{n === total ? "Tout désélectionner" : "Tout sélectionner"}</button>
    </span>
    <p class="compteur" aria-live="polite">
      {#if n === 0}Aucune appli ne sera désactivée{:else if n === 1}<b class:saut>1</b> appli sera désactivée{:else}<b class:saut>{n}</b> applis seront désactivées{/if}
    </p>
  </div>
  {#each groupes as g (g.cle)}
    <div role="group" aria-labelledby={`g-${g.cle}`}>
      <div class="groupe" id={`g-${g.cle}`}>{g.titre}</div>
      <ul class="liste" style="margin-top:0">
        {#each g.applis as a (a.paquet)}
          <li>
            <label class="ligne">
              <input type="checkbox" bind:checked={zen.desactiver[a.paquet]} />
              <span class="case"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3.5 8.5 3 3 6-7" /></svg></span>
              <span class="texte">
                <span class="nom">{a.libelle}
                  {#if a.recommande === "garder"}<span class="puce garder">Mieux vaut garder</span>
                  {:else if a.groupe === "option"}<span class="puce">Au choix</span>{/if}
                  {#if a.etat === "desactive"}<span class="puce">Déjà désactivée</span>{/if}
                </span>
                <span class="expl">{a.explication}</span>
              </span>
            </label>
          </li>
        {/each}
      </ul>
    </div>
  {/each}
  {#snippet pied()}
    <button class="btn primary" type="button" onclick={suite}><span>Continuer</span><Icone nom="fleche" /></button>
    <button class="lien" type="button" onclick={precedente}>Retour</button>
  {/snippet}
</Panneau>
