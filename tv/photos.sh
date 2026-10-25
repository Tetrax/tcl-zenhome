#!/bin/bash
# Envoie vos photos sur la TV (fonds de Zen Home) : redimensionnées en JPEG 1920 px max.
set -euo pipefail
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

aide() {
    command cat <<'AIDE'
Usage : tv/photos.sh [--replace] [--dry-run] <dossier>
        tv/photos.sh --reset [--dry-run]
        tv/photos.sh --help

Redimensionne (sips, sur le Mac) les .jpg/.jpeg/.png/.heic du dossier en JPEG 1920 px de large
maximum, qualité 85, les envoie dans /sdcard/ZenHome/, accorde la lecture des images puis relance l'accueil.

  --replace   vide /sdcard/ZenHome avant l'envoi (sinon les photos sont ajoutées)
  --reset     supprime /sdcard/ZenHome : retour aux photos fournies avec Zen Home
  --dry-run   fait la conversion dans un dossier temporaire mais n'exécute aucune commande adb
  -h, --help  cette aide

Le dossier n'est pas parcouru récursivement. Les fichiers cachés sont ignorés.
AIDE
}

REPLACE=0; RESET=0; DIR=""
for a in "$@"; do
    case "$a" in
        --replace) REPLACE=1 ;;
        --reset) RESET=1 ;;
        --dry-run) DRY_RUN=1 ;;
        -h|--help) aide; exit 0 ;;
        -*) aide >&2; die "option inconnue : $a" ;;
        *) [ -z "$DIR" ] || die "un seul dossier attendu."; DIR=$a ;;
    esac
done
charger_conf
verifier_adb

DEST=/sdcard/ZenHome
redemarrer_accueil() {
    adb_tv shell am force-stop "$PKG"
    touche_accueil
}

if [ "$RESET" = 1 ]; then
    [ -z "$DIR" ] || die "--reset n'accepte pas de dossier."
    connecter_tv
    adb_tv shell rm -rf "$DEST"
    redemarrer_accueil
    info "Photos personnelles supprimées : retour aux photos fournies."
    exit 0
fi

[ -n "$DIR" ] || { aide >&2; die "dossier manquant."; }
[ -d "$DIR" ] || die "dossier introuvable : $DIR"
command -v sips >/dev/null || die "sips introuvable (macOS requis)."

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

n=0
while IFS= read -r -d '' f; do
    base=$(basename "$f"); stem=${base%.*}
    out="$TMP/$stem.jpg"; i=2
    while [ -e "$out" ]; do out="$TMP/$stem-$i.jpg"; i=$((i + 1)); done   # IMG.jpg et IMG.png dans le même dossier
    larg=$(sips -g pixelWidth "$f" 2>/dev/null | awk '/pixelWidth/{print $2}')
    redim=()
    [ "${larg:-0}" -gt 1920 ] && redim=(--resampleWidth 1920)
    if sips -s format jpeg -s formatOptions 85 ${redim[@]+"${redim[@]}"} "$f" --out "$out" >/dev/null 2>&1; then
        n=$((n + 1))
    else
        warn "ignoré (conversion impossible) : $base"
    fi
done < <(find "$DIR" -maxdepth 1 -type f ! -name '.*' \( -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.png' -o -iname '*.heic' \) -print0)

[ "$n" -gt 0 ] || die "aucune image .jpg/.jpeg/.png/.heic exploitable dans $DIR"
info "$n photo(s) prête(s) ($(du -sh "$TMP" | awk '{print $1}'))."

connecter_tv
[ "$REPLACE" = 1 ] && adb_tv shell rm -rf "$DEST"
adb_tv shell mkdir -p "$DEST"
adb_tv push "$TMP/." "$DEST/"
accorder_permissions
redemarrer_accueil
info "Terminé."
