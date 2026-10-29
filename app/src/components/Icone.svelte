<script lang="ts">
  let { nom, taille = 18 }: { nom: "check" | "fleche" | "info" | "alerte" | "box" | "oeil"; taille?: number } = $props();
</script>

<svg viewBox="0 0 24 24" width={taille} height={taille} aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
  {#if nom === "check"}<path d="m5 12.5 4.5 4.5L19 7.5" />
  {:else if nom === "fleche"}<path d="M5 12h14m-6-6 6 6-6 6" />
  {:else if nom === "info"}<circle cx="12" cy="12" r="9.5" /><path d="M12 11v5.5M12 7.6v.1" />
  {:else if nom === "alerte"}<path d="M12 3.5 2.8 19.5h18.4L12 3.5ZM12 10v4.5M12 17.4v.1" />
  {:else if nom === "box"}<rect x="3.5" y="8" width="17" height="9" rx="2.5" /><path d="M8 12.5h.1M11 12.5h5" />
  {:else if nom === "oeil"}<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12Z" /><circle cx="12" cy="12" r="3" />{/if}
</svg>
