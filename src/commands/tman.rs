use crate::core::config::Config;
use crate::core::frontmatter;
use crate::vault::scan;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input, Select};
use serde_yaml::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(vault: &Path) -> anyhow::Result<()> {
    interactive_menu(vault)
}

pub fn list_tags(vault: &Path, include_archived: bool) -> anyhow::Result<()> {
    if include_archived {
        // For list-all, use interactive mode to filter archived tags
        list_tags_interactive(vault, include_archived)
    } else {
        // For list, use flat list with hierarchy
        list_flat_tags(vault)
    }
}

pub fn rename_tags(vault: &Path) -> anyhow::Result<()> {
    rename_tag(vault)
}

pub fn find_by_tag(vault: &Path) -> anyhow::Result<()> {
    search_files_by_tag(vault)
}

pub fn visual_selector() -> anyhow::Result<()> {
    anyhow::bail!("Visual selector (telescope) not implemented yet")
}

fn interactive_menu(vault: &Path) -> anyhow::Result<()> {
    println!("🏷️  Gestor de Tags para: {}\n", vault.display());
    println!("Tip: Presiona ESC para salir en cualquier momento\n");

    loop {
        let options = vec![
            "📋 Listar tags (flat)",
            "✏️  Renombrar tag",
            "🔍 Buscar por tag",
            "❌ Salir",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("¿Qué deseas hacer?")
            .items(&options)
            .default(0)
            .interact_opt()?;

        match selection {
            Some(0) => list_flat_tags(vault)?,
            Some(1) => rename_tag(vault)?,
            Some(2) => search_files_by_tag(vault)?,
            Some(3) | None => break,
            _ => {}
        }

        println!("\n---\n");
    }

    Ok(())
}

fn collect_all_tags(vault: &Path) -> anyhow::Result<HashMap<Vec<String>, Vec<PathBuf>>> {
    // Load config to get templates directory
    let config = Config::load_default()?;
    let templates_path = vault.join(&config.templates_dir);

    let mut tag_map: HashMap<Vec<String>, HashSet<PathBuf>> = HashMap::new();

    let items = scan::scan_tags(vault, &templates_path)?;
    for item in items {
        for tag in item.secondary_tags {
            tag_map
                .entry(tag.0)
                .or_insert_with(HashSet::new)
                .insert(item.path.clone());
        }
    }

    let tag_map = tag_map
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect::<HashMap<Vec<String>, Vec<PathBuf>>>();

    Ok(tag_map)
}

fn list_tags_interactive(vault: &Path, include_archived: bool) -> anyhow::Result<()> {
    let tag_map = collect_all_tags(vault)?;

    if tag_map.is_empty() {
        println!("No hay tags en el vault.");
        return Ok(());
    }

    // Filter out archived tags if include_archived is false
    let filtered_map: HashMap<Vec<String>, Vec<PathBuf>> = if include_archived {
        tag_map
    } else {
        tag_map
            .into_iter()
            .filter(|(path, _)| {
                // Exclude if path starts with "Archived" or contains "Archived" as any component
                !path.iter().any(|component| component == "Archived")
            })
            .collect()
    };

    if filtered_map.is_empty() {
        println!("No hay tags en el vault (excluyendo archivados).");
        return Ok(());
    }

    let mut tag_entries: Vec<(String, Vec<PathBuf>)> = filtered_map
        .iter()
        .map(|(path, files)| (path.join(" → "), files.clone()))
        .collect();

    tag_entries.sort_by(|a, b| a.0.cmp(&b.0));

    // Show tags with file count
    let tag_display: Vec<String> = tag_entries
        .iter()
        .map(|(tag, files)| format!("{} ({} archivos)", tag, files.len()))
        .collect();

    let selection = dialoguer::FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Selecciona un tag para ver sus archivos (ESC para cancelar)")
        .items(&tag_display)
        .default(0)
        .interact_opt()?;

    if let Some(idx) = selection {
        let (selected_tag, files) = &tag_entries[idx];

        if files.is_empty() {
            println!("\n📁 No hay archivos con el tag '{}'", selected_tag);
            return Ok(());
        }

        // Display files for the selected tag
        let file_display: Vec<String> = files
            .iter()
            .map(|f| f.strip_prefix(vault).unwrap_or(f).display().to_string())
            .collect();

        let file_selection = dialoguer::FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "Archivos con tag '{}' (ESC para volver)",
                selected_tag
            ))
            .items(&file_display)
            .default(0)
            .interact_opt()?;

        if let Some(file_idx) = file_selection {
            let selected_file = &files[file_idx];

            // Load config to get editor preference
            let config = Config::load_default()?;

            // Open file in editor
            println!("\nAbriendo: {}", selected_file.display());

            let editor_mode = config.editor_mode.as_deref().unwrap_or("integrated");

            if editor_mode == "integrated" {
                crate::ui::editor::open(selected_file, vault)?;
            } else {
                let editor = config.editor.as_deref().unwrap_or("vi");
                std::process::Command::new(editor)
                    .arg(selected_file)
                    .status()?;
            }
        }
    }

    Ok(())
}

fn list_flat_tags(vault: &Path) -> anyhow::Result<()> {
    let tag_map = collect_all_tags(vault)?;

    println!("\n📋 Lista de Tags (orden alfabético):\n");

    // Filter out archived tags
    let filtered_map: HashMap<Vec<String>, Vec<PathBuf>> = tag_map
        .into_iter()
        .filter(|(path, _)| {
            // Exclude if path starts with "Archived" or contains "Archived" as any component
            !path.iter().any(|component| component == "Archived")
        })
        .collect();

    if filtered_map.is_empty() {
        println!("No hay tags en el vault (excluyendo archivados).");
        return Ok(());
    }

    // Collect all unique tag paths including intermediates
    let mut all_paths = HashSet::new();
    for path in filtered_map.keys() {
        // Add all levels: parent, parent -> child, parent -> child -> grandchild
        for i in 1..=path.len() {
            all_paths.insert(path[..i].to_vec());
        }
    }

    // Convert to displayable format and sort
    let mut display_tags: Vec<(Vec<String>, String, usize)> = all_paths
        .iter()
        .map(|path| {
            let display = path.join(" → ");
            let count = filtered_map.get(path).map(|v| v.len()).unwrap_or(0);
            (path.clone(), display, count)
        })
        .collect();

    display_tags.sort_by(|a, b| a.1.cmp(&b.1));

    for (_, display, count) in &display_tags {
        if *count > 0 {
            println!("  {} ({} archivos)", display, count);
        } else {
            println!("  {}", display);
        }
    }

    println!("\nTotal: {} tags únicos", display_tags.len());

    Ok(())
}

fn search_files_by_tag(vault: &Path) -> anyhow::Result<()> {
    let tag_map = collect_all_tags(vault)?;

    let mut tag_paths: Vec<String> = tag_map.keys().map(|path| path.join(" → ")).collect();
    tag_paths.sort();

    if tag_paths.is_empty() {
        println!("No hay tags en el vault.");
        return Ok(());
    }

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Buscar tag (escribe para filtrar, ESC para cancelar)")
        .items(&tag_paths)
        .default(0)
        .interact_opt()?;

    if let Some(idx) = selection {
        let selected_path: Vec<String> =
            tag_paths[idx].split(" → ").map(|s| s.to_string()).collect();

        if let Some(files) = tag_map.get(&selected_path) {
            println!("\n📁 Archivos con tag '{}':", tag_paths[idx]);
            for file in files {
                println!("  → {}", file.display());
            }
            println!("\nTotal: {} archivos", files.len());
        }
    }

    Ok(())
}

fn rename_tag(vault: &Path) -> anyhow::Result<()> {
    let tag_map = collect_all_tags(vault)?;

    let mut all_tag_paths = HashSet::new();

    for path in tag_map.keys() {
        all_tag_paths.insert(path.join(" → "));
        for i in 1..path.len() {
            all_tag_paths.insert(path[..i].join(" → "));
        }
    }

    let mut tag_paths: Vec<String> = all_tag_paths.into_iter().collect();
    tag_paths.sort();

    if tag_paths.is_empty() {
        println!("No hay tags en el vault.");
        return Ok(());
    }

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Selecciona el tag a renombrar (ESC para cancelar)")
        .items(&tag_paths)
        .default(0)
        .interact_opt()?;

    let idx = match selection {
        Some(i) => i,
        None => return Ok(()),
    };

    let old_path: Vec<String> = tag_paths[idx].split(" → ").map(|s| s.to_string()).collect();

    let children: Vec<_> = tag_map
        .keys()
        .filter(|path| {
            if path.len() <= old_path.len() {
                return false;
            }
            path[..old_path.len()] == old_path[..]
        })
        .collect();

    let has_children = !children.is_empty();

    if has_children {
        println!("\n⚠️  Este tag tiene {} sub-tags:", children.len());
        for child in children.iter().take(5) {
            println!("   → {}", child.join(" → "));
        }
        if children.len() > 5 {
            println!("   ... y {} más", children.len() - 5);
        }
        println!();
    }

    println!("\nTag actual: {}", tag_paths[idx]);

    let rename_mode = if has_children {
        let options = vec![
            "Solo este nivel (sin afectar sub-tags)",
            "Este nivel y todos sus sub-tags",
        ];

        let mode_sel = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("¿Cómo quieres renombrar?")
            .items(&options)
            .default(0)
            .interact_opt()?;

        match mode_sel {
            Some(0) => "single",
            Some(1) => "recursive",
            None => return Ok(()),
            _ => "single",
        }
    } else {
        "single"
    };

    println!("Ingresa el nuevo tag (usa '/' para mantener jerarquía, vacío para cancelar):");

    let new_tag: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Nuevo tag")
        .allow_empty(true)
        .interact_text()?;

    if new_tag.trim().is_empty() {
        println!("Operación cancelada");
        return Ok(());
    }

    let new_path: Vec<String> = new_tag
        .split('/')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if new_path.is_empty() {
        println!("❌ Tag inválido");
        return Ok(());
    }

    let mut affected_files = HashSet::new();
    if rename_mode == "recursive" {
        for (path, files) in tag_map.iter() {
            if path.starts_with(&old_path) {
                for file in files {
                    affected_files.insert(file.clone());
                }
            }
        }
    } else {
        if let Some(files) = tag_map.get(&old_path) {
            for file in files {
                affected_files.insert(file.clone());
            }
        }
    }

    let confirm = dialoguer::Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "¿Renombrar '{}' a '{}' en {} archivos?",
            tag_paths[idx],
            new_path.join(" → "),
            affected_files.len()
        ))
        .default(false)
        .interact_opt()?;

    if !confirm.unwrap_or(false) {
        println!("Operación cancelada");
        return Ok(());
    }

    let mut updated = 0;
    for file_path in &affected_files {
        if let Ok(content) = fs::read_to_string(file_path) {
            if let Ok((mut fm, body)) = frontmatter::extract(&content) {
                for key in ["tags", "tag", "Tags", "Tag"] {
                    let key_val = Value::String((*key).to_string());
                    if let Some(Value::Sequence(tag_list)) = fm.get(&key_val) {
                        let mut new_list = Vec::new();
                        let mut any_updated = false;

                        // Process each tag INDEPENDENTLY (new model)
                        for tag in tag_list {
                            if let Value::String(t) = tag {
                                let trimmed = t.trim();

                                // Parse this single tag into path components
                                let current_path: Vec<String> = trimmed
                                    .split('/')
                                    .map(|p| p.trim().to_string())
                                    .filter(|p| !p.is_empty())
                                    .collect();

                                let should_update = if rename_mode == "recursive" {
                                    current_path.starts_with(&old_path)
                                } else {
                                    current_path == old_path
                                };

                                if should_update {
                                    // Build updated path
                                    let mut updated_path = new_path.clone();
                                    if rename_mode == "recursive" && current_path.len() > old_path.len() {
                                        updated_path.extend_from_slice(&current_path[old_path.len()..]);
                                    }

                                    // Write as slash-separated string
                                    let tag_string = updated_path.join("/");
                                    new_list.push(Value::String(tag_string));
                                    any_updated = true;
                                } else {
                                    // Keep original tag unchanged
                                    new_list.push(Value::String(trimmed.to_string()));
                                }
                            }
                        }

                        if any_updated {
                            fm.insert(Value::String(key.to_string()), Value::Sequence(new_list));

                            let new_content =
                                format!("---\n{}---{}", serde_yaml::to_string(&fm)?, body);

                            let backup_path = file_path.with_extension("md.bak");
                            fs::copy(file_path, &backup_path)?;

                            fs::write(file_path, new_content)?;
                            updated += 1;
                        }
                        break;
                    }
                }
            }
        }
    }
    println!("✅ {} archivos actualizados", updated);

    println!("Regenerando caché de tags...");
    regenerate_tag_cache()?;

    Ok(())
}

fn regenerate_tag_cache() -> anyhow::Result<()> {
    let cache_path = Config::cache_path()?;
    let _ = std::fs::remove_file(&cache_path);
    Ok(())
}
