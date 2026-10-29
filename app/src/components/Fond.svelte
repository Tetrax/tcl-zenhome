<!-- Décor : scène 3D (si WebGL2 est disponible) et repli en dégradés CSS. Purement décoratif, jamais focalisable. -->
<script lang="ts">
  import { onMount } from "svelte";
  import { creerFond, type Clairiere } from "../scene";

  let { scene = $bindable(null) }: { scene: Clairiere | null } = $props();
  let canvas: HTMLCanvasElement;

  onMount(() => {
    const reduit = matchMedia("(prefers-reduced-motion: reduce)").matches;
    document.body.classList.add("nogl"); // le repli sert tant que la 3D n'est pas prête
    let mort = false;
    let s: Clairiere | null = null;
    creerFond(canvas, reduit, (perdu) => document.body.classList.toggle("nogl", perdu)).then((r) => {
      if (mort) { r?.dispose(); return; }
      s = r; scene = r;
      if (r) document.body.classList.remove("nogl");
    });
    return () => { mort = true; s?.dispose(); scene = null; };
  });
</script>

<div id="fond" aria-hidden="true">
  <i data-m="aube"></i><i data-m="matin"></i><i data-m="jour"></i><i data-m="doree"></i><i data-m="crepuscule"></i><i data-m="nuit"></i>
</div>
<canvas id="scene" aria-hidden="true" bind:this={canvas}></canvas>
<div class="vignette" aria-hidden="true"></div>
