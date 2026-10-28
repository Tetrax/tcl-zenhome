<script lang="ts">
  import { ping, isDemo, type PingInfo } from "./lib/api";

  let info = $state<PingInfo | null>(null);
  let error = $state<string | null>(null);

  ping().then((r) => (info = r), (e) => (error = String(e)));
</script>

<main>
  <h1>Zen Home</h1>
  {#if error}
    <p class="err">Erreur : {error}</p>
  {:else if info}
    <p>
      Version {info.version} · {info.os} · {info.arch}
      {#if isDemo()}<span class="demo">(mode démo)</span>{/if}
    </p>
  {:else}
    <p>Chargement…</p>
  {/if}
</main>

<style>
  main {
    display: grid;
    place-content: center;
    height: 100%;
    text-align: center;
  }
  .demo { opacity: 0.6; }
  .err { color: #ff8080; }
</style>
