<!-- Désinstaller Zen Home de ce Mac : confirmation, avec le choix de garder ou non les sauvegardes. La box n'est pas touchée. -->
<script lang="ts">
  import { api, normaliserErreur } from "../lib/api";
  import { zen } from "../lib/zen.svelte";
  import type { ErreurZen } from "../lib/types";

  let { ouvert = $bindable(false) }: { ouvert: boolean } = $props();
  let dlg: HTMLDialogElement;
  let defaut = $state<HTMLButtonElement>();
  let etat = $state<"choix" | "en_cours" | "fini">("choix");
  let erreur = $state<ErreurZen | null>(null);
  const sauvegardes = $derived(zen.init?.sauvegardes_dir_existe ?? false);

  // Des sauvegardes ont pu être écrites depuis le démarrage : l'état est relu avant de proposer le choix.
  async function ouvrir() {
    try { zen.init = await api.etatInitial(); } catch { /* on garde l'état connu */ }
    if (!ouvert || dlg.open) return;
    etat = "choix"; erreur = null; dlg.showModal(); setTimeout(() => defaut?.focus(), 30);
  }

  $effect(() => {
    if (ouvert && !dlg.open) void ouvrir();
    else if (!ouvert && dlg.open) dlg.close();
  });

  async function desinstaller(garder: boolean) {
    etat = "en_cours"; erreur = null;
    try { await api.desinstallerMac(garder); etat = "fini"; } catch (e) { erreur = normaliserErreur(e); etat = "choix"; }
  }
</script>

<!-- Échap ferme, sauf pendant la désinstallation. -->
<dialog class="dlg" bind:this={dlg} aria-labelledby="dlg-desinstaller" oncancel={(e) => { if (etat === "en_cours") e.preventDefault(); }} onclose={() => (ouvert = false)}>
  {#if etat === "fini"}
    <h2 id="dlg-desinstaller" tabindex="-1">Zen Home a été retiré de ce Mac</h2>
    <p class="lead">L'application se ferme. Si elle reste ouverte, vous pouvez la fermer.</p>
  {:else if etat === "en_cours"}
    <h2 id="dlg-desinstaller" tabindex="-1">Désinstallation en cours…</h2>
    <p class="lead">Ne fermez pas cette fenêtre.</p>
  {:else}
    <h2 id="dlg-desinstaller" tabindex="-1">Désinstaller Zen Home de ce Mac ?</h2>
    <p class="lead">L'application et ses fichiers seront retirés de ce Mac. L'application est mise à la Corbeille.</p>
    <div class="note"><p>Votre Mi Box n'est pas modifiée : Zen Home reste installé sur la TV. Pour l'enlever de la TV, choisissez d'abord « Revenir à avant Zen Home ».</p></div>
    {#if sauvegardes}
      <p class="lead" style="margin-top:16px">Il y a des sauvegardes de la configuration d'origine de votre Mi Box. Que voulez-vous en faire ?</p>
    {/if}
    {#if erreur}<div class="note alerte"><p>{erreur.message}</p></div>{/if}
    <div class="pied" style="animation:none">
      {#if sauvegardes}
        <button class="btn primary" type="button" bind:this={defaut} onclick={() => desinstaller(true)}>Les garder dans Documents</button>
        <button class="btn danger" type="button" onclick={() => desinstaller(false)}>Les supprimer</button>
        <button class="btn" type="button" onclick={() => (ouvert = false)}>Annuler</button>
      {:else}
        <button class="btn primary" type="button" bind:this={defaut} onclick={() => (ouvert = false)}>Annuler</button>
        <button class="btn danger" type="button" onclick={() => desinstaller(false)}>Désinstaller de ce Mac</button>
      {/if}
    </div>
  {/if}
</dialog>
