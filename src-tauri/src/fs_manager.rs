use crate::data_models::{Config, HistoryItem};
use anyhow::Context;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use tauri::AppHandle;

const CONFIG_FILENAME: &str = "config.json";
const HISTORY_FILENAME: &str = "history.json";
const PICTURES_DIRNAME: &str = "pictures";

/// Gets the path to the specified data file within the app's data directory.
/// Ensures the directory exists.
pub fn get_data_file_path(app_handle: &AppHandle, filename: &str) -> Result<PathBuf, anyhow::Error> {
    let app_data_dir = app_handle
        .path_resolver()
        .app_data_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve app data directory."))?;

    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir).context(format!(
            "Failed to create app data directory at {:?}",
            app_data_dir
        ))?;
    }

    Ok(app_data_dir.join(filename))
}

/// Ensures and returns the pictures directory inside app data dir
pub fn ensure_pictures_dir(app_handle: &AppHandle) -> Result<PathBuf, anyhow::Error> {
    let base = app_handle
        .path_resolver()
        .app_data_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve app data directory."))?;

    if !base.exists() {
        fs::create_dir_all(&base).context(format!(
            "Failed to create app data directory at {:?}",
            base
        ))?;
    }

    let pictures_dir = base.join(PICTURES_DIRNAME);
    if !pictures_dir.exists() {
        fs::create_dir_all(&pictures_dir).context(format!(
            "Failed to create pictures directory at {:?}",
            pictures_dir
        ))?;
    }
    Ok(pictures_dir)
}

/// Saves PNG bytes to the pictures directory with the given stem (without extension)
pub fn save_png_to_pictures(
    app_handle: &AppHandle,
    file_stem: &str,
    png_bytes: &[u8],
) -> Result<PathBuf, anyhow::Error> {
    let dir = ensure_pictures_dir(app_handle)?;
    let path = dir.join(format!("{}.png", file_stem));
    let file = File::create(&path).context("Failed to create image file")?;
    let mut writer = BufWriter::new(file);
    writer.write_all(png_bytes).context("Failed to write image bytes")?;
    Ok(path)
}

/// Reads the application configuration from `config.json`.
///
/// If the file does not exist or cannot be deserialized (e.g., missing new fields),
/// it returns the default configuration and updates the file.
pub fn read_config(app_handle: &AppHandle) -> Result<Config, anyhow::Error> {
    let config_path = get_data_file_path(app_handle, CONFIG_FILENAME)?;

    match File::open(&config_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            match serde_json::from_reader::<_, Config>(reader) {
                Ok(mut config) => {
                    // 迁移旧提示词为新版默认（仅在检测到旧文案或为空时）
                    if config.migrate_prompts() {
                        let _ = write_config(app_handle, &config);
                    }
                    Ok(config)
                },
                Err(_) => {
                    // Failed to deserialize (likely due to missing fields in old config)
                    // Use default config and update the file
                    let default_config = Config::default();
                    if let Err(e) = write_config(app_handle, &default_config) {
                        eprintln!("Warning: Failed to update config file: {}", e);
                    }
                    Ok(default_config)
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // File doesn't exist, create with default config
            let default_config = Config::default();
            if let Err(e) = write_config(app_handle, &default_config) {
                eprintln!("Warning: Failed to create config file: {}", e);
            }
            Ok(default_config)
        }
        Err(e) => {
            // Other I/O error
            Err(anyhow::Error::new(e).context("Failed to read config.json"))
        }
    }
}

/// Writes the application configuration to `config.json`.
pub fn write_config(app_handle: &AppHandle, config: &Config) -> Result<(), anyhow::Error> {
    let config_path = get_data_file_path(app_handle, CONFIG_FILENAME)?;
    let file = File::create(config_path).context("Failed to create or truncate config.json")?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, config).context("Failed to serialize and write config")?;
    Ok(())
}

/// Reads the recognition history from `history.json`.
///
/// If the file does not exist, it returns an empty vector.
pub fn read_history(app_handle: &AppHandle) -> Result<Vec<HistoryItem>, anyhow::Error> {
    let history_path = get_data_file_path(app_handle, HISTORY_FILENAME)?;

    match File::open(history_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let history = serde_json::from_reader(reader)
                .context("Failed to deserialize history.json. Returning empty list.")?;
            Ok(history)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // File doesn't exist, return empty vector
            Ok(Vec::new())
        }
        Err(e) => {
            // Other I/O error
            Err(anyhow::Error::new(e).context("Failed to read history.json"))
        }
    }
}

/// Writes the recognition history to `history.json`.
pub fn write_history(app_handle: &AppHandle, history: &[HistoryItem]) -> Result<(), anyhow::Error> {
    let history_path = get_data_file_path(app_handle, HISTORY_FILENAME)?;
    let file = File::create(history_path).context("Failed to create or truncate history.json")?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, history)
        .context("Failed to serialize and write history")?;
    Ok(())
}

/// Returns the absolute path to history.json
pub fn get_history_path(app_handle: &AppHandle) -> Result<PathBuf, anyhow::Error> {
    get_data_file_path(app_handle, HISTORY_FILENAME)
}
