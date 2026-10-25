#!/bin/bash
# Compile ZenHome.apk sur macOS sans Android Studio.
# Prérequis : brew install openjdk@17 && brew install --cask android-commandlinetools
#   sdkmanager "platforms;android-34" "build-tools;34.0.0"
# La clé de signature (key.jks) et son mot de passe (keystore.pass) sont créés au premier build
# et ne sont jamais versionnés (voir .gitignore).
set -e
cd "$(dirname "$0")"
export JAVA_HOME="${JAVA_HOME:-/opt/homebrew/opt/openjdk@17}"
export PATH="$JAVA_HOME/bin:$PATH"
SDK_ROOT="${ANDROID_SDK_ROOT:-/opt/homebrew/share/android-commandlinetools}"
BT="$SDK_ROOT/build-tools/34.0.0"
JAR="$SDK_ROOT/platforms/android-34/android.jar"
[ -f "$JAR" ] && [ -x "$BT/aapt" ] || { echo "SDK manquant : sdkmanager \"platforms;android-34\" \"build-tools;34.0.0\""; exit 1; }

rm -rf out && mkdir -p out/classes
javac --release 8 -nowarn -encoding UTF-8 -cp "$JAR" -d out/classes $(find src -name '*.java')
"$BT/d8" --release --min-api 21 --lib "$JAR" --output out $(find out/classes -name '*.class')
"$BT/aapt" package -f -M AndroidManifest.xml -S res -A assets -I "$JAR" -F out/unsigned.apk
(cd out && "$BT/aapt" add unsigned.apk classes.dex >/dev/null)
"$BT/zipalign" -f 4 out/unsigned.apk out/aligned.apk

if [ ! -f key.jks ]; then
    umask 077
    [ -f keystore.pass ] || openssl rand -hex 24 > keystore.pass
    keytool -genkeypair -keystore key.jks -storepass "$(cat keystore.pass)" -keypass "$(cat keystore.pass)" \
        -alias k -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=ZenHome" >/dev/null
fi
"$BT/apksigner" sign --ks key.jks --ks-pass file:keystore.pass --out out/ZenHome.apk out/aligned.apk
echo "OK : $(pwd)/out/ZenHome.apk"
