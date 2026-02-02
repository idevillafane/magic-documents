use crate::core::config::Config;
use crate::core::note::NoteBuilder;
use std::fs;
use std::path::{Path, PathBuf};

/// Save the last opened note path
pub fn save_last_note(vault: &Path, note_path: &Path) -> anyhow::Result<()> {
    let config_dir = Config::config_dir()?;
    fs::create_dir_all(&config_dir)?;

    let last_note_path = Config::last_note_path()?;
    let relative_path = note_path
        .strip_prefix(vault)
        .unwrap_or(note_path)
        .to_string_lossy()
        .to_string();

    fs::write(last_note_path, relative_path)?;
    Ok(())
}

/// Open the last opened note
pub fn open_last_note(
    vault: PathBuf,
    config: Config,
    editor: Option<String>,
) -> anyhow::Result<()> {
    let last_note_path = Config::last_note_path()?;

    if !last_note_path.exists() {
        eprintln!("No hay ninguna nota reciente");
        return Ok(());
    }

    let relative_path = fs::read_to_string(last_note_path)?;
    let note_path = vault.join(relative_path.trim());

    if !note_path.exists() {
        eprintln!("La última nota ya no existe: {}", note_path.display());
        return Ok(());
    }

    println!(
        "Abriendo última nota: {}",
        note_path
            .strip_prefix(&vault)
            .unwrap_or(&note_path)
            .display()
    );
    NoteBuilder::add_timestamp_and_open(&note_path, &vault, &config, editor)?;

    Ok(())
}
