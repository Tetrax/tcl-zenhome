#!/usr/bin/env bash
# Construit, signe et prépare la release de l'appli Mac dans dist-release/ (rien n'est publié).
set -euo pipefail
cd "$(dirname "$0")"

CLE="${CLE:-$HOME/.tauri/zenhome-updater.key}"
DEPOT="Tetrax/tcl-zenhome"
[ -f "$CLE" ] || { echo "Clé de mise à jour absente : $CLE (à générer avec « npm run tauri signer generate -- -w $CLE »)." >&2; exit 1; }
# Le mot de passe (variable locale, jamais exportée) n'est transmis qu'à la commande de signature.
# MDP_SIGNATURE (même vide) ne sert qu'aux essais à blanc, avec SANS_BUILD=1 et BUNDLE/OUT pointant ailleurs.
MDP="${MDP_SIGNATURE-$(security find-generic-password -a zenhome -s zenhome-updater -w 2>/dev/null)}" \
  || { echo "Mot de passe de la clé absent du trousseau (compte zenhome, service zenhome-updater)." >&2; exit 1; }

VERSION="$(sed -n 's/^  "version": "\(.*\)",/\1/p' src-tauri/tauri.conf.json)"
[ -n "$VERSION" ] || { echo "Version introuvable dans tauri.conf.json." >&2; exit 1; }

BUNDLE="${BUNDLE:-src-tauri/target/universal-apple-darwin/release/bundle}"
APP="$BUNDLE/macos/Zen Home.app"
if [ -z "${SANS_BUILD:-}" ]; then
  # Aucun secret dans l'environnement du build (npm, vite, build.rs, proc-macros) : l'archive de mise à jour
  # est donc créée et signée plus bas, par nos soins.
  npm run tauri build -- --target universal-apple-darwin --bundles app,dmg --config '{"bundle":{"createUpdaterArtifacts":false}}'
  codesign --verify --deep --strict "$APP"
  # Sortie lue en entier : avec pipefail, « grep -q » couperait codesign (SIGPIPE) et ferait échouer le contrôle.
  grep -q '^Authority=Zen Home Dev$' <<< "$(codesign -dvv "$APP" 2>&1)" \
    || { echo "L'appli n'est pas signée par « Zen Home Dev »." >&2; exit 1; }
fi

OUT="${OUT:-dist-release}"
rm -rf "$OUT" && mkdir "$OUT"
TAR="Zen-Home-Mac_${VERSION}.app.tar.gz"
cp "$BUNDLE/dmg/Zen Home_${VERSION}_universal.dmg" "$OUT/Zen-Home-Mac.dmg"
cp "$OUT/Zen-Home-Mac.dmg" "$OUT/Zen-Home-Mac_${VERSION}.dmg"
# Format attendu par tauri-plugin-updater sur macOS : « Zen Home.app » à la racine de l'archive (COPYFILE_DISABLE : pas de fichiers ._).
COPYFILE_DISABLE=1 tar -C "$BUNDLE/macos" -czf "$OUT/$TAR" "Zen Home.app"
# --app-version inscrit la version dans le « trusted comment » signé (exigé par requireSignedVersion).
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$MDP" npx tauri signer sign -f "$CLE" --app-version "$VERSION" "$OUT/$TAR" >/dev/null
unset MDP MDP_SIGNATURE
[ -s "$OUT/$TAR.sig" ] || { echo "Signature de l'archive absente." >&2; exit 1; }

URL="https://github.com/$DEPOT/releases/download/app-v${VERSION}/$TAR"
SIG="$(tr -d '\n' < "$OUT/$TAR.sig")"
NOTES="${NOTES:-Zen Home $VERSION}"
printf '%s\n' "$NOTES" > "$OUT/notes.txt"
jq -n --arg v "$VERSION" --arg n "$NOTES" --arg d "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --arg s "$SIG" --arg u "$URL" \
  '{version: $v, notes: $n, pub_date: $d, platforms: {"darwin-aarch64": {signature: $s, url: $u}, "darwin-x86_64": {signature: $s, url: $u}}}' \
  > "$OUT/latest.json"
jq -e --arg v "$VERSION" '.version == $v and (.platforms | all(.[]; .signature != "" and (.url | startswith("https://"))))' "$OUT/latest.json" >/dev/null \
  || { echo "latest.json invalide." >&2; exit 1; }

echo "SHA-256 du dmg : $(shasum -a 256 "$OUT/Zen-Home-Mac.dmg" | cut -d' ' -f1)"
echo "Pour publier (à lancer vous-même) :"
# --latest : la release kit-v1.0 partage le dépôt, « latest » (donc l'URL de latest.json) doit rester l'appli.
echo "  gh release create app-v$VERSION $OUT/{*.dmg,*.tar.gz,*.sig,latest.json} --repo $DEPOT --title \"Zen Home $VERSION\" --notes-file $OUT/notes.txt --latest"
