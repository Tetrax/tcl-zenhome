# Fonctions communes à deployer.sh, verifier-tv.sh, photos.sh et restaurer-tv.sh (à sourcer, pas à exécuter).
# Attend que le script appelant ait fait `set -euo pipefail` et défini DRY_RUN (0 ou 1).

TV_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ROOT=$(dirname "$TV_DIR")
APK="$ROOT/launcher/out/ZenHome.apk"
PKG=tv.zenhome
KEYSVC_FULL=tv.zenhome/tv.zenhome.KeyService
KEYSVC_SHORT=tv.zenhome/.KeyService
DREAM=tv.zenhome/tv.zenhome.ZenDream
DRY_RUN=${DRY_RUN:-0}
RE_PAQUET='^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z0-9_]+)+$'

info() { printf '%s\n' "$*"; }
warn() { printf 'Attention : %s\n' "$*" >&2; }
die()  { printf 'Erreur : %s\n' "$*" >&2; exit 1; }

# Quote une valeur pour le shell de la TV (apostrophes protégées).
shq() { local s=${1//\'/\'\\\'\'}; printf "'%s'" "$s"; }

# TV_HOST, LAT, LON, CITY, PROFIL : environnement > tv/tv.conf > défaut.
# tv.conf n'est JAMAIS exécuté : lecture ligne à ligne des seules clés connues (VAR="valeur").
charger_conf() {
    local c_host="" c_lat="" c_lon="" c_city="" c_profil="" l k v
    if [ -f "$TV_DIR/tv.conf" ]; then
        while IFS= read -r l || [ -n "$l" ]; do
            l=${l%$'\r'}
            case "$l" in ''|'#'*) continue ;; esac
            k=${l%%=*}; v=${l#*=}
            k=${k// /}
            case "$v" in \"*\") v=${v#\"}; v=${v%\"} ;; esac
            case "$k" in
                TV_HOST) c_host=$v ;;
                LAT) c_lat=$v ;;
                LON) c_lon=$v ;;
                CITY) c_city=$v ;;
                PROFIL) c_profil=$v ;;
            esac
        done < "$TV_DIR/tv.conf"
    fi
    TV_HOST="${TV_HOST:-${c_host:-192.168.1.50}}"
    LAT="${LAT:-$c_lat}"; LON="${LON:-$c_lon}"; CITY="${CITY:-$c_city}"
    PROFIL="${PROFIL:-${c_profil:-tcl-g08}}"
    [[ $TV_HOST =~ ^[A-Za-z0-9.-]+(:[0-9]+)?$ ]] || die "TV_HOST invalide : « $TV_HOST » (attendu : 192.168.1.50 ou hôte:port)."
    case "$TV_HOST" in *:*) SERIAL="$TV_HOST" ;; *) SERIAL="$TV_HOST:5555" ;; esac
    charger_profil
}

# Propriétés de l'appareil selon PROFIL : PRODUIT (ro.product.name), DEBLOAT (liste dans tv/), ROLE_HOME (1 = rôle HOME
# géré par `cmd role`, Android 10+), ACCUEIL (accueil d'origine), VEILLE (écran de veille d'origine).
charger_profil() {
    case "$PROFIL" in
        tcl-g08)
            PRODUIT=G08_4K_GB; DEBLOAT=debloat-tcl-g08.txt; ROLE_HOME=1
            ACCUEIL=com.google.android.apps.tv.launcherx/.home.HomeActivity
            VEILLE=com.google.android.apps.tv.dreamx/.service.Backdrop ;;
        mibox4)
            PRODUIT=oneday; DEBLOAT=debloat-mibox4.txt; ROLE_HOME=0
            ACCUEIL=com.google.android.tvlauncher/.MainActivity
            VEILLE=com.google.android.backdrop/.Backdrop ;;
        *) die "PROFIL inconnu : « $PROFIL » (attendu : tcl-g08 ou mibox4)." ;;
    esac
}

# Commande adb qui modifie la TV (affichée au lieu d'être lancée en --dry-run).
adb_tv() {
    if [ "$DRY_RUN" = 1 ]; then printf '[dry-run] adb -s %s %s\n' "$SERIAL" "$*"; return 0; fi
    adb -s "$SERIAL" "$@"
}

# Commande adb de lecture : sortie sur stdout sans CR, vide en --dry-run, jamais d'échec.
adb_lire() {
    if [ "$DRY_RUN" = 1 ]; then printf '[dry-run] adb -s %s %s\n' "$SERIAL" "$*" >&2; return 0; fi
    { adb -s "$SERIAL" "$@" 2>/dev/null | tr -d '\r'; } || true
}

# Variante tolérante : affiche un avertissement si la commande échoue.
adb_essayer() {
    local sortie
    if [ "$DRY_RUN" = 1 ]; then adb_tv "$@"; return 0; fi
    if ! sortie=$(adb -s "$SERIAL" "$@" 2>&1) || printf '%s' "$sortie" | grep -qE 'Exception|Error|Unknown'; then
        warn "refusé ou sans effet : $* ($(printf '%s' "$sortie" | head -1))"
    fi
}

verifier_adb() {
    [ "$DRY_RUN" = 1 ] || command -v adb >/dev/null || die "adb introuvable (brew install android-platform-tools)."
}

# Connexion + vérification que c'est la bonne TV. Arrêt si injoignable ou refus de confirmer.
connecter_tv() {
    local nom
    if [ "$DRY_RUN" = 1 ]; then info "[dry-run] adb connect $SERIAL"; else adb connect "$SERIAL" || true; fi
    if [ "$DRY_RUN" = 1 ]; then
        adb_lire shell getprop ro.product.name
        info "[dry-run] vérification : ro.product.name doit valoir $PRODUIT (sinon confirmation demandée)"
        return 0
    fi
    [ "$(adb -s "$SERIAL" get-state 2>/dev/null || true)" = device ] \
        || die "la TV $SERIAL ne répond pas (débogage ADB activé ? autorisation acceptée à l'écran ? IP correcte ?)."
    nom=$(adb_lire shell getprop ro.product.name)
    if [ "$nom" != "$PRODUIT" ]; then
        warn "appareil inattendu : ro.product.name = « $nom » (attendu : $PRODUIT)."
        [ -t 0 ] || die "confirmation impossible hors terminal."
        local rep; read -r -p "Continuer quand même ? [o/N] " rep
        case "$rep" in o|O|oui|OUI) ;; *) die "abandon." ;; esac
    fi
    info "TV connectée : $SERIAL ($nom)"
}

# adb install -r. Retourne 1 en cas d'échec ; ne désinstalle jamais rien.
installer_apk() {
    local sortie
    if [ "$DRY_RUN" = 1 ]; then adb_tv install -r "$APK"; return 0; fi
    [ -f "$APK" ] || { warn "APK absent : $APK (lancer launcher/build-mac.sh)."; return 1; }
    if sortie=$(adb -s "$SERIAL" install -r "$APK" 2>&1); then
        info "Zen Home installé."
        return 0
    fi
    if printf '%s' "$sortie" | grep -q INSTALL_FAILED_UPDATE_INCOMPATIBLE; then
        warn "signature différente de la version installée (INSTALL_FAILED_UPDATE_INCOMPATIBLE)."
        warn "La clé launcher/key.jks n'est pas celle qui a signé l'appli présente sur la TV."
        warn "Retrouvez la bonne clé, ou désinstallez à la main (les réglages de Zen Home seront perdus) :"
        warn "    adb -s $SERIAL uninstall $PKG"
        warn "Rien n'a été désinstallé automatiquement."
    else
        warn "échec de l'installation : $sortie"
    fi
    return 1
}

# Ajoute KeyService à enabled_accessibility_services sans écraser les autres services.
activer_accessibilite() {
    local cur
    cur=$(adb_lire shell settings get secure enabled_accessibility_services)
    [ "$cur" = null ] && cur=""
    case ":$cur:" in
        *":$KEYSVC_FULL:"*|*":$KEYSVC_SHORT:"*)
            # Une réinstallation de l'APK délie le service sans le retirer de la liste : on le retire puis on le remet pour le relier.
            local rest
            rest=$(printf '%s' "$cur" | tr ':' '\n' | grep -vxF -e "$KEYSVC_FULL" -e "$KEYSVC_SHORT" | paste -sd: -)
            adb_tv shell settings put secure enabled_accessibility_services "$(shq "${rest:-\"\"}")"
            adb_tv shell settings put secure enabled_accessibility_services "$(shq "${rest:+$rest:}$KEYSVC_FULL")"
            info "Accessibilité : KeyService relancé." ;;
        *)
            [ "$DRY_RUN" = 1 ] && info "[dry-run] (valeur actuelle conservée, KeyService ajouté à la suite)"
            adb_tv shell settings put secure enabled_accessibility_services "$(shq "${cur:+$cur:}$KEYSVC_FULL")"
            ;;
    esac
    adb_tv shell settings put secure accessibility_enabled 1
}

definir_accueil() {
    if [ "$ROLE_HOME" = 1 ]; then adb_tv shell cmd role add-role-holder android.app.role.HOME "$PKG"; fi
    adb_tv shell cmd package set-home-activity "$PKG/.Home"
}

# Permissions runtime, tolérant : un refus n'arrête rien.
# READ_MEDIA_IMAGES n'existe qu'à partir d'Android 13 (SDK 33) ; READ_EXTERNAL_STORAGE est déclarée avec
# maxSdkVersion=32 : on ne la demande que sur Android 12L et moins.
accorder_permissions() {
    local sdk
    sdk=$(adb_lire shell getprop ro.build.version.sdk)
    if [ "$DRY_RUN" = 1 ] || { [[ $sdk =~ ^[0-9]+$ ]] && [ "$sdk" -ge 33 ]; }; then
        adb_essayer shell pm grant "$PKG" android.permission.READ_MEDIA_IMAGES
    fi
    # Stats d'usage (écran « Applis ouvertes ») : opération appops, pas une permission runtime
    adb_essayer shell appops set "$PKG" GET_USAGE_STATS allow
    if [ "$DRY_RUN" = 1 ]; then
        info "[dry-run] (READ_EXTERNAL_STORAGE : seulement si ro.build.version.sdk <= 32)"
        adb_tv shell pm grant "$PKG" android.permission.READ_EXTERNAL_STORAGE
    elif [[ $sdk =~ ^[0-9]+$ ]] && [ "$sdk" -le 32 ]; then
        adb_essayer shell pm grant "$PKG" android.permission.READ_EXTERNAL_STORAGE
    fi
}

touche_accueil() { adb_tv shell input keyevent KEYCODE_HOME; }
