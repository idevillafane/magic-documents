use super::cache;
use super::tree::TagNode;
use crate::ui::input::input_with_esc;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Select};
use std::path::Path;

/// Selects a tag using hierarchical navigation.
/// Returns slash-separated tag string (e.g., "padre/hijo/nieto")
pub fn select_hierarchical(vault: &Path) -> anyhow::Result<String> {
    let config_dir = crate::core::config::Config::config_dir()?;
    let tag_tree = cache::load(vault, &config_dir)?;

    if tag_tree.children.is_empty() {
        println!("No se encontraron tags en el vault.");
        return Ok(String::new());
    }

    let mut selected_path: Vec<String> = Vec::new();
    let mut current_node = &tag_tree;

    loop {
        let children = current_node.get_children_names();

        if children.is_empty() && !selected_path.is_empty() {
            let result = selected_path.join("/");
            println!("\n✅ Tag completo seleccionado: {}", result);
            return Ok(result);
        }

        let mut options = children.clone();
        if !selected_path.is_empty() {
            options.insert(0, "✓ Finalizar aquí".to_string());
        }
        options.push("+ Agregar tag personalizado".to_string());
        options.push("← Retroceder".to_string());

        let prompt = if selected_path.is_empty() {
            "Selecciona tag raíz (ESC para cancelar):".to_string()
        } else {
            format!(
                "📁 {} → Selecciona subtag (ESC para cancelar):",
                selected_path.join("/")
            )
        };

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(&prompt)
            .items(&options)
            .default(0)
            .interact_opt()?;

        let selection = match selection {
            Some(s) => s,
            None => return Err(anyhow::anyhow!("User cancelled")),
        };

        if !selected_path.is_empty() && selection == 0 {
            let result = selected_path.join("/");
            println!("\n✅ Tag seleccionado: {}", result);
            return Ok(result);
        }

        if selection == options.len() - 2 {
            match input_with_esc("Nuevo tag (puedes usar '/' para sub-niveles)")? {
                Some(new_tag) if !new_tag.trim().is_empty() => {
                    let parts: Vec<String> = new_tag
                        .split('/')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    selected_path.extend(parts);
                    let result = selected_path.join("/");
                    println!("✅ Tag personalizado: {}", result);
                    return Ok(result);
                }
                None => {
                    return Err(anyhow::anyhow!("User cancelled"));
                }
                _ => continue,
            }
        }

        if selection == options.len() - 1 {
            if selected_path.is_empty() {
                return Ok(String::new());
            } else {
                selected_path.pop();
                current_node = &tag_tree;
                for part in &selected_path {
                    if let Some(node) = current_node.get_child(part) {
                        current_node = node;
                    }
                }
                continue;
            }
        }

        let offset = if !selected_path.is_empty() { 1 } else { 0 };
        let selected_tag = children[selection - offset].clone();
        selected_path.push(selected_tag.clone());

        if let Some(node) = current_node.get_child(&selected_tag) {
            current_node = node;
        }
    }
}

/// Selects a tag using fuzzy search.
/// Returns slash-separated tag string (e.g., "padre/hijo/nieto")
pub fn select_with_fuzzy(vault: &Path) -> anyhow::Result<String> {
    let config_dir = crate::core::config::Config::config_dir()?;
    let tag_tree = cache::load(vault, &config_dir)?;

    if tag_tree.children.is_empty() {
        println!("No se encontraron tags en el vault.");
        return Ok(String::new());
    }

    // Filter out "Archived" from root level
    let mut filtered_tree = tag_tree.clone();
    filtered_tree.children.retain(|name, _| name != "Archived");

    if filtered_tree.children.is_empty() {
        println!("No se encontraron tags en el vault.");
        return Ok(String::new());
    }

    let mut selected_path: Vec<String> = Vec::new();

    loop {
        let mut current_node = &filtered_tree;
        for part in &selected_path {
            if let Some(node) = current_node.get_child(part) {
                current_node = node;
            } else {
                break;
            }
        }

        let mut children = current_node.get_children_names();

        // Sort children alphabetically
        children.sort();

        if children.is_empty() && !selected_path.is_empty() {
            break;
        }

        let mut options: Vec<String> = Vec::new();

        // Always show "+ Agregar tag personalizado" first
        options.push("+ Agregar tag personalizado".to_string());

        // First: add direct children (parents)
        for child in &children {
            options.push(child.clone());
        }

        // Then: add nested paths (padre → hijo → nieto)
        fn collect_all_paths(node: &TagNode, current_prefix: &str, out: &mut Vec<String>) {
            for (name, child) in &node.children {
                let prefix = if current_prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{} → {}", current_prefix, name)
                };

                out.push(prefix.clone());
                collect_all_paths(child, &prefix, out);
            }
        }

        let mut nested_paths = Vec::new();
        collect_all_paths(current_node, "", &mut nested_paths);

        // Remove direct children from nested_paths (they're already in options)
        nested_paths.retain(|path| !children.contains(path));

        // Sort nested paths alphabetically
        nested_paths.sort();

        options.extend(nested_paths);

        if !selected_path.is_empty() {
            options.push("✓ Terminar aquí".to_string());
        }

        let prompt = if selected_path.is_empty() {
            "Selecciona tag".to_string()
        } else {
            format!("{} → Selecciona subtag", selected_path.join("/"))
        };

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt(&prompt)
            .items(&options)
            .default(0)
            .interact_opt()?;

        let idx = match selection {
            Some(i) => i,
            None => return Err(anyhow::anyhow!("User cancelled")),
        };

        let selected = &options[idx];

        if selected.contains("Terminar aquí") {
            break;
        }

        if selected.contains("Agregar tag personalizado") {
            match input_with_esc("Nuevo tag (puedes usar '/' para sub-niveles)")? {
                Some(new_tag) if !new_tag.trim().is_empty() => {
                    let parts: Vec<String> = new_tag
                        .split('/')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    selected_path.extend(parts);
                    let result = selected_path.join("/");
                    println!("✅ Tag personalizado: {}", result);
                    return Ok(result);
                }
                None => {
                    return Err(anyhow::anyhow!("User cancelled"));
                }
                _ => continue,
            }
        }

        if selected.contains(" → ") {
            let parts: Vec<String> = selected
                .split(" → ")
                .map(|s| s.trim().to_string())
                .collect();
            selected_path.extend(parts);
            break;
        }

        selected_path.push(selected.clone());

        current_node = &tag_tree;
        for part in &selected_path {
            if let Some(node) = current_node.get_child(part) {
                current_node = node;
            } else {
                break;
            }
        }

        if current_node.children.is_empty() {
            break;
        }
    }

    let result = selected_path.join("/");
    if !result.is_empty() {
        println!("\n✅ Tag seleccionado: {}", result);
    }

    Ok(result)
}
