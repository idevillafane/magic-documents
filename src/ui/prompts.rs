use super::input::input_with_esc;
use dialoguer::{theme::ColorfulTheme, Select};
use std::path::Path;

/// Select or create a project interactively
pub fn select_project(projects: &[String], projects_file: &Path) -> anyhow::Result<Option<String>> {
    let mut opts = projects.to_vec();
    opts.push("otro".to_string());

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Seleccione un project (ESC para cancelar)")
        .items(&opts)
        .default(0)
        .interact_opt()?;

    let idx = match selection {
        Some(i) => i,
        None => return Ok(None),
    };

    if idx == opts.len() - 1 {
        match input_with_esc("Nuevo project")? {
            Some(new_proj) if !new_proj.trim().is_empty() => {
                crate::utils::file::append_project(projects_file, &new_proj)?;
                Ok(Some(new_proj))
            }
            _ => Ok(None),
        }
    } else {
        Ok(Some(opts[idx].clone()))
    }
}

/// Select aliases interactively - enter sentences one by one, empty to finish
/// Returns Ok(None) if user pressed ESC, Ok(Some(aliases)) otherwise
pub fn select_aliases() -> anyhow::Result<Option<Vec<String>>> {
    let mut aliases = Vec::new();

    println!("\nAliases (Enter vacío para terminar, ESC para cancelar):");

    loop {
        match input_with_esc("Alias")? {
            Some(alias) => {
                let trimmed = alias.trim();
                if trimmed.is_empty() {
                    // Empty input - finish normally
                    break;
                }
                aliases.push(trimmed.to_string());
            }
            None => {
                // User pressed ESC - cancel
                println!("Cancelado.");
                return Ok(None);
            }
        }
    }

    Ok(Some(aliases))
}
