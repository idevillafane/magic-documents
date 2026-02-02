use crate::core::config::Config;
use crate::core::frontmatter;
use crate::utils::vault::VaultWalker;
use serde_yaml::Value;
use std::fs;
use std::path::Path;

/// One-time migration: Convert array-style tags to slash-separated format
/// Example: `tags: ["padre", "hijo"]` → `tags: ["padre/hijo"]`
pub fn run(vault: &Path, config: &Config) -> anyhow::Result<()> {
    let templates_path = vault.join(&config.templates_dir);

    let mut converted = 0;
    let mut skipped = 0;
    let mut errors = 0;

    println!("🔄 Migrando tags en vault: {}", vault.display());
    println!("   Convirtiendo formato array a formato slash...\n");

    VaultWalker::new(vault)
        .exclude_templates(&templates_path)
        .walk(|path, content| {
            match migrate_file_inner(path, content) {
                Ok(Some(changes)) => {
                    println!("  ✅ {} ({})", path.display(), changes);
                    converted += 1;
                }
                Ok(None) => {
                    skipped += 1;
                }
                Err(e) => {
                    eprintln!("  ❌ {}: {}", path.display(), e);
                    errors += 1;
                }
            }
            Ok(())
        })?;

    println!(
        "\n✨ Migración completada: {} convertidos, {} sin cambios, {} errores",
        converted, skipped, errors
    );

    if converted > 0 {
        println!("\n💡 Se crearon archivos .bak como respaldo.");
        println!("   Para eliminarlos: find {} -name '*.bak' -delete", vault.display());
    }

    Ok(())
}

fn migrate_file_inner(path: &Path, content: &str) -> anyhow::Result<Option<String>> {
    let (mut fm, body) = frontmatter::extract(content)?;

    let mut changes = Vec::new();

    for key in ["tags", "tag", "Tags", "Tag"] {
        if let Some(Value::Sequence(tag_list)) = fm.get(&Value::String(key.to_string())) {
            // Check if migration is needed:
            // - More than one element in the array (old format)
            // - Or single element without slash that could be part of old format

            if tag_list.is_empty() {
                continue;
            }

            // If already single element with slash, likely already migrated
            if tag_list.len() == 1 {
                if let Value::String(s) = &tag_list[0] {
                    if s.contains('/') {
                        continue; // Already in new format
                    }
                }
            }

            // Check if this looks like old array-as-hierarchy format
            // Old format: multiple simple strings that form ONE hierarchy
            let mut all_simple = true;
            let mut parts = Vec::new();

            for tag in tag_list {
                if let Value::String(t) = tag {
                    let trimmed = t.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    // If any element contains '/', it might be mixed format
                    if trimmed.contains('/') {
                        // Split and add parts
                        for part in trimmed.split('/') {
                            let p = part.trim();
                            if !p.is_empty() {
                                parts.push(p.to_string());
                            }
                        }
                    } else {
                        parts.push(trimmed.to_string());
                    }
                } else {
                    all_simple = false;
                    break;
                }
            }

            if !all_simple || parts.is_empty() {
                continue;
            }

            // Only migrate if we have multiple parts (indicating old format)
            if parts.len() <= 1 {
                continue;
            }

            // Convert to single slash-separated tag
            let new_tag = parts.join("/");
            let old_display = tag_list
                .iter()
                .filter_map(|v| {
                    if let Value::String(s) = v {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");

            changes.push(format!("[{}] → [{}]", old_display, new_tag));

            // Update frontmatter
            let new_value = Value::Sequence(vec![Value::String(new_tag)]);
            fm.insert(Value::String(key.to_string()), new_value);
            break; // Only process first matching key
        }
    }

    if changes.is_empty() {
        return Ok(None);
    }

    // Create backup
    let backup_path = path.with_extension("md.bak");
    fs::copy(path, &backup_path)?;

    // Write updated content
    let new_content = format!("---\n{}---{}", serde_yaml::to_string(&fm)?, body);
    fs::write(path, new_content)?;

    Ok(Some(changes.join(", ")))
}
