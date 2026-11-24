//! Cerrojo: bloquea en el celular las apps de reels y video vertical, de
//! forma que solo se pueden volver a activar desde el PC.
//!
//! Sin argumentos abre la ventana. También:
//!   cerrojo estado [SERIAL]
//!   cerrojo bloquear PAQUETE... [-s SERIAL]
//!   cerrojo desbloquear PAQUETE... [-s SERIAL]
//!   cerrojo web [SERIAL]            (pone el filtro web como en la ventana)
//!   cerrojo web apagar|encender [-s SERIAL]
//!   cerrojo modo APP bloqueada|vertical|libre [-s SERIAL]
//!   cerrojo abrir APP MINUTOS [-s SERIAL]   (se vuelve a cerrar sola)
//!   cerrojo cerrar [-s SERIAL]      (acaba ya los desbloqueos temporales)
//!   cerrojo dueño [quitar] [-s SERIAL]      (protección máxima)
//!   cerrojo guardia [SERIAL]        (estado de lo anterior)
//!   cerrojo vistas [-s SERIAL]      (vistas de la pantalla de ahora, para el catálogo)
//!   cerrojo solo youtube|instagram si|no [-s SERIAL]   (solo sin Shorts/Reels)
//!   cerrojo apk PAQUETE ARCHIVO     (solo genera el candado)

// En Windows, sin consola al abrir la ventana; desde una terminal se engancha
// a la de la terminal para que las órdenes impriman (ver attach_console).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod apk;
mod catalog;
mod device;
mod gui;
mod keys;
mod store;

use anyhow::{Result, bail};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        return gui::run();
    };
    attach_console();
    let mut serial = None;
    let mut rest = Vec::new();
    let mut it = args[1..].iter();
    while let Some(a) = it.next() {
        if a == "-s" {
            serial = it.next().cloned();
        } else {
            rest.push(a.clone());
        }
    }
    let pick = |serial: Option<String>| -> Result<String> {
        if let Some(s) = serial {
            return Ok(s);
        }
        let (list, _) = device::devices();
        match list.as_slice() {
            [d] => Ok(d.serial.clone()),
            [] => bail!("no hay ningún celular conectado por adb"),
            _ => bail!("hay varios equipos: elige uno con -s SERIAL"),
        }
    };
    match cmd.as_str() {
        "apk" if rest.len() == 2 => std::fs::write(&rest[1], apk::build(&rest[0])?)?,
        "estado" => {
            let serial = pick(serial.or(rest.first().cloned()))?;
            let st = store::Store::load();
            for app in catalog::all() {
                println!("{:<20} modo {:?}", app.name, st.mode(&serial, &app.name));
                let pkgs: Vec<&str> = app.packages.iter().map(String::as_str).collect();
                let st = device::states(&serial, &pkgs)?;
                for p in &app.packages {
                    println!("{:<20} {:<40} {:?}", app.name, p, st.get(p).copied().unwrap_or(device::PkgState::Absent));
                }
            }
            println!("vigía de Shorts y Reels: {}", if device::guard_state(&serial)?.vertical { "encendido" } else { "apagado" });
        }
        // Lo de la versión del fork: «solo APP si» = modo sin vertical, «no» = bloqueada entera.
        "solo" if rest.len() == 2 => {
            let serial = pick(serial)?;
            let Some(app) = catalog::all().into_iter().find(|a| a.partial.as_ref().is_some_and(|p| p.key == rest[0])) else {
                bail!("«solo» sirve para: youtube, instagram, whatsapp, linkedin");
            };
            let on = matches!(rest[1].as_str(), "si" | "sí" | "on");
            if !on && app.never_blocked() {
                bail!("{} no se bloquea entera; para que vuelvan sus historias: cerrojo modo {} libre", app.name, rest[0]);
            }
            let mode = if on { store::Mode::NoVertical } else { store::Mode::Blocked };
            for e in store::apply_mode(&serial, &app, mode) {
                println!("{e}");
            }
            println!("{}: {:?}; vigía {}", app.name, mode, if device::guard_state(&serial)?.vertical { "encendido" } else { "apagado" });
        }
        "bloquear" | "desbloquear" if !rest.is_empty() => {
            let serial = pick(serial)?;
            for p in &rest {
                let r = if cmd == "bloquear" { device::block(&serial, p) } else { device::unblock(&serial, p) };
                match r {
                    Ok(()) => println!("{p}: ok"),
                    Err(e) => println!("{p}: {e:#}"),
                }
            }
        }
        "web" => {
            let action = rest.iter().find(|a| *a == "apagar" || *a == "encender").cloned();
            let serial = pick(serial.or(rest.iter().find(|a| Some(*a) != action.as_ref()).cloned()))?;
            if let Some(a) = action {
                let mut st = store::Store::load();
                st.set_web(&serial, a == "encender");
            }
            let n = store::sync_web(&serial)?;
            // La VPN tarda un instante en levantarse.
            std::thread::sleep(std::time::Duration::from_millis(3500));
            println!("filtro web: {n} dominios bloqueados; estado {:?}", device::web_state(&serial)?);
            println!("vigía de Shorts y Reels: {}", if device::guard_state(&serial)?.vertical { "encendido" } else { "apagado" });
        }
        "modo" if rest.len() == 2 => {
            let serial = pick(serial)?;
            let app = find_app(&rest[0])?;
            let Some(mode) = store::Mode::parse(&rest[1]) else { bail!("modo: bloqueada, vertical o libre") };
            if mode == store::Mode::NoVertical && !app.has_vertical() {
                bail!("{} no tiene modo «sin vertical» (el vigía no la conoce)", app.name);
            }
            if mode == store::Mode::Blocked && app.never_blocked() {
                bail!("{} no se bloquea entera: solo «vertical» o «libre»", app.name);
            }
            let errors = store::apply_mode(&serial, &app, mode);
            for e in &errors {
                println!("{e}");
            }
            println!("{}: {:?}{}", app.name, mode, if errors.is_empty() { "" } else { " (con errores)" });
        }
        "abrir" if rest.len() == 2 => {
            let serial = pick(serial)?;
            let app = find_app(&rest[0])?;
            let minutes: u32 = rest[1].trim_end_matches(['m', 'M']).parse().map_err(|_| anyhow::anyhow!("minutos no válidos"))?;
            let mode = store::Store::load().mode(&serial, &app.name);
            if mode == store::Mode::Free {
                bail!("{} ya está libre", app.name);
            }
            device::open_for(&serial, &app.packages, minutes, mode == store::Mode::Blocked)?;
            println!("{} abierta {minutes} min", app.name);
        }
        "cerrar" => {
            let serial = pick(serial)?;
            device::close_open(&serial)?;
            println!("ok");
        }
        "dueño" | "dueno" | "proteccion" => {
            let serial = pick(serial)?;
            if rest.first().is_some_and(|a| a == "quitar") {
                device::release_owner(&serial)?;
                // Sin propietario, las suspensiones vuelven a ser cosa de adb.
                for p in store::Store::load().blocked_packages(&serial) {
                    let _ = device::block(&serial, &p);
                }
                println!("protección máxima quitada");
            } else {
                device::make_owner(&serial, &store::Store::load().blocked_packages(&serial))?;
                store::sync_guard(&serial)?;
                println!("protección máxima puesta: ya puedes volver a añadir tus cuentas");
            }
        }
        "guardia" => {
            let serial = pick(serial.or(rest.first().cloned()))?;
            let g = device::guard_state(&serial)?;
            println!("instalado: {}  protección máxima: {}  sin vertical encendido: {}", g.installed, g.owner, g.vertical);
            for (p, secs) in g.open {
                println!("abierta {p}: quedan {} min {} s", secs / 60, secs % 60);
            }
        }
        "vistas" => {
            let serial = pick(serial)?;
            let (pkg, ids) = device::views(&serial)?;
            println!("{pkg}");
            for id in ids {
                println!("  {}", id.split(":id/").nth(1).unwrap_or(&id));
            }
        }
        "--ventana" | "--gui" => return gui::run(),
        _ => {
            eprintln!(
                "uso: cerrojo [estado | bloquear PAQ... | desbloquear PAQ... | web | modo APP MODO | solo APP si|no | abrir APP MIN | cerrar | dueño [quitar] | guardia | vistas | apk PAQ ARCHIVO] [-s SERIAL]"
            );
            std::process::exit(2);
        }
    }
    Ok(())
}

/// Una app del catálogo por su nombre (sin mayúsculas) o uno de sus paquetes.
fn find_app(q: &str) -> Result<catalog::App> {
    let q = q.to_lowercase();
    catalog::all()
        .into_iter()
        .find(|a| a.name.to_lowercase() == q || a.packages.iter().any(|p| *p == q) || a.partial.as_ref().is_some_and(|p| p.key == q))
        .ok_or_else(|| anyhow::anyhow!("no conozco esa app: {q}"))
}

#[cfg(windows)]
fn attach_console() {
    unsafe extern "system" {
        fn AttachConsole(pid: u32) -> i32;
    }
    // ATTACH_PARENT_PROCESS
    unsafe { AttachConsole(u32::MAX) };
}

#[cfg(not(windows))]
fn attach_console() {}
