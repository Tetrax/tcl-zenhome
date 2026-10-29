<!-- Mise à jour de l'appli : vérifiée une fois au démarrage (silence si rien ou si échec), proposée sur l'accueil seulement. -->
<script module lang="ts">
  import type { Update } from "@tauri-apps/plugin-updater";
  let verifiee = false;
  let trouvee = $state<Update | null>(null);
</script>

<script lang="ts">
  import { dansTauri } from "../lib/api";
  import { zen } from "../lib/zen.svelte";

  let etat = $state<"propose" | "telechargement" | "installation" | "erreur">("propose");
  let fraction = $state(0);

  $effect(() => {
    if (verifiee || !dansTauri() || !zen.init || zen.init.demo) return;
    verifiee = true;
    void import("@tauri-apps/plugin-updater").then((m) => m.check()).then((u) => (trouvee = u), () => {});
  });

  async function mettreAJour() {
    if (!trouvee) return;
    etat = "telechargement"; fraction = 0;
    let total = 0, recu = 0;
    try {
      await trouvee.downloadAndInstall((e) => {
        if (e.event === "Started") total = e.data.contentLength ?? 0;
        else if (e.event === "Progress") { recu += e.data.chunkLength; fraction = total ? Math.min(recu / total, 1) : 0; }
        else etat = "installation";
      });
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    } catch (e) {
      console.error(e);
      etat = "erreur";
    }
  }
</script>

{#if trouvee}
  <div class="note" role="status">
    {#if etat === "propose"}
      <p>Nouvelle version {trouvee.version} disponible. <button class="lien" type="button" onclick={mettreAJour}>Mettre à jour</button></p>
    {:else if etat === "telechargement"}
      <p>Téléchargement de la nouvelle version{fraction ? ` (${Math.round(fraction * 100)} %)` : ""}… Ne fermez pas Zen Home.</p>
    {:else if etat === "installation"}
      <p>Installation… Zen Home va se relancer tout seul.</p>
    {:else}
      <p>La mise à jour n'a pas pu se faire. Vérifiez votre connexion internet. <button class="lien" type="button" onclick={mettreAJour}>Réessayer</button></p>
    {/if}
  </div>
{/if}
