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
        Store { wanted, web_off, no_vertical }
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

    /// Para el celular: "pkg:id1,id2;pkg2:…" de las apps en "sin vertical".
    pub fn vertical_rules(&self, serial: &str) -> String {
        crate::catalog::all()
            .into_iter()
            .filter(|a| a.has_vertical() && self.mode(serial, &a.name) == Mode::NoVertical)
            .flat_map(|a| {
                let ids = a.vertical.join(",");
                a.packages.into_iter().map(move |p| format!("{p}:{ids}"))
            })
            .collect::<Vec<_>>()
            .join(";")
    }

    /// Los paquetes que deben estar bloqueados en este equipo.
    pub fn blocked_packages(&self, serial: &str) -> Vec<String> {
        crate::catalog::all()
            .into_iter()
            .filter(|a| self.wants(serial, &a.name))
            .flat_map(|a| a.packages)
            .collect()
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
        let keep = !st.vertical_rules(serial).is_empty() || crate::device::guard_state(serial)?.owner;
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
    let rules = st.vertical_rules(serial);
    crate::device::guard_apply(serial, &rules, &st.blocked_packages(serial))?;
    Ok(if rules.is_empty() { 0 } else { rules.split(';').count() })
}

/// Cambia el modo de una app y lo deja así en el celular: primero el
/// guardián (para que suelte lo que deja de estar bloqueado), luego los
/// candados y al final el filtro web. Devuelve los errores por paquete.
pub fn apply_mode(serial: &str, app: &crate::catalog::App, mode: Mode) -> Vec<String> {
    let mut st = Store::load();
    st.set_mode(serial, &app.name, mode);
    let mut errors = Vec::new();
    if mode != Mode::Blocked {
        if let Err(e) = sync_guard(serial) {
            errors.push(format!("{e:#}"));
        }
    }
    for p in &app.packages {
        let r = if mode == Mode::Blocked { crate::device::block(serial, p) } else { crate::device::unblock(serial, p) };
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
