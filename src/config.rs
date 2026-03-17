use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub telegram: TelegramConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub group_id: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TopicsCache {
    #[serde(default)]
    pub topics: HashMap<String, i64>,
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

fn cache_dir() -> Result<PathBuf> {
    let dir = std::env::var("XDG_CACHE_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| home::home_dir().map(|h| h.join(".cache")))
        .context("could not determine home directory")?
        .join("pygmy");
    Ok(dir)
}

fn topics_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("topics.toml"))
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    let data = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "could not read config at {}\nRun `pygmy init` to set up.",
            path.display()
        )
    })?;
    toml::from_str(&data).context("invalid config format")
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

pub fn load_topics() -> TopicsCache {
    topics_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|d| toml::from_str(&d).ok())
        .unwrap_or_default()
}

pub fn save_topics(cache: &TopicsCache) -> Result<()> {
    let path = topics_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = toml::to_string_pretty(cache)?;
    std::fs::write(&path, data)?;
    Ok(())
}

pub fn config_dir_display() -> String {
    config_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~/.config/pygmy".to_string())
}

pub fn cache_dir_display() -> String {
    cache_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~/.cache/pygmy".to_string())
}
