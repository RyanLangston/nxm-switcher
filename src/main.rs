use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dialoguer::Select;
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const MIME_TYPES: &[&str] = &["x-scheme-handler/nxm", "x-scheme-handler/nxm-protocol"];

const DEFAULT_CONFIG: &str = r#"handlers = [
  { name = "Vortex", desktop = "vortex-steamtinkerlaunch-dl.desktop" },
  { name = "Mod Organizer 2", desktop = "modorganizer2-nxm-handler.desktop" },
  { name = "Nexus Mods App", desktop = "com.nexusmods.app.desktop" },
]
"#;

#[derive(Debug, Deserialize, PartialEq)]
struct Handler {
    name: String,
    desktop: String,
}

#[derive(Debug, Deserialize, PartialEq)]
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
    /// Show the current NXM handler
    Status,
    /// Interactively select a new NXM handler
    Select,
    /// Non-interactively set a handler by name (useful for scripting)
    Set {
        /// Handler name as defined in config (case-insensitive)
        name: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = get_config_path()?;

    match cli.command {
        Commands::Status => cmd_status(&config_path),
        Commands::Select => cmd_select(&config_path),
        Commands::Set { name } => cmd_set(&config_path, &name),
    }
}

fn get_config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    let new_path = config_dir.join("nxm/config.toml");
    let legacy_path = config_dir.join("nxm-handler/config.toml");

    // Migrate from the old config directory if needed
    if !new_path.exists() && legacy_path.exists() {
        let migrated = new_path
            .parent()
            .is_some_and(|p| fs::create_dir_all(p).is_ok())
            && fs::rename(&legacy_path, &new_path).is_ok();

        if migrated {
            eprintln!(
                "ℹ Migrated config from {} to {}",
                legacy_path.display(),
                new_path.display()
            );
        } else {
            eprintln!(
                "⚠ Could not migrate config from {} to {}; using legacy path",
                legacy_path.display(),
                new_path.display()
            );
            return Ok(legacy_path);
        }
    }

    Ok(new_path)
}

fn cmd_status(config_path: &Path) -> Result<()> {
    let handlers = query_current_handlers()?;
    let primary = &handlers[0];
    let secondary = &handlers[1];

    if primary != secondary {
        eprintln!(
            "⚠ MIME types are out of sync:\n  {}: {}\n  {}: {}",
            MIME_TYPES[0], primary, MIME_TYPES[1], secondary
        );
    }

    // Resolve the friendly name from config if available (best-effort)
    let friendly = if config_path.exists() {
        match load_config(config_path) {
            Ok(s) => s
                .handlers
                .into_iter()
                .find(|h| h.desktop == *primary)
                .map(|h| h.name),
            Err(e) => {
                eprintln!("⚠ Could not read config ({}): {e:#}", config_path.display());
                None
            }
        }
    } else {
        None
    };

    if let Some(name) = friendly {
        println!("Current handler: {name} ({primary})");
    } else {
        println!("Current handler: {primary}");
    }

    Ok(())
}

fn cmd_select(config_path: &Path) -> Result<()> {
    ensure_config_exists(config_path)?;
    let settings = load_config(config_path)?;
    warn_missing_desktops(&settings);

    let current_desktop = query_current_handlers()
        .ok()
        .map(|handlers| handlers[0].clone());

    let options: Vec<&str> = settings.handlers.iter().map(|h| h.name.as_str()).collect();

    let default_index = current_desktop
        .as_deref()
        .and_then(|c| settings.handlers.iter().position(|h| h.desktop == c))
        .unwrap_or(0);

    let index = Select::new()
        .with_prompt("Select NXM handler")
        .items(&options)
        .default(default_index)
        .interact()?;

    set_handler(&settings.handlers[index])
}

fn cmd_set(config_path: &Path, name: &str) -> Result<()> {
    ensure_config_exists(config_path)?;
    let settings = load_config(config_path)?;
    warn_missing_desktops(&settings);

    let handler = settings
        .handlers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            let available: Vec<&str> = settings.handlers.iter().map(|h| h.name.as_str()).collect();
            anyhow::anyhow!(
                "No handler named {:?}. Available: {}",
                name,
                available.join(", ")
            )
        })?;

    set_handler(handler)
}

/// Creates the config file with defaults if it does not already exist.
fn ensure_config_exists(config_path: &Path) -> Result<()> {
    if config_path.exists() {
        return Ok(());
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let mut file = fs::File::create(config_path)
        .with_context(|| format!("Failed to create config file at {}", config_path.display()))?;

    file.write_all(DEFAULT_CONFIG.as_bytes())
        .context("Failed to write default config file")?;

    println!("🛠 Created default config at {}", config_path.display());
    Ok(())
}

/// Reads and parses the config file. Does NOT create the file if missing.
fn load_config(config_path: &Path) -> Result<Settings> {
    let contents = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file at {}", config_path.display()))?;
    toml::from_str(&contents).context("Failed to parse config file")
}

/// Emits a warning for any handler whose `.desktop` file cannot be found.
fn warn_missing_desktops(settings: &Settings) {
    let search_paths = desktop_search_dirs();
    for handler in &settings.handlers {
        let found = search_paths
            .iter()
            .any(|dir| dir.join(&handler.desktop).exists());
        if !found {
            eprintln!(
                "⚠ Desktop file not found for {:?}: {} (searched: {})",
                handler.name,
                handler.desktop,
                search_paths
                    .iter()
                    .map(|d| d.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
}

fn desktop_search_dirs() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(local_data) = dirs::data_local_dir() {
        paths.push(local_data.join("applications"));
    }
    paths.push(PathBuf::from("/usr/share/applications"));
    paths.push(PathBuf::from("/usr/local/share/applications"));
    paths
}

/// Queries the current handler for every MIME type in `MIME_TYPES`.
/// Returns one entry per MIME type in the same order.
fn query_current_handlers() -> Result<Vec<String>> {
    MIME_TYPES
        .iter()
        .map(|mime| {
            let output = Command::new("xdg-mime")
                .arg("query")
                .arg("default")
                .arg(mime)
                .env_remove("XDG_CURRENT_DESKTOP") // prevents qtpaths dependency
                .output()
                .with_context(|| format!("Failed to execute xdg-mime for {mime}"))?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if stderr.is_empty() {
                    Err(anyhow::anyhow!("xdg-mime query failed for {mime}"))
                } else {
                    Err(anyhow::anyhow!(
                        "xdg-mime query failed for {mime}: {stderr}"
                    ))
                }
            }
        })
        .collect()
}

/// Sets the given handler for all MIME types. Collects all errors and reports
/// them together so a partial failure is clearly visible.
fn set_handler(handler: &Handler) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    for mime in MIME_TYPES {
        let result = Command::new("xdg-mime")
            .arg("default")
            .arg(&handler.desktop)
            .arg(mime)
            .env_remove("XDG_CURRENT_DESKTOP") // prevents qtpaths dependency
            .status()
            .with_context(|| format!("Failed to invoke xdg-mime for {mime}"));

        match result {
            Ok(s) if s.success() => {}
            Ok(_) => errors.push(format!("xdg-mime failed to set handler for {mime}")),
            Err(e) => errors.push(format!("{e:#}")),
        }
    }

    if errors.is_empty() {
        println!("✅ Handler set to: {}", handler.name);
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Failed to set handler for some MIME types:\n  {}",
            errors.join("\n  ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CONFIG: &str = r#"
handlers = [
  { name = "Vortex", desktop = "vortex-steamtinkerlaunch-dl.desktop" },
  { name = "Mod Organizer 2", desktop = "modorganizer2-nxm-handler.desktop" },
  { name = "Nexus Mods App", desktop = "com.nexusmods.app.desktop" },
]
"#;

    fn sample_settings() -> Settings {
        toml::from_str(SAMPLE_CONFIG).expect("sample config must parse")
    }

    // --- Config parsing ---

    #[test]
    fn test_parse_config_handler_count() {
        let settings = sample_settings();
        assert_eq!(settings.handlers.len(), 3);
    }

    #[test]
    fn test_parse_config_handler_fields() {
        let settings = sample_settings();
        assert_eq!(settings.handlers[0].name, "Vortex");
        assert_eq!(
            settings.handlers[0].desktop,
            "vortex-steamtinkerlaunch-dl.desktop"
        );
        assert_eq!(settings.handlers[1].name, "Mod Organizer 2");
        assert_eq!(
            settings.handlers[1].desktop,
            "modorganizer2-nxm-handler.desktop"
        );
    }

    #[test]
    fn test_parse_default_config() {
        let settings: Settings = toml::from_str(DEFAULT_CONFIG).expect("default config must parse");
        assert_eq!(settings.handlers.len(), 3);
    }

    #[test]
    fn test_parse_config_missing_field_errors() {
        let bad = r#"handlers = [{ name = "Vortex" }]"#;
        let result: Result<Settings, _> = toml::from_str(bad);
        assert!(result.is_err(), "missing desktop field should fail parsing");
    }

    #[test]
    fn test_parse_config_empty_handlers() {
        let cfg = "handlers = []\n";
        let settings: Settings = toml::from_str(cfg).expect("empty handlers should parse");
        assert!(settings.handlers.is_empty());
    }

    // --- Handler lookup (used by cmd_set) ---

    #[test]
    fn test_find_handler_exact_name() {
        let settings = sample_settings();
        let found = settings
            .handlers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("Mod Organizer 2"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().desktop, "modorganizer2-nxm-handler.desktop");
    }

    #[test]
    fn test_find_handler_case_insensitive() {
        let settings = sample_settings();
        let found = settings
            .handlers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("vortex"));
        assert!(found.is_some());
        assert_eq!(
            found.unwrap().desktop,
            "vortex-steamtinkerlaunch-dl.desktop"
        );
    }

    #[test]
    fn test_find_handler_missing_returns_none() {
        let settings = sample_settings();
        let found = settings
            .handlers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("NonExistent"));
        assert!(found.is_none());
    }

    // --- Handler lookup by desktop file (used by cmd_status) ---

    #[test]
    fn test_resolve_friendly_name_from_desktop() {
        let settings = sample_settings();
        let desktop = "com.nexusmods.app.desktop";
        let name = settings
            .handlers
            .iter()
            .find(|h| h.desktop == desktop)
            .map(|h| h.name.as_str());
        assert_eq!(name, Some("Nexus Mods App"));
    }

    #[test]
    fn test_resolve_friendly_name_unknown_desktop() {
        let settings = sample_settings();
        let name = settings
            .handlers
            .iter()
            .find(|h| h.desktop == "unknown.desktop")
            .map(|h| h.name.as_str());
        assert_eq!(name, None);
    }

    // --- Default index for select (pre-highlighting) ---

    #[test]
    fn test_default_index_matches_current() {
        let settings = sample_settings();
        let current = "modorganizer2-nxm-handler.desktop";
        let index = settings
            .handlers
            .iter()
            .position(|h| h.desktop == current)
            .unwrap_or(0);
        assert_eq!(index, 1);
    }

    #[test]
    fn test_default_index_falls_back_to_zero_when_not_found() {
        let settings = sample_settings();
        let current = "unknown.desktop";
        let index = settings
            .handlers
            .iter()
            .position(|h| h.desktop == current)
            .unwrap_or(0);
        assert_eq!(index, 0);
    }

    // --- ensure_config_exists ---

    #[test]
    fn test_ensure_config_creates_file_with_default_content() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("nxm/config.toml");

        assert!(!config_path.exists());
        ensure_config_exists(&config_path).expect("should succeed");
        assert!(config_path.exists());

        let contents = fs::read_to_string(&config_path).expect("read");
        let settings: Settings = toml::from_str(&contents).expect("should be valid TOML");
        assert!(!settings.handlers.is_empty());
    }

    #[test]
    fn test_ensure_config_is_idempotent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("config.toml");

        fs::write(&config_path, "handlers = []\n").expect("write");
        ensure_config_exists(&config_path).expect("should succeed");

        // File should NOT be overwritten
        let contents = fs::read_to_string(&config_path).expect("read");
        assert_eq!(contents, "handlers = []\n");
    }
}
