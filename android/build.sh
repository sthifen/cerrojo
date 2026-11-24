#!/usr/bin/env bash
# Compila el filtro web (android/) y deja sus piezas en assets/filtro/:
# Cerrojo las empaqueta y firma él mismo (src/apk.rs), como los candados.
# Hace falta el Android SDK (build-tools 36.1.0 y android-36) solo para
# quien cambie el código Java; para compilar Cerrojo no.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
SDK="${ANDROID_HOME:-$HOME/Android/Sdk}"
BT="$SDK/build-tools/36.1.0"
JAR="$SDK/platforms/android-36/android.jar"
OUT="$HERE/build"
DEST="$HERE/../assets/filtro"
# Sube el código al cambiar el Java, para que el PC reinstale el filtro.
VERSION_CODE=2
if command -v javac >/dev/null; then JAVAC=(javac); else JAVAC=(java -m jdk.compiler/com.sun.tools.javac.Main); fi

rm -rf "$OUT" "$DEST/res" && mkdir -p "$OUT"/{classes,dex,flat} "$DEST/res/xml"
"$BT/aapt2" compile --dir "$HERE/res" -o "$OUT/flat"
"$BT/aapt2" link -o "$OUT/base.apk" -I "$JAR" --manifest "$HERE/AndroidManifest.xml" "$OUT"/flat/*.flat \
  --min-sdk-version 24 --target-sdk-version 33 \
  --version-code $VERSION_CODE --version-name "filtro $VERSION_CODE"
"${JAVAC[@]}" -source 11 -target 11 -nowarn -Xlint:-options -encoding UTF-8 \
  -classpath "$JAR" -d "$OUT/classes" $(find "$HERE/src" -name '*.java')
"$BT/d8" --release --min-api 24 --lib "$JAR" --output "$OUT/dex" $(find "$OUT/classes" -name '*.class')
(cd "$OUT" && unzip -oq base.apk AndroidManifest.xml resources.arsc 'res/xml/*')
cp "$OUT/AndroidManifest.xml" "$OUT/resources.arsc" "$OUT/dex/classes.dex" "$DEST/"
cp "$OUT"/res/xml/*.xml "$DEST/res/xml/"
ls -lR "$DEST"
