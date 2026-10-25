#!/bin/sh
# TV_HOST : environnement > tv/tv.conf (lu, jamais exécuté) > défaut
CH=
if [ -f "$(dirname "$0")/tv.conf" ]; then
    while IFS= read -r l || [ -n "$l" ]; do
        case "$l" in TV_HOST=*) CH=${l#TV_HOST=}; CH=${CH#\"}; CH=${CH%\"} ;; esac
    done < "$(dirname "$0")/tv.conf"
fi
H=${TV_HOST:-${CH:-192.168.1.50}}
case "$H" in
    ''|*[!A-Za-z0-9.:-]*|*:*:*|:*|*:) echo "Erreur : TV_HOST invalide : $H" >&2; exit 1 ;;
    *:*) T=$H ;;
    *) T=$H:5555 ;;
esac
# Quote une valeur pour le shell de la TV
shq() { printf "'%s'" "$(printf '%s' "$1" | sed "s/'/'\\\\''/g")"; }
adb connect "$T"
adb -s $T shell pm enable com.tcl.channelplus
adb -s $T shell pm enable com.tcl.tv.tclhome_passive
adb -s $T shell pm enable com.tcl.waterfall.overseas
adb -s $T shell pm enable com.tcl.esticker
adb -s $T shell pm enable com.tcl.exhibit
adb -s $T shell pm enable com.tcl.interactive
adb -s $T shell pm enable com.tcl.messagebox
adb -s $T shell pm enable com.tcl.usercenter
adb -s $T shell pm enable com.tcl.ttvs
adb -s $T shell pm enable android.autoinstalls.config.google.gtvpai
adb -s $T shell pm enable com.tcl.logkit
adb -s $T shell pm enable com.tcl.guard
adb -s $T shell pm enable com.google.android.feedback
adb -s $T shell pm enable com.android.tv.feedbackconsent
adb -s $T shell pm enable com.tcl.smartalexa
adb -s $T shell pm enable com.tcl.gamebar
adb -s $T shell pm enable com.tcl.repairguide
adb -s $T shell pm enable com.tcl.ocean.instructions
adb -s $T shell pm enable com.google.android.play.games
adb -s $T shell pm enable com.google.android.youtube.tvmusic
adb -s $T shell pm enable com.google.android.apps.tv.dreamx
adb -s $T shell pm enable com.tcl.browser
adb -s $T shell pm enable com.tcl.partnercustomizer
adb -s $T shell pm enable com.tcl.ui_mediaCenter
adb -s $T shell pm enable com.google.android.katniss
adb -s $T shell pm enable com.google.android.apps.mediashell
adb -s $T shell pm enable com.google.android.tv.remote.service
for k in window_animation_scale transition_animation_scale; do adb -s $T shell settings put global $k 1.0; done
adb -s $T shell settings delete global animator_duration_scale
adb -s $T shell settings put secure screensaver_components com.google.android.apps.tv.dreamx/.service.Backdrop
# Spotify : levée des restrictions d'arrière-plan
adb -s $T shell cmd appops set com.spotify.tv.android RUN_ANY_IN_BACKGROUND allow
adb -s $T shell am set-standby-bucket com.spotify.tv.android active
# Zen Home : remise de l'accueil Google TV
adb -s $T shell pm enable com.google.android.tungsten.setupwraith
adb -s $T shell pm enable com.google.android.apps.tv.launcherx
adb -s $T shell cmd role add-role-holder android.app.role.HOME com.google.android.apps.tv.launcherx
adb -s $T shell cmd package set-home-activity com.google.android.apps.tv.launcherx/.home.HomeActivity
adb -s $T uninstall tv.zenhome
adb -s $T shell input keyevent KEYCODE_HOME
# Annule la configuration de deployer.sh : retire seulement KeyService des services d'accessibilité (les autres sont conservés)
A=$(adb -s $T shell settings get secure enabled_accessibility_services | tr -d '\r')
R=$(printf '%s' "$A" | tr ':' '\n' | grep -v -e '^tv.zenhome/' -e '^null$' -e '^$' | paste -sd: -)
if [ -n "$R" ]; then
    adb -s $T shell settings put secure enabled_accessibility_services "$(shq "$R")"
else
    adb -s $T shell settings delete secure enabled_accessibility_services
    adb -s $T shell settings put secure accessibility_enabled 0
fi
# Réglages du kit (zenhome_*) ; l'écran de veille Google est déjà remis plus haut. Photos : tv/photos.sh --reset
for k in zenhome_dock zenhome_layout zenhome_rev zenhome_interval zenhome_title_mode zenhome_title_b64; do adb -s $T shell settings delete global $k; done
