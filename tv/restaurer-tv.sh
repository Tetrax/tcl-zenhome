#!/bin/bash
# Remet l'appareil comme avant Zen Home : réactive les paquets désactivés, rend l'accueil et l'écran de veille d'origine.
set -euo pipefail
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

aide() {
    command cat <<'AIDE'
Usage : tv/restaurer-tv.sh [--dry-run] [--help]

Annule deployer.sh et le debloat : réactive tous les paquets désactivés (pm list packages -d),
remet l'écran de veille et l'accueil d'origine du profil, désinstalle Zen Home, retire KeyService
de l'accessibilité et supprime les réglages zenhome_*. Un échec n'arrête pas la restauration.
Photos personnelles : tv/photos.sh --reset.

  --dry-run   affiche les commandes adb sans les lancer
  -h, --help  cette aide

Appareil choisi par PROFIL (tv/tv.conf ou environnement) ; TV_HOST comme pour les autres scripts.
AIDE
}

for a in "$@"; do
    case "$a" in
        --dry-run) DRY_RUN=1 ;;
        -h|--help) aide; exit 0 ;;
        *) aide >&2; die "option inconnue : $a" ;;
    esac
done
charger_conf
verifier_adb

info "TV : $SERIAL"
connecter_tv

# Paquets désactivés (par le debloat ou à la main) : tous réactivés, seuls les noms valides sont utilisés.
for l in $(adb_lire shell pm list packages -d); do
    [[ $l == package:* ]] || continue
    l=${l#package:}
    [[ $l =~ $RE_PAQUET ]] && adb_essayer shell pm enable "$l"
done
adb_essayer shell settings put secure screensaver_components "$VEILLE"
for k in window_animation_scale transition_animation_scale; do adb_essayer shell settings put global "$k" 1.0; done
adb_essayer shell settings delete global animator_duration_scale
# Spotify : levée des restrictions d'arrière-plan
adb_essayer shell cmd appops set com.spotify.tv.android RUN_ANY_IN_BACKGROUND allow
adb_essayer shell am set-standby-bucket com.spotify.tv.android active
# Remise de l'accueil d'origine
if [ "$ROLE_HOME" = 1 ]; then adb_essayer shell cmd role add-role-holder android.app.role.HOME "${ACCUEIL%%/*}"; fi
adb_essayer shell cmd package set-home-activity "$ACCUEIL"
adb_essayer uninstall "$PKG"
adb_essayer shell input keyevent KEYCODE_HOME
# Retire seulement KeyService des services d'accessibilité (les autres sont conservés)
A=$(adb_lire shell settings get secure enabled_accessibility_services)
R=$(printf '%s' "$A" | tr ':' '\n' | grep -v -e '^tv.zenhome/' -e '^null$' -e '^$' | paste -sd: - || true)
if [ -n "$R" ]; then
    adb_essayer shell settings put secure enabled_accessibility_services "$(shq "$R")"
else
    adb_essayer shell settings delete secure enabled_accessibility_services
    adb_essayer shell settings put secure accessibility_enabled 0
fi
# Réglages du kit (zenhome_*)
for k in zenhome_dock zenhome_layout zenhome_rev zenhome_interval zenhome_title_mode zenhome_title_b64; do adb_essayer shell settings delete global "$k"; done
info "Terminé."
