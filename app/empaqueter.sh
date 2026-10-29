#!/usr/bin/env bash
# Construit, signe et prépare la release de l'appli Mac dans dist-release/ (rien n'est publié).
set -euo pipefail
cd "$(dirname "$0")"

CLE="$HOME/.tauri/zenhome-updater.key"
DEPOT="Tetrax/tcl-zenhome"
[ -f "$CLE" ] || { echo "Clé de mise à jour absente : $CLE (à générer avec « npm run tauri signer generate -- -w $CLE »)." >&2; exit 1; }
MDP="$(security find-generic-password -a zenhome -s zenhome-updater -w 2>/dev/null)" \
  || { echo "Mot de passe de la clé absent du trousseau (compte zenhome, service zenhome-updater)." >&2; exit 1; }
export TAURI_SIGNING_PRIVATE_KEY="$CLE"   # chemin ou contenu, les deux sont acceptés
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$MDP"

VERSION="$(sed -n 's/^  "version": "\(.*\)",/\1/p' src-tauri/tauri.conf.json)"
[ -n "$VERSION" ] || { echo "Version introuvable dans tauri.conf.json." >&2; exit 1; }

npm run tauri build -- --target universal-apple-darwin --bundles app,dmg

BUNDLE="src-tauri/target/universal-apple-darwin/release/bundle"
APP="$BUNDLE/macos/Zen Home.app"
codesign --verify --deep --strict "$APP"
codesign -dvv "$APP" 2>&1 | grep -q '^Authority=Zen Home Dev$' \
  || { echo "L'appli n'est pas signée par « Zen Home Dev »." >&2; exit 1; }

OUT=dist-release
rm -rf "$OUT" && mkdir "$OUT"
TAR="Zen-Home-Mac_${VERSION}.app.tar.gz"
cp "$BUNDLE/dmg/Zen Home_${VERSION}_universal.dmg" "$OUT/Zen-Home-Mac.dmg"
cp "$OUT/Zen-Home-Mac.dmg" "$OUT/Zen-Home-Mac_${VERSION}.dmg"
cp "$BUNDLE/macos/Zen Home.app.tar.gz" "$OUT/$TAR"
cp "$BUNDLE/macos/Zen Home.app.tar.gz.sig" "$OUT/$TAR.sig"

URL="https://github.com/$DEPOT/releases/download/app-v${VERSION}/$TAR"
SIG="$(tr -d '\n' < "$OUT/$TAR.sig")"
NOTES="${NOTES:-Zen Home $VERSION}"
cat > "$OUT/latest.json" <<JSON
{
  "version": "$VERSION",
  "notes": "$NOTES",
  "pub_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "platforms": {
    "darwin-aarch64": { "signature": "$SIG", "url": "$URL" },
    "darwin-x86_64": { "signature": "$SIG", "url": "$URL" }
  }
}
JSON

echo "SHA-256 du dmg : $(shasum -a 256 "$OUT/Zen-Home-Mac.dmg" | cut -d' ' -f1)"
echo "Pour publier (à lancer vous-même) :"
echo "  gh release create app-v$VERSION $OUT/* --repo $DEPOT --title \"Zen Home $VERSION\" --notes \"$NOTES\""
