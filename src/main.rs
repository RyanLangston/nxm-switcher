// Current code is written by chatgpt, I plan to redo it
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config as config_rs;
use dialoguer::Select;
use serde::Deserialize;
use std::process::Command;

const MIME_TYPES: &[&str] = &["x-scheme-handler/nxm", "x-scheme-handler/nxm-protocol"];

#[derive(Debug, Deserialize)]
struct Handler {
    name: String,
    desktop: String,
}

#[derive(Debug, Deserialize)]
struct Settings {
    handlers: Vec<Handler>,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show the current handler
    Status,
    /// Select a new handler
    Select,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let settings = load_config()?;

    match cli.command {
        Commands::Status => {
            let current = get_current_handler()?;
            println!("Current handler: {}", current);
        }
        Commands::Select => {
            let options: Vec<String> = settings.handlers.iter().map(|h| h.name.clone()).collect();

            let index = Select::new()
                .with_prompt("Select NXM handler")
                .items(&options)
                .default(0)
                .interact()?;

            let handler = &settings.handlers[index];
            set_handler(handler)?;
        }
    }

    Ok(())
}

fn load_config() -> Result<Settings> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

    let config_path = config_dir.join("nxm-handler/config.toml");

    ensure_config_exists(&config_path)?;

    let settings = config_rs::Config::builder()
        .add_source(config_rs::File::from(config_path))
        .build()
        .context("Failed to read config file")?;

    settings
        .try_deserialize::<Settings>()
        .context("Failed to parse config file")
}

fn ensure_config_exists(config_path: &std::path::Path) -> Result<()> {
    use std::fs;
    use std::io::Write;

    if config_path.exists() {
        return Ok(());
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let mut file = fs::File::create(config_path)
        .with_context(|| format!("Failed to create config file at {}", config_path.display()))?;

    let default_config = r#"
handlers = [
  { name = "Vortex", desktop = "vortex-steamtinkerlaunch-dl.desktop" },
  { name = "Mod Organizer 2", desktop = "modorganizer2-nxm-handler.desktop" },
  { name = "Nexus Mods App", desktop = "com.nexusmods.app.desktop" },
]
"#;

    file.write_all(default_config.trim_start().as_bytes())
        .context("Failed to write default config file")?;

    println!("🛠 Created default config at {}", config_path.display());
    Ok(())
}

fn set_handler(handler: &Handler) -> Result<()> {
    for mime in MIME_TYPES {
        let status = Command::new("xdg-mime")
            .arg("default")
            .arg(&handler.desktop)
            .arg(mime)
            .env_remove("XDG_CURRENT_DESKTOP") // prevents qtpaths dependency
            .status()
            .with_context(|| format!("Failed to set handler for {}", mime))?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "xdg-mime failed to set handler for {}",
                mime
            ));
        }
    }

    println!("✅ Handler set to: {}", handler.name);
    Ok(())
}

fn get_current_handler() -> Result<String> {
    let output = Command::new("xdg-mime")
        .arg("query")
        .arg("default")
        .arg(MIME_TYPES[0])
        .env_remove("XDG_CURRENT_DESKTOP")
        .output()
        .context("Failed to execute xdg-mime")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(anyhow::anyhow!("xdg-mime query failed"))
    }
}
