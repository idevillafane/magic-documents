use crate::core::config::Config;
use crate::utils::vault::VaultWalker;
use dialoguer::{theme::ColorfulTheme, Confirm, MultiSelect};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct Task {
    path: PathBuf,
    line_number: usize,
    line: String,
    file_stem: String,
}

pub fn run(vault: PathBuf, config: Config, mark_all: bool) -> anyhow::Result<()> {
    let tasks = collect_tasks(&vault, &config)?;

    if tasks.is_empty() {
        println!("No se encontraron tareas pendientes en el vault.");
        return Ok(());
    }

    println!("\nTareas pendientes encontradas:\n");
    for (idx, task) in tasks.iter().enumerate() {
        let relative = task.path.strip_prefix(&vault).unwrap_or(&task.path);
        let title = format!(" [{}]", task.file_stem);
        println!(
            "{:>3}. {}:{}{} {}",
            idx + 1,
            relative.display(),
            task.line_number,
            title,
            task.line.trim_end()
        );
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
        let mark = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("¿Quieres marcar tareas como listas?")
            .default(false)
            .interact()?;

        if !mark {
            return Ok(());
        }

        let items: Vec<String> = tasks
            .iter()
            .map(|task| {
                let relative = task.path.strip_prefix(&vault).unwrap_or(&task.path);
                let title = format!(" [{}]", task.file_stem);
                format!(
                    "{}:{}{} {}",
                    relative.display(),
                    task.line_number,
                    title,
                    task.line.trim_end()
                )
            })
            .collect();

        let selection = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Selecciona las tareas a marcar como listas (ESC para cancelar)")
            .items(&items)
            .interact_opt()?;

        let Some(selected_indices) = selection else {
            return Ok(());
        };

        if selected_indices.is_empty() {
            println!("No se seleccionaron tareas.");
            return Ok(());
        }

        selected_indices
    };

    let mut by_file: HashMap<PathBuf, Vec<usize>> = HashMap::new();
    for idx in selected_indices {
        let task = &tasks[idx];
        by_file
            .entry(task.path.clone())
            .or_default()
            .push(task.line_number);
    }

    let mut updated_tasks = 0usize;

    for (path, line_numbers) in by_file {
        updated_tasks += mark_tasks_in_file(&path, &line_numbers)?;
    }

    println!("\n✅ Tareas marcadas como listas: {}", updated_tasks);

    Ok(())
}

fn collect_tasks(vault: &Path, config: &Config) -> anyhow::Result<Vec<Task>> {
    let templates_path = vault.join(&config.templates_dir);
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
                        file_stem: file_stem.clone(),
                    });
                }
            }
            Ok(())
        })?;

    Ok(tasks)
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
