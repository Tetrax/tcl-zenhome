<script lang="ts">
  import { onMount, type Snippet } from "svelte";

  let { titre, grand = false, large = false, marque, children, pied }: {
    titre: string; grand?: boolean; large?: boolean; marque?: Snippet; children: Snippet; pied?: Snippet;
  } = $props();
  let entete = $state<HTMLElement>();
  let panneau: HTMLElement;

  // Le titre prend le focus à l'arrivée : le lecteur d'écran annonce l'écran, le clavier repart d'ici.
  onMount(() => {
    // Les écrans de confirmation marquent leur bouton sûr par data-defaut : il prend le focus à la place du titre.
    const t = setTimeout(() => (panneau.querySelector<HTMLElement>("[data-defaut]") ?? entete)?.focus({ preventScroll: true }), 400);
    return () => clearTimeout(t);
  });
</script>

<section class="screen">
  <div class="panel" class:large bind:this={panneau}>
    <div class="entete">
      {#if marque}{@render marque()}{/if}
      {#if grand}<h1 bind:this={entete} tabindex="-1">{titre}</h1>{:else}<h2 bind:this={entete} tabindex="-1">{titre}</h2>{/if}
    </div>
    <div class="corps">{@render children()}</div>
    {#if pied}<div class="pied">{@render pied()}</div>{/if}
  </div>
</section>
