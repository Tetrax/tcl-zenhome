#!/bin/bash
# Contrôle l'état de la TV après une mise à jour TCL/Google ; --fix réapplique ce qui est KO.
set -euo pipefail
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

aide() {
    command cat <<'AIDE'
Usage : tv/verifier-tv.sh [--fix] [--dry-run] [--help]

Affiche un tableau OK / KO / AVERT :
  Zen Home installé (version), rôle HOME (accueil préféré sous Android 9), accueil d'origine
  (launcherx sur la TCL, tvlauncher sur la Mi Box) et setupwraith désactivés, paquets de la liste
  du profil (tv/debloat-<profil>.txt) désactivés, KeyService activé, écran de veille ZenDream,
  restriction d'arrière-plan de Spotify (appops RUN_ANY_IN_BACKGROUND = ignore), Wi-Fi non lié à setupwraith.

  --fix       réapplique uniquement ce qui est KO, dans un ordre sûr (Zen Home et rôle HOME avant de
              désactiver l'accueil d'origine ; setupwraith seulement si le Wi-Fi n'y est pas lié), appuie sur
              Accueil et vérifie que tv.zenhome est au premier plan, sinon réactive l'accueil d'origine
  --dry-run   affiche les commandes adb sans les lancer (tous les contrôles sont alors simulés KO)
  -h, --help  cette aide

AVERT = situation connue, non corrigée volontairement (n'entraîne pas de code de sortie 1).
setupwraith n'est désactivé que si le Wi-Fi est vérifié non lié ; si dumpsys wifi est vide ou inattendu : AVERT « non vérifié ».
Code de sortie : 0 si aucun KO et état des paquets lisible, 1 sinon.
AIDE
}

FIX=0
for a in "$@"; do
    case "$a" in
        --fix) FIX=1 ;;
        --dry-run) DRY_RUN=1 ;;
        -h|--help) aide; exit 0 ;;
        *) aide >&2; die "option inconnue : $a" ;;
    esac
done
charger_conf
verifier_adb

LAUNCHERX=${ACCUEIL%%/*}
WRAITH=com.google.android.tungsten.setupwraith
SPOTIFY=com.spotify.tv.android
LISTE="$TV_DIR/$DEBLOAT"
[ -f "$LISTE" ] || die "liste introuvable : $LISTE"
PAQUETS=()
while IFS= read -r l || [ -n "$l" ]; do
    l=${l%%#*}; l=${l//[[:space:]]/}
    [ -n "$l" ] || continue
    [[ $l =~ $RE_PAQUET ]] || die "nom de paquet invalide dans $(basename "$LISTE") : « $l »"
    PAQUETS+=("$l")
done < "$LISTE"
[ "${#PAQUETS[@]}" -gt 0 ] || die "aucun paquet dans $(basename "$LISTE")"

R_NOM=(); R_ST=(); R_DET=(); KO=0
# enreg NOM ETAT(1 OK, 0 KO, 2 AVERT) DETAIL [reel] -> positionne LAST_OK. En --dry-run tout est simulé KO sauf « reel ».
enreg() {
    local etat=$2 det=$3
    if [ "$DRY_RUN" = 1 ] && [ "${4:-}" != reel ]; then etat=0; det="simulé (dry-run)"; fi
    if [ "$etat" = 2 ] && [ -z "$det" ]; then det="non vérifié"; fi
    R_NOM+=("$1"); R_ST+=("$etat"); R_DET+=("$det"); LAST_OK=$etat
    [ "$etat" = 0 ] && KO=$((KO + 1))
    return 0
}

# Paquet qui détient l'accueil : rôle HOME si géré (Android 10+), sinon accueil préféré (set-home-activity).
# Sans rôle, l'accueil d'origine (priorité système plus haute) passe devant l'accueil préféré tant qu'il est actif.
detenteur_home() {
    local h
    if [ "$ROLE_HOME" = 1 ]; then adb_lire shell cmd role get-role-holders --user 0 android.app.role.HOME
    else h=$(adb_lire shell dumpsys package preferred-xml | awk -F'"' '/<item name=/ { n = $2 } /category.HOME/ { print n; exit }'); printf '%s\n' "${h%%/*}"; fi
}

# Vrai si le paquet est désactivé, ou absent de la TV.
inactif() {
    grep -Fqx "package:$1" <<<"$DESACTIVES" && return 0
    grep -Fqx "package:$1" <<<"$TOUS" && return 1
    return 0
}
# 1 désactivé/absent, 0 actif, 2 inconnu (liste des paquets illisible : on ne conclut pas).
etat_pkg() { if [ "$INCONNU" = 1 ]; then echo 2; elif inactif "$1"; then echo 1; else echo 0; fi; }

controler() {
    R_NOM=(); R_ST=(); R_DET=(); KO=0
    local TOUS DESACTIVES dump ver holder a11y dream spot wifi p n etat det usage
    # Wi-Fi : 1 vérifié et non lié, 2 lié à setupwraith, 3 non vérifiable (sortie vide ou format inattendu).
    # Dans les cas 2 et 3, setupwraith n'est JAMAIS désactivé.
    wifi=$(adb_lire shell dumpsys wifi)
    n=$(grep -ciE 'Configured networks|cuid=|cname=|creatorname' <<<"$wifi" || true)
    if [ "$n" -eq 0 ]; then ST_WIFI=3
    else
        n=$({ grep -iE 'creatorname|cname' <<<"$wifi" | grep -ci setupwraith; } || true)
        if [ "$n" -gt 0 ]; then ST_WIFI=2; else ST_WIFI=1; fi
    fi
    TOUS=$(adb_lire shell pm list packages)
    DESACTIVES=$(adb_lire shell pm list packages -d)
    INCONNU=0
    if [ "$DRY_RUN" != 1 ] && ! grep -Fq 'package:' <<<"$TOUS"; then INCONNU=1; warn "liste des paquets illisible : état des paquets inconnu."; fi

    if [ "$INCONNU" = 1 ]; then enreg "Zen Home installé" 2 ""
    elif grep -Fqx "package:$PKG" <<<"$TOUS"; then
        dump=$(adb_lire shell dumpsys package "$PKG")
        ver=$(awk -F= '/versionName=/{print $2; exit}' <<<"$dump")
        enreg "Zen Home installé" 1 "version ${ver:-?}"
    else enreg "Zen Home installé" 0 "absent"; fi
    ST_INST=$LAST_OK

    holder=$(detenteur_home)
    if [ "$holder" = "$PKG" ]; then enreg "Rôle HOME = $PKG" 1 ""; else enreg "Rôle HOME = $PKG" 0 "actuel : ${holder:-aucun}"; fi
    ST_ROLE=$LAST_OK

    for p in $LAUNCHERX $WRAITH; do
        etat=$(etat_pkg "$p"); det=""
        if [ "$p" = "$WRAITH" ] && [ "$etat" = 0 ] && [ "$ST_WIFI" != 1 ]; then
            etat=2; det="gardé actif : Wi-Fi $([ "$ST_WIFI" = 2 ] && echo "lié à setupwraith" || echo "non vérifié")"
        fi
        enreg "Désactivé : $p" "$etat" "$det"
        if [ "$p" = "$LAUNCHERX" ]; then ST_LX=$LAST_OK; else ST_SW=$LAST_OK; fi
    done

    DEBLOAT_KO=()
    for p in "${PAQUETS[@]}"; do
        enreg "Désactivé : $p" "$(etat_pkg "$p")" ""
        if [ "$LAST_OK" = 0 ]; then DEBLOAT_KO+=("$p"); fi
    done

    a11y=$(adb_lire shell settings get secure enabled_accessibility_services)
    case ":$a11y:" in
        *":$KEYSVC_FULL:"*|*":$KEYSVC_SHORT:"*) enreg "Accessibilité : KeyService" 1 "" ;;
        *) enreg "Accessibilité : KeyService" 0 "absent de enabled_accessibility_services" ;;
    esac
    ST_A11Y=$LAST_OK

    dream=$(adb_lire shell settings get secure screensaver_components)
    case "$dream" in
        "$DREAM"|"$PKG/.ZenDream") enreg "Écran de veille = ZenDream" 1 "" ;;
        *) enreg "Écran de veille = ZenDream" 0 "actuel : ${dream:-aucun}" ;;
    esac
    ST_DREAM=$LAST_OK

    usage=$(adb_lire shell appops get "$PKG" GET_USAGE_STATS)
    if grep -q 'GET_USAGE_STATS: allow' <<<"$usage"; then enreg "Stats d'usage (Applis ouvertes)" 1 ""
    else enreg "Stats d'usage (Applis ouvertes)" 0 "appops GET_USAGE_STATS non accordé"; fi
    ST_USAGE=$LAST_OK

    if [ "$INCONNU" = 0 ] && [ "$DRY_RUN" != 1 ] && ! grep -Fqx "package:$SPOTIFY" <<<"$TOUS"; then
        enreg "Spotify : arrière-plan restreint" 1 "Spotify absent"
    else
        spot=$(adb_lire shell cmd appops get "$SPOTIFY" RUN_ANY_IN_BACKGROUND)
        n=$(grep -c 'RUN_ANY_IN_BACKGROUND: ignore' <<<"$spot" || true)
        if [ "$n" -gt 0 ]; then enreg "Spotify : arrière-plan restreint" 1 ""; else enreg "Spotify : arrière-plan restreint" 0 "pas en « ignore »"; fi
    fi
    ST_SPOT=$LAST_OK

    case "$ST_WIFI" in
        1) enreg "Wi-Fi non lié à setupwraith" 1 "" reel ;;
        2) enreg "Wi-Fi non lié à setupwraith" 2 "réseau créé par setupwraith : ne PAS le désactiver" reel ;;
        *) enreg "Wi-Fi non lié à setupwraith" 2 "non vérifié (dumpsys wifi vide ou format inattendu) : setupwraith gardé actif" reel ;;
    esac
}

afficher() {
    local i
    printf '\n%-7s %-58s %s\n' ETAT CONTRÔLE DÉTAIL
    for ((i = 0; i < ${#R_NOM[@]}; i++)); do
        local e=OK; [ "${R_ST[$i]}" = 0 ] && e=KO; [ "${R_ST[$i]}" = 2 ] && e=AVERT
        printf '%-7s %-58s %s\n' "$e" "${R_NOM[$i]}" "${R_DET[$i]}"
    done
    printf '\nKO : %d\n' "$KO"
}

role_ok() {
    [ "$DRY_RUN" = 1 ] && return 0
    [ "$(detenteur_home)" = "$PKG" ]
}

au_premier_plan() {
    local n
    n=$({ adb_lire shell dumpsys activity activities | grep -E 'topResumedActivity|mResumedActivity' | grep -c "$PKG/"; } || true)
    [ "$n" -gt 0 ]
}

corriger() {
    local p ok_home=0 lx_desactive=0 sw_desactive=0
    # 1. Zen Home d'abord : installation, accueil, services
    if [ "$ST_INST" = 1 ]; then ok_home=1
    elif [ "$ST_INST" = 0 ] && installer_apk; then ok_home=1; fi
    if [ "$ok_home" = 1 ]; then
        [ "$ST_ROLE" = 1 ] || definir_accueil
        [ "$ST_A11Y" = 1 ] || activer_accessibilite
        [ "$ST_DREAM" = 1 ] || adb_tv shell settings put secure screensaver_components "$DREAM"
        [ "$ST_USAGE" = 1 ] || adb_tv shell appops set "$PKG" GET_USAGE_STATS allow
    fi
    # 2. Debloat
    for p in ${DEBLOAT_KO[@]+"${DEBLOAT_KO[@]}"}; do adb_tv shell pm disable-user --user 0 "$p"; done
    [ "$ST_SPOT" = 1 ] || adb_tv shell cmd appops set "$SPOTIFY" RUN_ANY_IN_BACKGROUND ignore
    # 3. launcherx : seulement si Zen Home est installé ET détenteur du rôle HOME
    if [ "$ST_LX" = 0 ]; then
        if [ "$ok_home" = 1 ] && role_ok; then
            adb_tv shell pm disable-user --user 0 "$LAUNCHERX"; lx_desactive=1
        else
            warn "launcherx laissé actif : Zen Home n'est pas installé ou n'a pas le rôle HOME."
        fi
    fi
    # 4. setupwraith : seulement si le Wi-Fi est VÉRIFIÉ non lié (ST_WIFI = 1) ; échec fermé sinon
    if [ "$ST_SW" = 0 ] && [ "$ST_WIFI" = 1 ]; then
        adb_tv shell pm disable-user --user 0 "$WRAITH"; sw_desactive=1
    fi
    # 5. Contrôle de l'accueil, retour arrière si nécessaire
    touche_accueil
    if [ "$DRY_RUN" = 1 ]; then
        info "[dry-run] attente 3 s puis contrôle du premier plan ; si $PKG absent : adb -s $SERIAL shell pm enable $LAUNCHERX (et $WRAITH si désactivé par ce passage)"
    else
        sleep 3
        if ! au_premier_plan; then
            warn "$PKG n'est pas au premier plan après Accueil."
            if [ "$lx_desactive" = 1 ] || [ "$sw_desactive" = 1 ]; then
                warn "retour arrière : réactivation de launcherx et de setupwraith."
                adb_tv shell pm enable "$LAUNCHERX"
                adb_tv shell pm enable "$WRAITH"
                touche_accueil
            fi
        else info "$PKG est bien au premier plan."; fi
    fi
}

info "TV : $SERIAL"
connecter_tv
controler
afficher
if [ "$FIX" = 1 ] && [ "$KO" -gt 0 ]; then
    info "== Réparation"
    corriger
    [ "$DRY_RUN" = 1 ] || { info "== Nouveau contrôle"; controler; afficher; }
fi
[ "$DRY_RUN" = 1 ] && exit 0
[ "$KO" -eq 0 ] && [ "$INCONNU" = 0 ]
