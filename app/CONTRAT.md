# Contrat entre l'interface (Svelte) et le cœur (Rust)

L'interface n'envoie que des **intentions** ; elle ne voit jamais de commande adb, de chemin, de `Reprise` ni de nom de paquet qu'elle n'a pas reçu du cœur. Tous les textes affichés venant de la box sont déjà nettoyés et bornés par le cœur ; l'interface les affiche en interpolation Svelte, **jamais** avec `{@html}`.

Les commandes longues reçoivent un `Channel<Evenement>` (Tauri `ipc::Channel`) pour la progression. Une seule action à la fois : si une action est en cours, toute autre renvoie l'erreur `occupe`.

## Commandes

| Commande | Arguments | Résultat |
|---|---|---|
| `etat_initial` | — | `{ version, adb_installe: bool, demo: bool, sauvegardes_dir_existe: bool, reprise_en_attente: bool, origine: { date: string } \| null }` |
| `installer_adb` | `accord: bool`, `canal` | `null` ; progression via `{type:"progression", fraction}` |
| `chercher_box` | — | `[{ id: number, nom: string, ip: string }]` (les `id` désignent une liste gardée côté Rust) |
| `connecter` | `cible: {id: number} \| {ip: string}`, `canal` | `{ nom, ip, modele }` |
| `chercher_villes` | `saisie: string` | `[{ id: number, nom, region?: string, pays?: string }]` |
| `inventaire` | `canal` | `Inventaire` (voir plus bas) |
| `installer` | `ville_id: number \| null`, `jeton: string \| null`, `canal` | `null` |
| `appliquer` | `choix: { garder: string[], desactiver: string[], desinstaller: string[] }`, `canal` | `RapportApplication` |
| `verifier` | `reparer: bool`, `canal` | `{ lignes: [{ etat: "ok"\|"ko"\|"avert", controle, detail }], ko: number }` |
| `restaurer` | `canal` | `{ incomplete: bool, avertissements: string[], depuis_sauvegarde: string \| null }` |
| `decider` | `demande: number`, `decision: "reessayer" \| "arreter"` | `null` |
| `annuler` | — | `null` (demande l'arrêt propre de l'action en cours) |
| `couper_debogage` | — | `null` (coupe le débogage USB de la box, puis ferme la session adb) |
| `ouvrir_sauvegardes` | — | `null` (ouvre le dossier des sauvegardes de l'appli dans le Finder) |
| `desinstaller_mac` | `garder_sauvegardes: bool` | `null` (retire Zen Home de ce Mac, voir plus bas ; l'appli se ferme ensuite) |

`Inventaire` :
```
{
  applis_systeme: [{ paquet, libelle, explication, recommande: "desactiver" | "garder",
                     etat: "actif" | "desactive" | "garde" }],
  applis_utilisateur: [{ paquet, libelle, taille: string, derniere_utilisation: string }],
  resume: { modele, android, nb_applis, nb_desactivees }
}
```
Précisions : `libelle` des applis de l'utilisateur = nom donné par Zen Home une fois installée, à défaut le nom du paquet ; `taille` et `derniere_utilisation` valent `"—"` si inconnues ; `resume.nb_applis` = nombre total de paquets de la box, `nb_desactivees` = paquets désactivés ; seules les applis de la liste « à alléger » présentes sur la box figurent dans `applis_systeme`.

`RapportApplication` : `{ desactivees: number, gardees: number, desinstallees: string[], echecs: string[] }`. `desactivees` = nombre d'applis **à désactiver** (la désactivation elle-même est faite par le `verifier(true)` qui suit) ; `desinstallees` inclut celles déjà absentes ; `echecs` = noms de paquets.

### Sauvegarde d'origine et « Revenir à avant Zen Home »

La sauvegarde d'origine d'une box est la plus ancienne sauvegarde de sa série (suffixe du nom du dossier) dont `paquets-tous.txt` existe, est lisible et ne contient pas `tv.zenhome` : simple recherche dans `sauvegardes/`, rien n'est copié. `etat_initial().origine` est la plus récente de ces origines toutes box confondues (pas de session au démarrage), `date` au format `JJ/MM/AAAA HH:MM`, `null` s'il n'y en a pas.

`restaurer` utilise l'origine de la box de la session si elle existe : seules les applis désactivées depuis (et non avant) Zen Home sont réactivées ; écran de veille, animations et accueil reprennent les valeurs de la sauvegarde (valeur absente ou « null » : le réglage est supprimé ; valeur invalide : valeur par défaut) ; la garde de désinstallation de Zen Home porte sur le paquet de cet accueil, qui doit être présent et actif. `depuis_sauvegarde` donne la date de l'origine utilisée, `null` pour la restauration générique (aucune origine, ou fichiers illisibles ou invalides : les sauvegardes sont des entrées non fiables, analysées strictement). Le reste (accessibilité, `zenhome_*`, Spotify, reprise) est inchangé.

### Désinstaller Zen Home de ce Mac

`desinstaller_mac(garder_sauvegardes)` ne touche pas la box (Zen Home y reste installé : « Revenir à avant Zen Home » d'abord). Dans l'ordre : copie de `sauvegardes/` vers `~/Documents/Zen Home – sauvegardes AAAA-MM-JJ HHhMM/` si demandé (dossier neuf, jamais d'écrasement ; en cas d'échec rien n'est supprimé), fermeture de la session et arrêt du serveur adb dédié, mise à la Corbeille du bundle `.app` (NSFileManager, release seulement, et seulement si l'exécutable est bien dans un bundle `*.app` ; en cas d'échec rien n'est supprimé), suppression du dossier de données de l'appli (seulement s'il est exactement `<données utilisateur>/fr.tetrax.zenhome`, lien symbolique refusé), puis sortie de l'appli. Erreur `interne` lisible si une étape échoue. En mode démo, rien n'est copié, supprimé ni mis à la Corbeille et l'appli ne se ferme pas.

### Appel côté JS (Tauri v2)

- Les **arguments** de commande passent en camelCase : `ville_id` s'envoie `villeId`. Les autres (`accord`, `canal`, `cible`, `saisie`, `jeton`, `choix`, `reparer`, `demande`, `decision`) sont des mots simples, donc inchangés. Les champs des objets (`cible`, `choix`, résultats, événements) restent en snake_case tels que dans ce document.
- `canal` : `const canal = new Channel<Evenement>(); canal.onmessage = (e) => ...`, puis `invoke("connecter", { cible, canal })`.
- `cible` : `{ id: 0 }` (id issu du dernier `chercher_box`) ou `{ ip: "192.168.1.20" }` (IP privée ou link-local, sinon erreur `injoignable`).
- `installer` : `{ villeId: number | null, jeton: string | null, canal }`. Le jeton TMDB n'est jamais renvoyé ni journalisé ; vide ou `null` = pas de jeton.
- `decider` : `{ demande: <id reçu>, decision: "reessayer" | "arreter" }`. L'identifiant est à usage unique ; sans réponse après 5 minutes, l'action s'arrête (comme « arreter »). Une demande inconnue ou déjà répondue renvoie `choix_invalide`.
- `desinstaller_mac` : `{ garderSauvegardes: bool }`, sans `canal`.
- `couper_debogage` n'a pas de `canal` (pas d'événements : si la box ne répond pas, l'action s'arrête).

### Déroulé et état gardé par le cœur

1. `installer_adb` (si `adb_installe` est faux ; `accord: false` → `refus_licence`), 2. `chercher_box` puis `connecter` (ouvre la session ; il faut une session pour tout ce qui suit, sinon `injoignable`), 3. `inventaire` (lit la box, écrit une sauvegarde, garde l'inventaire dans la session ; **requis avant `appliquer`**, sinon `lecture`), 4. `chercher_villes` puis `installer`, 5. `appliquer`, 6. `verifier(reparer: true)` (c'est lui qui désactive les applis choisies), 7. `couper_debogage` (facultatif).
- Une seule action à la fois pour **toutes** les commandes sauf `etat_initial`, `decider`, `annuler`, `ouvrir_sauvegardes` : sinon erreur `occupe`.
- `annuler` : l'action en cours s'arrête avant sa prochaine opération et échoue avec le code `annule` (rien à défaire ; elle peut être relancée). Délai global d'une action : 30 minutes (erreur `interne`).
- Après `appliquer`/`verifier`, rappeler `inventaire` pour rafraîchir l'affichage.
- `reprise_en_attente: true` au démarrage : une vérification précédente a été interrompue après avoir désactivé des applis ; la prochaine `verifier(reparer: true)` sur la même box en tient compte pour le retour arrière (l'interface peut le dire à l'utilisateur et proposer « Vérifier »).

## Erreurs

Toute commande peut échouer avec `{ code, message }` où `message` est en français et prêt à afficher. Codes : `occupe`, `annule`, `adb_absent`, `refus_licence`, `injoignable`, `non_autorise`, `autre_modele`, `autre_box`, `lecture`, `choix_invalide`, `reseau`, `interne`.
`injoignable` couvre aussi une IP refusée et l'absence de session ; `choix_invalide` : choix/paquet/ville/jeton refusé, id périmé, décision inconnue ; `lecture` : réponse de la box illisible ou inventaire pas encore lu ; `interne` : APK embarqué absent ou altéré, délai dépassé, signature différente, erreur de fichier.

## Événements (`Channel<Evenement>`)

```
{ type: "etape", nom: string }                 // libellé français de l'étape en cours (le numéro n'est pas fourni)
{ type: "progression", fraction: number }      // 0..1, quand c'est mesurable
{ type: "connexion_perdue", essai: number }
{ type: "reconnectee" }
{ type: "attente_utilisateur", demande: number, raison: "injoignable" | "autorisation" }
{ type: "avertissement", texte: string }
```
Sur `attente_utilisateur`, l'interface affiche la consigne (réveiller la box / accepter l'autorisation sur la TV) avec « Réessayer » / « Arrêter », puis appelle `decider(demande, …)`.

## Mode démo

En build debug, ou si la variable d'environnement `ZENHOME_DEMO=1` est définie (`ZENHOME_DEMO=0` force le mode réel même en debug), le cœur utilise une box simulée (FakeBox) : tout le parcours fonctionne sans box. La box simulée est une Mi Box « SALON » (192.168.50.14) ; `installer_adb` simule une progression, `chercher_villes` renvoie 3 villes factices dont « Montpellier », `ouvrir_sauvegardes` ne lance rien, `desinstaller_mac` ne supprime rien. Le cœur dépose une sauvegarde d'origine factice (15/09/2026 21:00, série de la box simulée) dans le dossier de démo pour que le parcours propose « Revenir à avant Zen Home » depuis cette sauvegarde. `adb_installe` vaut `false` au départ en démo, jusqu'à `installer_adb`. `etat_initial().demo` vaut alors `true` et l'interface affiche un discret bandeau « Mode démo ». Dans un navigateur (hors Tauri), `api.ts` simule ces mêmes réponses.
