use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Default)]
pub struct Config {
    pub uvinit: UvinitConfig,
    pub cargonew: CargonewConfig,
    pub tuarinew: TuarinewConfig,
}

#[derive(Deserialize, Serialize)]
pub struct UvinitConfig {
    /// Directories to skip during search
    #[serde(default = "default_skip_dirs")]
    pub skip_dirs: Vec<String>,
    /// Whether to add hatch-vcs to build-system.requires
    #[serde(default = "default_true")]
    pub add_hatch_vcs: bool,
    /// Whether to set dynamic versioning
    #[serde(default = "default_true")]
    pub enable_dynamic_version: bool,
    /// Additional build system requirements
    #[serde(default)]
    pub additional_requires: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct CargonewConfig {
    /// Default template for new Cargo projects
    #[serde(default = "default_cargo_template")]
    pub default_template: String,
    /// Whether to initialize git repository
    #[serde(default = "default_true")]
    pub init_git: bool,
}

#[derive(Deserialize, Serialize)]
pub struct TuarinewConfig {
    /// Default frontend framework
    #[serde(default = "default_tauri_frontend")]
    pub default_frontend: String,
    /// Whether to use TypeScript
    #[serde(default = "default_true")]
    pub use_typescript: bool,
}

impl Default for UvinitConfig {
    fn default() -> Self {
        Self {
            skip_dirs: default_skip_dirs(),
            add_hatch_vcs: true,
            enable_dynamic_version: true,
            additional_requires: Vec::new(),
        }
    }
}

impl Default for CargonewConfig {
    fn default() -> Self {
        Self {
            default_template: default_cargo_template(),
            init_git: true,
        }
    }
}

impl Default for TuarinewConfig {
    fn default() -> Self {
        Self {
            default_frontend: default_tauri_frontend(),
            use_typescript: true,
        }
    }
}

fn default_skip_dirs() -> Vec<String> {
    vec![
        ".git".to_string(),
        ".venv".to_string(),
        "venv".to_string(),
        "__pycache__".to_string(),
        ".pytest_cache".to_string(),
        "node_modules".to_string(),
        ".tox".to_string(),
        "build".to_string(),
        "dist".to_string(),
        ".eggs".to_string(),
        "target".to_string(),
    ]
}

fn default_true() -> bool {
    true
}
fn default_cargo_template() -> String {
    "bin".to_string()
}
fn default_tauri_frontend() -> String {
    "vanilla".to_string()
}

pub fn get_config_path() -> Result<PathBuf> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    Ok(home_dir.join(".config").join("post-init.toml"))
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        // Create default config if it doesn't exist
        let default_config = Config::default();
        save_config(&default_config)?;
        return Ok(default_config);
    }

    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    let config: Config = toml::from_str(&content).with_context(|| "Failed to parse config file")?;

    Ok(config)
}

pub fn save_config(config: &Config) -> Result<()> {
    let config_path = get_config_path()?;

    // Create .config directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let content = toml::to_string_pretty(config).with_context(|| "Failed to serialize config")?;

    fs::write(&config_path, content)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() -> Result<()> {
        let config = Config::default();
        assert!(config.uvinit.enable_dynamic_version);
        assert!(config.uvinit.add_hatch_vcs);
        assert!(!config.uvinit.skip_dirs.is_empty());
        Ok(())
    }
}
