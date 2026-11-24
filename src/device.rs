//! Lo que se hace en el celular, todo por adb.
//!
//! Bloquear un paquete:
//! - Si ya hay un candado nuestro: se asegura de que esté suspendido.
//! - App de fábrica (YouTube en casi todos los Android): no se puede
//!   reemplazar, así que se deja instalada y se **suspende**. La tienda la ve
//!   instalada y al abrirla sale "app pausada".
//! - App instalada normal: se desinstala y se pone el candado.
//! - No instalada: se pone el candado.
//!
//! El candado y la suspensión hechos por el shell de adb solo se quitan con
//! adb: desde el celular no hay forma.

use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::apk;

#[derive(Clone, Debug)]
pub struct Device {
    pub serial: String,
    pub model: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PkgState {
    /// No está en el celular: se podría instalar.
    Absent,
    /// La app real, usable.
    Live,
    /// Candado nuestro o app de fábrica suspendida.
    Blocked,
}

#[derive(Clone, Debug, Default)]
struct Info {
    present: bool,
    installed: bool,
    system: bool,
    suspended: bool,
    ours: bool,
}

/// adb del PATH o, si no está (una app abierta desde el Finder o el menú de
/// Inicio no hereda el PATH de la terminal), de los sitios de siempre.
fn adb_path() -> std::path::PathBuf {
    use std::path::PathBuf;
    use std::sync::OnceLock;
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        if let Some(p) = std::env::var_os("CERROJO_ADB") {
            return PathBuf::from(p);
        }
        let exe = if cfg!(windows) { "adb.exe" } else { "adb" };
        if let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                if dir.join(exe).is_file() {
                    return dir.join(exe);
                }
            }
        }
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default();
        let mut candidates = vec![
            home.join("Android/Sdk/platform-tools").join(exe),
            home.join("Library/Android/sdk/platform-tools").join(exe),
            home.join("platform-tools").join(exe),
            PathBuf::from("/opt/homebrew/bin/adb"),
            PathBuf::from("/usr/local/bin/adb"),
        ];
        if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
            candidates.push(local.join("Android/Sdk/platform-tools/adb.exe"));
            candidates.push(local.join("Microsoft/WinGet/Links/adb.exe"));
        }
        // Junto a cerrojo.exe (quien descomprima platform-tools en la misma carpeta).
        if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.to_path_buf())) {
            candidates.push(dir.join(exe));
            candidates.push(dir.join("platform-tools").join(exe));
        }
        candidates.into_iter().find(|c| c.is_file()).unwrap_or_else(|| PathBuf::from(exe))
    })
    .clone()
}

fn adb() -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(adb_path());
    #[cfg(windows)]
    {
        // Sin esto, cada llamada a adb abre una consola negra un instante.
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c
}

pub fn adb_available() -> bool {
    adb().arg("version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

/// Equipos listos (los "unauthorized" no cuentan: hay que aceptar en el celular).
pub fn devices() -> (Vec<Device>, usize) {
    let Ok(out) = adb().args(["devices", "-l"]).stderr(Stdio::null()).output() else {
        return (Vec::new(), 0);
    };
    let mut list = Vec::new();
    let mut unauthorized = 0;
    for line in String::from_utf8_lossy(&out.stdout).lines().skip(1) {
        let mut it = line.split_whitespace();
        let (Some(serial), Some(state)) = (it.next(), it.next()) else { continue };
        match state {
            "device" => {
                let model = it
                    .find_map(|t| t.strip_prefix("model:"))
                    .unwrap_or(serial)
                    .replace('_', " ");
                list.push(Device { serial: serial.into(), model: nice_model(serial, &model) });
            }
            "unauthorized" => unauthorized += 1,
            _ => {}
        }
    }
    (list, unauthorized)
}

/// "SM X800" no le dice nada a nadie: se pregunta el nombre comercial.
fn nice_model(serial: &str, fallback: &str) -> String {
    for prop in ["ro.product.marketname", "ro.config.marketing_name"] {
        if let Ok(o) = shell(serial, &format!("getprop {prop}")) {
            let v = o.trim();
            if !v.is_empty() {
                return v.to_string();
            }
        }
    }
    if let Ok(o) = shell(serial, "settings get global device_name") {
        let v = o.trim();
        if !v.is_empty() && v != "null" {
            return v.to_string();
        }
    }
    fallback.to_string()
}

fn shell(serial: &str, cmd: &str) -> Result<String> {
    let out = adb()
        .args(["-s", serial, "shell", cmd])
        .stdin(Stdio::null())
        .output()
        .context("no se pudo ejecutar adb")?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr))
}

/// Estado de varios paquetes con una sola llamada a adb.
fn infos(serial: &str, packages: &[&str]) -> Result<HashMap<String, Info>> {
    let mut script = String::new();
    for p in packages {
        script.push_str(&format!(
            "echo '@@{p}'; dumpsys package {p} | grep -E 'Package \\[|versionName=|pkgFlags=|User 0:'; "
        ));
    }
    let out = shell(serial, &script)?;
    let mut map: HashMap<String, Info> = HashMap::new();
    let mut current: Option<String> = None;
    for line in out.lines() {
        let line = line.trim();
        if let Some(p) = line.strip_prefix("@@") {
            current = Some(p.to_string());
            map.insert(p.to_string(), Info::default());
            continue;
        }
        let Some(p) = &current else { continue };
        let info = map.get_mut(p).unwrap();
        if line.starts_with(&format!("Package [{p}]")) {
            info.present = true;
        } else if let Some(v) = line.strip_prefix("versionName=") {
            // Si hay varias versiones (de fábrica + actualización), basta una.
            info.ours |= v.starts_with(apk::MARK);
        } else if line.starts_with("pkgFlags=") {
            info.system |= line.contains(" SYSTEM ");
        } else if line.starts_with("User 0:") && line.contains("installed=") {
            // Con versión de fábrica + actualización hay dos bloques: basta uno.
            info.installed |= line.contains("installed=true");
            info.suspended |= line.contains("suspended=true");
        }
    }
    Ok(map)
}

fn classify(i: &Info) -> PkgState {
    if !i.present || !i.installed {
        // De fábrica pero quitada para el usuario: la tienda la reinstala con
        // un toque, así que cuenta como libre.
        PkgState::Absent
    } else if i.ours || i.suspended {
        PkgState::Blocked
    } else {
        PkgState::Live
    }
}

pub fn states(serial: &str, packages: &[&str]) -> Result<HashMap<String, PkgState>> {
    Ok(infos(serial, packages)?.into_iter().map(|(p, i)| (p, classify(&i))).collect())
}

fn install_stub(serial: &str, package: &str) -> Result<()> {
    let bytes = apk::build(package)?;
    let path = std::env::temp_dir().join(format!("cerrojo-{}-{package}.apk", std::process::id()));
    std::fs::File::create(&path)?.write_all(&bytes)?;
    let out = adb().arg("-s").arg(serial).arg("install").arg("-r").arg(&path).stdin(Stdio::null()).output();
    let _ = std::fs::remove_file(&path);
    let out = out?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    if !out.status.success() || !text.contains("Success") {
        bail!("no se pudo instalar el candado de {package}: {}", text.trim());
    }
    Ok(())
}

fn suspend(serial: &str, package: &str, on: bool) -> Result<()> {
    let verb = if on { "suspend" } else { "unsuspend" };
    let out = shell(serial, &format!("cmd package {verb} --user 0 {package}"))?;
    if out.contains("active DO or PO") {
        // Con protección máxima solo el guardián puede suspender: lo hace él
        // con la lista que le manda sync_guard.
        return Ok(());
    }
    if !out.contains("new suspended state") {
        bail!("no se pudo {} {package}: {}", if on { "suspender" } else { "reactivar" }, out.trim());
    }
    Ok(())
}

pub fn block(serial: &str, package: &str) -> Result<()> {
    let info = infos(serial, &[package])?.remove(package).unwrap_or_default();
    if info.present && info.system && !info.ours {
        if !info.installed {
            shell(serial, &format!("cmd package install-existing --user 0 {package}"))?;
        }
        return suspend(serial, package, true);
    }
    if info.present && info.ours {
        if !info.installed {
            shell(serial, &format!("cmd package install-existing --user 0 {package}"))?;
        }
        return suspend(serial, package, true);
    }
    if info.present {
        let out = adb().args(["-s", serial, "uninstall", package]).stdin(Stdio::null()).output()?;
        if !String::from_utf8_lossy(&out.stdout).contains("Success") {
            // Puede quedar para otros usuarios; con quitarla del principal basta.
            shell(serial, &format!("pm uninstall --user 0 {package}"))?;
        }
    }
    install_stub(serial, package)?;
    suspend(serial, package, true)
}

pub fn unblock(serial: &str, package: &str) -> Result<()> {
    let info = infos(serial, &[package])?.remove(package).unwrap_or_default();
    if !info.present {
        return Ok(());
    }
    if info.ours {
        let out = adb().args(["-s", serial, "uninstall", package]).stdin(Stdio::null()).output()?;
        if !String::from_utf8_lossy(&out.stdout).contains("Success") {
            bail!("no se pudo quitar el candado de {package}");
        }
        return Ok(());
    }
    if info.suspended {
        suspend(serial, package, false)?;
    }
    Ok(())
}

// ---- Filtro web ----
//
// Una VPN local (android/) que solo ve el DNS y responde "no existe" a los
// dominios de las apps bloqueadas. Todo por adb, sin tocar el celular:
// - appops ACTIVATE_VPN: el permiso de VPN sin el diálogo de "¿confías?".
// - deviceidle whitelist: que el ahorro de batería no la duerma.
// - always_on_vpn_app: Android la vuelve a encender al reiniciar.
// - DNS privado apagado: si no, el celular preguntaría por fuera del túnel.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebState {
    /// No está instalado.
    Absent,
    /// Instalado pero sin la VPN puesta (la quitaste en Ajustes, por ejemplo).
    Off,
    On,
}

pub fn web_state(serial: &str) -> Result<WebState> {
    let p = apk::FILTER_PKG;
    let out = shell(
        serial,
        &format!(
            "pm path {p}; echo @@; dumpsys package {p} 2>/dev/null | grep -m1 versionCode=; echo @@; \
             dumpsys connectivity 2>/dev/null | grep -c '10.111.222.2/32'"
        ),
    )?;
    let parts: Vec<&str> = out.split("@@").collect();
    if parts.len() < 3 || !parts[0].contains("package:") {
        return Ok(WebState::Absent);
    }
    let version: u32 = parts[1]
        .split("versionCode=")
        .nth(1)
        .and_then(|v| v.split_whitespace().next())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if version < apk::FILTER_VERSION {
        return Ok(WebState::Absent);
    }
    // Solo la línea del número: los avisos de stderr van al final.
    let vpn = parts[2].lines().find_map(|n| n.trim().parse::<u32>().ok()).unwrap_or(0);
    Ok(if vpn > 0 { WebState::On } else { WebState::Off })
}

fn install_filter(serial: &str) -> Result<()> {
    let bytes = apk::build_filter()?;
    let path = std::env::temp_dir().join(format!("cerrojo-{}-filtro.apk", std::process::id()));
    std::fs::File::create(&path)?.write_all(&bytes)?;
    // -t: el filtro es testOnly, para que la protección máxima siempre se
    // pueda quitar por adb.
    let install = || adb().arg("-s").arg(serial).args(["install", "-r", "-t"]).arg(&path).stdin(Stdio::null()).output();
    let mut out = install()?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    if text.contains("INSTALL_FAILED_UPDATE_INCOMPATIBLE") {
        // Lo puso Cerrojo desde otro PC (otra firma): se cambia por el nuestro.
        shell(serial, &format!("pm uninstall {}", apk::FILTER_PKG))?;
        out = install()?;
        text = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    }
    let _ = std::fs::remove_file(&path);
    if !out.status.success() || !text.contains("Success") {
        bail!("no se pudo instalar el filtro web: {}", text.trim());
    }
    Ok(())
}

/// Pone (o actualiza) el filtro con estos dominios.
pub fn web_on(serial: &str, domains: &[String]) -> Result<()> {
    let p = apk::FILTER_PKG;
    if web_state(serial)? == WebState::Absent {
        install_filter(serial)?;
    }
    let list: Vec<&str> = domains
        .iter()
        .map(String::as_str)
        .filter(|d| d.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '!' | '*')))
        .collect();
    let out = shell(
        serial,
        &format!(
            "appops set {p} ACTIVATE_VPN allow; cmd deviceidle whitelist +{p} >/dev/null; \
             settings put global private_dns_mode off; settings put secure always_on_vpn_app {p}; \
             am start -n {p}/.Orden --es dominios '{}'",
            list.join(" ")
        ),
    )?;
    if out.contains("Error") || out.contains("Exception") {
        bail!("no se pudo encender el filtro web: {}", out.trim());
    }
    Ok(())
}

/// Apaga el filtro web. Con keep_app la app se queda (hace falta para el
/// «sin vertical» o la protección máxima); si no, se desinstala.
pub fn web_off(serial: &str, keep_app: bool) -> Result<()> {
    let p = apk::FILTER_PKG;
    if !shell(serial, &format!("pm path {p}"))?.contains("package:") {
        return Ok(());
    }
    let uninstall = if keep_app {
        String::new()
    } else {
        format!("cmd deviceidle whitelist -{p} >/dev/null; dpm remove-active-admin --user 0 {p}/.Admin >/dev/null 2>&1; sleep 1; pm uninstall {p}")
    };
    shell(
        serial,
        &format!(
            "am start -n {p}/.Orden --ez apagar true; \
             [ \"$(settings get secure always_on_vpn_app)\" = {p} ] && settings delete secure always_on_vpn_app; \
             settings put global private_dns_mode opportunistic; {uninstall}"
        ),
    )?;
    Ok(())
}

// ---- Guardián: sin vertical, protección máxima y desbloqueos temporales ----
//
// La misma app del filtro (android/Guardia.java, Vigia.java, Admin.java).
// - «Sin vertical»: el vigía (android/…/Vigia.java), un servicio de
//   accesibilidad que cierra el reproductor de Shorts/Reels/Estados y tapa su
//   pestaña. Se enciende por adb (settings secure) y la app tiene
//   WRITE_SECURE_SETTINGS para volver a encenderlo si lo apagas en Ajustes.
//   ACCESS_RESTRICTED_SETTINGS evita el aviso de "ajuste restringido" de
//   Android 13+ con apps que no vienen de una tienda.
// - Protección máxima: la app es propietaria del equipo (Device Owner). Así
//   suspende sola las apps bloqueadas que aparezcan, no deja desinstalar los
//   candados ni el filtro, ni tocar la VPN, y cierra sola los desbloqueos
//   temporales aunque el PC no esté.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GuardState {
    pub installed: bool,
    pub owner: bool,
    /// El vigía («sin vertical») está encendido en Ajustes.
    pub vertical: bool,
    /// Desbloqueos temporales: paquete → segundos que quedan.
    pub open: Vec<(String, u64)>,
}

pub fn guard_state(serial: &str) -> Result<GuardState> {
    let p = apk::FILTER_PKG;
    let out = shell(
        serial,
        &format!(
            "pm path {p}; echo @@; settings get secure enabled_accessibility_services; echo @@; \
             dumpsys device_policy 2>/dev/null | grep -A3 -i 'device owner' | grep -c {p}; echo @@; \
             dumpsys activity service {p}/.Guardia"
        ),
    )?;
    let parts: Vec<&str> = out.split("@@").collect();
    if parts.len() < 4 || !parts[0].contains("package:") {
        return Ok(GuardState::default());
    }
    let mut g = GuardState { installed: true, ..Default::default() };
    g.vertical = parts[1].split(':').any(|s| s.trim() == format!("{p}/{p}.Vigia") || s.trim() == format!("{p}/.Vigia"));
    g.owner = parts[2].lines().find_map(|n| n.trim().parse::<u32>().ok()).unwrap_or(0) > 0;
    for line in parts[3].lines() {
        let mut it = line.split_whitespace();
        if it.next() == Some("abierto") {
            if let (Some(pkg), Some(secs)) = (it.next(), it.next().and_then(|s| s.parse().ok())) {
                g.open.push((pkg.to_string(), secs));
            }
        }
    }
    Ok(g)
}

fn ensure_filter(serial: &str) -> Result<()> {
    if web_state(serial)? == WebState::Absent {
        install_filter(serial)?;
    }
    Ok(())
}

/// La app de YouTube, a la que se le quita el picture-in-picture con el vigía:
/// un Short en ventana flotante quedaría fuera de su alcance.
const YT: &str = "com.google.android.youtube";

/// Le pasa al guardián qué vigila el vigía ("youtube instagram…") y la lista
/// de paquetes bloqueados, y enciende (o apaga) el servicio de accesibilidad.
pub fn guard_apply(serial: &str, keys: &[String], blocked: &[String]) -> Result<()> {
    let p = apk::FILTER_PKG;
    let keys: Vec<&str> =
        keys.iter().map(String::as_str).filter(|k| k.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')).collect();
    let pip = if keys.contains(&"youtube") { "ignore" } else { "default" };
    let g = guard_state(serial)?;
    if keys.is_empty() && !g.owner {
        if g.installed {
            // Apagar el vigía si lo pusimos nosotros.
            shell(
                serial,
                &format!("am start -n {p}/.Orden --es vigia '' >/dev/null; {}; appops set {YT} PICTURE_IN_PICTURE default", accessibility_script(false)),
            )?;
        }
        return Ok(());
    }
    ensure_filter(serial)?;
    let blocked: Vec<&str> = blocked.iter().map(String::as_str).filter(|p| apk::valid_package(p)).collect();
    let out = shell(
        serial,
        &format!(
            "pm grant {p} android.permission.WRITE_SECURE_SETTINGS; appops set {p} ACCESS_RESTRICTED_SETTINGS allow 2>/dev/null; \
             appops set {p} SCHEDULE_EXACT_ALARM allow 2>/dev/null; cmd deviceidle whitelist +{p} >/dev/null; \
             dpm set-active-admin --user 0 {p}/.Admin >/dev/null 2>&1; \
             am start -n {p}/.Orden --es vigia '{}' --es bloquear '{}' >/dev/null; sleep 1; {}; \
             appops set {YT} PICTURE_IN_PICTURE {pip} 2>/dev/null",
            keys.join(" "),
            blocked.join(" "),
            accessibility_script(!keys.is_empty())
        ),
    )?;
    if out.contains("Exception") {
        bail!("no se pudo poner el guardián: {}", out.trim());
    }
    Ok(())
}

/// Pone o quita nuestro servicio en la lista de accesibilidad sin tocar los demás.
fn accessibility_script(on: bool) -> String {
    let me = format!("{}/.Vigia", apk::FILTER_PKG);
    let rest = format!(
        "l=$(settings get secure enabled_accessibility_services); [ \"$l\" = null ] && l=''; \
         r=$(echo \"$l\" | tr ':' '\\n' | grep -v -i -e '^{me}$' -e '^{p}/{p}.Vigia$' -e '^{p}/{p}.Vertical$' -e '^{p}/.Vertical$' | grep . | tr '\\n' ':' | sed 's/:$//')",
        p = apk::FILTER_PKG
    );
    if on {
        format!(
            "{rest}; if [ -z \"$r\" ]; then n='{me}'; else n=\"$r:{me}\"; fi; \
             settings put secure enabled_accessibility_services \"$n\"; settings put secure accessibility_enabled 1"
        )
    } else {
        format!(
            "{rest}; if [ \"$r\" != \"$l\" ]; then if [ -z \"$r\" ]; then settings delete secure enabled_accessibility_services; \
             else settings put secure enabled_accessibility_services \"$r\"; fi; fi"
        )
    }
}

/// Las cuentas del celular que impiden la protección máxima.
pub fn accounts(serial: &str) -> Result<Vec<String>> {
    let out = shell(serial, "dumpsys account | grep -E '^ *Account \\{'")?;
    Ok(out
        .lines()
        .filter_map(|l| l.split("type=").nth(1).map(|t| t.trim_end_matches(['}', ' ']).to_string()))
        .collect())
}

/// Hace al filtro propietario del equipo. Antes hay que quitar las cuentas.
/// Con propietario, adb ya no puede suspender ni reactivar apps: las
/// suspensiones hechas por adb se quitan antes y las pone el guardián.
pub fn make_owner(serial: &str, blocked: &[String]) -> Result<()> {
    ensure_filter(serial)?;
    let acc = accounts(serial)?;
    if !acc.is_empty() {
        bail!("primero quita estas cuentas del celular (Ajustes › Cuentas): {}", acc.join(", "));
    }
    let suspended: Vec<String> = infos(serial, &blocked.iter().map(String::as_str).collect::<Vec<_>>())?
        .into_iter()
        .filter(|(_, i)| i.present && i.installed && i.suspended)
        .map(|(p, _)| p)
        .collect();
    for p in &suspended {
        suspend(serial, p, false)?;
    }
    let out = shell(serial, &format!("dpm set-device-owner {}/.Admin", apk::FILTER_PKG))?;
    if !out.contains("Success") {
        for p in &suspended {
            let _ = suspend(serial, p, true);
        }
        let why = if out.contains("several users") || out.contains("multiple users") {
            "hay otro usuario o la Carpeta Segura; quítalos primero".to_string()
        } else {
            out.trim().to_string()
        };
        bail!("Android no dejó activarla: {why}");
    }
    Ok(())
}

pub fn release_owner(serial: &str) -> Result<()> {
    let p = apk::FILTER_PKG;
    shell(serial, &format!("am start -n {p}/.Orden --ez soltar true"))?;
    std::thread::sleep(std::time::Duration::from_millis(1500));
    if guard_state(serial)?.owner {
        let out = shell(serial, &format!("dpm remove-active-admin {p}/.Admin"))?;
        if !out.contains("Success") {
            bail!("no se pudo quitar: {}", out.trim());
        }
    }
    Ok(())
}

/// Desbloqueo temporal: la app (o su video vertical) vuelve a cerrarse sola
/// a los `minutes`, esté o no conectado el PC.
pub fn open_for(serial: &str, packages: &[String], minutes: u32, blocked: bool) -> Result<()> {
    let p = apk::FILTER_PKG;
    if blocked {
        if !guard_state(serial)?.owner {
            bail!("para abrir un rato una app bloqueada hace falta la protección máxima");
        }
        let info = infos(serial, &packages.iter().map(String::as_str).collect::<Vec<_>>())?;
        if info.values().any(|i| i.present && i.ours) {
            bail!("tiene candado (no está la app real); ponla en «sin vertical» o desbloquéala");
        }
        for (pkg, i) in &info {
            if i.present && i.suspended {
                // Quita la suspensión del shell; la del guardián la quita él.
                shell(serial, &format!("cmd package unsuspend --user 0 {pkg}"))?;
            }
        }
    }
    let list: Vec<&str> = packages.iter().map(String::as_str).filter(|p| apk::valid_package(p)).collect();
    shell(serial, &format!("am start -n {p}/.Orden --es abrir '{}' --ei minutos {minutes}", list.join(" ")))?;
    Ok(())
}

/// Acaba ya los desbloqueos temporales.
pub fn close_open(serial: &str) -> Result<()> {
    shell(serial, &format!("am start -n {}/.Orden --ez cerrar true", apk::FILTER_PKG))?;
    Ok(())
}

/// Las vistas con nombre de la pantalla de ahora (para sacar las de video
/// vertical de una app: ábrelo en el celular y corre `cerrojo vistas`).
pub fn views(serial: &str) -> Result<(String, Vec<String>)> {
    let out = shell(serial, "uiautomator dump /sdcard/cerrojo-vistas.xml >/dev/null && cat /sdcard/cerrojo-vistas.xml; rm -f /sdcard/cerrojo-vistas.xml")?;
    let mut ids = Vec::new();
    let mut pkg = String::new();
    for part in out.split("resource-id=\"").skip(1) {
        let id = part.split('"').next().unwrap_or("");
        if !id.is_empty() && !ids.iter().any(|x: &String| x == id) {
            ids.push(id.to_string());
        }
    }
    if let Some(p) = out.split("package=\"").nth(1) {
        pkg = p.split('"').next().unwrap_or("").to_string();
    }
    Ok((pkg, ids))
}
