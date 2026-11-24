//! Las apps que Cerrojo conoce. Cada una puede tener varios paquetes
//! (versión Lite, versión de otra región, clientes parcheados…): bloquearla
//! bloquea todos.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct App {
    pub name: String,
    pub hint: String,
    pub packages: Vec<String>,
    /// Sus páginas web: al bloquear la app también se bloquean en el
    /// navegador (filtro DNS, ver device.rs). Cuenta el dominio y todos sus
    /// subdominios; con "!" delante, una excepción.
    #[serde(default)]
    pub domains: Vec<String>,
    /// Vistas (resource-id) que solo aparecen en su video vertical: con la
    /// app en "sin vertical", el celular te saca en cuanto se ve una. Se
    /// sacan con `cerrojo vistas` en esa pantalla. Vacío: no hay ese modo.
    #[serde(default)]
    pub vertical: Vec<String>,
}

fn app(name: &str, hint: &str, packages: &[&str]) -> App {
    App {
        name: name.into(),
        hint: hint.into(),
        packages: packages.iter().map(|p| p.to_string()).collect(),
        domains: Vec::new(),
        vertical: Vec::new(),
    }
}

impl App {
    fn web(mut self, domains: &[&str]) -> App {
        self.domains = domains.iter().map(|d| d.to_string()).collect();
        self
    }

    fn vertical(mut self, ids: &[&str]) -> App {
        self.vertical = ids.iter().map(|d| d.to_string()).collect();
        self
    }

    /// Si se puede dejar la app quitándole solo el video vertical.
    pub fn has_vertical(&self) -> bool {
        !self.vertical.is_empty()
    }
}

/// Servidores de DNS cifrado (DoH) conocidos: con el filtro puesto, un
/// navegador configurado para usarlos se lo saltaría.
pub const BYPASS: &[&str] = &[
    "dns.google", "dns.google.com", "cloudflare-dns.com", "dns.nextdns.io", "doh.opendns.com",
    "dns.quad9.net", "dns.adguard-dns.com", "dns.adguard.com", "doh.cleanbrowsing.org",
    "dns.controld.com", "freedns.controld.com", "doh.mullvad.net", "dns.mullvad.net",
];

pub fn builtin() -> Vec<App> {
    vec![
        // Vistas sacadas del A36 con YouTube 21.45 (Shorts abierto).
        app("YouTube", "Shorts", &["com.google.android.youtube"]).vertical(&["reel_watch_player", "reel_recycler"]),
        app("YouTube Music", "Samples", &["com.google.android.apps.youtube.music"]),
        app(
            "YouTube parcheado",
            "Morphe, ReVanced",
            &[
                "app.morphe.android.youtube",
                "app.morphe.android.apps.youtube.music",
                "app.morphe.manager",
                "app.revanced.android.youtube",
                "app.revanced.android.apps.youtube.music",
                "app.revanced.manager.flutter",
            ],
        ),
        // YouTube y YouTube Music solo como app: la web se deja.
        app("TikTok", "Para ti", &["com.zhiliaoapp.musically", "com.zhiliaoapp.musically.go", "com.ss.android.ugc.trill"])
            .web(&["tiktok.com", "tiktokv.com", "tiktokcdn.com", "tiktokcdn-us.com", "tiktokw.us", "musical.ly"]),
        app("Instagram", "Reels", &["com.instagram.android", "com.instagram.lite"])
            .web(&["instagram.com", "cdninstagram.com", "instagr.am", "ig.me"]),
        app("Threads", "Videos en el feed", &["com.instagram.barcelona"]).web(&["threads.net", "threads.com"]),
        // graph.facebook.com se deja: lo usan WhatsApp Business y los
        // inicios de sesión con Facebook de otras apps.
        app("Facebook", "Reels", &["com.facebook.katana", "com.facebook.lite"])
            .web(&["facebook.com", "fb.com", "fb.me", "fb.watch", "m.me", "messenger.com", "!graph.facebook.com"]),
        app("Snapchat", "Spotlight", &["com.snapchat.android"]).web(&["snapchat.com", "sc-cdn.net"]),
        app("Pinterest", "Videos", &["com.pinterest"]).web(&[
            "pinterest.com", "pinimg.com", "pin.it", "pinterest.es", "pinterest.com.mx", "pinterest.cl",
            "pinterest.co.uk", "pinterest.ca", "pinterest.fr", "pinterest.de",
        ]),
        app("Kwai", "Videos cortos", &["com.kwai.video"]).web(&["kwai.com", "kwai.net", "kwaicdn.com", "kwai-pro.com"]),
        app("Likee", "Videos cortos", &["video.like"]).web(&["likee.video", "like.video", "likeevideo.com"]),
        app("Snaptube", "Cliente de YouTube", &["com.snaptube.premium"]).web(&["snaptube.app", "snaptubeapp.com"]),
    ]
}

pub fn config_dir() -> PathBuf {
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(x).join("cerrojo");
    }
    if cfg!(windows) {
        if let Some(app) = std::env::var_os("APPDATA") {
            return PathBuf::from(app).join("Cerrojo");
        }
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).unwrap_or_default();
    if cfg!(target_os = "macos") {
        return PathBuf::from(home).join("Library/Application Support/Cerrojo");
    }
    PathBuf::from(home).join(".config/cerrojo")
}

/// Las del catálogo más las que añadiste tú (~/.config/cerrojo/extra.json).
/// En ~/.config/cerrojo/vertical.json ({"Instagram": ["clips_viewer_view_pager"]})
/// se pueden poner o corregir las vistas de video vertical sin esperar a una
/// versión nueva de Cerrojo.
pub fn all() -> Vec<App> {
    let mut apps = builtin();
    apps.extend(load_extra());
    let extra_vertical: std::collections::HashMap<String, Vec<String>> = std::fs::read(config_dir().join("vertical.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    for a in &mut apps {
        if let Some(v) = extra_vertical.get(&a.name) {
            a.vertical = v.clone();
        }
    }
    apps
}

pub fn load_extra() -> Vec<App> {
    std::fs::read(config_dir().join("extra.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn add_extra(package: &str, name: &str) -> anyhow::Result<()> {
    let mut extra = load_extra();
    if all().iter().any(|a| a.packages.iter().any(|p| p == package)) {
        return Ok(());
    }
    extra.push(app(name, package, &[package]));
    std::fs::create_dir_all(config_dir())?;
    std::fs::write(config_dir().join("extra.json"), serde_json::to_vec_pretty(&extra)?)?;
    Ok(())
}
