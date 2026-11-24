#!/usr/bin/env bash
# Compila el ayudante del celular (android/: filtro web y vigía) y deja sus piezas en assets/filtro/:
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
VERSION_CODE=19
if command -v javac >/dev/null; then JAVAC=(javac); else JAVAC=(java -m jdk.compiler/com.sun.tools.javac.Main); fi

rm -rf "$OUT" && mkdir -p "$OUT"/{classes,dex} "$DEST"
"$BT/aapt2" compile --dir "$HERE/res" -o "$OUT/res.zip"
"$BT/aapt2" link -o "$OUT/base.apk" -I "$JAR" --manifest "$HERE/AndroidManifest.xml" "$OUT/res.zip" \
  --min-sdk-version 24 --target-sdk-version 33 \
  --version-code $VERSION_CODE --version-name "filtro $VERSION_CODE"
# Con arreglos, no $(find …): la ruta puede tener espacios ("SEMESTRE VIII").
mapfile -d '' SOURCES < <(find "$HERE/src" -name '*.java' -print0)
"${JAVAC[@]}" -source 11 -target 11 -nowarn -Xlint:-options -encoding UTF-8 \
  -classpath "$JAR" -d "$OUT/classes" "${SOURCES[@]}"
mapfile -d '' CLASSES < <(find "$OUT/classes" -name '*.class' -print0)
"$BT/d8" --release --min-api 24 --lib "$JAR" --output "$OUT/dex" "${CLASSES[@]}"
(cd "$OUT" && unzip -oq base.apk AndroidManifest.xml resources.arsc 'res/*')
cp "$OUT/AndroidManifest.xml" "$OUT/resources.arsc" "$OUT/dex/classes.dex" "$DEST/"
# Los recursos sueltos (res/xml/vigia.xml…): build.rs los mete todos en el APK.
rm -rf "$DEST/res" && cp -r "$OUT/res" "$DEST/res"
ls -lR "$DEST"
echo "Recuerda: FILTER_VERSION en src/apk.rs debe ser $VERSION_CODE."
