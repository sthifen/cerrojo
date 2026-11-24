//! Qué apps quieres bloqueadas en cada equipo (~/.config/cerrojo/equipos.json).
//! Sirve para volver a poner el candado solo cuando el celular se conecta y
//! falta alguno (por ejemplo, si lo desinstalaste desde la Play Store).

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::config_dir;

#[derive(Default)]
pub struct Store {
    pub wanted: BTreeMap<String, BTreeSet<String>>,
    /// Equipos en los que apagaste el filtro web (~/.config/cerrojo/web.json).
    pub web_off: BTreeSet<String>,
    /// Apps que quieres sin su video vertical (~/.config/cerrojo/sin-vertical.json).
    pub no_vertical: BTreeMap<String, BTreeSet<String>>,
    /// Las opciones extra que apagaste en cada equipo (encendidas si no están
    /// aquí; ~/.config/cerrojo/opciones.json).
    pub extras_off: BTreeMap<String, BTreeSet<String>>,
}

/// Cómo quieres una app en un equipo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Blocked,
    NoVertical,
    Free,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Mode> {
        match s {
            "bloqueada" | "bloquear" => Some(Mode::Blocked),
            "vertical" | "sin-vertical" | "sinvertical" => Some(Mode::NoVertical),
            "libre" => Some(Mode::Free),
            _ => None,
        }
    }
}

impl Store {
    pub fn load() -> Self {
        let wanted = std::fs::read(config_dir().join("equipos.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let web_off = std::fs::read(config_dir().join("web.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let no_vertical = std::fs::read(config_dir().join("sin-vertical.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let extras_off = std::fs::read(config_dir().join("opciones.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let mut st = Store { wanted, web_off, no_vertical, extras_off };
        st.migrate();
        st
    }

    /// La versión del fork guardaba «solo los Shorts» como bloqueada + una
    /// marca en parcial.json, y WhatsApp/LinkedIn como bloqueadas a secas:
    /// pasan a «sin vertical».
    fn migrate(&mut self) {
        let path = config_dir().join("parcial.json");
        let old: BTreeMap<String, BTreeSet<String>> =
            std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let only: Vec<String> = crate::catalog::all().into_iter().filter(|a| a.never_blocked()).map(|a| a.name).collect();
        let mut changed = false;
        for (serial, set) in self.wanted.iter_mut() {
            let moved: Vec<String> = set
                .iter()
                .filter(|a| only.contains(a) || old.get(serial).is_some_and(|o| o.contains(*a)))
                .cloned()
                .collect();
            for a in moved {
                set.remove(&a);
                self.no_vertical.entry(serial.clone()).or_default().insert(a);
                changed = true;
            }
        }
        if changed || path.exists() {
            self.save();
            let _ = std::fs::remove_file(path);
        }
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(config_dir());
        if let Ok(b) = serde_json::to_vec_pretty(&self.wanted) {
            let _ = std::fs::write(config_dir().join("equipos.json"), b);
        }
        if let Ok(b) = serde_json::to_vec_pretty(&self.web_off) {
            let _ = std::fs::write(config_dir().join("web.json"), b);
        }
        if let Ok(b) = serde_json::to_vec_pretty(&self.no_vertical) {
            let _ = std::fs::write(config_dir().join("sin-vertical.json"), b);
        }
        if let Ok(b) = serde_json::to_vec_pretty(&self.extras_off) {
            let _ = std::fs::write(config_dir().join("opciones.json"), b);
        }
    }

    /// Encender o apagar una opción extra (los directos) en este equipo.
    pub fn set_extra(&mut self, serial: &str, key: &str, on: bool) {
        let set = self.extras_off.entry(serial.to_string()).or_default();
        if on {
            set.remove(key);
        } else {
            set.insert(key.to_string());
        }
        self.save();
    }

    pub fn extra_on(&self, serial: &str, key: &str) -> bool {
        !self.extras_off.get(serial).is_some_and(|s| s.contains(key))
    }

    pub fn mode(&self, serial: &str, app: &str) -> Mode {
        if self.wants(serial, app) {
            Mode::Blocked
        } else if self.no_vertical.get(serial).is_some_and(|s| s.contains(app)) {
            Mode::NoVertical
        } else {
            Mode::Free
        }
    }

    pub fn set_mode(&mut self, serial: &str, app: &str, mode: Mode) {
        let nv = self.no_vertical.entry(serial.to_string()).or_default();
        if mode == Mode::NoVertical {
            nv.insert(app.to_string());
        } else {
            nv.remove(app);
        }
        self.set(serial, app, mode == Mode::Blocked);
    }

    /// Lo que debe vigilar el vigía en este equipo ("youtube",
    /// "youtube-directos", "instagram"…): las apps en «sin vertical».
    pub fn guard_keys(&self, serial: &str) -> Vec<String> {
        crate::catalog::all()
            .into_iter()
            .filter(|a| self.mode(serial, &a.name) == Mode::NoVertical)
            .filter_map(|a| a.partial)
            .flat_map(|p| {
                let extra = p.extra.map(|e| e.key).filter(|k| self.extra_on(serial, k));
                std::iter::once(p.key).chain(extra)
            })
            .collect()
    }

    /// Los paquetes que deben estar bloqueados en este equipo: los de las apps
    /// bloqueadas y, de las que van «sin vertical», los que el vigía no mira
    /// (Instagram Lite…).
    pub fn blocked_packages(&self, serial: &str) -> Vec<String> {
        crate::catalog::all().into_iter().flat_map(|a| packages_to_block(&a, self.mode(serial, &a.name))).collect()
    }

    /// Encender o apagar el filtro web en este equipo.
    pub fn set_web(&mut self, serial: &str, on: bool) {
        if on {
            self.web_off.remove(serial);
        } else {
            self.web_off.insert(serial.to_string());
        }
        self.save();
    }

    pub fn web_enabled(&self, serial: &str) -> bool {
        !self.web_off.contains(serial)
    }

    pub fn set(&mut self, serial: &str, app: &str, on: bool) {
        let set = self.wanted.entry(serial.to_string()).or_default();
        if on {
            set.insert(app.to_string());
            if let Some(nv) = self.no_vertical.get_mut(serial) {
                nv.remove(app);
            }
        } else {
            set.remove(app);
        }
        self.save();
    }

    pub fn wants(&self, serial: &str, app: &str) -> bool {
        self.wanted.get(serial).is_some_and(|s| s.contains(app))
    }
}

impl Store {
    /// Los dominios que deben estar bloqueados en este equipo: los de las apps
    /// que quieres bloqueadas y, si hay alguno, los DNS cifrados que servirían
    /// para saltarse el filtro.
    pub fn web_domains(&self, serial: &str) -> Vec<String> {
        if !self.web_enabled(serial) {
            return Vec::new();
        }
        let mut out: Vec<String> = crate::catalog::all()
            .into_iter()
            .filter(|a| self.wants(serial, &a.name))
            .flat_map(|a| a.domains)
            .collect();
        if out.iter().any(|d| !d.starts_with('!')) {
            out.extend(crate::catalog::BYPASS.iter().map(|d| d.to_string()));
        } else {
            out.clear();
        }
        out
    }
}

/// Deja el filtro web como dice equipos.json: puesto con esos dominios, o
/// quitado si no queda nada por bloquear. Devuelve cuántos dominios quedan.
pub fn sync_web(serial: &str) -> anyhow::Result<usize> {
    let st = Store::load();
    let domains = st.web_domains(serial);
    if domains.is_empty() {
        let keep = !st.guard_keys(serial).is_empty() || crate::device::guard_state(serial)?.owner;
        crate::device::web_off(serial, keep)?;
    } else {
        crate::device::web_on(serial, &domains)?;
    }
    Ok(domains.iter().filter(|d| !d.starts_with('!')).count())
}

/// Deja el "sin vertical" y la protección máxima como dicen los archivos.
/// Devuelve cuántas apps quedan sin vertical.
pub fn sync_guard(serial: &str) -> anyhow::Result<usize> {
    let st = Store::load();
    let keys = st.guard_keys(serial);
    crate::device::guard_apply(serial, &keys, &st.blocked_packages(serial))?;
    Ok(crate::catalog::all().iter().filter(|a| st.mode(serial, &a.name) == Mode::NoVertical).count())
}

/// Cambia el modo de una app y lo deja así en el celular: primero el
/// guardián (para que suelte lo que deja de estar bloqueado), luego los
/// candados y al final el filtro web. Devuelve los errores por paquete.
/// Los paquetes de una app que van con candado en este modo.
pub fn packages_to_block(app: &crate::catalog::App, mode: Mode) -> Vec<String> {
    match (mode, &app.partial) {
        (Mode::Blocked, _) => app.packages.clone(),
        (Mode::NoVertical, Some(p)) => app.packages.iter().filter(|x| !p.packages.contains(x)).cloned().collect(),
        _ => Vec::new(),
    }
}

pub fn apply_mode(serial: &str, app: &crate::catalog::App, mode: Mode) -> Vec<String> {
    let mut st = Store::load();
    st.set_mode(serial, &app.name, mode);
    let mut errors = Vec::new();
    let lock = packages_to_block(app, mode);
    if mode != Mode::Blocked {
        if let Err(e) = sync_guard(serial) {
            errors.push(format!("{e:#}"));
        }
    }
    for p in &app.packages {
        let r = if lock.contains(p) { crate::device::block(serial, p) } else { crate::device::unblock(serial, p) };
        if let Err(e) = r {
            errors.push(format!("{p}: {e:#}"));
        }
    }
    if mode == Mode::Blocked {
        if let Err(e) = sync_guard(serial) {
            errors.push(format!("{e:#}"));
        }
    }
    if let Err(e) = sync_web(serial) {
        errors.push(format!("filtro web: {e:#}"));
    }
    errors
}
