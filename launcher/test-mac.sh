#!/bin/bash
# Compile SANS android.jar les classes pures + les tests, puis les lance (code de sortie 1 en cas d'echec).
# Une classe « pure » qui importe android.* fait echouer la compilation.
set -e
cd "$(dirname "$0")"
export JAVA_HOME="${JAVA_HOME:-/opt/homebrew/opt/openjdk@17}"
export PATH="$JAVA_HOME/bin:$PATH"

PURE="Net Fmt Wmo Greeting DockMetrics Movie MovieRules ConfigRules Nav InputRules SleepRules WeatherRules KeyRules GaugeRules StandbyRules IconRules ThemeRules Attribution"

SRC=""
for c in $PURE; do SRC="$SRC src/tv/zenhome/$c.java"; done
rm -rf out/test && mkdir -p out/test
javac --release 8 -nowarn -encoding UTF-8 -d out/test $SRC $(find test -name '*.java')
java -Dfile.encoding=UTF-8 -cp out/test tv.zenhome.Tests
