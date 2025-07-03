use anyhow::{Context, Result};

use crate::config::*;

pub fn show_config(show_path: bool) -> Result<()> {
    let config_path = get_config_path()?;

    if show_path {
        println!("📄 Config file: {}", config_path.display());
        return Ok(());
    }

    let config = load_config()?;
    let config_str =
        toml::to_string_pretty(&config).with_context(|| "Failed to serialize config")?;

    println!("📄 Current configuration:");
    println!("{}", config_str);

    Ok(())
}
