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
    /// Si se puede dejar la app y quitarle solo los videos cortos.
    #[serde(default)]
    pub partial: Option<Partial>,
}

/// El modo «solo los Shorts»: la app se deja usar y el vigía del celular
/// (android/…/Vigia.java) cierra el reproductor de videos cortos y tapa su
/// pestaña. Sus páginas web no se bloquean: la app las necesita.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Partial {
    /// Lo que se quita, para los textos: "Shorts", "Reels".
    pub what: String,
    /// El nombre que entiende el vigía.
    pub key: String,
    /// Los paquetes que se dejan usar; los demás de la app (Lite…) se bloquean.
    pub packages: Vec<String>,
    /// Algo más que puede quitar el vigía en este modo (los directos de YouTube).
    #[serde(default)]
    pub extra: Option<Extra>,
    /// Solo existe este modo: la app nunca se bloquea entera (WhatsApp, LinkedIn).
    #[serde(default)]
    pub only: bool,
}

/// Una opción del modo «solo los Shorts», con su interruptor en la ventana.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Extra {
    /// Lo que entiende el vigía ("youtube-directos").
    pub key: String,
    /// El texto de la casilla.
    pub label: String,
}

fn app(name: &str, hint: &str, packages: &[&str]) -> App {
    App {
        name: name.into(),
        hint: hint.into(),
        packages: packages.iter().map(|p| p.to_string()).collect(),
        domains: Vec::new(),
        partial: None,
    }
}

impl App {
    fn web(mut self, domains: &[&str]) -> App {
        self.domains = domains.iter().map(|d| d.to_string()).collect();
        self
    }

    fn partial(mut self, what: &str, key: &str, packages: &[&str]) -> App {
        self.partial = Some(Partial {
            what: what.into(),
            key: key.into(),
            packages: packages.iter().map(|p| p.to_string()).collect(),
            extra: None,
            only: false,
        });
        self
    }

    /// Sin «Toda la app»: solo se le quitan los videos o historias.
    fn only_partial(mut self) -> App {
        if let Some(p) = self.partial.as_mut() {
            p.only = true;
        }
        self
    }

    /// Si se puede dejar la app quitándole solo los videos cortos (modo
    /// «sin vertical», lo hace el vigía).
    pub fn has_vertical(&self) -> bool {
        self.partial.is_some()
    }

    /// Lo que quita el modo «sin vertical», para los textos ("Shorts").
    pub fn vertical_what(&self) -> &str {
        self.partial.as_ref().map_or(self.hint.as_str(), |p| p.what.as_str())
    }

    /// Las que nunca se bloquean enteras (WhatsApp, LinkedIn).
    pub fn never_blocked(&self) -> bool {
        self.partial.as_ref().is_some_and(|p| p.only)
    }

    fn extra(mut self, key: &str, label: &str) -> App {
        if let Some(p) = self.partial.as_mut() {
            p.extra = Some(Extra { key: key.into(), label: label.into() });
        }
        self
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
        app("YouTube", "Shorts", &["com.google.android.youtube"]).partial("Shorts", "youtube", &["com.google.android.youtube"])
            // Con «No me interesa» en cada directo del feed.
            .extra("youtube-directos", "Quitar también los directos"),
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
            .web(&["instagram.com", "cdninstagram.com", "instagr.am", "ig.me"])
            .partial("Reels", "instagram", &["com.instagram.android"]),
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
        // Estas se dejan siempre: solo se les quitan las historias y los videos.
        app("WhatsApp", "Estados · los chats siguen igual", &["com.whatsapp", "com.whatsapp.w4b"])
            .partial("Estados", "whatsapp", &["com.whatsapp", "com.whatsapp.w4b"])
            .only_partial(),
        app("LinkedIn", "Feed y Videos · mensajes, empleos y lo demás siguen igual", &["com.linkedin.android"])
            .partial("feed", "linkedin", &["com.linkedin.android"])
            .only_partial(),
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
pub fn all() -> Vec<App> {
    let mut apps = builtin();
    apps.extend(load_extra());
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
