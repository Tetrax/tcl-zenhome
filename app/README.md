# Zen Home (appli de bureau)

Appli Tauri v2 (Rust + Vite/Svelte 5/TypeScript) qui remplace le kit bash pour installer Zen Home sur la Mi Box 4.

## Prérequis

- Rust via [rustup](https://rustup.rs) (`. "$HOME/.cargo/env"` si le PATH n'est pas à jour)
- Node 22+ et npm
- macOS : Xcode (outils en ligne de commande)

## Commandes

```sh
npm install                          # une fois
npm run tauri dev                    # appli en développement
npm run dev                          # front seul dans un navigateur (données de démo)
npm run build                        # front seul (dist/)
npm run tauri build -- --bundles app # .app dans src-tauri/target/release/bundle/macos/
npm run tauri build -- --bundles dmg # .dmg
cd src-tauri && cargo clippy -- -D warnings && cargo test
```

## Sécurité

L'interface n'a accès à aucun plugin Tauri : uniquement les commandes Rust déclarées dans `src-tauri/build.rs`
et autorisées dans `src-tauri/capabilities/default.json`. Tout le réseau passe par Rust.
