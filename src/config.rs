use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub telegram: Option<TelegramConfig>,
    pub discord_webhook: Option<DiscordWebhookConfig>,
    pub ntfy: Option<NtfyConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token: String,
    pub channel_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordWebhookConfig {
    pub enabled: bool,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NtfyConfig {
    pub enabled: bool,
    pub server: String,
    pub topic: String,
    pub token: Option<String>,
}

fn config_dir() -> Result<PathBuf> {
    let dir = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| home::home_dir().map(|h| h.join(".config")))
        .context("could not determine home directory")?
        .join("pygmy");
    Ok(dir)
}

fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    let data = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "could not read config at {}\nRun `pygmy init <backend>` to set up.",
            path.display()
        )
    })?;
    toml::from_str(&data).context("invalid config format")
}

/// Load existing config or return an empty default (for init flows that merge).
pub fn load_config_or_default() -> Config {
    load_config().unwrap_or_default()
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = toml::to_string_pretty(config)?;
    std::fs::write(&path, data)?;
    Ok(())
}

pub fn config_dir_display() -> String {
    config_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~/.config/pygmy".to_string())
}

/// Whether a backend is set up, and if so whether it's enabled.
#[derive(Debug, Clone, PartialEq)]
pub enum BackendState {
    /// Enabled, with a short human-readable detail (e.g. "channel: -100123").
    Enabled(String),
    Disabled,
    NotConfigured,
}

/// The state of every backend, in a fixed order, keyed by its CLI name.
/// Shared by `pygmy status` and the `pygmy usage` footer.
pub fn backend_states(cfg: &Config) -> [(&'static str, BackendState); 3] {
    fn state<T>(
        section: Option<&T>,
        enabled: impl Fn(&T) -> bool,
        detail: impl Fn(&T) -> String,
    ) -> BackendState {
        match section {
            Some(s) if enabled(s) => BackendState::Enabled(detail(s)),
            Some(_) => BackendState::Disabled,
            None => BackendState::NotConfigured,
        }
    }
    [
        (
            "telegram",
            state(
                cfg.telegram.as_ref(),
                |t| t.enabled,
                |t| format!("channel: {}", t.channel_id),
            ),
        ),
        (
            "discord-webhook",
            state(
                cfg.discord_webhook.as_ref(),
                |d| d.enabled,
                |_| "webhook configured".to_string(),
            ),
        ),
        (
            "ntfy",
            state(
                cfg.ntfy.as_ref(),
                |n| n.enabled,
                |n| format!("topic: {}, server: {}", n.topic, n.server),
            ),
        ),
    ]
}
