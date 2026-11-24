<p align="center">
  <img src="docs/banner.png" alt="Cerrojo: los reels, bajo llave" />
</p>

<p align="center"><strong>Español</strong> · <a href="README.en.md">English</a></p>

<p align="center">
  <a href="https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-x86_64.AppImage"><img src="https://img.shields.io/badge/Linux-AppImage-00C3E3?logo=linux&logoColor=white" alt="Linux" /></a>
  <a href="https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-windows-x86_64.zip"><img src="https://img.shields.io/badge/Windows-zip-00C3E3?logo=windows&logoColor=white" alt="Windows" /></a>
  <a href="https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-macos-arm64.zip"><img src="https://img.shields.io/badge/macOS-Apple%20Silicon-00C3E3?logo=apple&logoColor=white" alt="macOS" /></a>
  <a href="https://github.com/sthifen/cerrojo/releases/latest"><img src="https://img.shields.io/github/v/release/sthifen/cerrojo?label=versi%C3%B3n&color=1d2027" alt="Versión" /></a>
  <img src="https://img.shields.io/badge/licencia-MIT-1d2027" alt="MIT" />
</p>

---

## Cinco minutos de reels que se vuelven dos horas

Abres el celular para ver un mensaje y, sin saber cómo, llevas una hora deslizando videos verticales. Las
apps de "bienestar digital" no sirven: el límite se quita con un toque, la app se reinstala en diez segundos y
tu fuerza de voluntad a las dos de la mañana no es la misma que al mediodía.

**Cerrojo cambia las reglas.** Conectas el celular al PC, pulsas un botón y TikTok, YouTube Shorts, Instagram
Reels y compañía desaparecen. Lo importante es que **no puedes volver a instalarlos desde el celular**: ni desde
la Play Store, ni desde otra tienda, ni con un APK descargado. La llave se queda en tu PC.

<p align="center"><img src="docs/todo.png" width="760" alt="Cerrojo con todas las apps bloqueadas" /></p>

## Lo que hace

- 🔒 **Bloqueo de verdad, sin root.** No es un temporizador que se salta: Android simplemente no deja instalar la app.
- 🛍️ **La Play Store sigue funcionando** para todo lo demás. Bloqueas los reels, no el teléfono.
- 💻 **Solo el PC desbloquea.** En el celular no hay ningún ajuste, contraseña ni botón que lo deshaga.
- 🔁 **Se repara solo.** Si quitas un candado desde el celular, Cerrojo lo vuelve a poner en cuanto lo conectas al PC.
- 🎯 **Un toque, todo listo.** YouTube, YouTube Music, TikTok (y Lite), Instagram (y Lite), Threads, Facebook (y Lite),
  Snapchat, Pinterest, Kwai, Likee, Snaptube y los YouTube parcheados (ReVanced, Morphe).
- 🌐 **También sus páginas web.** Al bloquear Instagram, TikTok, Facebook, Threads… sus sitios dejan de abrir en
  cualquier navegador del celular. WhatsApp, LinkedIn y YouTube web siguen funcionando. Se apaga y se enciende con
  su propio interruptor, *Páginas web*.
- ✂️ **O solo sin su video vertical.** YouTube, Instagram, WhatsApp y LinkedIn se pueden dejar usables, pero sin
  lo que engancha. Cada app elige: *bloqueada*, *sin vertical* o *libre*. En YouTube e Instagram el reproductor de
  Shorts/Reels se cierra en cuanto aparece y su pestaña de abajo queda tapada con un 🧠. Los bloques de Shorts del
  feed se quitan con «No me interesa», y con la casilla *Quitar también los directos*, los directos también.
- 💬 **Sin Estados de WhatsApp ni feed de LinkedIn.** Estas dos nunca se bloquean enteras. En WhatsApp los chats
  siguen igual, pero un Estado queda tapado y se cierra en cuanto se abre, y la pestaña *Novedades* queda tapada. En
  LinkedIn el feed de *Inicio* y la pestaña *Video* quedan tapados; buscador, mensajes, red, notificaciones y
  empleos siguen abiertos.
- ⏱️ **Abrir un rato.** «Abrir… 20 min» desde el PC: pasado el tiempo se vuelve a cerrar sola, aunque desconectes
  el cable.
- 🛡️ **Protección máxima (opcional).** El celular se cuida solo, sin el PC: no deja desinstalar los candados ni
  Cerrojo, ni apagar la VPN, y si instalas una app bloqueada la suspende al instante.
- ➕ **Cualquier otra app** se añade escribiendo su nombre de paquete.
- 📱 **Varios equipos**: el celular, la tablet, el de tu hermano… cada uno recuerda su lista.
- 🪶 **Nada que hacer en el celular.** Todo va por `adb`, la herramienta oficial de Android: sin cuentas, sin
  diálogos, sin apps espía.
- 🆓 Libre, gratis y sin cuentas, anuncios ni telemetría.

| Cuando aún hay reels instalados | Desbloquear cuesta un poco, a propósito |
|---|---|
| ![Apps de reels instaladas, candado abierto en rojo](docs/abiertas.png) | ![Confirmación antes de desbloquear](docs/confirmar.png) |

## Descargar

| Sistema | Descarga | Notas |
|---|---|---|
| **Linux** x86_64 | [Cerrojo-x86_64.AppImage](https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-x86_64.AppImage) | Cualquier distro reciente. Dale permiso de ejecución y ábrela. |
| **Linux** ARM64 | [Cerrojo-aarch64.AppImage](https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-aarch64.AppImage) | Raspberry Pi 4/5 con sistema de 64 bits, portátiles ARM. |
| **Windows** 10/11 | [Cerrojo-windows-x86_64.zip](https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-windows-x86_64.zip) | Portátil: descomprime y abre `Cerrojo.exe`. |
| **macOS** 12+ (Apple Silicon) | [Cerrojo-macos-arm64.zip](https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-macos-arm64.zip) | La primera vez: clic derecho › **Abrir** (no está firmada por Apple). |

**Instalación en Linux con una línea** (la añade al menú de aplicaciones):

```sh
curl -fsSL https://raw.githubusercontent.com/sthifen/cerrojo/main/packaging/install.sh | sh
```

> Windows y macOS se compilan automáticamente pero se han probado menos que Linux. Si algo falla,
> [abre un issue](https://github.com/sthifen/cerrojo/issues) y lo arreglamos.

## Tutorial: preparar el celular y adb (una sola vez)

Cerrojo habla con el celular mediante **adb** (Android Debug Bridge), la herramienta oficial de Google para
desarrolladores. Hay que hacer dos cosas: activar la depuración USB en el celular e instalar adb en el PC.

### 1. Activa las opciones de desarrollador en el celular

1. Abre **Ajustes › Acerca del teléfono**.
2. Toca **7 veces** seguidas **Número de compilación** hasta que diga *«Ahora eres desarrollador»*.
   - **Samsung**: Ajustes › Acerca del teléfono › **Información de software** › Número de compilación.
   - **Xiaomi / Redmi / POCO**: Ajustes › Sobre el teléfono › **Versión de HyperOS** (o MIUI).
   - **Motorola, Pixel, Nokia**: Ajustes › Acerca del teléfono › Número de compilación.
   - **Huawei / Honor**: Ajustes › Acerca del teléfono › Número de compilación.
3. Si te pide el PIN, escríbelo.

### 2. Activa la depuración USB

1. Ve a **Ajustes › Sistema › Opciones de desarrollador** (en Samsung y Xiaomi aparecen al final de Ajustes).
2. Activa **Depuración USB**.
3. **Xiaomi**: activa también **Depuración USB (ajustes de seguridad)**; si no, Cerrojo no podrá instalar los candados.

### 3. Instala adb en el PC

| Sistema | Cómo |
|---|---|
| **Windows** | En PowerShell: `winget install Google.PlatformTools`. Con un **Samsung**, instala también el [driver USB de Samsung](https://developer.samsung.com/android-usb-driver). |
| **macOS** | Con [Homebrew](https://brew.sh): `brew install android-platform-tools` |
| **Fedora** | `sudo dnf install android-tools` |
| **Ubuntu / Debian / Mint** | `sudo apt install adb` |
| **Arch / Manjaro** | `sudo pacman -S android-tools` |

Si prefieres no usar la terminal, descarga
[platform-tools](https://developer.android.com/tools/releases/platform-tools) de Google y descomprímelo **junto
a Cerrojo** (en Windows, al lado de `Cerrojo.exe`): Cerrojo lo encuentra solo.

### 4. Conecta y acepta

1. Conecta el celular con un cable USB que transmita datos (algunos solo cargan).
2. En el celular sale **«¿Permitir depuración USB?»**: marca **Permitir siempre desde este equipo** y toca **Permitir**.
3. Abre Cerrojo: verás el nombre de tu celular arriba a la derecha.

<p align="center"><img src="docs/tutorial.png" width="640" alt="Cerrojo esperando el celular, con los pasos en pantalla" /></p>

Cerrojo te muestra los mismos pasos en pantalla mientras no detecta el celular.

## Cómo funciona

Android no deja instalar una app si ya hay **otra con el mismo nombre de paquete y una firma distinta**. Cerrojo
aprovecha esa regla:

1. **Desinstala la app** (por ejemplo, `com.zhiliaoapp.musically`, TikTok).
2. **Instala en su lugar un candado**: una app vacía, sin icono ni código, con ese mismo nombre de paquete,
   firmada con una clave que se genera en tu PC y nunca sale de él.
3. **La suspende** con `cmd package suspend`, que solo se puede revertir por adb.

Para Android, TikTok "ya está instalada", así que la Play Store solo ofrece *Desinstalar*, y cualquier APK de
TikTok falla por firma distinta. Las apps que vienen de fábrica (como YouTube en casi todos los Android) no se
pueden reemplazar, así que se dejan instaladas pero **suspendidas**: al abrirlas, Android dice que están en pausa.

**Las páginas web** las bloquea el *filtro*, una mini app (`android/`) sin icono que Cerrojo instala por adb: una
VPN local que **solo ve el DNS**. Cuando el navegador pregunta por `instagram.com`, responde "no existe"; el resto
de preguntas las reenvía al DNS de tu red, y el tráfico normal no pasa por ella. adb le da el permiso de VPN
(`appops`) sin el diálogo de confirmación y la deja como VPN siempre activa, así que vuelve sola al reiniciar.

**Sin vertical** lo hace el *vigía*, un servicio de accesibilidad de esa misma mini app. Solo mira la pantalla de
las apps que elegiste: si se abre el reproductor de Shorts o de Reels, pulsa *Atrás*; si ve su pestaña en la barra
de abajo, le pone encima un recuadro con un 🧠 que no se deja tocar. adb lo enciende escribiendo
`enabled_accessibility_services`, sin pasar por Ajustes, y la app tiene permiso para volver a encenderlo si lo
apagas en *Ajustes › Accesibilidad*. Además queda como *administrador del dispositivo*, así que no se puede forzar
su detención ni desinstalarla sin quitar eso primero. En este modo las páginas web de la app no se bloquean,
porque la app las necesita.

**Protección máxima** hace a esa mini app *propietaria del equipo* (Device Owner), lo mismo que usan las empresas
con los celulares de trabajo:

- Bloquea la desinstalación de los candados y de sí misma, así que la Play Store ya no puede quitarlos.
- Prohíbe tocar la VPN y el modo seguro.
- Suspende al momento cualquier app bloqueada que aparezca.
- Lleva el reloj de los desbloqueos temporales: usa el reloj interno, así que cambiar la hora no alarga nada, y
  un reinicio los cierra.

Android solo deja ponerla en un celular **sin cuentas**: hay que quitar las de Google y Samsung (y la Carpeta
Segura), pulsar *Protección máxima* y volver a añadirlas. Se quita desde Cerrojo, o a mano con
`adb shell dpm remove-active-admin dev.cerrojo.filtro/.Admin`. No bloquea el restablecimiento de fábrica.

Los candados y el filtro se generan dentro de Cerrojo, sin necesitar el SDK de Android: el manifiesto, el zip y la firma
APK v2 están hechos en Rust (`src/apk.rs`).

## Preguntas

**¿Necesita root?** No. Todo se hace con los permisos normales de adb.

**¿Puedo saltármelo desde el celular?** Hay una sola forma: en la ficha de la app en la Play Store, tocar
*Desinstalar* (quita el candado) y luego instalar la app real. Cuesta varios pasos conscientes, que es justo lo que
corta el impulso, y **Cerrojo vuelve a poner el candado la próxima vez que conectes el celular al PC**. Con la
*protección máxima* ese camino se cierra: la tienda no puede desinstalar el candado. Siempre queda el
restablecimiento de fábrica; Cerrojo busca que saltárselo sea incómodo, no imposible.

**¿El «sin vertical» funciona con Instagram, Facebook…?** Trae YouTube, Instagram, WhatsApp y LinkedIn. Facebook
aún no: se bloquea entera.

**¿Y desde el navegador?** También: el filtro web bloquea los sitios de las apps que tengas bloqueadas (y los DNS
cifrados que servirían para saltárselo). Se puede quitar desde *Ajustes › VPN* o desinstalándolo, pero Cerrojo lo
vuelve a poner al conectar el celular. YouTube web se deja a propósito; los Shorts dentro de youtube.com no se
pueden separar por DNS.

**¿El modo «sin vertical» es igual de fuerte?** No. Se puede apagar en *Ajustes › Accesibilidad* (el guardián lo
vuelve a encender, y si no, Cerrojo al conectar el celular), y si YouTube o Instagram cambian su diseño por dentro puede dejar de reconocer los
Shorts hasta que se actualice Cerrojo. Algunos celulares avisan de vez en cuando de que un servicio "puede ver
la pantalla": es el vigía, que no guarda ni envía nada. Si los Shorts te pueden, usa *Toda la app*.

**¿Hace más lento el internet?** Solo pasa por el filtro la pregunta de "¿qué IP tiene este sitio?", no las
descargas ni los videos. En Samsung, la primera vez que se instala puede salir Play Protect: toca *Instalar de
todos modos*.

**¿Pierdo mis datos?** Al bloquear una app se desinstala, así que su contenido local se borra (tus cuentas siguen
en los servidores de cada red social). Al desbloquearla, se instala de nuevo desde la Play Store.

**¿Qué pasa si restablezco el celular?** Se va todo, candados incluidos. Conéctalo a Cerrojo y pulsa *Bloquear todo*.

**¿Puedo usarlo desde la terminal?**

```sh
cerrojo estado                      # qué está bloqueado
cerrojo bloquear com.zhiliaoapp.musically
cerrojo desbloquear com.pinterest
cerrojo web                         # pone o quita el filtro web según lo bloqueado
cerrojo web apagar                  # lo desactiva en este equipo (encender lo vuelve a poner)
cerrojo modo youtube vertical       # YouTube sí, Shorts no (bloqueada | vertical | libre)
cerrojo modo whatsapp vertical      # WhatsApp sin Estados (linkedin: sin feed)
cerrojo solo instagram si           # lo mismo que «modo instagram vertical» (no: bloqueada)
cerrojo abrir youtube 20            # 20 minutos y se vuelve a cerrar sola
cerrojo cerrar                      # acaba ya los desbloqueos temporales
cerrojo dueño                       # protección máxima (cerrojo dueño quitar la quita)
cerrojo guardia                     # estado de lo anterior
cerrojo vistas                      # vistas de la pantalla de ahora (para depurar el vigía)
cerrojo bloquear PAQUETE -s SERIAL  # con varios equipos conectados
```

## Compilar

```sh
# Fedora: sudo dnf install gtk4-devel   ·   Ubuntu: sudo apt install libgtk-4-dev
cargo build --release
./target/release/cerrojo
```

Rust estable y GTK 4.6 o más nuevo. El ayudante del celular (filtro web y vigía) va compilado en `assets/filtro/`;
solo si cambias su Java o sus recursos hace falta el SDK de Android (`android/build.sh`). Las versiones se publican solas con GitHub Actions al subir una etiqueta `vX.Y.Z`.

## Licencia

[MIT](LICENSE). Úsalo, compártelo y mejóralo.

<p align="center"><sub>Hecho por alguien a quien los reels también le estaban ganando.</sub></p>
