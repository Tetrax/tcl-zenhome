<script lang="ts">
  import Panneau from "./Panneau.svelte";
  import Icone from "./Icone.svelte";
  import { zen } from "../lib/zen.svelte";

  const inv = $derived(zen.inventaire);
  const choix = $derived(zen.choix());
  const nom = (p: string) => inv?.applis_systeme.find((a) => a.paquet === p)?.libelle ?? inv?.applis_utilisateur.find((a) => a.paquet === p)?.libelle ?? p;
</script>

<Panneau titre="Récapitulatif" large>
  <p class="lead">Voici ce qui va être fait sur {zen.lot ? "vos box" : "votre box"}. Rien ne change avant votre clic sur « Appliquer ».</p>
  <p class="petit">Profil : <b>{{ recommande: "Recommandé", simple: "Simple", leger: "Léger", perso: "Personnalisé" }[zen.profil]}</b></p>
  <dl class="grille">
    <div><dt>Applis système désactivées (réversible)</dt><dd class="grand">{choix.desactiver.length}</dd></div>
    <div><dt>Applis système gardées</dt><dd class="grand">{choix.garder.length}</dd></div>
  </dl>
  {#if choix.desactiver.length}
    <details><summary>Voir les applis désactivées</summary><ul class="petit" style="margin:4px 0 0;padding-left:20px">{#each choix.desactiver as p}<li>{nom(p)}</li>{/each}</ul></details>
  {/if}
  {#if choix.desinstaller.length}
    <div class="note alerte"><Icone nom="alerte" /><div><p><b>Applis désinstallées (définitif) : {choix.desinstaller.length}</b></p><ul class="petit" style="margin:4px 0 0;padding-left:18px">{#each choix.desinstaller as p}<li>{nom(p)}</li>{/each}</ul></div></div>
  {/if}
  <p class="petit" style="margin-top:16px">
    Météo : <b>{zen.ville ? [zen.ville.nom, zen.ville.pays].filter(Boolean).join(", ") : "aucune"}</b> ·
    Films : <b>{zen.jeton ? "avec votre jeton TMDB" : zen.init?.films_inclus ? "inclus" : "sans"}</b>
  </p>
  <p class="petit" style="margin-top:6px">Une sauvegarde de l'état actuel {zen.lot ? "de chaque box est déjà enregistrée" : "est déjà enregistrée"} sur ce Mac.</p>
  {#snippet pied()}
    <button class="btn primary" type="button" onclick={() => zen.appliquer()}><span>Appliquer</span><Icone nom="fleche" /></button>
    <button class="btn" type="button" onclick={() => { zen.profil = "perso"; zen.aller("systeme"); }}><span>Modifier en détail</span></button>
    {#if (inv?.applis_utilisateur.length ?? 0) > 0}<button class="lien" type="button" onclick={() => zen.aller("utilisateur")}>Supprimer aussi des applis…</button>{/if}
    <button class="lien" type="button" onclick={() => zen.aller("profil")}>Retour</button>
  {/snippet}
</Panneau>
