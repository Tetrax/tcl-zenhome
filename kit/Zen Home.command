#!/bin/bash
# Zen Home : kit « double-clic » pour installer Zen Home et alléger une Xiaomi Mi Box 4.
# Les choix passent par des fenêtres macOS (osascript) ; le Terminal montre la progression.
# KIT_TEST=1 (tests) : osascript est remplacé par un faux qui lit ses réponses dans le fichier KIT_REPONSES
# (une ligne par fenêtre ; CANCEL ou fin de fichier = fenêtre fermée, VIDE = réponse vide), sans pause ni attente.
set -uo pipefail
# Outils du système uniquement (adb : celui téléchargé et vérifié par le kit), sauf en test (faux adb dans le PATH).
if [ -z "${KIT_TEST:-}" ]; then PATH=/usr/bin:/bin:/usr/sbin:/sbin; export PATH; fi

KIT_VERSION=1.0
PT_URL=https://dl.google.com/android/repository/platform-tools_r37.0.1-darwin.zip
PT_SHA256=ee39ad5967e95c2a07f04dbcbde96b1a0c916ba376096db5d2f498b7727a5d1d
APPSUP="$HOME/Library/Application Support/ZenHome"
PT_DIR="$APPSUP/platform-tools"
RE_PKG='^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$'
# Jamais désinstallés par le kit (verifier-tv.sh --fix gère lui-même l'accueil d'origine et setupwraith).
PROTEGES=" tv.zenhome com.google.android.tvlauncher com.google.android.tungsten.setupwraith com.android.vending com.android.tv.settings "
PAUSE=3; if [ -n "${KIT_TEST:-}" ]; then PAUSE=0; fi
umask 077

TOKFILE=""; DLDIR=""; SAUV=""; RAPPORT=""
D_PKG=(); D_TAG=(); D_LAB=(); D_EXP=()      # liste debloat : paquet, balise (option|garder|vide), libellé, explication
L_PKG=(); L_LAB=()                          # noms lisibles renvoyés par Zen Home
OPT_G=(); OPT_D=()                          # options gardées / désactivées
U_PKG=(); U_ITEM=(); UN_PKG=(); UN_ITEM=()  # applis de l'utilisateur proposées / choisies pour la désinstallation
OK_VERIF=0

# ---------------------------------------------------------------- fenêtres

# osa MODE TITRE TEXTE ARGS... ; les textes sont des ARGUMENTS d'AppleScript, jamais collés dans son code.
#   btn  DEFAUT BOUTON... -> bouton cliqué      txt DEFAUT -> texte saisi      pwd -> texte saisi (masqué)
#   list MULTI(0|1) ITEM... -> éléments choisis, un par ligne
#   cases N DEFAUT1..DEFAUTN ITEM... -> cases à cocher (multi), les N premiers arguments sont précochés
# Statut 1 si l'utilisateur ferme ou annule.
osa() {
    if [ -n "${KIT_TEST:-}" ]; then osa_faux "$@"; return; fi
    osascript - "$@" <<'APPLESCRIPT'
on run argv
    set mode to item 1 of argv
    set t to item 2 of argv
    set m to item 3 of argv
    if mode is "btn" then
        return button returned of (display dialog m with title t buttons (items 5 thru -1 of argv) default button (item 4 of argv))
    else if mode is "txt" then
        return text returned of (display dialog m with title t default answer (item 4 of argv) buttons {"Annuler", "OK"} default button "OK" cancel button "Annuler")
    else if mode is "pwd" then
        return text returned of (display dialog m with title t default answer "" buttons {"Annuler", "OK"} default button "OK" cancel button "Annuler" with hidden answer)
    else if mode is "cases" then
        set n to (item 4 of argv) as integer
        set defs to {}
        if n > 0 then set defs to items 5 thru (4 + n) of argv
        set r to choose from list (items (5 + n) thru -1 of argv) with title t with prompt m default items defs multiple selections allowed true empty selection allowed true OK button name "Continuer" cancel button name "Annuler"
        if r is false then error number -128
        set AppleScript's text item delimiters to linefeed
        return r as text
    else
        set multi to (item 4 of argv is "1")
        set r to choose from list (items 5 thru -1 of argv) with title t with prompt m multiple selections allowed multi empty selection allowed multi OK button name "Continuer" cancel button name "Annuler"
        if r is false then error number -128
        set AppleScript's text item delimiters to linefeed
        return r as text
    end if
end run
APPLESCRIPT
}

# Faux osascript : journalise la fenêtre dans $KIT_REPONSES.dialogues, répond avec la ligne de même rang du fichier.
# btn/txt : DEFAUT = valeur par défaut, sinon réponse telle quelle. list : motifs séparés par | (éléments qui les contiennent), NONE = rien.
osa_faux() {
    local n rep item p
    printf '%s\n' "$(printf '%s ¦ ' "$@" | tr '\n' '~')" >> "$KIT_REPONSES.dialogues"
    n=$(wc -l < "$KIT_REPONSES.dialogues" | tr -d ' ')
    rep=$(sed -n "${n}p" "$KIT_REPONSES")
    case "$rep" in ''|CANCEL) return 1 ;; VIDE) echo; return 0 ;; esac
    case "$1" in
        btn|txt) if [ "$rep" = DEFAUT ]; then printf '%s\n' "$4"; else printf '%s\n' "$rep"; fi ;;
        pwd) printf '%s\n' "$rep" ;;
        cases)   # DEFAUT = les éléments précochés ; sinon comme list
            shift 3; local nd=$1; shift
            if [ "$rep" = DEFAUT ]; then printf '%s\n' "${@:1:nd}"; return 0; fi
            shift "$nd"
            [ "$rep" = NONE ] && return 0
            for item in "$@"; do
                for p in $(printf '%s' "$rep" | tr '|' ' '); do
                    case "$item" in *"$p"*) printf '%s\n' "$item"; break ;; esac
                done
            done ;;
        list)
            shift 4
            [ "$rep" = NONE ] && return 0
            for item in "$@"; do
                for p in $(printf '%s' "$rep" | tr '|' ' '); do
                    case "$item" in *"$p"*) printf '%s\n' "$item"; break ;; esac
                done
            done ;;
    esac
}

fenetre() { osa btn "$1" "$2" OK OK >/dev/null || true; }

erreur() {
    printf '\nErreur : %s\n' "$1" >&2
    fenetre "Zen Home : un problème" "$1"
    exit 1
}

MODIFIE=0   # 1 dès que des choix ont commencé à être appliqués
arreter() {
    if [ "$MODIFIE" = 1 ]; then printf '\n%s\n' "Arrêté à votre demande. Une partie des choix a pu être appliquée : relancez le kit et choisissez « Réparer » pour terminer."
    else printf '\n%s\n' "Arrêté à votre demande. Aucune appli n'a été désactivée ni désinstallée."; fi
    exit 0
}

fin() {
    if [ -n "$TOKFILE" ]; then rm -f "$TOKFILE"; fi
    if [ -n "$DLDIR" ]; then rm -rf "$DLDIR"; fi
    if [ -z "${KIT_TEST:-}" ]; then printf '\nAppuyez sur Entrée pour fermer cette fenêtre. '; read -r _ </dev/tty; fi
}
trap fin EXIT
trap 'exit 130' INT TERM HUP

etape() { printf '\n== %s\n' "$*"; }

# ---------------------------------------------------------------- racine du kit et lib.sh

DIR=$(cd "$(dirname "$0")" && pwd)
if [ -f "$DIR/tv/lib.sh" ]; then ROOT=$DIR
elif [ -f "$DIR/../tv/lib.sh" ]; then ROOT=$DIR/..
else erreur "Le kit est incomplet (dossier « tv » introuvable). Décompressez l'archive en entier avant de lancer ce fichier."
fi
# Dans l'archive, un tv.conf ne peut venir que d'une erreur d'empaquetage : il enverrait la position de quelqu'un d'autre.
if [ "$ROOT" = "$DIR" ] && [ -e "$ROOT/tv/tv.conf" ]; then
    erreur "Le kit contient un fichier tv/tv.conf qui ne devrait pas s'y trouver. Supprimez-le, ou retéléchargez le kit."
fi
# shellcheck source=../tv/lib.sh
. "$ROOT/tv/lib.sh"

paquet_valide() { local LC_ALL=C; [ "${#1}" -le 255 ] && [[ $1 =~ $RE_PKG ]]; }
protege() { case "$PROTEGES" in *" $1 "*) return 0 ;; esac; return 1; }
dans_liste() { grep -Fqx "package:$2" <<<"$1"; }   # liste « package:x » par lignes
dans_csv() { case "$1" in *",$2,"*) return 0 ;; esac; return 1; }
idx_debloat() { local i; IDX=-1; for ((i = 0; i < ${#D_PKG[@]}; i++)); do [ "${D_PKG[i]}" = "$1" ] && { IDX=$i; return 0; }; done; return 1; }
lab_debloat() { if idx_debloat "$1"; then printf '%s' "${D_LAB[IDX]}"; else printf '%s' "$1"; fi; }
puces() { local p; for p in "$@"; do printf '• %s\n' "$(lab_debloat "$p")"; done; }

# ---------------------------------------------------------------- 1. adb

trouver_adb() {
    command -v adb >/dev/null 2>&1 && return 0
    [ -x "$PT_DIR/adb" ] || telecharger_adb
    PATH="$PT_DIR:$PATH"; export PATH
}

telecharger_adb() {
    local rep got
    rep=$(osa btn "Un outil est nécessaire" "Pour dialoguer avec votre box, ce kit a besoin d'« adb », un outil officiel et gratuit de Google (Android SDK Platform-Tools).

Il va être téléchargé depuis dl.google.com (environ 18 Mo), vérifié par empreinte numérique, puis rangé dans le dossier Application Support/ZenHome de votre Bibliothèque. Rien d'autre n'est installé sur votre Mac.

Page officielle et conditions d'utilisation :
https://developer.android.com/studio/releases/platform-tools

En cliquant sur « J'accepte », vous acceptez ces conditions." "J'accepte" "Annuler" "J'accepte") || arreter
    [ "$rep" = "J'accepte" ] || arreter
    DLDIR=$(mktemp -d "$APPSUP/telechargement.XXXXXX") || erreur "Impossible de créer un dossier temporaire."
    etape "Téléchargement d'adb (environ 18 Mo)"
    curl --fail --proto '=https' --proto-redir '=https' --location --progress-bar --max-time 600 -o "$DLDIR/pt.zip" "$PT_URL" \
        || erreur "Le téléchargement d'adb a échoué. Vérifiez votre connexion Internet puis relancez le kit."
    got=$(shasum -a 256 "$DLDIR/pt.zip" | awk '{print $1}')
    if [ "$got" != "$PT_SHA256" ]; then
        rm -f "$DLDIR/pt.zip"
        erreur "Le fichier téléchargé n'est pas celui attendu (empreinte différente). Par sécurité il a été supprimé et rien n'a été installé."
    fi
    ditto -x -k "$DLDIR/pt.zip" "$DLDIR/x" && [ -x "$DLDIR/x/platform-tools/adb" ] || erreur "Le fichier téléchargé est inutilisable. Relancez le kit."
    rm -rf "$PT_DIR"
    mv "$DLDIR/x/platform-tools" "$PT_DIR" || erreur "Impossible de ranger adb dans $PT_DIR."
    info "adb installé dans $PT_DIR"
}

# ---------------------------------------------------------------- 2-3. adresse IP, connexion

ip_valide() {
    local re='^([0-9]{1,3}\.){3}[0-9]{1,3}$' o IFS=.
    [[ $1 =~ $re ]] || return 1
    for o in $1; do [ "$((10#$o))" -le 255 ] || return 1; done
}

# dns-sd ARGS... pendant $1 secondes (il ne s'arrête jamais seul) ; sortie dans le fichier $2.
dnssd_pendant() {
    local duree=$1 f=$2 pid; shift 2
    dns-sd "$@" > "$f" 2>/dev/null & pid=$!
    sleep "$duree"; kill "$pid" 2>/dev/null; wait "$pid" 2>/dev/null
}

# Mi Box allumées sur le réseau : elles s'annoncent en Chromecast (_googlecast._tcp) avec md=MIBOX…
# Remplit F_NOM / F_IP. Tout ce qui vient du réseau est validé ou filtré avant usage.
chercher_box() {
    local d inst host nom ip
    F_NOM=(); F_IP=()
    command -v dns-sd >/dev/null 2>&1 || return 0
    d=$(mktemp -d "${TMPDIR:-/tmp}/zenhome-recherche.XXXXXX") || return 0
    etape "Recherche de la Mi Box sur le réseau"
    dnssd_pendant 4 "$d/b" -B _googlecast._tcp local
    awk '$2 == "Add" { sub(/^.*_googlecast\._tcp\.[ \t]+/, ""); print }' "$d/b" | sort -u > "$d/inst"
    while IFS= read -r inst; do
        [[ $inst =~ ^MIBOX[A-Za-z0-9-]{1,80}$ ]] || continue
        dnssd_pendant 2 "$d/l" -L "$inst" _googlecast._tcp local
        grep -q 'md=MIBOX' "$d/l" || continue
        host=$(sed -n 's/.* can be reached at \([^ :]*\):[0-9]*.*/\1/p' "$d/l" | head -1)
        [[ $host =~ ^[A-Za-z0-9.-]{1,120}$ ]] || continue
        nom=$(perl -ne 'if (/\bfn=((?:\\.|[^ \\])*)/) { ($n = $1) =~ s/\\(.)/$1/g; print $n; exit }' "$d/l" 2>/dev/null \
            | LC_ALL=C tr -cd 'A-Za-z0-9 ._-' | cut -c1-30)
        dnssd_pendant 2 "$d/g" -G v4 "$host"
        ip=$(grep -oE '([0-9]{1,3}\.){3}[0-9]{1,3}' "$d/g" | head -1)
        ip_valide "$ip" || continue
        case " ${F_IP[*]-} " in *" $ip "*) continue ;; esac
        F_NOM+=("${nom:-Mi Box}"); F_IP+=("$ip")
    done < "$d/inst"
    rm -rf "$d"
    info "${#F_IP[@]} Mi Box trouvée(s)."
}

saisir_ip() {
    local prec="" texte rep i items=() intro=""
    if [ -f "$APPSUP/ip.txt" ]; then prec=$(head -1 "$APPSUP/ip.txt" | tr -d ' \r'); fi
    ip_valide "$prec" || prec=""
    chercher_box
    if [ "${#F_IP[@]}" -eq 1 ]; then
        rep=$(osa btn "Mi Box trouvée" "J'ai trouvé votre Mi Box « ${F_NOM[0]} » à l'adresse ${F_IP[0]}.

Est-ce bien la vôtre ?" "Oui" "Autre adresse" "Oui") || arreter
        if [ "$rep" = Oui ]; then IP=${F_IP[0]}; PROFIL=mibox4; TV_HOST=$IP; export PROFIL TV_HOST; charger_conf; return 0; fi
    elif [ "${#F_IP[@]}" -gt 1 ]; then
        for ((i = 0; i < ${#F_IP[@]}; i++)); do items+=("${F_NOM[i]} (${F_IP[i]})"); done
        items+=("Aucune : je tape l'adresse moi-même")
        rep=$(osa list "Mi Box trouvées" "J'ai trouvé plusieurs Mi Box sur ce réseau. Laquelle voulez-vous configurer ?" 0 "${items[@]}") || arreter
        for ((i = 0; i < ${#F_IP[@]}; i++)); do
            if [ "$rep" = "${items[i]}" ]; then IP=${F_IP[i]}; PROFIL=mibox4; TV_HOST=$IP; export PROFIL TV_HOST; charger_conf; return 0; fi
        done
    else
        intro="Je n'ai pas trouvé de Mi Box allumée sur ce réseau. Vérifiez qu'elle est allumée (pas en veille) et sur le même Wi-Fi que ce Mac, ou indiquez son adresse.

"
    fi
    texte="${intro}Quelle est l'adresse IP de votre box ?

Sur la box : Paramètres > Préférences de l'appareil > À propos > État > Adresse IP.
Elle ressemble à 192.168.1.23.

Le « débogage USB » doit être activé sur la box (Paramètres > Préférences de l'appareil > Options pour les développeurs). Pour faire apparaître ces options : À propos, puis cliquez 7 fois sur « Build »."
    while :; do
        IP=$(osa txt "Adresse de la box" "$texte" "$prec") || arreter
        IP=$(printf '%s' "$IP" | tr -d '[:space:]')
        ip_valide "$IP" && break
        prec=""; texte="« $(printf '%s' "$IP" | tr -cd '0-9A-Za-z.:-' | cut -c1-40) » n'est pas une adresse valide. Elle doit avoir 4 nombres de 0 à 255 séparés par des points, par exemple 192.168.1.23.

Sur la box : Paramètres > Préférences de l'appareil > À propos > État > Adresse IP."
    done
    PROFIL=mibox4; TV_HOST=$IP; export PROFIL TV_HOST
    charger_conf
}

connecter() {
    local essai co etat nom
    etape "Connexion à la box ($SERIAL)"
    adb disconnect "$SERIAL" >/dev/null 2>&1   # repart d'un lien neuf : un ancien lien peut rester « offline »
    for essai in 1 2 3 4 5; do
        co=$(adb connect "$SERIAL" 2>&1 | tr -d '\r')
        etat=$(adb -s "$SERIAL" get-state 2>&1 | tr -d '\r')
        [ "$etat" = device ] && break
        case "$co $etat" in
            *unauthorized*|*authenticate*|*offline*)
                rep=$(osa btn "Autorisation à accepter" "Un message s'affiche sur l'écran de la TV : acceptez-le, en cochant « Toujours autoriser cet ordinateur ».

Puis cliquez sur « Réessayer »." Réessayer Annuler Réessayer) || arreter
                [ "$rep" = Réessayer ] || arreter ;;
            *) info "La box ne répond pas encore (essai $essai/5)…"; sleep "$PAUSE" ;;
        esac
    done
    [ "$etat" = device ] || erreur "La box ne répond pas. Vérifiez que :
• le débogage USB est activé (Paramètres > Préférences de l'appareil > Options pour les développeurs) ;
• l'adresse IP $IP est la bonne ;
• la box est allumée (pas en veille) et sur le même réseau Wi-Fi que ce Mac ;
• vous avez accepté l'autorisation affichée sur la TV."
    nom=$(adb_lire shell getprop ro.product.name)
    if [ "$nom" != "$PRODUIT" ]; then
        erreur "Cet appareil n'est pas une Xiaomi Mi Box 4 (modèle détecté : « $(printf '%s' "$nom" | tr -cd 'A-Za-z0-9._ -' | cut -c1-40) »).

Ce kit n'a été vérifié que sur la Mi Box 4 : par sécurité il s'arrête ici, sans rien modifier."
    fi
    printf '%s\n' "$IP" > "$APPSUP/ip.txt"
    info "Box connectée : $SERIAL"
}

# ---------------------------------------------------------------- 4. état des lieux et sauvegarde (lecture seule)

creer_sauvegarde() {
    local serie
    serie=$(adb_lire shell getprop ro.serialno | tr -cd 'A-Za-z0-9._-' | cut -c1-40)
    SAUV="$HOME/Documents/Zen Home/sauvegardes/$(date +%Y%m%d-%H%M%S)-${serie:-inconnu}"
    mkdir -p "$SAUV" || erreur "Impossible de créer le dossier $SAUV."
    RAPPORT="$SAUV/rapport.txt"
    printf 'Zen Home, kit %s : %s\n' "$KIT_VERSION" "$(date '+%d/%m/%Y %H:%M')" > "$RAPPORT"
}

lire_etat() {
    local g e gl=() essai
    for essai in 1 2 3; do   # une lecture vide = lien coupé : reconnexion puis nouvel essai
        assurer_connexion
        TOUS=$(adb_lire shell pm list packages)
        DESACTIVES=$(adb_lire shell pm list packages -d)
        USERS=$(adb_lire shell pm list packages -3)
        grep -Fq 'package:' <<<"$TOUS" && break
        adb disconnect "$SERIAL" >/dev/null 2>&1
    done
    grep -Fq 'package:' <<<"$TOUS" || erreur "Impossible de lire la liste des applis de la box. Redémarrez la box puis relancez le kit."
    GARDER_ACTUEL=","
    g=$(adb_lire shell settings get global zenhome_garder)
    IFS=, read -ra gl <<<"$g" || true
    for e in ${gl[@]+"${gl[@]}"}; do if paquet_valide "$e"; then GARDER_ACTUEL="$GARDER_ACTUEL$e,"; fi; done
}

# « paquet<TAB>octets » depuis dumpsys diskstats (tableaux JSON alignés ; octets = appli + données + cache), puis taille lisible.
tailles() {
    awk '
    function liste(s, a) { sub(/^[^[]*\[/, "", s); sub(/\].*$/, "", s); gsub(/"/, "", s); return split(s, a, ",") }
    function hum(b) {
        if (b >= 1073741824) { s = sprintf("%.1f Go", b / 1073741824); sub(/\./, ",", s); return s }
        if (b >= 1048576) return sprintf("%d Mo", b / 1048576 + 0.5)
        return "moins de 1 Mo"
    }
    /Package Names:/ { np = liste($0, P) }
    /App Sizes:/ { liste($0, A) }
    /App Data Sizes:/ { liste($0, D) }
    /Cache Sizes:/ { liste($0, C) }
    END { for (i = 1; i <= np; i++) printf "%s\t%s\n", P[i], hum(A[i] + D[i] + C[i]) }' "$1" 2>/dev/null || true
}

# « paquet<TAB>JJ/MM/AAAA » : dernière utilisation (section « In-memory yearly stats » de dumpsys usagestats).
utilisations() {
    awk '
    /In-memory yearly stats/ { on = 1; next }
    on && /In-memory [a-z]+ stats/ { on = 0 }
    on && /package=/ && /lastTime="/ {
        p = $0; sub(/.*package=/, "", p); sub(/[ \t].*$/, "", p)
        d = $0; sub(/.*lastTime="/, "", d); d = substr(d, 1, 10)
        if (d ~ /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$/ && d > "1971" && d > M[p]) M[p] = d
    }
    END { for (p in M) printf "%s\t%s/%s/%s\n", p, substr(M[p], 9, 2), substr(M[p], 6, 2), substr(M[p], 1, 4) }' "$1" 2>/dev/null || true
}

sauver() { local f=$1; shift; adb_lire shell "$@" > "$SAUV/$f.txt"; }

inventaire() {
    local tai der l p t d
    etape "État des lieux et sauvegarde (rien n'est modifié)"
    assurer_connexion
    sauver paquets-tous pm list packages -f
    printf '%s\n' "$DESACTIVES" > "$SAUV/paquets-desactives.txt"
    printf '%s\n' "$USERS" > "$SAUV/paquets-utilisateur.txt"
    sauver paquets-systeme pm list packages -s
    sauver reglages-global settings list global
    sauver reglages-secure settings list secure
    sauver reglages-system settings list system
    sauver proprietes getprop
    sauver diskstats dumpsys diskstats
    sauver usagestats dumpsys usagestats
    sauver accueil cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.HOME
    grep -q '^package:' "$SAUV/paquets-tous.txt" 2>/dev/null || erreur "La sauvegarde de la box a échoué ($SAUV). Rien n'a été modifié ; relancez le kit."
    tai=$(tailles "$SAUV/diskstats.txt"); der=$(utilisations "$SAUV/usagestats.txt")
    rm -f "$SAUV/usagestats.txt"   # historique d'usage détaillé : seule la date de dernière utilisation est gardée (resume.txt)
    APPS=""
    while IFS= read -r l; do
        [[ $l == package:* ]] || continue
        p=${l#package:}
        paquet_valide "$p" || continue
        protege "$p" && continue
        t=$(awk -F'\t' -v p="$p" '$1 == p { print $2; exit }' <<<"$tai")
        d=$(awk -F'\t' -v p="$p" '$1 == p { print $2; exit }' <<<"$der")
        APPS="$APPS$p"$'\t'"${t:-inconnue}"$'\t'"${d:-inconnue}"$'\n'
    done <<<"$USERS"
    {
        printf 'État des lieux de la box, le %s\n' "$(date '+%d/%m/%Y %H:%M')"
        printf 'Modèle : %s (%s), Android %s\n' "$(adb_lire shell getprop ro.product.model)" "$PRODUIT" "$(adb_lire shell getprop ro.build.version.release)"
        printf 'Applis installées : %s, dont désactivées : %s\n' "$(grep -c '^package:' <<<"$TOUS" || true)" "$(grep -c '^package:' <<<"$DESACTIVES" || true)"
        printf 'Accueil actuel : %s\n' "$(tail -1 "$SAUV/accueil.txt" 2>/dev/null | tr -cd 'A-Za-z0-9._/$ -')"
        printf '\nApplis installées par vous (paquet : taille, dernière utilisation) :\n'
        while IFS=$'\t' read -r p t d; do [ -n "$p" ] && printf '  %s : %s, %s\n' "$p" "$t" "$d"; done <<<"$APPS"
    } > "$SAUV/resume.txt"
    info "Sauvegarde écrite dans : $SAUV"
}

# ---------------------------------------------------------------- 5. installation

charger_liste() {
    local l p c tag
    while IFS= read -r l || [ -n "$l" ]; do
        case "$l" in *'#'*) ;; *) continue ;; esac
        p=${l%%#*}; p=${p//[[:space:]]/}
        [ -n "$p" ] || continue
        [[ $p =~ $RE_PAQUET ]] || continue
        c=${l#*#}; c=${c# }; tag=""
        case "$c" in
            "[option:garder]"*) tag=garder; c=${c#"[option:garder]"} ;;
            "[option]"*) tag=option; c=${c#"[option]"} ;;
        esac
        c=${c# }
        D_PKG+=("$p"); D_TAG+=("$tag"); D_LAB+=("${c%% : *}")
        case "$c" in *' : '*) c=${c#* : }; D_EXP+=("$(printf '%s' "${c:0:1}" | LC_ALL=C tr '[:lower:]' '[:upper:]')${c:1}") ;; *) D_EXP+=("") ;; esac   # 1re lettre en majuscule
    done < "$TV_DIR/$DEBLOAT"
    [ "${#D_PKG[@]}" -gt 0 ] || erreur "La liste des applis à alléger est vide ou illisible ($DEBLOAT)."
}

choisir_meteo() {
    local ville json nom num='^-?[0-9]+(\.[0-9]+)?$'
    LAT=""; LON=""; CITY=""
    ville=$(osa txt "Météo (facultatif)" "Dans quelle ville habitez-vous ? Zen Home affichera sa météo.

Le nom de la ville est envoyé au service gratuit Open-Meteo pour trouver sa position.
Laissez vide pour ne pas avoir de météo." "") || arreter
    ville=$(printf '%s' "$ville" | LC_ALL=C tr -d '\000-\037\177')
    [ -n "${ville//[[:space:]]/}" ] || return 0
    json=$(curl --fail --silent --proto '=https' --proto-redir '=https' --max-time 15 -G --data-urlencode "name=$ville" \
        --data 'count=1&language=fr&format=json' https://geocoding-api.open-meteo.com/v1/search 2>/dev/null) || json=""
    LAT=$(grep -oE '"latitude":-?[0-9.]+' <<<"$json" | head -1 | cut -d: -f2)
    LON=$(grep -oE '"longitude":-?[0-9.]+' <<<"$json" | head -1 | cut -d: -f2)
    if ! [[ $LAT =~ $num && $LON =~ $num ]]; then
        LAT=""; LON=""
        fenetre "Météo" "Je n'ai pas trouvé cette ville (ou le service est injoignable). L'installation continue sans météo ; vous pourrez la régler plus tard."
        return 0
    fi
    nom=$(grep -o '"name":"[^"]*"' <<<"$json" | head -1 | cut -d'"' -f4)
    CITY=${nom:-$ville}
    case "$CITY" in *\\*) CITY="" ;; esac
    [ "${#CITY}" -le 60 ] || CITY=""
    info "Météo : ${CITY:-ville sans nom} ($LAT, $LON)"
}

choisir_jeton() {
    local tok re='^[A-Za-z0-9._~+/=-]+$'
    tok=$(osa pwd "Films (facultatif)" "Pour afficher des films et des bandes-annonces, Zen Home utilise le service gratuit TMDB. Il faut un « jeton » personnel :

1. Créez un compte gratuit sur https://www.themoviedb.org
2. Ouvrez Paramètres > API
3. Copiez le « API Read Access Token » (un long texte)
4. Collez-le ci-dessous (il reste masqué)

Vous pouvez laisser vide : tout fonctionnera, sauf les films.") || arreter
    tok=$(printf '%s' "$tok" | tr -d ' \t\r\n')
    [ -n "$tok" ] || return 0
    if ! [[ $tok =~ $re ]]; then
        fenetre "Films" "Ce texte ne ressemble pas à un jeton TMDB. L'installation continue sans les films."
        return 0
    fi
    TOKFILE=$(mktemp "${TMPDIR:-/tmp}/zenhome-jeton.XXXXXX") || erreur "Impossible de créer un fichier temporaire."
    printf '%s' "$tok" > "$TOKFILE"
}

# ---------------------------------------------------------------- coupures de la box (veille, Wi-Fi)

connexion_ok() { [ "$(adb -s "$SERIAL" get-state 2>/dev/null | tr -d '\r')" = device ]; }

# Reconnecte la box si elle a décroché ; sinon demande à l'utilisateur de la réveiller (Réessayer / Arrêter).
assurer_connexion() {
    local essai rep
    while :; do
        connexion_ok && return 0
        for essai in 1 2 3 4 5; do
            info "Connexion à la box perdue, reconnexion (essai $essai/5)…"
            adb disconnect "$SERIAL" >/dev/null 2>&1; adb connect "$SERIAL" >/dev/null 2>&1
            sleep "$PAUSE"
            if connexion_ok; then info "Box reconnectée."; return 0; fi
        done
        rep=$(osa btn "La box ne répond plus" "La connexion avec la Mi Box ($IP) a été perdue.

Vérifiez qu'elle est allumée et pas en veille (appuyez sur une touche de la télécommande), et toujours sur le même Wi-Fi que ce Mac. Puis cliquez sur « Réessayer ».

Ce qui a déjà été fait est conservé : le kit reprend là où il s'est arrêté." Réessayer Arrêter Réessayer) || arreter
        [ "$rep" = Réessayer ] || arreter
    done
}

# Lance un script du kit (rejouable sans risque) ; s'il échoue parce que la box a décroché, reconnecte et recommence.
lancer_script() {
    local n rc=1 out
    out=$(mktemp "${TMPDIR:-/tmp}/zenhome-sortie.XXXXXX") || out=/dev/null
    for n in 1 2 3; do
        assurer_connexion
        /bin/bash "$@" 2>&1 | tee -a "$RAPPORT" "$out"; rc=${PIPESTATUS[0]}
        [ "$rc" -eq 0 ] && break
        # vraie erreur, sauf si la box a décroché (même brièvement : messages de coupure d'adb dans la sortie)
        if connexion_ok && ! grep -Eq 'device offline|device .* not found|ne répond pas|error: closed|no devices|Connection refused|No route to host' "$out" 2>/dev/null; then break; fi
        info "La box a décroché pendant l'opération : nouvelle tentative."
        [ "$out" = /dev/null ] || : > "$out"
    done
    [ "$out" = /dev/null ] || rm -f "$out"
    return "$rc"
}

installer() {
    etape "Installation de Zen Home"
    lancer_script "$ROOT/tv/deployer.sh" --sans-build || erreur "L'installation de Zen Home a échoué (détails dans la fenêtre Terminal et dans $RAPPORT). Rien n'a été désactivé ni désinstallé."
}

# ---------------------------------------------------------------- 6. noms lisibles

lire_libelles() {
    local essai sortie data l pkg
    L_PKG=(); L_LAB=()
    etape "Noms lisibles des applis"
    assurer_connexion
    for essai in 1 2 3; do
        sortie=$(adb_lire shell am broadcast -a tv.zenhome.CONFIG -n tv.zenhome/.ConfigReceiver --ez labels true)
        case "$sortie" in *'Broadcast completed: result=1, data="'*) break ;; esac
        sortie=""; sleep "$PAUSE"
    done
    if [ -z "$sortie" ]; then info "Noms indisponibles : les noms techniques seront affichés."; return 0; fi
    data=${sortie#*'Broadcast completed: result=1, data="'}
    data=${data%\"*}
    while IFS= read -r l || [ -n "$l" ]; do
        case "$l" in *$'\t'*$'\t'*) continue ;; *$'\t'*) ;; *) continue ;; esac   # exactement 2 champs
        pkg=${l%%$'\t'*}
        paquet_valide "$pkg" || continue
        L_PKG+=("$pkg"); L_LAB+=("${l#*$'\t'}")
    done <<<"$data"
    info "${#L_PKG[@]} noms reçus."
}

# Libellé nettoyé (contrôles, bidi, guillemets, 60 caractères) ; le nom du paquet si vide ou si le nettoyage échoue.
libelle_de() {
    local i brut="" out=""
    for ((i = 0; i < ${#L_PKG[@]}; i++)); do [ "${L_PKG[i]}" = "$1" ] && { brut=${L_LAB[i]}; break; }; done
    if [ -n "$brut" ]; then
        out=$(printf '%s' "$brut" | perl -CS -pe 's/[\p{Cc}\p{Cf}\p{Zl}\p{Zp}"]//g; s/^\s+|\s+$//g; $_ = substr($_, 0, 60); s/\s+$//' 2>/dev/null) || out=""
    fi
    printf '%s' "${out:-$1}"
}

# ---------------------------------------------------------------- 7. choix

defaut_option() {   # paquet, balise -> Garder | Désactiver
    if dans_csv "$GARDER_ACTUEL" "$1"; then echo Garder
    elif dans_liste "$DESACTIVES" "$1"; then echo Désactiver
    elif [ "$2" = garder ]; then echo Garder
    else echo Désactiver; fi
}

choisir_applis() {   # applis installées par l'utilisateur ; rien de coché d'avance
    local p t d l lab sel i conf rep
    U_PKG=(); U_ITEM=(); UN_PKG=(); UN_ITEM=()
    while IFS=$'\t' read -r p t d; do
        [ -n "$p" ] || continue
        idx_debloat "$p" && continue
        lab=$(libelle_de "$p")
        U_PKG+=("$p"); U_ITEM+=("$lab — $t, dernière utilisation : $d ($p)")
    done <<<"$APPS"
    [ "${#U_PKG[@]}" -gt 0 ] || return 0
    while :; do
        sel=$(osa list "Vos applis" "Voici les applis que vous avez installées vous-même, avec leur taille et leur dernière utilisation.

Cochez CELLES QUE VOUS VOULEZ SUPPRIMER (rien n'est coché d'avance). Pour tout garder, ne cochez rien et cliquez sur Continuer." 1 "${U_ITEM[@]}") || arreter
        UN_PKG=(); UN_ITEM=(); conf=""
        while IFS= read -r l; do
            for ((i = 0; i < ${#U_ITEM[@]}; i++)); do
                if [ "${U_ITEM[i]}" = "$l" ]; then UN_PKG+=("${U_PKG[i]}"); UN_ITEM+=("${U_ITEM[i]}"); conf="${conf}• ${U_ITEM[i]}"$'\n'; fi
            done
        done <<<"$sel"
        [ "${#UN_PKG[@]}" -gt 0 ] || return 0
        rep=$(osa btn "Supprimer ces applis ?" "Vous allez DÉSINSTALLER :

$conf
C'est IRRÉVERSIBLE : leurs données (comptes, réglages, historique) seront perdues. Vous pourrez réinstaller ces applis, pas retrouver ces données." Annuler Annuler Désinstaller) || arreter
        [ "$rep" = Désinstaller ] && return 0
    done
}

# Applis système de la liste : une fenêtre (recommandé / choisir / arrêter), puis une liste à cocher avec descriptif.
# En réparation, pas de question : on garde ce qui est noté dans zenhome_garder, on désactive le reste.
choisir_systeme() {
    local i p def rep sel l court tout="▶ Tout sélectionner (tout désactiver)" items=() pk=() defs=() reco_d="" reco_g=""
    OPT_G=(); OPT_D=()
    for ((i = 0; i < ${#D_PKG[@]}; i++)); do
        p=${D_PKG[i]}
        dans_liste "$TOUS" "$p" || continue
        if [ "$MODE" = reparer ]; then
            if dans_csv "$GARDER_ACTUEL" "$p"; then OPT_G+=("$p"); else OPT_D+=("$p"); fi
            continue
        fi
        def=$(defaut_option "$p" "${D_TAG[i]}")
        court=${D_EXP[i]%%. *}; court=${court%.}; court=${court:0:90}
        pk+=("$p"); items+=("${D_LAB[i]}${court:+ — $court}")
        if [ "$def" = Désactiver ]; then defs+=("${items[${#items[@]}-1]}"); reco_d="${reco_d}• ${D_LAB[i]}"$'\n'
        else reco_g="${reco_g}• ${D_LAB[i]}"$'\n'; fi
    done
    [ "$MODE" = reparer ] && return 0
    [ "${#pk[@]}" -gt 0 ] || return 0
    rep=$(osa btn "Désactiver les applis inutiles" "La box contient ${#pk[@]} applis qui ne servent à rien avec Zen Home (publicité, pistage, rapports envoyés à Xiaomi ou Google) ou dont vous n'avez peut-être pas besoin. Elles sont DÉSACTIVÉES, pas supprimées : « Tout annuler » les remet.

Choix recommandés, à désactiver :
${reco_d:-• aucune
}
À garder :
${reco_g:-• aucune
}
« Choisir moi-même » affiche la liste complète avec un descriptif de chaque appli." "Choix recommandés" "Tout arrêter" "Choisir moi-même" "Choix recommandés") || arreter
    case "$rep" in
        "Choix recommandés") sel=$(printf '%s\n' ${defs[@]+"${defs[@]}"}) ;;
        "Choisir moi-même")
            sel=$(osa cases "Applis à désactiver" "Cochez les applis à DÉSACTIVER, décochez celles à garder. Les choix recommandés sont déjà cochés ; « Tout sélectionner » les désactive toutes.
(Cmd + clic pour cocher ou décocher une ligne.)" "${#defs[@]}" ${defs[@]+"${defs[@]}"} "$tout" "${items[@]}") || arreter ;;
        *) arreter ;;
    esac
    if grep -Fqx -- "$tout" <<<"$sel"; then OPT_D=(${pk[@]+"${pk[@]}"}); return 0; fi
    for ((i = 0; i < ${#pk[@]}; i++)); do
        if grep -Fqx -- "${items[i]}" <<<"$sel"; then OPT_D+=("${pk[i]}"); else OPT_G+=("${pk[i]}"); fi
    done
}

choisir() {
    local i rep
    choisir_systeme
    if [ "$MODE" = installer ]; then
        choisir_applis
        rep=$(osa btn "Récapitulatif" "Voici ce qui va être fait sur votre box :

Applis système DÉSACTIVÉES (réversible) :
$(puces ${OPT_D[@]+"${OPT_D[@]}"})
Applis système GARDÉES :
$(puces ${OPT_G[@]+"${OPT_G[@]}"})
Applis DÉSINSTALLÉES (définitif) :
$(for i in ${UN_ITEM[@]+"${!UN_ITEM[@]}"}; do printf '• %s\n' "${UN_ITEM[i]}"; done)
Une sauvegarde de l'état actuel est déjà enregistrée dans Documents/Zen Home." Appliquer Annuler Appliquer) || arreter
        [ "$rep" = Appliquer ] || arreter
    fi
}

# ---------------------------------------------------------------- 8. application

appliquer() {
    local p g="" rel sortie essai
    etape "Application de vos choix"
    MODIFIE=1
    for p in ${OPT_G[@]+"${OPT_G[@]}"}; do
        if paquet_valide "$p" && idx_debloat "$p"; then g="${g:+$g,}$p"; fi
    done
    for essai in 1 2 3; do   # écriture puis relecture : recommencée si la box a décroché entre les deux
        assurer_connexion
        if [ -z "$g" ]; then adb_tv shell settings delete global zenhome_garder
        else adb_tv shell settings put global zenhome_garder "$g"; fi
        rel=$(adb_lire shell settings get global zenhome_garder)
        if [ "$rel" = null ]; then rel=""; fi
        [ "$rel" = "$g" ] && break
        connexion_ok && break
    done
    [ "$rel" = "$g" ] || erreur "Vos choix n'ont pas pu être mémorisés sur la box. Rien d'autre n'a été modifié ; relancez le kit."
    for p in ${OPT_G[@]+"${OPT_G[@]}"}; do
        if dans_liste "$DESACTIVES" "$p"; then info "Réactivation de $(lab_debloat "$p")"; assurer_connexion; adb_tv shell pm enable "$p"; fi
    done
    DESINSTALLES=""; ECHECS=""
    for p in ${UN_PKG[@]+"${UN_PKG[@]}"}; do
        protege "$p" && continue
        paquet_valide "$p" && dans_liste "$USERS" "$p" || continue
        assurer_connexion
        sortie=$(adb_tv uninstall "$p" 2>&1 | tr -d '\r')
        case "$sortie" in
            *Success*) DESINSTALLES="${DESINSTALLES}• $p"$'\n'; info "Désinstallée : $p" ;;
            *) ECHECS="${ECHECS}• $p"$'\n'; warn "échec de la désinstallation de $p" ;;
        esac
    done
}

verifier() {
    etape "Vérification et réglage final"
    if lancer_script "$ROOT/tv/verifier-tv.sh" --fix; then OK_VERIF=1; else OK_VERIF=0; fi
}

# ---------------------------------------------------------------- 9. fin

terminer() {
    local etat resume rep
    if [ "$OK_VERIF" = 1 ]; then etat="Vérification finale : tout est en ordre."
    else etat="Vérification finale : quelques points sont à regarder (voir le Terminal et le rapport)."; fi
    resume="Zen Home est prêt.

Applis système désactivées : ${#OPT_D[@]}
Applis système gardées : ${#OPT_G[@]}
Applis désinstallées : $(printf '%s' "${DESINSTALLES:-}" | grep -c . || true)
$etat

Rapport et sauvegarde : Documents/Zen Home/sauvegardes"
    printf '\n%s\n' "$resume" | tee -a "$RAPPORT"
    rep=$(osa btn "Terminé" "$resume

Pour votre sécurité, vous pouvez couper le « débogage USB » de la box : sinon n'importe quel appareil de votre réseau Wi-Fi pourrait la piloter. Il faudra le réactiver pour utiliser « Réparer »." "Laisser activé" "Couper le débogage" "Laisser activé") || rep="Laisser activé"
    if [ "$rep" = "Couper le débogage" ]; then assurer_connexion; adb_tv shell settings put global adb_enabled 0; info "Débogage coupé."; fi
}

# ---------------------------------------------------------------- déroulé

printf 'Zen Home, kit %s\nNe fermez pas cette fenêtre : elle montre ce qui se passe.\n' "$KIT_VERSION"

rep=$(osa btn "Zen Home, kit $KIT_VERSION" "Bienvenue ! Avant de commencer :

1. Allumez la TV et la Mi Box, et laissez-les allumées pendant toute l'installation (environ 10 minutes).
2. La Mi Box et ce Mac doivent être sur le même réseau Wi-Fi.
3. Le « débogage USB » doit être activé sur la box : Paramètres > Préférences de l'appareil > Options pour les développeurs. Si ce menu n'existe pas : Préférences de l'appareil > À propos, puis cliquez 7 fois sur « Build » (voir le mode d'emploi LISEZMOI).
4. Gardez la télécommande à portée de main : une demande d'autorisation va s'afficher sur la TV, il faudra l'accepter.

macOS peut aussi demander si « Terminal » peut trouver des appareils sur votre réseau local : acceptez, c'est ce qui permet de retrouver la box." "C'est prêt" Quitter "C'est prêt") || arreter
[ "$rep" = "C'est prêt" ] || arreter

choix=$(osa list "Zen Home, kit $KIT_VERSION" "Que voulez-vous faire ?" 0 \
    "Installer ou modifier mes choix" "Réparer (après une mise à jour de la box)" "Tout annuler") || arreter
case "$choix" in
    Installer*) MODE=installer ;;
    Réparer*) MODE=reparer ;;
    "Tout annuler") MODE=restaurer ;;
    *) arreter ;;
esac

etape "Outil adb"
mkdir -p "$APPSUP" || erreur "Impossible de créer le dossier $APPSUP."
trouver_adb
saisir_ip
connecter
creer_sauvegarde

case "$MODE" in
    installer)
        [ -f "$APK" ] || erreur "Le kit est incomplet (launcher/out/ZenHome.apk introuvable). Décompressez l'archive en entier."
        charger_liste
        lire_etat
        inventaire
        choisir_meteo
        choisir_jeton
        export LAT LON CITY TMDB_TOKEN_FILE=$TOKFILE
        installer
        if [ -n "$TOKFILE" ]; then rm -f "$TOKFILE"; TOKFILE=""; fi   # jeton envoyé : plus besoin du fichier
        lire_libelles
        choisir
        appliquer
        verifier
        terminer ;;
    reparer)
        charger_liste
        lire_etat
        choisir
        appliquer
        verifier
        terminer ;;
    restaurer)
        rep=$(osa btn "Tout annuler ?" "Cela va :
• réactiver toutes les applis désactivées ;
• remettre l'accueil Xiaomi/Google d'origine ;
• désinstaller Zen Home.

Les applis que vous avez désinstallées ne reviennent pas." Annuler "Tout annuler" Annuler) || arreter
        [ "$rep" = "Tout annuler" ] || arreter
        etape "Retour à l'état d'origine"
        lancer_script "$ROOT/tv/restaurer-tv.sh"; rc=$?
        assurer_connexion; adb_tv shell settings delete global zenhome_garder
        [ "$rc" -eq 0 ] || erreur "La restauration a rencontré un problème (voir le Terminal et $RAPPORT)."
        fenetre "Terminé" "Votre box est revenue à son état d'origine." ;;
esac
