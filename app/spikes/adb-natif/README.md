# adb-natif : essai d'un client adb natif (étape 3b)

Petit binaire pour évaluer la crate `adb_client` 3.2.3 face à une Mi Box 4 (Android 9, adbd TCP 5555),
sans le binaire adb de Google. Aucune commande n'est lancée automatiquement : tout est manuel.

## Compilation

    . "$HOME/.cargo/env"
    cd app/spikes/adb-natif
    cargo build --release
    cargo clippy -- -D warnings

## Commandes (remplacer 192.168.1.50 par l'IP de la box)

    B=target/release/adb-natif
    $B connecter --ip 192.168.1.50
    $B shell "getprop ro.build.version.release" --ip 192.168.1.50
    $B installer ../../../launcher/out/ZenHome.apk --ip 192.168.1.50            # équivaut à adb install -r
    $B installer ../../../launcher/out/ZenHome.apk --sans-r --ip 192.168.1.50   # chemin natif de la crate (sans -r)
    $B veille-test --ip 192.168.1.50                                             # 10 min, un getprop toutes les 10 s

Par défaut la clé est `~/.android/adbkey`. Option `--cle FICHIER` pour une clé dédiée (PEM PKCS#8, `BEGIN PRIVATE KEY`) ;
la TV demandera alors une autorisation (à accepter dans les 10 s). Le fichier doit exister (sinon la crate
génère une clé en mémoire, jamais sauvegardée, et la TV redemanderait à chaque fois). Exemple :

    openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out /chemin/adbkey && chmod 600 /chemin/adbkey

Codes de sortie : 0 ok, 1 échec, 2 mauvais arguments. Délai de 15 s par opération (60 s pour `installer`).

## Limites connues (voir l'analyse dans le compte rendu)

- Pas de code de retour ni de stderr séparé (shell v1 uniquement).
- Si la box ferme la connexion proprement, la crate boucle à 100 % CPU dans un thread (constaté en local) :
  le programme abandonne l'attente au délai et se termine avec `exit`.
