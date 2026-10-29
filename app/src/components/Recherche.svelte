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
  const aucune = $derived(zen.rechercheFaite && zen.boxes.length === 0);
  const formulaire = $derived(manuel || aucune);
  const unique = $derived(zen.boxes.length === 1 ? zen.boxes[0] : null);

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
  const toutes = $derived(nbCoches === zen.boxes.length);
  function valider() {
    if (unique) void zen.choisirBoxes([unique.id]);
    else if (nbCoches) void zen.choisirBoxes(zen.boxes.filter((b) => zen.coches.includes(b.id)).map((b) => b.id));
  }
</script>

{#if zen.erreur}
  <BlocErreur />
{:else if cherche}
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
  <Panneau titre={aucune ? "Je ne trouve pas votre box" : unique ? "Mi Box trouvée" : zen.boxes.length > 1 ? "Plusieurs Mi Box trouvées" : "Adresse de la box"}>
    {#if aucune}
      <div class="note alerte"><Icone nom="alerte" /><p>Je n'ai pas trouvé de Mi Box allumée sur ce réseau. Vérifiez qu'elle est allumée (pas en veille) et sur le même Wi-Fi que ce Mac, ou indiquez son adresse.</p></div>
    {:else if unique}
      <div class="carte">
        <span class="icone"><Icone nom="check" taille={24} /></span>
        <div>
          <div class="gros">Mi Box « {unique.nom} »</div>
          <p class="ip" style="margin-top:2px">{unique.ip}</p>
        </div>
      </div>
      <p class="lead" style="margin-top:14px">Est-ce bien la vôtre ?</p>
    {:else if zen.boxes.length > 1}
      <p class="lead">J'ai trouvé plusieurs Mi Box sur ce réseau. Cochez celles que vous voulez configurer : elles auront les mêmes choix.</p>
      <p style="margin:6px 0 0"><button class="lien" type="button" onclick={() => (zen.coches = toutes ? [] : zen.boxes.map((b) => b.id))}>{toutes ? "Tout désélectionner" : "Tout sélectionner"}</button></p>
      <ul class="liste" aria-label="Mi Box trouvées">
        {#each zen.boxes as b (b.id)}
          <li>
            <label class="ligne">
              <input type="checkbox" value={b.id} bind:group={zen.coches} />
              <span class="case"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3.5 8.5 3 3 6-7" /></svg></span>
              <span class="texte"><span class="nom">« {b.nom} »</span><span class="expl ip">{b.ip}</span></span>
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
      {:else if zen.boxes.length > 1 && !formulaire}
        <button class="btn primary" type="button" onclick={valider} disabled={nbCoches === 0}>{nbCoches > 1 ? `Configurer ces ${nbCoches} box` : "Choisir cette box"}</button>
      {/if}
      {#if !formulaire}<button class="btn" type="button" onclick={ouvrirManuel}>Autre adresse</button>{/if}
      <button class="lien" type="button" onclick={() => { manuel = false; void zen.chercher(); }}>Relancer la recherche</button>
      <button class="lien" type="button" onclick={() => zen.retourAccueil()}>Retour</button>
    {/snippet}
  </Panneau>
{/if}
