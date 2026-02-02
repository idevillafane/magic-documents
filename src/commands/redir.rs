use crate::core::config::Config;
use crate::core::frontmatter;
use crate::tags::TagPath;
use crate::utils::vault::VaultWalker;
use dialoguer::{theme::ColorfulTheme, Select};
use std::fs;
use std::path::{Path, PathBuf};

/// Move files to directories matching their tags
/// - `md --redir file.md` - move single file
/// - `md --redir .` - move all files recursively in current directory
/// - `md --redir file.md --no-bak` - move without creating backup
pub fn run(vault: &Path, config: &Config, target: &str, no_backup: bool) -> anyhow::Result<()> {
    if target == "." {
        redir_recursive(vault, config, no_backup)
    } else {
        let path = Path::new(target);
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(target)
        };
        redir_file(vault, config, &abs_path, no_backup)
    }
}

fn redir_recursive(vault: &Path, config: &Config, no_backup: bool) -> anyhow::Result<()> {
    let current_dir = std::env::current_dir()?;
    let templates_path = vault.join(&config.templates_dir);

    // Collect files first to avoid iterator invalidation during moves
    let mut files_to_process: Vec<PathBuf> = Vec::new();

    VaultWalker::new(&current_dir)
        .exclude_templates(&templates_path)
        .walk(|path, _content| {
            files_to_process.push(path.to_path_buf());
            Ok(())
        })?;

    println!(
        "Procesando {} archivos en: {}",
        files_to_process.len(),
        current_dir.display()
    );

    let mut moved = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for path in files_to_process {
        match redir_file_inner(vault, config, &path, no_backup) {
            Ok(Some(dest)) => {
                println!("  ✅ {} → {}", path.display(), dest.display());
                moved += 1;
            }
            Ok(None) => {
                skipped += 1;
            }
            Err(e) => {
                eprintln!("  ❌ {}: {}", path.display(), e);
                errors += 1;
            }
        }
    }

    println!(
        "\nRedir completado: {} movidos, {} sin cambios, {} errores",
        moved, skipped, errors
    );
    Ok(())
}

fn redir_file(vault: &Path, config: &Config, path: &Path, no_backup: bool) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("Archivo no encontrado: {}", path.display());
    }

    match redir_file_inner(vault, config, path, no_backup) {
        Ok(Some(dest)) => println!("✅ Movido: {} → {}", path.display(), dest.display()),
        Ok(None) => println!("ℹ️  Sin cambios (ya está en ubicación correcta o sin tags)"),
        Err(e) => eprintln!("❌ Error: {}", e),
    }
    Ok(())
}

fn redir_file_inner(vault: &Path, config: &Config, path: &Path, no_backup: bool) -> anyhow::Result<Option<PathBuf>> {
    let content = fs::read_to_string(path)?;
    let (fm, _body) = frontmatter::extract(&content)?;

    let tags = TagPath::from_frontmatter(&fm);

    if tags.is_empty() {
        return Ok(None); // No tags, skip
    }

    // If multiple tags, prompt user to select
    let selected_tag = if tags.len() == 1 {
        &tags[0]
    } else {
        let tag_strings: Vec<String> = tags.iter().map(|t| t.to_slash_string()).collect();

        println!("\nEl archivo tiene {} tags:", tags.len());
        for (i, tag) in tag_strings.iter().enumerate() {
            println!("  {}. {}", i + 1, tag);
        }

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Selecciona el tag destino (ESC para omitir)")
            .items(&tag_strings)
            .default(0)
            .interact_opt()?;

        match selection {
            Some(idx) => &tags[idx],
            None => return Ok(None), // User cancelled
        }
    };

    // Build destination path: vault/notes_dir/tag_path/filename.md
    let notes_dir = vault.join(&config.notes_dir);
    let tag_path: PathBuf = selected_tag.0.iter().collect();
    let dest_dir = notes_dir.join(&tag_path);

    // Check if already in correct location
    if let Some(current_parent) = path.parent() {
        if let (Ok(current_canonical), Ok(dest_canonical)) =
            (current_parent.canonicalize(), dest_dir.canonicalize())
        {
            if current_canonical == dest_canonical {
                return Ok(None); // Already in correct location
            }
        }
    }

    // Create backup if not disabled
    if !no_backup {
        let backup_path = path.with_extension("md.bak");
        fs::copy(path, &backup_path)?;
    }

    // Create destination directory
    fs::create_dir_all(&dest_dir)?;

    // Build destination file path
    let filename = path.file_name().ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
    let dest_path = dest_dir.join(filename);

    // Check for collision
    if dest_path.exists() {
        anyhow::bail!(
            "Archivo destino ya existe: {}",
            dest_path.display()
        );
    }

    // Move file
    fs::rename(path, &dest_path)?;

    Ok(Some(dest_path))
}
