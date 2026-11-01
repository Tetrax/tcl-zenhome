# Contrat entre l'interface (Svelte) et le cœur (Rust)

L'interface n'envoie que des **intentions** ; elle ne voit jamais de commande adb, de chemin, de `Reprise` ni de nom de paquet qu'elle n'a pas reçu du cœur. Tous les textes affichés venant de la box sont déjà nettoyés et bornés par le cœur ; l'interface les affiche en interpolation Svelte, **jamais** avec `{@html}`.

Les commandes longues reçoivent un `Channel<Evenement>` (Tauri `ipc::Channel`) pour la progression. Une seule action à la fois : si une action est en cours, toute autre renvoie l'erreur `occupe`.

## Commandes

| Commande | Arguments | Résultat |
|---|---|---|
| `etat_initial` | — | `{ version, adb_installe: bool, demo: bool, sauvegardes_dir_existe: bool, reprise_en_attente: bool, origine: { date: string } \| null, boxes_memorisees: [{ nom: string, ip: string }], films_inclus: bool }` |
| `installer_adb` | `accord: bool`, `canal` | `null` ; progression via `{type:"progression", fraction}` |
| `chercher_box` | — | `[{ id: number, nom: string, ip: string, appareil: string \| null, libelle: string }]` (`appareil` : identifiant de l'appareil supposé d'après l'annonce réseau, `null` s'il est inconnu, simple indice : la connexion vérifie ; `libelle` : son nom court, « Appareil à vérifier » si inconnu ; les `id` désignent la dernière liste gardée côté Rust ; ils ne sont jamais réutilisés d'une recherche à l'autre : un id d'une recherche précédente donne `choix_invalide`) |
| `connecter` | `cible: {id: number} \| {ip: string}`, `canal` | `{ nom, ip, modele, appareil }` (`modele` : modèle affiché de l'appareil reconnu ; `appareil` : son identifiant, le PROFIL du kit, `"mibox4"` ou `"tcl-g08"`) |
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
| `rapport_texte` | `erreur: string`, `resume: string` | `string` (rapport à copier : version, système, erreur et résumé, filtrés ; voir « Rapport ») |
| `rapport_appareil` | `cible: {id: number} \| {ip: string}`, `canal` | `string` (rapport d'appareil complet, déjà filtré : voir « Rapport d'appareil ») |
| `envoyer_rapport` | `erreur: string`, `resume: string` | `null` (ouvre la messagerie avec un e-mail prêt, voir « Rapport ») |
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

### Appareils pris en charge

Le cœur reconnaît l'appareil à la connexion (table `moteur/appareils.rs`, identifiants = PROFIL du kit : `mibox4`, `tcl-g08`). Identification stricte : `ro.product.name` dans la liste exacte de l'appareil, `ro.product.brand` attendue, `ro.product.device` si elle est connue, SDK au moins égal au minimum ; une lecture vide est relue (3 essais), jamais prise pour un autre modèle. Un SDK plus récent que le dernier testé donne un événement `avertissement`, pas un refus. Tout autre appareil : erreur `autre_modele` (« Cet appareil n'est pas encore pris en charge. Rien n'a été modifié. »), rien n'est envoyé. L'appareil reconnu est gardé dans la session et décide de la liste d'allègement (`tv/debloat-<id>.txt`), de l'accueil et de l'écran de veille d'origine, du mode d'accueil (rôle HOME sous Android 10+, accueil préféré sous Android 9) et des paquets protégés (jamais désactivés ni désinstallés ; ils ne figurent pas dans `applis_utilisateur`). Réglage `zenhome_garder` : clé absente = aucun choix fait, les entrées `[option:garder]` restent actives (AVERT) à la réparation ; `appliquer` écrit `-` quand l'utilisateur ne garde rien. Une réparation relancée après une coupure qui avait désactivé des paquets contrôle toujours le premier plan (retour arrière si besoin) avant d'oublier la reprise. La recherche (`chercher_box`) utilise les indices mDNS de tous les appareils ; ce n'est qu'une supposition, la connexion vérifie l'appareil. `boxes.json` garde l'identifiant d'appareil de chaque box (champ facultatif : l'ancien format reste lisible).

### Rapport d'appareil (appareil non reconnu)

Un appareil inconnu est refusé (`autre_modele`, rien n'est modifié). `rapport_appareil(cible, canal)` permet d'en préparer le rapport pour l'ajouter à une version suivante : il se connecte SANS contrôle du modèle (ferme la session en cours, comme `connecter`) et ne fait QUE des lectures sur liste blanche (l'accueil, le rôle HOME et l'écran de veille qui nomment une appli de l'utilisateur sont remplacés par « (appli de l'utilisateur) ») (`moteur/rapport_appareil.rs`, journal de la FakeBox vérifié par les tests) : `getprop` de `ro.product.{name,model,brand,manufacturer,device}`, `ro.build.version.{sdk,release,security_patch}`, `ro.build.characteristics` ; numéro de série « lisible » ou « illisible » (jamais sa valeur) ; `pm list packages -s` et `-d` (les désactivés sont recoupés avec les paquets système : aucun paquet de l'utilisateur) ; le NOMBRE d'applis de l'utilisateur ; l'accueil résolu ; le détenteur du rôle HOME (SDK 29 au moins) ; `settings get secure screensaver_components`. Exclus : série, IP, nom Cast, Wi-Fi, noms des applis de l'utilisateur. Texte borné (400 noms par liste, 16 000 caractères), valeurs passées par `assainir`, puis `rapport::complet` (en-tête version et système, masquage des longues suites). Le champ mDNS `md` n'est pas repris : la recherche ne le garde que pour les appareils reconnus par un indice. Côté interface : « Copier le rapport » (texte complet) et « Envoyer » (`envoyer_rapport` avec les premières lignes `ro.` et la mention « collez le rapport copié », le lien mailto étant limité à 1500 caractères). Erreurs : `injoignable`, `non_autorise`, `annule`, `occupe`.

### Box mémorisées dans la recherche

`etat_initial().boxes_memorisees` donne les box de `boxes.json` (déjà validées par le cœur, la plus récente d'abord ; vide si le fichier est absent ou invalide). L'interface les relit à chaque recherche et les affiche tout de suite (« configurée récemment »), pendant que `chercher_box` tourne. Une box mémorisée est proposée par son adresse : `connecter({ ip })` (modèle et série revérifiés, nom mémorisé gardé). Quand la recherche finit, les box trouvées (avec `id`) et les mémorisées sont fusionnées par IP (une ligne par adresse ; une mémorisée absente des résultats reste proposée, mention « n'a pas répondu »). Comme le cœur ne fait qu'une chose à la fois, un clic pendant la recherche appelle d'abord `annuler`, attend que `chercher_box` ait rendu la main, puis appelle `connecter`. En démo, `boxes_memorisees` vaut `[SALON]`.

### Rapport (« Envoyer le rapport » / « Copier le rapport »)

Le cœur n'écrit pas de `rapport.txt` (seul le kit en écrit un) : le dossier de sauvegarde ne contient que l'état de la box à la lecture (`resume.txt`…). Le rapport est donc composé par l'interface : `erreur` = `code : message` de l'erreur affichée (vide sinon), `resume` = résumé de la dernière action (mode, nombres d'applis, lignes de vérification à regarder ou à noter, avertissements de restauration, état de chaque box d'un lot ; ni nom de box, ni IP, ni jeton). Le cœur reste maître du résultat : il retire les caractères de contrôle, masque toute suite de 24 caractères ou plus de `[A-Za-z0-9_-]` et tout ce qui commence par `eyJ` (jeton, clé), borne l'entrée (la fin est gardée), ajoute la version de l'appli et le système (macOS + version). `envoyer_rapport` construit lui-même `mailto:hebertval34@gmail.com?subject=Zen%20Home%20%3A%20rapport&body=…` (corps encodé de 1500 caractères au plus : en-tête et erreur toujours, puis la fin du résumé précédée de « … », puis la mention du dossier des sauvegardes) et l'ouvre avec `open` (`explorer` / `xdg-open` ailleurs) ; tout lien qui ne commence pas par ce préfixe fixe est refusé. L'interface ne fournit jamais d'URL. En démo, rien n'est ouvert. « Copier » écrit dans le presse-papier le texte complet de `rapport_texte` (non tronqué à 1500).

### Désinstaller Zen Home de ce Mac

`desinstaller_mac(garder_sauvegardes)` ne touche pas la box (Zen Home y reste installé : « Revenir à avant Zen Home » d'abord). Dans l'ordre : copie de `sauvegardes/` vers `~/Documents/Zen Home – sauvegardes AAAA-MM-JJ HHhMM/` si demandé (dossier neuf, jamais d'écrasement ; en cas d'échec rien n'est supprimé), fermeture de la session et arrêt du serveur adb dédié, mise à la Corbeille du bundle `.app` (NSFileManager, release seulement, et seulement si l'exécutable est bien dans un bundle `*.app` ; en cas d'échec rien n'est supprimé), suppression du dossier de données de l'appli (seulement s'il est exactement `<données utilisateur>/fr.tetrax.zenhome`, lien symbolique refusé), puis sortie de l'appli. Erreur `interne` lisible si une étape échoue. En mode démo, rien n'est copié, supprimé ni mis à la Corbeille et l'appli ne se ferme pas.

### Appel côté JS (Tauri v2)

- Les **arguments** de commande passent en camelCase : `ville_id` s'envoie `villeId`. Les autres (`accord`, `canal`, `cible`, `saisie`, `jeton`, `choix`, `reparer`, `demande`, `decision`) sont des mots simples, donc inchangés. Les champs des objets (`cible`, `choix`, résultats, événements) restent en snake_case tels que dans ce document.
- `canal` : `const canal = new Channel<Evenement>(); canal.onmessage = (e) => ...`, puis `invoke("connecter", { cible, canal })`.
- `cible` : `{ id: 0 }` (id issu du dernier `chercher_box`, jamais d'une recherche plus ancienne) ou `{ ip: "192.168.1.20" }` (IP privée ou link-local, sinon erreur `injoignable`).
- `installer` : `{ villeId: number | null, jeton: string | null, canal }`. Le jeton TMDB n'est jamais renvoyé ni journalisé ; vide ou `null` = jeton du projet s'il existe, sinon pas de films. Le jeton du projet est injecté à la compilation (variable `ZENHOME_TMDB_JETON`, lue par `app/empaqueter.sh` dans le trousseau, jamais dans le dépôt) et reste côté Rust : l'interface ne reçoit que `etat_initial().films_inclus` (vrai s'il est compilé). Un jeton personnel fourni est prioritaire. Dans le navigateur, `?sansfilms` simule `films_inclus = false`.
- `decider` : `{ demande: <id reçu>, decision: "reessayer" | "arreter" }`. L'identifiant est à usage unique ; sans réponse après 5 minutes, l'action s'arrête (comme « arreter »). Une demande inconnue ou déjà répondue renvoie `choix_invalide`.
- `desinstaller_mac` : `{ garderSauvegardes: bool }`, sans `canal`.
- `couper_debogage` n'a pas de `canal` (pas d'événements : si la box ne répond pas, l'action s'arrête).

### Déroulé et état gardé par le cœur

1. `installer_adb` (si `adb_installe` est faux ; `accord: false` → `refus_licence`), 2. `chercher_box` puis `connecter` (ouvre la session ; il faut une session pour tout ce qui suit, sinon `injoignable`), 3. `inventaire` (lit la box, écrit une sauvegarde, garde l'inventaire dans la session ; **requis avant `appliquer`**, sinon `lecture`), 4. `chercher_villes` puis `installer`, 5. `appliquer`, 6. `verifier(reparer: true)` (c'est lui qui désactive les applis choisies), 7. `couper_debogage` (facultatif).
- Une seule action à la fois pour **toutes** les commandes sauf `etat_initial`, `decider`, `annuler`, `ouvrir_sauvegardes`, `rapport_texte`, `envoyer_rapport` : sinon erreur `occupe`.
- `annuler` pendant `chercher_box` : la recherche s'arrête aussitôt (au plus le délai d'une connexion TCP en cours, 1,5 s) et rend ce qu'elle a déjà trouvé, **sans erreur** (drapeau d'arrêt passé à `zh_reseau::chercher_box`, vérifié dans sa boucle ; le balayage et les sondes s'arrêtent avec elle).
- `annuler` : l'action en cours s'arrête avant sa prochaine opération et échoue avec le code `annule` (rien à défaire ; elle peut être relancée). Délai global d'une action : 30 minutes (erreur `interne`).
- Après `appliquer`/`verifier`, rappeler `inventaire` pour rafraîchir l'affichage.
- `reprise_en_attente: true` au démarrage : une vérification précédente a été interrompue après avoir désactivé des applis, pour au moins une box ; la prochaine `verifier(reparer: true)` sur cette box en tient compte pour le retour arrière (l'interface peut le dire à l'utilisateur et proposer « Vérifier »). La reprise est mémorisée **par numéro de série** (un seul fichier, 32 box au plus, la plus ancienne saute au-delà) : `connecter` charge celle de la box connectée, `verifier`/`restaurer` n'effacent que celle de la box connectée ; traiter une box ne touche pas à la reprise d'une autre. Un fichier modifié ou invalide est ignoré en entier.
- Mémoire des box : après un `connecter` réussi (modèle vérifié, série lue), le cœur inscrit la box (`serie`, `nom`, `ip`) dans `boxes.json` du dossier de données (16 au plus, la plus ancienne saute ; une entrée par série et par IP, jamais deux fois la même IP ; écriture atomique ; pas en mode démo). `chercher_box` sonde en parallèle ces IP (connexion TCP 5555) en même temps que le mDNS et le balayage : une box mémorisée qui répond est rendue avec son nom mémorisé (le nom mDNS prime s'il arrive, dédoublonnage par IP), et la recherche s'arrête `GRACE` après la première box trouvée. Le fichier est lu comme non fiable (taille bornée, IP privée ou link-local, nom nettoyé, série valide) ; une entrée invalide le fait ignorer en entier. `connecter` revérifie toujours modèle et série (`autre_modele`, `autre_box`) : une IP reprise par une autre machine ne passe pas. Le nom affiché est celui mémorisé pour la série lue, sinon le nom mDNS (jamais le nom mémorisé d'une IP dont la série diffère), sinon « Mi Box ». `desinstaller_mac` l'efface avec le dossier de données.
- Changer de box (`connecter` sur une autre cible) ferme la session précédente par un simple `adb disconnect` : le serveur adb dédié reste lancé. Il n'est arrêté que par `couper_debogage`, la fermeture de l'appli et `desinstaller_mac`.

### Plusieurs box

Le cœur n'a qu'**une session à la fois** ; c'est l'interface qui enchaîne les box avec les commandes ci-dessus, une box après l'autre, sans nouvelle commande ni identifiant de box dans les événements.

- Passage 1 (installer), pour chaque box : `connecter` → `inventaire` → `installer` → `inventaire`. Les questions communes (ville, jeton) sont posées une fois et réutilisées pour toutes les box. Le jeton TMDB reste en mémoire dans l'interface jusqu'à la sortie du lot (« Terminer » ou retour à l'accueil), pour qu'une box réessayée depuis le récap final soit installée avec lui ; il n'est jamais écrit ni journalisé.
- Passage 2 (appliquer), pour chaque box : `connecter` → `inventaire` → `appliquer` (le choix de l'utilisateur est réduit aux paquets présents sur cette box) → `verifier(true)`.
- Réparer : `connecter` → `verifier(true)`. Revenir à avant Zen Home : `connecter` → `restaurer`.
- Une box en échec n'arrête pas les autres (l'interface note l'erreur et passe à la suivante) ; `annuler` (code `annule`) arrête tout le lot.
- Les ids de `chercher_box` restent valables tant qu'aucune nouvelle recherche n'a lieu : ne pas relancer `chercher_box` en cours de lot.

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

En build debug, ou si la variable d'environnement `ZENHOME_DEMO=1` est définie (`ZENHOME_DEMO=0` force le mode réel même en debug), le cœur utilise une box simulée (FakeBox) : tout le parcours fonctionne sans box. Trois appareils simulés, tous renvoyés par `chercher_box` : « SALON » (Mi Box 4, 192.168.50.14, série `ZH-DEMO-0001`), « CHAMBRE » (TV TCL G08, `tcl-g08`, rôle HOME, 192.168.50.15, série `ZH-DEMO-0002`) et « BUREAU » (appareil inconnu « Streamer X5 », 192.168.50.16, série `ZH-DEMO-0003`, appareil `null` dans la recherche : refusé par `autre_modele`, pour montrer le rapport d'appareil), chacun avec sa FakeBox (Netflix est commune ; VLC et Disney+ sont propres à SALON, Kodi et Prime Video à CHAMBRE) ; `installer_adb` simule une progression, `chercher_villes` renvoie 3 villes factices dont « Montpellier », `ouvrir_sauvegardes` ne lance rien, `desinstaller_mac` ne supprime rien. Le cœur dépose une sauvegarde d'origine factice (15/09/2026 21:00, série de SALON seulement) dans le dossier de démo pour que le parcours propose « Revenir à avant Zen Home » depuis cette sauvegarde ; CHAMBRE n'en a pas de semée (sa propre première lecture, avant l'installation, sert d'origine comme sur une vraie box). `adb_installe` vaut `false` au départ en démo, jusqu'à `installer_adb`. `etat_initial().demo` vaut alors `true` et l'interface affiche un discret bandeau « Mode démo ». Dans un navigateur (hors Tauri), `api.ts` simule ces mêmes réponses.

## Mises à jour

L'appli se met à jour seule via `tauri-plugin-updater` : au démarrage (jamais en mode démo ni hors Tauri), l'accueil vérifie en silence `https://github.com/Tetrax/tcl-zenhome/releases/latest/download/latest.json` ; l'échec réseau est ignoré. S'il existe une version plus récente, un discret « Nouvelle version x.y.z disponible » propose de la télécharger, de l'installer puis de relancer l'appli. Rien n'est proposé pendant une action sur une box. Les mises à jour sont signées avec une clé dédiée (`~/.tauri/zenhome-updater.key`, mot de passe dans le trousseau macOS) dont la clé publique est dans `tauri.conf.json` ; `empaqueter.sh` construit le dmg, le `.app.tar.gz` signé et `latest.json` dans `dist-release/` (release GitHub `app-vX.Y.Z`).
