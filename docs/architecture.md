# Zen Home : architecture cible et décisions

Proposition de l'agent architect du 2026-10-25, validée par l'utilisateur.

## Découpage (A2, atteint en passant par A1)

Paquet unique `tv.zenhome`, visibilité package. Pas de bibliothèque ; classes anonymes conservées pendant le refactor.

**Classes pures** (aucun `android.*`, testées sur le JVM du Mac) :
- `Net` : client HTTPS avec liste blanche d'hôtes
- `Fmt`
- `Wmo` : codes météo WMO → icône / libellé / phrase
- `Greeting`
- `DockMetrics`
- `Movie`
- `MovieRules` : `imgPath`, filtre/tri « Bientôt », nom des affiches, échantillonnage, clés de cache
- `ConfigRules` : validation jeton, coordonnées, ville, CSV du dock
- `Nav` : machine d'états de la navigation

**Classes Android** :
- `Home` : composition, cycle de vie, routage des touches, `Host`
- `Ui`, `Icons`, `Apps`, `Async`, `HomeConfig`
- `Overlays`, `AppsGrid`
- `Wallpaper`, `Header`, `Weather`
- `Movies`, `MovieDetail`
- `Dock`, `Carousel`, `Shortcuts`
- `Editor`
- `Levels` : applique les états de `Nav`

**Communication** : Home appelle les modules ; les modules remontent par une seule interface `Host`. Ils ne se connaissent pas entre eux.

**Exécution** : un seul Executor et un seul Handler UI (`Async`). Les modules visuels reçoivent un `Context` et un `ViewGroup` parent, jamais l'Activity, pour qu'un DreamService puisse les réutiliser.

**Préférences** : le fichier `zenhome` et ses clés forment un contrat avec ConfigReceiver. On n'y touche pas.

## Tests

`launcher/test-mac.sh` compile les classes pures **sans** android.jar. Ça sert de garde-fou : une classe « pure » qui importe android.* fait échouer la compilation.

Lanceur maison : `test/tv/zenhome/Tests.java`, code de sortie 1 en cas d'échec.

JSON : la lecture se fait à la frontière (B1). Les règles pures travaillent sur des objets Java simples. org.json n'est pas utilisé dans les tests.

## Accueil en niveaux (décisions)

| Niveau | Contenu |
|---|---|
| REPOS (dock) | photo dégagée, salutation, heure/date, météo du jour sur une ligne, RAM discrète, raccourcis, dock |
| Haut → FILMS | rangée d'affiches + mise en avant ; dock atténué |
| Haut → ONGLETS | onglets |
| Haut → RACCOURCIS (TOP) | barre de raccourcis |
| TOP, Gauche → MÉTÉO | semaine visible au focus |
| Bas depuis REPOS → APPLIS | carrousel sous le dock ; le bloc du bas remonte |

Navigation :
- Sans films (pas de jeton TMDB) : Haut depuis REPOS mène à TOP.
- Retour depuis n'importe quel niveau : on redescend à REPOS. Accueil : tout est fermé et on revient à REPOS.
- Retour d'une appli lancée depuis un niveau : on revient au même niveau, sur la même tuile.
- Bas depuis ONGLETS : dernière affiche visitée.
- Menu sur une tuile : menu contextuel. Menu ailleurs : mode édition.
- « Modifier l'accueil » devient un raccourci crayon.

Animations :
- Translations verticales et fondus de 200 à 220 ms sur des conteneurs, avec `withLayer()`.
- Le focus change immédiatement ; l'animation n'est que visuelle.

Fond :
- Le zoom continu est **supprimé**. Photo fixe et fondu entre deux photos.

## Modules ajoutés depuis (partie D et suites)

**Classes pures** (ajoutées à `PURE` dans `test-mac.sh`) :
- `InputRules` : filtre, libellé, état, tri et icône des entrées TV ; détection des barres de son d'après le nom
- `SleepRules` : échéance du minuteur, textes, décisions du receiver
- `WeatherRules` : heures locales, pluie imminente, nuit/jour (Open-Meteo)
- `KeyRules` : décisions sur les touches interceptées par `KeyService`
- `GaugeRules` : jauges RAM / disque
- `StandbyRules` : textes de l'écran de veille (météo détaillée, prochaine sortie de Ma liste)
- `IconRules` : icônes Phosphor (nom → codepoint, météo → icône)
- `ThemeRules` : palettes `glass`, `night`, `sand`, `forest`, `auto` et lecture de `zenhome_theme`

**Classes Android** :
- `Inputs` : panneau des entrées en cartes (`TvInputManager`), ouvert par la touche Source
- `Sleep`, `SleepReceiver` : minuteur de veille (alarmes, badge sur le raccourci)
- `Ambient` : mode ambiance après 3 min sans touche ; `Standby` : contenu partagé avec l'économiseur `ZenDream` (DreamService)
- `Banner` : bandeau « film de Ma liste qui sort aujourd'hui », une fois par jour
- `Theme` : applique la palette aux vues
- `KeyService` : service d'accessibilité qui relaie les boutons d'applis de la télécommande TCL, la touche Source (accueil au premier plan) et la touche Réglages (menu `SettingsMenu` ou panneau rapide TCL via `Apps.openTclPanel`)
- `ConfigReceiver` : configuration par adb (ville, jeton TMDB), protégée par la permission `DUMP`

**Décisions** :
- Backdrop du film : `Wallpaper.override`, au plus 2 bitmaps plein écran à la fois.
- Bande-annonce : TMDB videos, puis intent vers l'appli YouTube. Aucun nouvel hôte dans `Net`.
- « Ma liste » : 4ᵉ onglet géré par `Movies` (préférences `my_list`, plafond 30), locale, survit à `clear_tmdb`.

## Critères communs à chaque étape

- **[B]** `./build-mac.sh` réussit.
- **[T]** `./test-mac.sh` réussit.
- **[D]** Pendant le refactor, le diff ne contient que des déplacements (`git diff --color-moved`).
- **[S]** Liste de contrôle sur la TV, faite par l'orchestrateur.
- **[M]** PSS de `tv.zenhome` à ±1 Mo de la référence (43 Mo), puis 45 Mo au plus.
