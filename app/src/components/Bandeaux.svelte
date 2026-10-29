<!-- Bandeaux discrets : mode démo (permanent) et état de la connexion avec la box. -->
<script lang="ts">
  import { zen } from "../lib/zen.svelte";
</script>

{#if zen.init?.demo}<div class="demo">Mode démo</div>{/if}
<div class="bandeaux" role="status" aria-live="polite">
  {#if zen.reco}
    {#if "essai" in zen.reco}
      <div class="bandeau alerte"><span class="pulse-dot"></span>Connexion avec la box perdue, nouvelle tentative (essai {zen.reco.essai})…</div>
    {:else}
      <div class="bandeau">Box reconnectée, on continue.</div>
    {/if}
  {/if}
</div>
