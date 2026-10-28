<script lang="ts">
  // Écran de TEST temporaire (étape 3a) : vérifie la permission « Réseau local ».
  import { ping, testReseau, type PingInfo, type Rapport } from "./lib/api";

  let info = $state<PingInfo | null>(null);
  let ip = $state("");
  let enCours = $state(false);
  let rapport = $state<Rapport | null>(null);
  let erreur = $state<string | null>(null);

  ping().then((r) => (info = r), () => {});

  async function lancer() {
    enCours = true;
    rapport = null;
    erreur = null;
    try {
      rapport = await testReseau(ip);
    } catch (e) {
      erreur = String(e);
    } finally {
      enCours = false;
    }
  }
</script>

<main>
  <h1>Zen Home – test réseau</h1>
  <p class="version">Version {info?.version ?? "…"}</p>

  <div class="ligne">
    <input placeholder="IP manuelle (optionnel)" bind:value={ip} disabled={enCours} />
    <button onclick={lancer} disabled={enCours}>Lancer le test réseau</button>
  </div>

  {#if enCours}
    <p>Recherche mDNS (5 s) puis sondage adb… patientez.</p>
  {/if}
  {#if erreur}<p class="err">Erreur : {erreur}</p>{/if}

  {#if rapport}
    <section>
      <h2>Découverte</h2>
      <p>{rapport.annonces_googlecast} annonce(s) googlecast vue(s), {rapport.mi_box.length} Mi Box.</p>
      <ul>
        {#each rapport.mi_box as b}<li>{b.nom || "(sans nom)"} – {b.ip}</li>{/each}
      </ul>
    </section>
    <section>
      <h2>adb {rapport.ip_ciblee ? `(${rapport.ip_ciblee})` : ""}</h2>
      {#if !rapport.adb.length}<p>Non exécuté.</p>{/if}
      {#each rapport.adb as r}
        <div class="adb">
          <code>{r.commande}</code>
          <span>code : {r.code ?? "aucun"}{r.delai_depasse ? " · délai dépassé" : ""}</span>
          {#if r.stdout}<pre>{r.stdout}</pre>{/if}
          {#if r.stderr}<pre class="err">{r.stderr}</pre>{/if}
        </div>
      {/each}
    </section>
    {#if rapport.erreurs.length}
      <section>
        <h2>Erreurs</h2>
        <ul>{#each rapport.erreurs as e}<li class="err">{e}</li>{/each}</ul>
      </section>
    {/if}
  {/if}
</main>

<style>
  main { padding: 24px 32px; max-width: 760px; margin: 0 auto; overflow-y: auto; height: 100%; box-sizing: border-box; }
  .version { opacity: 0.6; margin-top: 0; }
  .ligne { display: flex; gap: 12px; margin: 16px 0; }
  input { flex: 1; padding: 8px; }
  button { padding: 8px 16px; }
  section { margin-top: 20px; }
  .adb { margin-bottom: 12px; display: grid; gap: 4px; }
  pre { margin: 0; padding: 8px; background: rgba(255, 255, 255, 0.06); white-space: pre-wrap; word-break: break-all; }
  .err { color: #ff8080; }
</style>
