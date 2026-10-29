<!-- Envoyer ou copier le rapport de la dernière action (le cœur ajoute la version et le système, filtre et borne le texte). -->
<script lang="ts">
  import Icone from "./Icone.svelte";
  import { api, normaliserErreur } from "../lib/api";
  import { zen } from "../lib/zen.svelte";

  let message = $state("");
  const erreur = () => (zen.erreur ? `${zen.erreur.code} : ${zen.erreur.message}` : "");

  async function envoyer() {
    try {
      await api.envoyerRapport(erreur(), zen.resumeRapport());
      message = "Votre messagerie s'ouvre avec le message prêt : il ne reste qu'à l'envoyer.";
    } catch (e) { message = normaliserErreur(e).message; }
  }
  async function copier() {
    try {
      // le texte est donné sous forme de promesse : WebKit exige que l'écriture démarre pendant le clic
      const texte = api.rapportTexte(erreur(), zen.resumeRapport()).then((t) => new Blob([t], { type: "text/plain" }));
      await navigator.clipboard.write([new ClipboardItem({ "text/plain": texte })]);
      message = "Rapport copié. Collez-le dans un message à hebertval34@gmail.com.";
    } catch { message = "La copie n'a pas fonctionné : utilisez « Envoyer le rapport »."; }
  }
</script>

<div class="note" style="margin-top:20px">
  <Icone nom="info" />
  <div>
    <p>Un souci ? Vous pouvez m'envoyer un rapport : il ne contient ni mot de passe ni clé, et le détail complet reste dans le dossier des sauvegardes.</p>
    <p style="margin-top:10px">
      <button class="btn small" type="button" onclick={envoyer}>Envoyer le rapport</button>
      <button class="btn small" type="button" onclick={copier}>Copier le rapport</button>
    </p>
    <p class="petit" role="status">{message}</p>
  </div>
</div>
