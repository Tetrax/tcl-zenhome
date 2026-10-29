<script lang="ts">
  import { onMount } from "svelte";
  import Panneau from "./Panneau.svelte";
  import Travail from "./Travail.svelte";
  import BlocErreur from "./BlocErreur.svelte";
  import Icone from "./Icone.svelte";
  import { zen } from "../lib/zen.svelte";
  import { ipLocale, ipValide } from "../lib/validation";

  let manuel = $state(zen.forcerManuel);
  let ip = $state("");
  let erreurIp = $state("");
  let champ = $state<HTMLInputElement>();

  onMount(() => {
    if (!zen.rechercheFaite && !zen.rechercheEnCours) void zen.chercher();
  });
  const cherche = $derived(zen.rechercheEnCours || !zen.rechercheFaite);
  // Les box mémorisées sont proposées tout de suite, la recherche continue derrière.
  const props = $derived(zen.propositions);
  const aucune = $derived(zen.rechercheFaite && props.length === 0);
  const formulaire = $derived(manuel || aucune);
  const unique = $derived(props.length === 1 ? props[0] : null);
  const detail = (p: { recente: boolean }) => (p.recente ? (zen.rechercheFaite ? "configurée récemment, n'a pas répondu à la recherche" : "configurée récemment") : "");

  function ouvrirManuel() {
    manuel = true;
    setTimeout(() => champ?.focus(), 80);
  }
  function envoyer(e: SubmitEvent) {
    e.preventDefault();
    const v = ip.replace(/\s+/g, "");
    if (!ipValide(v)) {
      erreurIp = "Cette adresse n'a pas l'air valide. Elle doit avoir 4 nombres de 0 à 255 séparés par des points, par exemple 192.168.1.23.";
      return;
    }
    if (!ipLocale(v)) {
      erreurIp = "Cette adresse n'est pas celle d'un appareil de votre réseau local. Elle commence en général par 192.168, 10 ou 172.16 à 31 : vérifiez-la sur la box.";
      return;
    }
    erreurIp = "";
    void zen.connecter({ ip: v });
  }
  // Plusieurs box : on coche celles à configurer ; une seule cochée = parcours habituel.
  const nbCoches = $derived(zen.coches.length);
  const toutes = $derived(nbCoches === props.length);
  function valider() {
    if (unique) void zen.choisirBoxes([unique.ip]);
    else if (nbCoches) void zen.choisirBoxes(zen.coches);
  }
</script>

{#if zen.erreur}
  <BlocErreur />
{:else if cherche && props.length === 0}
  <Panneau titre="Recherche de votre Mi Box">
    <div class="travail" style="align-items:flex-start;text-align:left">
      <div class="radar" aria-hidden="true"><i></i><i></i><i></i></div>
      <p class="lead" style="margin-top:0" aria-live="polite">Gardez la TV allumée. Votre ordinateur et la box doivent être sur le même réseau Wi-Fi.</p>
      <p class="petit">Un instant, Zen Home regarde qui répond autour de lui<span class="dots" aria-hidden="true"><i>.</i><i>.</i><i>.</i></span></p>
    </div>
    {#snippet pied()}
      <button class="btn small" type="button" onclick={() => { manuel = true; void zen.annuler(); }}>Saisir l'adresse moi-même</button>
      <button class="lien" type="button" onclick={() => zen.retourAccueil()}>Retour</button>
    {/snippet}
  </Panneau>
{:else}
  <Panneau titre={cherche ? "Recherche de votre Mi Box" : aucune ? "Je ne trouve pas votre box" : unique ? (unique.recente ? "Mi Box connue" : "Mi Box trouvée") : props.length > 1 ? "Plusieurs Mi Box trouvées" : "Adresse de la box"}>
    {#if cherche}
      <p class="petit" style="margin:0 0 12px" aria-live="polite">La recherche continue<span class="dots" aria-hidden="true"><i>.</i><i>.</i><i>.</i></span> Vous pouvez déjà choisir une box configurée récemment.</p>
    {/if}
    {#if aucune}
      <div class="note alerte"><Icone nom="alerte" /><p>Je n'ai pas trouvé de Mi Box allumée sur ce réseau. Vérifiez qu'elle est allumée (pas en veille) et sur le même Wi-Fi que ce Mac, ou indiquez son adresse.</p></div>
    {:else if unique}
      <div class="carte">
        <span class="icone"><Icone nom="check" taille={24} /></span>
        <div>
          <div class="gros">Mi Box « {unique.nom} »</div>
          <p class="ip" style="margin-top:2px">{unique.ip}{detail(unique) ? ` · ${detail(unique)}` : ""}</p>
        </div>
      </div>
      <p class="lead" style="margin-top:14px">Est-ce bien la vôtre ?</p>
    {:else if props.length > 1}
      <p class="lead">Plusieurs Mi Box sont proposées sur ce réseau. Cochez celles que vous voulez configurer : elles auront les mêmes choix.</p>
      <p style="margin:6px 0 0"><button class="lien" type="button" onclick={() => (zen.coches = toutes ? [] : props.map((b) => b.ip))}>{toutes ? "Tout désélectionner" : "Tout sélectionner"}</button></p>
      <ul class="liste" aria-label="Mi Box trouvées">
        {#each props as b (b.ip)}
          <li>
            <label class="ligne">
              <input type="checkbox" value={b.ip} bind:group={zen.coches} />
              <span class="case"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3.5 8.5 3 3 6-7" /></svg></span>
              <span class="texte"><span class="nom">« {b.nom} »</span><span class="expl ip">{b.ip}{detail(b) ? ` · ${detail(b)}` : ""}</span></span>
            </label>
          </li>
        {/each}
      </ul>
    {/if}

    {#if formulaire}
      <form class="champ" onsubmit={envoyer} novalidate>
        <label for="champ-ip">Quelle est l'adresse IP de votre box ?</label>
        <div class="rang">
          <input id="champ-ip" bind:this={champ} bind:value={ip} inputmode="decimal" autocomplete="off" spellcheck="false" placeholder="192.168.1.23"
            aria-invalid={erreurIp ? "true" : undefined} aria-describedby="aide-ip erreur-ip" />
          <button class="btn primary small" type="submit">Utiliser</button>
        </div>
        <p class="msg" id="erreur-ip" role="alert">{erreurIp}</p>
        <p class="petit" id="aide-ip" style="margin-top:0">
          Sur la box : <span class="chemin">Paramètres &gt; Préférences de l'appareil &gt; À propos &gt; État &gt; Adresse IP</span>. Elle ressemble à 192.168.1.23.
          Le « débogage USB » doit être activé (Options pour les concepteurs).
        </p>
      </form>
    {/if}
    {#snippet pied()}
      {#if unique && !formulaire}
        <button class="btn primary" type="button" onclick={valider}>C'est elle</button>
      {:else if props.length > 1 && !formulaire}
        <button class="btn primary" type="button" onclick={valider} disabled={nbCoches === 0}>{nbCoches > 1 ? `Configurer ces ${nbCoches} box` : "Choisir cette box"}</button>
      {/if}
      {#if !formulaire}<button class="btn" type="button" onclick={ouvrirManuel}>Autre adresse</button>{/if}
      {#if !cherche}<button class="lien" type="button" onclick={() => { manuel = false; void zen.chercher(); }}>Relancer la recherche</button>{/if}
      <button class="lien" type="button" onclick={() => zen.retourAccueil()}>Retour</button>
    {/snippet}
  </Panneau>
{/if}
