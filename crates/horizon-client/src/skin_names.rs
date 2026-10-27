//! Skins by nickname in offline play: the local player wears the skin of a
//! Mojang account named in Skin Customization (`horizon-skin.txt`), as an
//! online-mode server would give it. Single player's server is offline, so
//! its profile has no `textures` property and every player wore a default
//! skin. On login of the world's owner the server attaches that account's
//! signed `textures` property, looked up by name on Mojang's API; the client
//! then downloads and draws the skin as for any profile (`skins`).
//!
//! The last textures found are kept in the file too, for play without the
//! internet.
use horizon_protocol::packets::login::Property;
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

pub const FILE: &str = "horizon-skin.txt";

#[derive(Default)]
struct State {
    dir: Option<PathBuf>,
    /// The nickname whose skin to wear; empty for the default skin.
    name: String,
    /// The single player owner's own player name (`set_owner`).
    owner: Option<String>,
    /// The textures last found for `name`.
    textures: Option<Property>,
    /// `name`'s lookup is done (found or not) for this run.
    looked_up: bool,
}

static STATE: Mutex<State> = Mutex::new(State {
    dir: None,
    name: String::new(),
    owner: None,
    textures: None,
    looked_up: false,
});

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Reads `horizon-skin.txt` in `dir` (the game's folder) once.
pub fn load(dir: &Path) {
    let mut s = state();
    if s.dir.is_some() {
        return;
    }
    s.dir = Some(dir.to_owned());
    let Ok(text) = std::fs::read_to_string(dir.join(FILE)) else {
        return;
    };
    let mut value = None;
    let mut signature = None;
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("name:") {
            s.name = name.trim().to_owned();
        } else if let Some(v) = line.strip_prefix("textures:") {
            value = Some(v.trim().to_owned());
        } else if let Some(v) = line.strip_prefix("signature:") {
            signature = Some(v.trim().to_owned());
        }
    }
    s.textures = value.map(|value| Property {
        name: "textures".into(),
        value,
        signature,
    });
}

fn save(s: &State) {
    let Some(dir) = &s.dir else { return };
    let mut text = format!("name:{}\n", s.name);
    if let Some(t) = &s.textures {
        text.push_str(&format!("textures:{}\n", t.value));
        if let Some(sig) = &t.signature {
            text.push_str(&format!("signature:{sig}\n"));
        }
    }
    if let Err(e) = std::fs::write(dir.join(FILE), text) {
        log::warn!("Horizon: could not keep the skin name: {e}");
    }
}

/// The nickname whose skin the local player wears.
pub fn name() -> String {
    state().name.clone()
}

/// Wear `name`'s skin from the next world opened; empty for the default.
pub fn set_name(name: &str) {
    let mut s = state();
    let name = name.trim();
    if s.name == name {
        return;
    }
    s.name = name.to_owned();
    s.textures = None;
    s.looked_up = false;
    save(&s);
}

/// The single player owner, whose login gets the skin (the integrated server
/// sets it as it starts).
pub fn set_owner(player: &str) {
    state().owner = Some(player.to_owned());
}

/// The profile properties for an offline login of `player`: the skin's
/// `textures` for the owner, none for anyone else. May wait a few seconds for
/// Mojang's API the first time in a run.
pub fn login_properties(player: &str) -> Vec<Property> {
    let name = {
        let s = state();
        if s.owner.as_deref() != Some(player) || s.name.is_empty() {
            return Vec::new();
        }
        if s.looked_up {
            return s.textures.clone().into_iter().collect();
        }
        s.name.clone()
    };
    let found = lookup(&name);
    let mut s = state();
    if s.name == name {
        s.looked_up = true;
        match found {
            Some(textures) => {
                s.textures = Some(textures);
                save(&s);
            }
            // Offline or not found: the textures kept from last time, if any.
            None => log::warn!("Horizon: no skin found for {name}; keeping the last one"),
        }
    }
    s.textures.clone().into_iter().collect()
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(6))
        .build()
}

/// `name`'s signed `textures` property from Mojang's API; why not, in the log.
fn lookup(name: &str) -> Option<Property> {
    let get = |url: &str| -> Option<String> {
        match agent().get(url).call() {
            Ok(response) => response.into_string().ok(),
            Err(ureq::Error::Status(code, response)) => {
                let server = response.header("server").unwrap_or("").to_owned();
                let cf = response.header("cf-ray").unwrap_or("").to_owned();
                let body = response.into_string().unwrap_or_default();
                let body: String = body.chars().take(300).collect();
                log::warn!(
                    "Horizon: skin lookup: {url}: status {code} (server {server:?}, cf-ray {cf:?}): {body:?}"
                );
                None
            }
            Err(e) => {
                log::warn!("Horizon: skin lookup: {url}: {e}");
                None
            }
        }
    };
    let body = get(&format!(
        "https://api.mojang.com/users/profiles/minecraft/{}",
        urlencode(name)
    ))?;
    let profile: serde_json::Value = serde_json::from_str(&body).ok()?;
    let Some(id) = profile.get("id").and_then(|v| v.as_str()) else {
        log::warn!("Horizon: skin lookup: no account named {name}");
        return None;
    };
    let body = get(&format!(
        "https://sessionserver.mojang.com/session/minecraft/profile/{id}?unsigned=false"
    ))?;
    let found = crate::auth::parse_joined(&body)?
        .properties
        .into_iter()
        .find(|p| p.name == "textures");
    match &found {
        Some(_) => log::info!("Horizon: wearing {name}'s skin"),
        None => log::warn!("Horizon: skin lookup: {name} has no textures"),
    }
    found
}

fn urlencode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The textures the local player wears in single player, when found: the client takes them
/// from here too, not only from its server's player list.
pub fn own_textures() -> Option<Property> {
    let s = state();
    if s.name.is_empty() {
        return None;
    }
    s.textures.clone()
}
