use crate::core::config::Config;
use crate::core::frontmatter;
use crate::utils::vault::VaultWalker;
use serde_yaml::Value;
use std::fs;
use std::path::Path;

/// Re-tag files based on their directory location
/// - `md --retag file.md` - retag single file
/// - `md --retag .` - retag all files recursively in current directory
/// - `md --retag file.md --no-bak` - retag without creating backup
pub fn run(vault: &Path, config: &Config, target: &str, no_backup: bool) -> anyhow::Result<()> {
    if target == "." {
        retag_recursive(vault, config, no_backup)
    } else {
        let path = Path::new(target);
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(target)
        };
        retag_file(vault, config, &abs_path, no_backup)
    }
}

fn retag_recursive(vault: &Path, config: &Config, no_backup: bool) -> anyhow::Result<()> {
    let current_dir = std::env::current_dir()?;
    let templates_path = vault.join(&config.templates_dir);

    let mut updated = 0;
    let mut skipped = 0;
    let mut errors = 0;

    println!("Re-tagging archivos en: {}", current_dir.display());

    VaultWalker::new(&current_dir)
        .exclude_templates(&templates_path)
        .walk(|path, content| {
            match retag_file_inner(vault, config, path, content, no_backup) {
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

fn retag_file(vault: &Path, config: &Config, path: &Path, no_backup: bool) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("Archivo no encontrado: {}", path.display());
    }

    let content = fs::read_to_string(path)?;
    match retag_file_inner(vault, config, path, &content, no_backup) {
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
) -> anyhow::Result<bool> {
    let (mut fm, body) = frontmatter::extract(content)?;

    // Derive tag from path relative to vault, excluding notes_dir
    let new_tag = derive_tag_from_path(vault, config, path)?;

    if new_tag.is_empty() {
        return Ok(false);
    }

    // Check if tag already matches
    for key in ["tags", "tag", "Tags", "Tag"] {
        if let Some(Value::Sequence(tag_list)) = fm.get(&Value::String(key.to_string())) {
            if tag_list.len() == 1 {
                if let Value::String(existing) = &tag_list[0] {
                    if existing == &new_tag {
                        return Ok(false); // Already has correct tag
                    }
                }
            }
        }
    }

    // Update tags in frontmatter
    let tags_value = Value::Sequence(vec![Value::String(new_tag.clone())]);
    fm.insert(Value::String("tags".to_string()), tags_value);

    // Create backup if not disabled
    if !no_backup {
        let backup_path = path.with_extension("md.bak");
        fs::copy(path, &backup_path)?;
    }

    let new_content = format!("---\n{}---{}", serde_yaml::to_string(&fm)?, body);
    fs::write(path, new_content)?;

    Ok(true)
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
