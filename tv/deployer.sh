#!/bin/bash
# Teste, compile, installe et configure Zen Home sur la TV, en une commande.
set -euo pipefail
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

aide() {
    command cat <<'AIDE'
Usage : tv/deployer.sh [--dry-run] [--sans-build] [--capture] [--reset-dock] [--help]

Enchaîne : tests, build, connexion adb (vérifie le modèle attendu par PROFIL), installation,
configuration (ville, jeton TMDB, permissions, accessibilité, écran de veille, accueil),
touche Accueil. Idempotent : peut être relancé sans risque.

  --dry-run      affiche les commandes (tests, build, adb) sans les exécuter ; le jeton est masqué (***)
  --sans-build   saute les tests et le build ; exige un APK déjà compilé (launcher/out/ZenHome.apk)
  --capture      enregistre une capture d'écran dans captures/ à la fin
  --reset-dock   réinitialise le dock et la disposition (zenhome_dock=auto, zenhome_layout=4).
                 Sans cette option, les réglages faits sur la TV ne sont jamais touchés.
  -h, --help     cette aide

Configuration : tv/tv.conf (TV_HOST, LAT, LON, CITY, PROFIL ; voir tv.conf.example), jeton TMDB dans ./tmdb.token.
TV_HOST et PROFIL peuvent aussi venir de l'environnement.
AIDE
}

CAPTURE=0; RESET_DOCK=0; SANS_BUILD=0
for a in "$@"; do
    case "$a" in
        --dry-run) DRY_RUN=1 ;;
        --sans-build) SANS_BUILD=1 ;;
        --capture) CAPTURE=1 ;;
        --reset-dock) RESET_DOCK=1 ;;
        -h|--help) aide; exit 0 ;;
        *) aide >&2; die "option inconnue : $a" ;;
    esac
done
charger_conf
verifier_adb

lancer_local() {
    if [ "$DRY_RUN" = 1 ]; then info "[dry-run] $*"; else "$@"; fi
}

# Broadcast de configuration. $1 = arguments (déjà quotés), $2 = version affichable (jeton masqué).
# La commande passe par l'entrée standard du shell adb : rien dans la liste des processus du Mac, aucun fichier.
envoyer_config() {
    local cmd="am broadcast -a tv.zenhome.CONFIG -n tv.zenhome/.ConfigReceiver $1" sortie
    if [ "$DRY_RUN" = 1 ]; then info "[dry-run] adb -s $SERIAL shell am broadcast -a tv.zenhome.CONFIG -n tv.zenhome/.ConfigReceiver $2"; return 0; fi
    sortie=$(printf '%s\n' "$cmd" | adb -s "$SERIAL" shell 2>&1 | tr -d '\r') || true
    # La sortie n'est jamais réaffichée (elle pourrait contenir le jeton).
    printf '%s' "$sortie" | grep -Fq 'Broadcast completed' || warn "broadcast de configuration sans confirmation (sortie non affichée)."
}

if [ "$SANS_BUILD" = 1 ]; then
    [ -f "$APK" ] || die "--sans-build : APK absent ($APK ; lancer launcher/build-mac.sh)."
    info "== 1/6 Tests : sautés (--sans-build)"
    info "== 2/6 Build : sauté (--sans-build), APK existant : $APK"
else
    info "== 1/6 Tests"
    lancer_local "$ROOT/launcher/test-mac.sh"
    info "== 2/6 Build"
    lancer_local "$ROOT/launcher/build-mac.sh"
fi
info "== 3/6 Connexion"
connecter_tv
info "== 4/6 Installation"
installer_apk || exit 1

info "== 5/6 Configuration"
# Ville / coordonnées
if [ -n "$LAT" ] && [ -n "$LON" ]; then
    num='^-?[0-9]+(\.[0-9]+)?$'
    [[ $LAT =~ $num && $LON =~ $num ]] || die "LAT/LON invalides dans tv.conf (attendu : 48.86)."
    args="--es lat $LAT --es lon $LON"
    [ -n "$CITY" ] && args="$args --es city $(shq "$CITY")"
    envoyer_config "$args" "$args"
else
    warn "LAT/LON absents de tv/tv.conf : météo non configurée."
fi
# Jeton TMDB : lu en mémoire, jamais affiché
if [ -f "$ROOT/tmdb.token" ]; then
    tok=$(tr -d ' \t\r\n' < "$ROOT/tmdb.token")
    [[ -n $tok && $tok =~ ^[A-Za-z0-9._~+/=-]+$ ]] || die "tmdb.token vide ou au format inattendu (contenu non affiché)."
    envoyer_config "--es tmdb_token $(shq "$tok")" "--es tmdb_token ***"
    tok=""
else
    warn "tmdb.token absent : films désactivés (voir README)."
fi
accorder_permissions
activer_accessibilite
adb_tv shell settings put secure screensaver_components "$DREAM"
definir_accueil
if [ "$RESET_DOCK" = 1 ]; then
    adb_tv shell settings put global zenhome_dock auto
    adb_tv shell settings put global zenhome_layout 4
    adb_tv shell settings put global zenhome_rev "$(date +%s)"
fi

info "== 6/6 Accueil"
touche_accueil
if [ "$CAPTURE" = 1 ]; then
    f="$ROOT/captures/zenhome-$(date +%Y%m%d-%H%M%S).png"
    if [ "$DRY_RUN" = 1 ]; then
        info "[dry-run] adb -s $SERIAL exec-out screencap -p > $f"
    else
        mkdir -p "$ROOT/captures"; sleep 2
        adb -s "$SERIAL" exec-out screencap -p > "$f"
        info "Capture : $f"
    fi
fi
info "Terminé."
