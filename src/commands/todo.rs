use crate::core::config::Config;
use crate::utils::vault::VaultWalker;
use chrono::{DateTime, Local, NaiveDate};
use crossterm::terminal;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct Task {
    path: PathBuf,
    line_number: usize,
    line: String,
    meta_date: String,
    meta_label: String,
}

pub fn run(vault: PathBuf, config: Config, mark_all: bool) -> anyhow::Result<()> {
    loop {
        let tasks = collect_tasks(&vault, &config)?;

        if tasks.is_empty() {
            println!("No se encontraron tareas pendientes en el vault.");
            return Ok(());
        }

        let selected_indices: Vec<usize> = if mark_all {
            let mark = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("¿Quieres marcar TODAS las tareas como listas?")
                .default(false)
                .interact()?;

            if !mark {
                return Ok(());
            }

            (0..tasks.len()).collect()
        } else {
            let selection = run_task_selector(&tasks)?;
            let Some(selected_idx) = selection else {
                return Ok(());
            };

            vec![selected_idx]
        };

        let mut by_file: HashMap<PathBuf, Vec<usize>> = HashMap::new();
        for idx in &selected_indices {
            let task = &tasks[*idx];
            by_file
                .entry(task.path.clone())
                .or_default()
                .push(task.line_number);
        }

        let mut updated_tasks = 0usize;

        for (path, line_numbers) in by_file {
            updated_tasks += mark_tasks_in_file(&path, &line_numbers)?;
        }

        if mark_all {
            println!("\n✅ Tareas marcadas como listas: {}", updated_tasks);
            return Ok(());
        }

        let task = &tasks[selected_indices[0]];
        println!(
            "\n✓ Tarea marcada como lista: {}",
            resumen_tarea(&task.line)
        );
        std::thread::sleep(std::time::Duration::from_millis(800));
    }
}

fn run_task_selector(tasks: &[Task]) -> anyhow::Result<Option<usize>> {
    let (term_width, _) = terminal::size().unwrap_or((80, 24));
    let term_width = term_width as usize;

    let checkbox_width = 4;
    let meta_sample = "(00/00 diario)";
    let meta_width = meta_sample.chars().count();
    let available_width = term_width.saturating_sub(checkbox_width + meta_width + 10);

    let items: Vec<String> = tasks
        .iter()
        .map(|task| {
            let meta_label = task.meta_label.clone();
            let mut meta = format!("({} {})", task.meta_date, meta_label);

            let max_meta = term_width.saturating_sub(checkbox_width + 6);
            if meta.chars().count() > max_meta {
                let keep = max_meta.saturating_sub(4);
                let truncated: String = meta.chars().take(keep).collect();
                meta = format!("{}...)", truncated);
            }

            let title = resumen_tarea(&task.line);
            let title = if title.chars().count() > available_width {
                let truncated: String = title.chars().take(available_width.saturating_sub(3)).collect();
                format!("{}...", truncated)
            } else {
                title
            };

            let title_len = title.chars().count();
            let padding = available_width.saturating_sub(title_len);

            format!("[ ] {}{:width$}{}",
                title,
                "",
                meta,
                width = padding
            )
        })
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(0)
        .interact_opt()?;

    Ok(selection)
}

fn collect_tasks(vault: &Path, config: &Config) -> anyhow::Result<Vec<Task>> {
    let templates_path = vault.join(&config.templates_dir);
    let diario_dir = vault.join(&config.diary_dir);
    let mut tasks = Vec::new();

    VaultWalker::new(vault)
        .exclude_templates(&templates_path)
        .walk(|path, content| {
            let mut in_code_block = false;
            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("sin-titulo")
                .to_string();
            let meta_label = if path.starts_with(&diario_dir) {
                "diario".to_string()
            } else {
                file_stem
            };
            let meta_date = task_meta_date(path, &diario_dir)?;

            for (idx, line) in content.split('\n').enumerate() {
                let trimmed = line.trim_start();

                if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                    in_code_block = !in_code_block;
                    continue;
                }

                if !in_code_block && line.starts_with("- [ ] ") {
                    tasks.push(Task {
                        path: path.to_path_buf(),
                        line_number: idx + 1,
                        line: line.to_string(),
                        meta_date: meta_date.clone(),
                        meta_label: meta_label.clone(),
                    });
                }
            }
            Ok(())
        })?;

    Ok(tasks)
}

fn resumen_tarea(line: &str) -> String {
    line.trim_start()
        .strip_prefix("- [ ] ")
        .unwrap_or(line)
        .trim_end()
        .to_string()
}

fn task_meta_date(path: &Path, diario_dir: &Path) -> anyhow::Result<String> {
    if path.starts_with(diario_dir) {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if let Ok(date) = NaiveDate::parse_from_str(stem, "%Y-%m-%d") {
                return Ok(date.format("%d/%m").to_string());
            }
        }
    }

    let metadata = fs::metadata(path)?;
    let modified = metadata.modified()?;
    let datetime: DateTime<Local> = modified.into();
    Ok(datetime.format("%d/%m").to_string())
}

fn mark_tasks_in_file(path: &Path, line_numbers: &[usize]) -> anyhow::Result<usize> {
    let content = fs::read_to_string(path)?;
    let mut lines: Vec<String> = content.split('\n').map(|s| s.to_string()).collect();

    let mut updated = 0usize;
    for &line_number in line_numbers {
        if line_number == 0 {
            continue;
        }
        let idx = line_number - 1;
        if idx >= lines.len() {
            continue;
        }
        let line = &lines[idx];
        if line.starts_with("- [ ] ") {
            lines[idx] = line.replacen("- [ ] ", "- [x] ", 1);
            updated += 1;
        }
    }

    if updated > 0 {
        fs::write(path, lines.join("\n"))?;
    }

    Ok(updated)
}
