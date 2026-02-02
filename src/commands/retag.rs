use crate::core::config::Config;
use crate::core::frontmatter;
use crate::tags;
use crate::utils::vault::VaultWalker;
use chrono::Local;
use serde_yaml::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Re-tag files based on their directory location
/// - `md --retag file.md` - retag single file
/// - `md --retag .` - retag all files recursively in current directory
/// - `md --retag file.md --no-bak` - retag without creating backup
/// - `md --retag file.md --no-alias` - retag without adding old tag to aliases
pub fn run(vault: &Path, config: &Config, target: &str, no_backup: bool, no_alias: bool) -> anyhow::Result<()> {
    if target == "." {
        retag_recursive(vault, config, no_backup, no_alias)
    } else {
        let path = Path::new(target);
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(target)
        };
        retag_file(vault, config, &abs_path, no_backup, no_alias)
    }
}

fn retag_recursive(vault: &Path, config: &Config, no_backup: bool, no_alias: bool) -> anyhow::Result<()> {
    let current_dir = std::env::current_dir()?;
    let templates_path = vault.join(&config.templates_dir);

    let mut updated = 0;
    let mut skipped = 0;
    let mut errors = 0;

    println!("Re-tagging archivos en: {}", current_dir.display());

    VaultWalker::new(&current_dir)
        .exclude_templates(&templates_path)
        .walk(|path, content| {
            match retag_file_inner(vault, config, path, content, no_backup, no_alias) {
                Ok(true) => {
                    println!("  ✅ {}", path.display());
                    updated += 1;
                }
                Ok(false) => {
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
        "\nRetag completado: {} actualizados, {} sin cambios, {} errores",
        updated, skipped, errors
    );
    Ok(())
}

fn retag_file(vault: &Path, config: &Config, path: &Path, no_backup: bool, no_alias: bool) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("Archivo no encontrado: {}", path.display());
    }

    let content = fs::read_to_string(path)?;
    match retag_file_inner(vault, config, path, &content, no_backup, no_alias) {
        Ok(true) => println!("✅ Actualizado: {}", path.display()),
        Ok(false) => println!("ℹ️  Sin cambios: {}", path.display()),
        Err(e) => eprintln!("❌ Error: {}", e),
    }
    Ok(())
}

fn retag_file_inner(
    vault: &Path,
    config: &Config,
    path: &Path,
    content: &str,
    no_backup: bool,
    no_alias: bool,
) -> anyhow::Result<bool> {
    let (mut fm, body) = frontmatter::extract(content)?;

    // Derive new primary tag from path
    let new_tag_str = derive_tag_from_path(vault, config, path)?;

    if new_tag_str.is_empty() {
        return Ok(false);
    }

    // Parse new tag as TagPath
    let new_tag = tags::TagPath(
        new_tag_str
            .split('/')
            .map(|s| s.to_string())
            .collect()
    );

    // Extract current primary tag from body
    let old_tag_opt = tags::parser::extract_primary_tag(&body);

    // Check if tag already matches
    if let Some(ref old_tag) = old_tag_opt {
        if old_tag.to_slash_string() == new_tag_str {
            return Ok(false); // Already has correct tag
        }
    }

    // If old tag exists and differs from new tag, add to aliases (unless --no-alias)
    if !no_alias {
        if let Some(old_tag) = old_tag_opt {
            let old_tag_str = old_tag.to_slash_string();
            if old_tag_str != new_tag_str {
                let now = Local::now();
                let date = now.format(&config.date).to_string();
                let alias_entry = format!("{} {}", date, old_tag_str);

                // Get or create aliases array
                let aliases = fm
                    .get(&Value::String("aliases".to_string()))
                    .and_then(|v| {
                        if let Value::Sequence(seq) = v {
                            Some(seq.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                // Add new alias
                let mut new_aliases = aliases;
                new_aliases.push(Value::String(alias_entry));

                fm.insert(
                    Value::String("aliases".to_string()),
                    Value::Sequence(new_aliases),
                );
            }
        }
    }

    // Update primary tag in body
    let new_body = tags::parser::replace_primary_tag(&body, &new_tag);

    // Create backup if not disabled
    if !no_backup {
        create_backup(vault, path)?;
    }

    // Write updated file
    let new_content = format!("---\n{}---\n{}", serde_yaml::to_string(&fm)?, new_body);
    fs::write(path, new_content)?;

    Ok(true)
}

/// Create backup in vault/.arc/backups/ with timestamp
/// Backups are stored flat (no directory structure) with format: filename_YYYYMMDD_HHMMSS.md.bak
fn create_backup(vault: &Path, file_path: &Path) -> anyhow::Result<()> {
    let backup_dir = vault.join(".arc").join("backups");
    fs::create_dir_all(&backup_dir)?;

    // Get filename without path
    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

    // Generate timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();

    // Build backup filename: original_20260202_131045.md.bak
    let backup_filename = if let Some(stem) = filename.strip_suffix(".md") {
        format!("{}_{}.md.bak", stem, timestamp)
    } else {
        format!("{}_{}.bak", filename, timestamp)
    };

    let backup_path = backup_dir.join(backup_filename);
    fs::copy(file_path, &backup_path)?;

    Ok(())
}

/// Derive tag from file path relative to vault
/// Example: vault/Notas/proyecto/cliente/nota.md -> "proyecto/cliente"
/// tag_root (ej: "Notas") se excluye del tag generado
fn derive_tag_from_path(vault: &Path, config: &Config, path: &Path) -> anyhow::Result<String> {
    let tag_root = vault.join(&config.tag_root);

    // Get path relative to tag_root
    let relative = path
        .strip_prefix(&tag_root)
        .map_err(|_| anyhow::anyhow!("Path must be inside tag_root ({})", tag_root.display()))?;

    // Get parent directory (exclude filename)
    let parent = relative.parent().unwrap_or(Path::new(""));

    // Convert to slash-separated tag
    let tag = parent
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/");

    Ok(tag)
}
