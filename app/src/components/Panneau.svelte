<script lang="ts">
  import { onMount, type Snippet } from "svelte";

  let { titre, grand = false, large = false, marque, children, pied }: {
    titre: string; grand?: boolean; large?: boolean; marque?: Snippet; children: Snippet; pied?: Snippet;
  } = $props();
  let entete = $state<HTMLElement>();
  let panneau: HTMLElement;
  let corps: HTMLElement;
  // Vrai tant qu'il reste du contenu caché sous le bas du corps : un fondu signale qu'on peut faire défiler.
  let suite = $state(false);
  // Vrai quand le corps est défilé : un fondu en haut évite que le texte paraisse chevaucher le titre.
  let haut = $state(false);
  const mesurer = () => {
    suite = corps.scrollHeight - corps.scrollTop - corps.clientHeight > 4;
    haut = corps.scrollTop > 4;
  };

  onMount(() => {
    const ro = new ResizeObserver(mesurer);
    ro.observe(corps);
    for (const e of corps.children) ro.observe(e);
    return () => ro.disconnect();
  });

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
    <div class="corps" class:suite class:haut bind:this={corps} onscroll={mesurer}>{@render children()}</div>
    {#if pied}<div class="pied">{@render pied()}</div>{/if}
  </div>
</section>
