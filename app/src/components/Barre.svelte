<script lang="ts">
  import { zen, ETAPES, ETAPE_DE } from "../lib/zen.svelte";
  const courante = $derived(ETAPE_DE[zen.ecran]);
</script>

<header class="topbar">
  <div class="brand">
    <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true"><circle cx="12" cy="12" r="10.5" fill="none" stroke="currentColor" stroke-width="1.3" /><path d="M7 16c0-5 3.5-8 10-8 0 5-3 8-8 8-.8 0-1.4-.1-2 0Z" fill="currentColor" opacity=".9" /><path d="M7 17c2-3 4-5 7-6.5" stroke="#10202c" stroke-width="1" fill="none" stroke-linecap="round" /></svg>
    <span>Zen Home</span>
  </div>
  <ol class="steps" aria-label="Étapes">
    {#each ETAPES as nom, i}
      <li aria-current={i === courante ? "step" : undefined} class:fait={i < courante}><b></b><span>{nom}</span></li>
    {/each}
  </ol>
</header>
