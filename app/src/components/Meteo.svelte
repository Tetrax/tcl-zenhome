<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import { zen } from "../lib/zen.svelte";
  import { api, normaliserErreur } from "../lib/api";
  import type { Ville } from "../lib/types";

  let saisie = $state(zen.ville?.nom ?? "");
  let propositions = $state<Ville[]>([]);
  let etat = $state<"repos" | "cherche" | "fait" | "erreur">("repos");
  let message = $state("");
  let jeton = 0;

  const lib = (v: Ville) => [v.region, v.pays].filter(Boolean).join(", ");

  async function chercher(e?: Event) {
    e?.preventDefault();
    const s = saisie.trim();
    if (s.length < 2) { propositions = []; etat = "repos"; message = s ? "Tapez au moins 2 lettres." : ""; return; }
    const moi = ++jeton;
    etat = "cherche"; message = "";
    try {
      const r = await api.chercherVilles(s);
      if (moi !== jeton) return;
      propositions = r; etat = "fait";
      if (zen.ville && !r.some((v) => v.id === zen.ville!.id)) zen.ville = null;
    } catch (err) {
      if (moi !== jeton) return;
      propositions = []; etat = "erreur";
      message = normaliserErreur(err).message + " Vous pouvez continuer sans météo.";
    }
  }
  function surSaisie() {
    zen.ville = null;
    clearTimeout(minuteur);
    minuteur = setTimeout(() => void chercher(), 450);
  }
  let minuteur: ReturnType<typeof setTimeout>;
</script>

<Panneau titre="Météo (facultatif)">
  <p class="lead">Dans quelle ville habitez-vous ? Zen Home affichera sa météo.</p>
  <p class="petit">Le nom de la ville est envoyé au service gratuit Open-Meteo pour trouver sa position. Laissez vide pour ne pas avoir de météo.</p>
  <form class="champ" onsubmit={chercher}>
    <label for="ville">Ville</label>
    <div class="rang">
      <input id="ville" bind:value={saisie} oninput={surSaisie} autocomplete="off" spellcheck="false" placeholder="Lyon" aria-describedby="etat-ville" />
      <button class="btn small" type="submit">Chercher</button>
    </div>
  </form>
  <div id="etat-ville" aria-live="polite">
    {#if etat === "cherche"}<p class="petit">Recherche…</p>
    {:else if etat === "fait" && propositions.length === 0}<p class="petit">Je n'ai trouvé aucune ville de ce nom. Essayez une autre orthographe, ou continuez sans météo.</p>
    {:else if message}<p class="msg {etat === 'erreur' ? '' : 'info'}">{message}</p>
    {:else if etat === "fait"}<p class="petit">{propositions.length} proposition{propositions.length > 1 ? "s" : ""} : choisissez la bonne.</p>{/if}
  </div>
  {#if propositions.length}
    <ul class="liste" role="radiogroup" aria-label="Villes proposées" style="margin-top:6px">
      {#each propositions as v (v.id)}
        <li>
          <label class="ligne">
            <input type="radio" name="ville" checked={zen.ville?.id === v.id} onchange={() => (zen.ville = v)} />
            <span class="case"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3.5 8.5 3 3 6-7" /></svg></span>
            <span class="texte"><span class="nom">{v.nom}</span>{#if lib(v)}<span class="expl">{lib(v)}</span>{/if}</span>
          </label>
        </li>
      {/each}
    </ul>
  {/if}
  {#snippet pied()}
    <button class="btn primary" type="button" onclick={() => zen.aller("films")}>
      <span>{zen.ville ? `Choisir ${zen.ville.nom}` : "Passer"}</span><Icone nom="fleche" />
    </button>
    <button class="lien" type="button" onclick={() => zen.aller("resume")}>Retour</button>
  {/snippet}
</Panneau>
