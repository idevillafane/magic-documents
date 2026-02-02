use crate::core::config::Config;
use crate::core::note::NoteBuilder;
use dialoguer::{theme::ColorfulTheme, Confirm};
use std::fs;
use std::path::{Path, PathBuf};

/// Create/open note in Obsidian vault from productive directory
/// - Uses dir_mappings from config to map work directories to documentation paths
/// - Derives tag from the mapping and relative path
/// - Creates mirrored directory structure in Obsidian if needed
pub fn run(
    vault: &Path,
    config: Config,
    title: String,
    editor_cmd: Option<String>,
) -> anyhow::Result<()> {
    let current_dir = std::env::current_dir()?;
    let current_canonical = current_dir.canonicalize()?;

    // Find matching mapping from config
    let (work_prefix, doc_subpath) = find_matching_mapping(&current_canonical, &config)?;

    // Calculate relative path from work prefix to current dir
    let relative_path = current_canonical
        .strip_prefix(&work_prefix)
        .map_err(|_| anyhow::anyhow!("Failed to calculate relative path"))?;

    // Build tag components: doc_subpath + relative_path
    let mut tag_components: Vec<String> = if !doc_subpath.is_empty() {
        doc_subpath
            .split('/')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    } else {
        Vec::new()
    };

    tag_components.extend(
        relative_path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .map(String::from)
    );

    if tag_components.is_empty() {
        anyhow::bail!("No se puede crear nota en el directorio raíz del mapeo. Navega a un subdirectorio.");
    }

    let tag = tag_components.join("/");
    println!("\x1b[32m#{}\x1b[0m", tag);

    // Build destination directory in Obsidian: vault/tag_root/doc_subpath/relative_path
    let tag_root = vault.join(&config.tag_root);
    let dest_dir = if !doc_subpath.is_empty() {
        tag_root.join(doc_subpath).join(relative_path)
    } else {
        tag_root.join(relative_path)
    };

    // Check if destination directory exists
    if !dest_dir.exists() {
        println!("\x1b[32m{}\x1b[0m", dest_dir.display());

        let should_create = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Crear directorio?")
            .default(true)
            .interact_opt()?;

        match should_create {
            Some(true) => {
                fs::create_dir_all(&dest_dir)?;
            }
            Some(false) | None => {
                return Ok(());
            }
        }
    }

    // Create note using NoteBuilder
    NoteBuilder::new(vault.to_path_buf(), config)
        .title(Some(title))
        .target_directory(dest_dir)
        .hierarchical_tags(true)
        .editor(editor_cmd)
        .create()?;

    Ok(())
}

/// Find matching directory mapping from config
/// Returns (work_directory_prefix, documentation_subpath)
fn find_matching_mapping(
    current_canonical: &Path,
    config: &Config,
) -> anyhow::Result<(PathBuf, String)> {
    let dir_mappings = config.dir_mappings.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "No hay mapeos de directorios configurados en dir_mappings.\n\
            Agrega mapeos en ~/.config/magic-documents/config.toml:\n\
            [dir_mappings]\n\
            \"/ruta/trabajo\" = \"documentacion\""
        )
    })?;

    if dir_mappings.is_empty() {
        anyhow::bail!(
            "No hay mapeos de directorios configurados en dir_mappings.\n\
            Agrega mapeos en ~/.config/magic-documents/config.toml:\n\
            [dir_mappings]\n\
            \"/ruta/trabajo\" = \"documentacion\""
        );
    }

    // Find the longest matching prefix
    let mut best_match: Option<(PathBuf, String)> = None;
    let mut best_match_len = 0;

    for (work_dir, doc_path) in dir_mappings {
        let work_path = PathBuf::from(work_dir);

        // Try to canonicalize, skip if it doesn't exist
        let work_canonical = match work_path.canonicalize() {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Check if current directory is under this work directory
        if current_canonical.starts_with(&work_canonical) {
            let match_len = work_canonical.components().count();
            if match_len > best_match_len {
                best_match_len = match_len;
                best_match = Some((work_canonical, doc_path.clone()));
            }
        }
    }

    match best_match {
        Some((work_prefix, doc_subpath)) => Ok((work_prefix, doc_subpath)),
        None => {
            anyhow::bail!(
                "El directorio actual no coincide con ningún mapeo configurado.\n\
                Directorio actual: {}\n\
                Mapeos disponibles: {:?}",
                current_canonical.display(),
                dir_mappings.keys()
            )
        }
    }
}
