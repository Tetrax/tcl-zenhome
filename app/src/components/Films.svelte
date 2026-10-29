<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import { zen, jetonValide } from "../lib/zen.svelte";

  let montre = $state(false);
  let erreur = $state("");
  function suite(e: Event) {
    e.preventDefault();
    const j = zen.jeton.replace(/[\s]+/g, "");
    if (j && !jetonValide(j)) {
      erreur = "Ce texte ne ressemble pas à un jeton TMDB : il ne contient que des lettres, des chiffres et quelques signes (. _ - ~ + / =). Vérifiez ce que vous avez collé, ou laissez vide.";
      return;
    }
    zen.jeton = j; erreur = "";
    void zen.installerZenHome();
  }
</script>

<Panneau titre="Films (facultatif)" large>
  <p class="lead">Pour afficher des films et des bandes-annonces, Zen Home utilise le service gratuit TMDB. Il faut un « jeton » personnel :</p>
  <ol class="consignes">
    <li>Créez un compte gratuit sur <span class="url">https://www.themoviedb.org</span></li>
    <li>Ouvrez <span class="chemin">Paramètres &gt; API</span></li>
    <li>Copiez le « API Read Access Token » (un très long texte)</li>
    <li>Collez-le ci-dessous (il reste masqué)</li>
  </ol>
  <form class="champ" onsubmit={suite} id="form-films" novalidate>
    <label for="jeton">Jeton TMDB</label>
    <div class="rang">
      <input id="jeton" type={montre ? "text" : "password"} bind:value={zen.jeton} autocomplete="off" autocapitalize="off" spellcheck="false" placeholder="Collez le jeton ici"
        aria-invalid={erreur ? "true" : undefined} aria-describedby="erreur-jeton" />
      <button class="btn small" type="button" aria-pressed={montre} onclick={() => (montre = !montre)}><Icone nom="oeil" taille={16} />{montre ? "Masquer" : "Afficher"}</button>
    </div>
    <p class="msg" id="erreur-jeton" role="alert">{erreur}</p>
  </form>
  <p class="petit">Vous pouvez laisser vide : tout fonctionnera, sauf les films.</p>
  {#snippet pied()}
    <button class="btn primary" type="submit" form="form-films"><span>{zen.jeton.trim() ? "Continuer" : "Passer"}</span><Icone nom="fleche" /></button>
    <button class="lien" type="button" onclick={() => zen.aller("meteo")}>Retour</button>
  {/snippet}
</Panneau>
