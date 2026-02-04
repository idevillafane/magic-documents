use crate::core::config::Config;
use crate::core::note::NoteBuilder;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn run(
    vault: PathBuf,
    config: Config,
    count: usize,
    editor: Option<String>,
) -> anyhow::Result<()> {
    let mut notes = collect_notes(&vault)?;

    if notes.is_empty() {
        println!("No se encontraron notas en el vault.");
        return Ok(());
    }

    // Sort by modification time (most recent first)
    notes.sort_by(|a, b| b.1.cmp(&a.1));

    // Limit to requested count
    notes.truncate(count);

    // If only one note requested, open it directly
    if count == 1 && !notes.is_empty() {
        let selected_path = &notes[0].0;
        println!("\nAbriendo: {}", selected_path.display());
        NoteBuilder::add_timestamp_and_open(selected_path, &vault, &config, editor)?;
        return Ok(());
    }

    // Build display items
    let display_items: Vec<String> = notes
        .iter()
        .map(|(path, mtime)| {
            let relative = path.strip_prefix(&vault).unwrap_or(path);
            let time_str = format_time(*mtime);
            format!("{} ({})", relative.display(), time_str)
        })
        .collect();

    println!(
        "\nÚltimas {} notas editadas (ESC para cancelar):\n",
        notes.len()
    );

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Selecciona una nota para abrir")
        .items(&display_items)
        .default(0)
        .interact_opt()?;

    if let Some(idx) = selection {
        let selected_path = &notes[idx].0;
        println!("\nAbriendo: {}", selected_path.display());
        NoteBuilder::add_timestamp_and_open(selected_path, &vault, &config, editor)?;
    }

    Ok(())
}

fn collect_notes(vault: &Path) -> anyhow::Result<Vec<(PathBuf, SystemTime)>> {
    let mut notes = Vec::new();

    crate::utils::vault::VaultWalker::new(vault).walk_paths(|path| {
        if let Ok(metadata) = fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                notes.push((path.to_path_buf(), modified));
            }
        }
        Ok(())
    })?;

    Ok(notes)
}

fn format_time(time: SystemTime) -> String {
    use chrono::{DateTime, Local};

    let datetime: DateTime<Local> = time.into();
    let now = Local::now();

    let duration = now.signed_duration_since(datetime);

    if duration.num_seconds() < 60 {
        "hace unos segundos".to_string()
    } else if duration.num_minutes() < 60 {
        format!("hace {} min", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("hace {} horas", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("hace {} días", duration.num_days())
    } else {
        datetime.format("%Y-%m-%d %H:%M").to_string()
    }
}
