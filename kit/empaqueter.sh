#!/bin/bash
# Produit l'archive du kit Mac à publier en Release GitHub : kit/dist/ZenHome-MiBox4-<version>.zip
# Usage : kit/empaqueter.sh <version>   (doit être égale à KIT_VERSION de « Zen Home.command »)
# Ne publie rien : la commande gh release create est affichée à la fin.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$(pwd)

die() { printf 'Erreur : %s\n' "$*" >&2; exit 1; }

[ $# -eq 1 ] || die "usage : kit/empaqueter.sh <version>  (ex. 1.0)"
VERSION=$1
[[ $VERSION =~ ^[0-9]+(\.[0-9]+)*$ ]] || die "version invalide : « $VERSION » (attendu : 1.0)."
[ -z "$(git status --porcelain)" ] || die "l'arbre git est modifié (git status --porcelain non vide). Commitez ou annulez vos changements avant d'empaqueter."
KIT_VERSION=$(sed -n 's/^KIT_VERSION=//p' "kit/Zen Home.command" | head -1)
[ "$VERSION" = "$KIT_VERSION" ] || die "version demandée ($VERSION) différente de KIT_VERSION dans « Zen Home.command » ($KIT_VERSION)."

echo "== Tests"
launcher/test-mac.sh
echo "== Build"
launcher/build-mac.sh

echo "== Certificat de signature"
export JAVA_HOME="${JAVA_HOME:-/opt/homebrew/opt/openjdk@17}"
export PATH="$JAVA_HOME/bin:$PATH"
APKSIGNER="${ANDROID_SDK_ROOT:-/opt/homebrew/share/android-commandlinetools}/build-tools/34.0.0/apksigner"
[ -x "$APKSIGNER" ] || die "apksigner introuvable ($APKSIGNER)."
APK=launcher/out/ZenHome.apk
ACTUEL=$("$APKSIGNER" verify --print-certs "$APK" | sed -n 's/.*certificate SHA-256 digest: *//p' | head -1 | tr -d '[:space:]')
ATTENDU=$(head -1 kit/certificat-attendu.txt | tr -d '[:space:]')
[ -n "$ACTUEL" ] && [ -n "$ATTENDU" ] || die "empreinte du certificat introuvable (APK ou kit/certificat-attendu.txt)."
if [ "$ACTUEL" != "$ATTENDU" ]; then
    die "le certificat de l'APK ($ACTUEL) n'est pas celui attendu ($ATTENDU).
La clé launcher/key.jks a changé : Android refuserait les mises à jour de Zen Home chez vos amis (INSTALL_FAILED_UPDATE_INCOMPATIBLE).
Retrouvez la clé d'origine (launcher/key.jks + launcher/keystore.pass). Ne modifiez kit/certificat-attendu.txt que si vous changez volontairement de clé."
fi
echo "Certificat conforme : $ACTUEL"

echo "== Assemblage"
NOM="ZenHome-MiBox4-$VERSION"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/zenhome-kit.XXXXXX")
trap 'rm -rf "$TMP"' EXIT
DEST="$TMP/$NOM"
FICHIERS=("Zen Home.command" LISEZMOI.html
    tv/lib.sh tv/deployer.sh tv/verifier-tv.sh tv/restaurer-tv.sh tv/debloat-mibox4.txt
    launcher/out/ZenHome.apk LICENSE launcher/LICENSE launcher/CREDITS.md
    launcher/assets/Poppins-OFL.txt launcher/assets/Phosphor-LICENSE.txt)   # les 2 derniers sont cités par CREDITS.md
for f in "${FICHIERS[@]}"; do
    case "$f" in "Zen Home.command"|LISEZMOI.html) src="kit/$f" ;; *) src=$f ;; esac
    [ -f "$src" ] || die "fichier manquant : $src"
    mkdir -p "$DEST/$(dirname "$f")"
    cp "$src" "$DEST/$f"
done

echo "== Contrôles"
INTERDITS=$(find "$DEST" \( -name tv.conf -o -name tmdb.token -o -name '*.jks' -o -name keystore.pass -o -name .DS_Store -o -name 'sauvegarde-tv*' \) | head -5)
[ -z "$INTERDITS" ] || die "fichier interdit dans le kit : $INTERDITS"
ATTENDUS=$(printf '%s\n' "${FICHIERS[@]}" | sort)
PRESENTS=$(cd "$DEST" && find . -type f | sed 's|^\./||' | sort)
[ "$ATTENDUS" = "$PRESENTS" ] || die "le contenu du dossier diffère de la liste blanche."
for f in "Zen Home.command" tv/deployer.sh tv/verifier-tv.sh tv/restaurer-tv.sh; do
    [ -x "$DEST/$f" ] || die "$f n'est pas exécutable."
done
for f in "Zen Home.command" tv/lib.sh tv/deployer.sh tv/verifier-tv.sh tv/restaurer-tv.sh; do bash -n "$DEST/$f" || die "erreur de syntaxe : $f"; done

echo "== Archive"
mkdir -p kit/dist
ZIP="$ROOT/kit/dist/$NOM.zip"
rm -f "$ZIP"
ditto -c -k --keepParent --norsrc --noextattr --noacl "$DEST" "$ZIP"
SHA=$(shasum -a 256 "$ZIP" | awk '{print $1}')
unzip -l "$ZIP"
echo "SHA-256 : $SHA  $ZIP"

command cat <<FIN

Publication (manuelle), à lancer quand vous êtes prêt :
  git tag kit-v$VERSION && git push origin kit-v$VERSION
  gh release create kit-v$VERSION "$ZIP" --title "Kit Mac Mi Box 4 $VERSION" --notes "SHA-256 : $SHA"
FIN
