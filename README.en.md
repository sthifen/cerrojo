<p align="center">
  <img src="docs/banner.png" alt="Cerrojo" />
</p>

<p align="center"><a href="README.md">Español</a> · <strong>English</strong></p>

<p align="center">
  <a href="https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-x86_64.AppImage"><img src="https://img.shields.io/badge/Linux-AppImage-00C3E3?logo=linux&logoColor=white" alt="Linux" /></a>
  <a href="https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-windows-x86_64.zip"><img src="https://img.shields.io/badge/Windows-zip-00C3E3?logo=windows&logoColor=white" alt="Windows" /></a>
  <a href="https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-macos-arm64.zip"><img src="https://img.shields.io/badge/macOS-Apple%20Silicon-00C3E3?logo=apple&logoColor=white" alt="macOS" /></a>
</p>

## Short-form video, locked away

Plug your Android phone into your computer, press one button, and TikTok, YouTube Shorts, Instagram Reels and
friends are gone. **And you can't reinstall them from the phone**: not from the Play Store, not from another store,
not from a downloaded APK. The key stays on your computer.

<p align="center"><img src="docs/todo.png" width="760" alt="Cerrojo with every app locked" /></p>

- 🔒 **Real blocking, no root.** Android simply refuses to install the app.
- 🛍️ **The Play Store keeps working** for everything else.
- 💻 **Only your computer can unlock.** There is no setting, password or button on the phone.
- 🔁 **Self-repairing.** Remove a lock from the phone and Cerrojo puts it back the next time you connect.
- 🎯 YouTube, YouTube Music, TikTok, Instagram, Threads, Facebook, Snapchat, Pinterest, Kwai, Likee, Snaptube and
  patched YouTube clients (ReVanced, Morphe), plus any package you add.
- 🪶 Nothing installed on the phone, no accounts, no ads, no telemetry. MIT licensed.

## Download

| System | File |
|---|---|
| Linux x86_64 | [Cerrojo-x86_64.AppImage](https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-x86_64.AppImage) |
| Linux ARM64 | [Cerrojo-aarch64.AppImage](https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-aarch64.AppImage) |
| Windows 10/11 | [Cerrojo-windows-x86_64.zip](https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-windows-x86_64.zip) (portable) |
| macOS 12+ Apple Silicon | [Cerrojo-macos-arm64.zip](https://github.com/sthifen/cerrojo/releases/latest/download/Cerrojo-macos-arm64.zip) (right-click › Open the first time) |

The interface is in Spanish.

## Setup (once)

1. **Developer options**: Settings › About phone › tap **Build number** 7 times (Samsung: inside *Software information*).
2. **USB debugging**: Settings › System › Developer options › enable **USB debugging** (Xiaomi: also *USB debugging (Security settings)*).
3. **Install adb**: Windows `winget install Google.PlatformTools` (plus the [Samsung USB driver](https://developer.samsung.com/android-usb-driver) for Samsung phones) ·
   macOS `brew install android-platform-tools` · Fedora `sudo dnf install android-tools` · Ubuntu `sudo apt install adb` · Arch `sudo pacman -S android-tools`.
4. Plug in the phone, accept **Allow USB debugging?** (tick *Always allow*), and open Cerrojo.

## How it works

Android won't install an app if another one with the **same package name and a different signature** is already
installed. Cerrojo uninstalls the app, installs an empty placeholder with that package name (signed with a key
generated on your computer) and suspends it with `cmd package suspend`, which can only be undone over adb.
Preinstalled apps such as YouTube can't be replaced, so they are suspended instead. Placeholders are built inside
Cerrojo (manifest, zip and APK Signature Scheme v2 in Rust), so no Android SDK is required.

The one way around it from the phone is to tap *Uninstall* on the app's Play Store page and then install the real
app. That takes several deliberate steps, and Cerrojo restores the lock the next time you connect. Websites are
blocked by a small DNS-only local VPN that Cerrojo installs over adb.

Each app can also be set to **no vertical video**: the app stays usable, but an accessibility service backs out
as soon as its short-form player shows up. It ships with YouTube Shorts; for other apps, run `cerrojo vistas`
on that screen and add the view ids to `~/.config/cerrojo/vertical.json`. **Open for a while** (`cerrojo abrir
youtube 20`) relocks by itself even with the cable unplugged. **Maximum protection** makes the companion app the
*device owner*. It then blocks uninstalling the placeholders and itself, as well as VPN changes and safe mode, and
it suspends any blocked app the moment it is installed. Android only allows this on a phone with no accounts, so
you remove them first and add them back afterwards. To remove it, use Cerrojo or `adb shell dpm
remove-active-admin dev.cerrojo.filtro/.Admin`.

## Build

```sh
cargo build --release   # needs Rust and GTK 4.6+ (gtk4-devel / libgtk-4-dev)
```

[MIT](LICENSE)
