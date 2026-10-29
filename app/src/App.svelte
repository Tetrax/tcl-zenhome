<script lang="ts">
  import { onMount } from "svelte";
  import Fond from "./components/Fond.svelte";
  import Barre from "./components/Barre.svelte";
  import Bandeaux from "./components/Bandeaux.svelte";
  import DialogueAttente from "./components/DialogueAttente.svelte";
  import Accueil from "./components/Accueil.svelte";
  import OutilAdb from "./components/OutilAdb.svelte";
  import Recherche from "./components/Recherche.svelte";
  import Connexion from "./components/Connexion.svelte";
  import EtatDesLieux from "./components/EtatDesLieux.svelte";
  import Installation from "./components/Installation.svelte";
  import Resume from "./components/Resume.svelte";
  import Meteo from "./components/Meteo.svelte";
  import Films from "./components/Films.svelte";
  import AppliSysteme from "./components/AppliSysteme.svelte";
  import VosApplis from "./components/VosApplis.svelte";
  import Recap from "./components/Recap.svelte";
  import Application from "./components/Application.svelte";
  import Reparation from "./components/Reparation.svelte";
  import Resultat from "./components/Resultat.svelte";
  import Restauration from "./components/Restauration.svelte";
  import Adieu from "./components/Adieu.svelte";
  import Lot from "./components/Lot.svelte";
  import RecapLot from "./components/RecapLot.svelte";
  import { zen, MOMENT, ECRANS_TRAVAIL } from "./lib/zen.svelte";
  import { moment as fondu } from "./lib/transitions";
  import type { Clairiere, Moment, Signal } from "./scene";

  let scene = $state.raw<Clairiere | null>(null);
  onMount(() => void zen.charger());

  // ?moment=nuit : fige le décor sur un moment (vérification visuelle du décor).
  const force = new URLSearchParams(location.search).get("moment") as Moment | null;
  const travail = $derived(ECRANS_TRAVAIL.includes(zen.ecran));

  // Moment affiché : pilote l'accent et le verre (CSS), le repli CSS et la scène 3D.
  $effect(() => { document.body.dataset.moment = force ?? (travail ? "crepuscule" : MOMENT[zen.ecran]); });

  let posee = false;
  $effect(() => {
    const s = scene, e = zen.ecran;
    if (!s) return;
    if (!posee) { posee = true; s.poser(force ?? MOMENT[e]); return; }
    if (force) return;
    if (ECRANS_TRAVAIL.includes(e)) return; // pendant une action longue, la lumière suit la progression réelle
    s.allerA(MOMENT[e], e === "resultat" || e === "fin_restauration" || e === "recap_lot" ? { duree: 5.5, delai: 1.2 } : {});
  });

  let suivi = false;
  $effect(() => {
    const s = scene;
    if (!s) return;
    if (!travail || force) { suivi = false; return; }
    const f = zen.ecran === "application" ? zen.fractionGlobale() : zen.ecran === "lot" ? zen.fractionLot() : zen.travail.fraction;
    s.setProgression(f ?? 0, undefined, suivi ? undefined : "crepuscule");
    suivi = true;
  });

  const signal = $derived.by((): Signal => {
    switch (zen.ecran) {
      case "accueil": case "adb": return "aucun";
      case "recherche":
        if (zen.erreur) return "aucun";
        if (zen.rechercheEnCours || !zen.rechercheFaite) return "recherche";
        return zen.boxes.length ? "trouvee" : "aucun";
      case "connexion": return "trouvee";
      default: return "repos";
    }
  });
  $effect(() => scene?.setSignal(signal));

  let succesVu = 0;
  $effect(() => {
    const n = zen.succes;
    if (n > succesVu) scene?.impulsion();
    succesVu = n;
  });
</script>

<Fond bind:scene />
<Barre />
<main>
  {#key zen.ecran}
    <div class="vue" in:fondu={{ delay: 350 }} out:fondu={{ duration: 350, sortie: true }} onoutrostart={(e) => ((e.currentTarget as HTMLElement).inert = true)}>
      {#if zen.ecran === "accueil"}<Accueil />
      {:else if zen.ecran === "adb"}<OutilAdb />
      {:else if zen.ecran === "recherche"}<Recherche />
      {:else if zen.ecran === "connexion"}<Connexion />
      {:else if zen.ecran === "etat"}<EtatDesLieux />
      {:else if zen.ecran === "installation"}<Installation />
      {:else if zen.ecran === "resume"}<Resume />
      {:else if zen.ecran === "meteo"}<Meteo />
      {:else if zen.ecran === "films"}<Films />
      {:else if zen.ecran === "systeme"}<AppliSysteme />
      {:else if zen.ecran === "utilisateur"}<VosApplis />
      {:else if zen.ecran === "recap"}<Recap />
      {:else if zen.ecran === "application"}<Application />
      {:else if zen.ecran === "reparation"}<Reparation />
      {:else if zen.ecran === "resultat"}<Resultat />
      {:else if zen.ecran === "lot"}<Lot />
      {:else if zen.ecran === "recap_lot"}<RecapLot />
      {:else if zen.ecran === "adieu"}<Adieu />
      {:else}<Restauration />{/if}
    </div>
  {/key}
</main>
<Bandeaux />
<DialogueAttente />
