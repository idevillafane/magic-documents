use crate::core::config::Config;
use crate::core::frontmatter;
use crate::core::template;
use crate::tags;
use crate::ui::editor;
use crate::ui::prompts;
use crate::utils::file;
use chrono::Local;
use serde_yaml::Value;
use slug::slugify;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct NoteBuilder {
    vault: PathBuf,
    config: Config,
    title: Option<String>,
    target_dir: Option<PathBuf>,
    use_hierarchical_tags: bool,
    editor_override: Option<String>,
}

impl NoteBuilder {
    pub fn new(vault: PathBuf, config: Config) -> Self {
        Self {
            vault,
            config,
            title: None,
            target_dir: None,
            use_hierarchical_tags: false,
            editor_override: None,
        }
    }

    pub fn title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    pub fn target_directory(mut self, dir: PathBuf) -> Self {
        self.target_dir = Some(dir);
        self
    }

    pub fn hierarchical_tags(mut self, use_hierarchical: bool) -> Self {
        self.use_hierarchical_tags = use_hierarchical;
        self
    }

    pub fn editor(mut self, editor: Option<String>) -> Self {
        self.editor_override = editor;
        self
    }

    pub fn create(self) -> anyhow::Result<()> {
        // Determine target directory
        let notas_dir = if let Some(ref dir) = self.target_dir {
            // Use specified directory (resolve relative to current dir)
            if dir.is_absolute() {
                dir.clone()
            } else {
                std::env::current_dir()?.join(dir)
            }
        } else {
            self.vault.join(&self.config.notes_dir)
        };
        std::fs::create_dir_all(&notas_dir)?;

        let target_file = self.build_target_path(&notas_dir)?;

        // If file exists, handle reopening
        if target_file.exists() {
            return self.reopen_existing_file(&target_file);
        }

        // Create new note
        self.create_new_note(&target_file, &notas_dir)
    }

    fn build_target_path(&self, notas_dir: &Path) -> anyhow::Result<PathBuf> {
        let now = Local::now();
        let date = now.format(&self.config.date).to_string();
        let time = now.format(&self.config.time).to_string();

        let filename_stem = if let Some(t) = self.title.as_ref() {
            if t.ends_with(".md")
                || t.ends_with(".txt")
                || t.ends_with(".canvas")
                || t.ends_with(".excalidraw")
            {
                t.strip_suffix(".md")
                    .or_else(|| t.strip_suffix(".txt"))
                    .or_else(|| t.strip_suffix(".canvas"))
                    .or_else(|| t.strip_suffix(".excalidraw"))
                    .unwrap_or(t)
                    .to_string()
            } else {
                slugify(t)
            }
        } else {
            // Sin título: usa date + time para diferenciarse de daily note
            format!("{} {}", date, time)
        };

        let filename = format!("{}.md", filename_stem);
        Ok(notas_dir.join(&filename))
    }

    fn reopen_existing_file(&self, target_file: &Path) -> anyhow::Result<()> {
        Self::add_timestamp_and_open(
            target_file,
            &self.vault,
            &self.config,
            self.editor_override.clone(),
        )
    }

    /// Public method to add timestamp and open existing file
    pub fn add_timestamp_and_open(
        target_file: &Path,
        vault: &Path,
        config: &Config,
        editor_override: Option<String>,
    ) -> anyhow::Result<()> {
        let do_timeprint = config.timeprint.unwrap_or(false);

        if do_timeprint {
            let timestamp_now = Local::now();
            let date = timestamp_now.format(&config.date).to_string();
            let time = timestamp_now.format(&config.time).to_string();
            let stamp = format!("@{} {}", date, time);
            let mut f = OpenOptions::new().append(true).open(target_file)?;
            writeln!(f)?;
            writeln!(f, "{}", stamp)?;
            writeln!(f)?;
        }

        // Use editor_override if provided, otherwise use config
        if let Some(ref editor_cmd) = editor_override {
            std::process::Command::new(editor_cmd)
                .arg(target_file)
                .status()?;
        } else {
            let editor_mode = config.editor_mode.as_deref().unwrap_or("integrated");

            if editor_mode == "integrated" {
                editor::open(target_file, vault)?;
            } else {
                let editor = config.editor.as_deref().unwrap_or("vi");
                std::process::Command::new(editor)
                    .arg(target_file)
                    .status()?;
            }
        }

        Ok(())
    }

    fn create_new_note(&self, target_file: &Path, notas_dir: &Path) -> anyhow::Result<()> {
        // Try to load template from centralized Templates directory first
        let centralized_template = self
            .vault
            .join(&self.config.templates_dir)
            .join(format!("{}.md", &self.config.notes_dir));
        let local_template = notas_dir.join("template.txt");

        let template_path = if centralized_template.exists() {
            centralized_template
        } else {
            local_template
        };

        let (frontmatter_map, body) = template::read(&template_path)?;

        // Select tags - now returns slash-separated string (e.g., "padre/hijo/nieto")
        let selected_tag = if self.target_dir.is_some() {
            // Derive tag from directory path relative to vault
            self.derive_tag_from_dir(notas_dir)?
        } else if self.use_hierarchical_tags {
            match tags::selector::select_hierarchical(&self.vault) {
                Ok(tag) => tag,
                Err(_) => {
                    println!("\nCreación de nota cancelada.");
                    return Ok(());
                }
            }
        } else {
            match tags::selector::select_with_fuzzy(&self.vault) {
                Ok(tag) => tag,
                Err(_) => {
                    println!("\nCreación de nota cancelada.");
                    return Ok(());
                }
            }
        };

        // Select aliases
        let selected_aliases = match prompts::select_aliases()? {
            Some(aliases) => aliases,
            None => {
                println!("\nCreación de nota cancelada.");
                return Ok(());
            }
        };

        // Build variables
        let vars = self.build_variables()?;

        // Render and build frontmatter
        let mut rendered_map = frontmatter::render(frontmatter_map, &vars);

        // Add tags as slash-separated string: ["padre/hijo/nieto"]
        if !selected_tag.is_empty() {
            let tags_value = Value::Sequence(vec![Value::String(selected_tag)]);
            rendered_map.insert(Value::String("tags".to_string()), tags_value);
        }

        // Add aliases
        if !selected_aliases.is_empty() {
            let aliases_value = Value::Sequence(
                selected_aliases
                    .iter()
                    .map(|a| Value::String(a.clone()))
                    .collect(),
            );
            rendered_map.insert(Value::String("aliases".to_string()), aliases_value);
        }

        // Render body
        let rendered_body = template::render_body(&body, &vars);

        // Write file
        file::write_note(target_file, &rendered_map, &rendered_body)?;

        // Open editor
        self.open_editor_new_file(target_file)
    }

    fn build_variables(&self) -> anyhow::Result<BTreeMap<String, String>> {
        let now = Local::now();
        let date = now.format(&self.config.date).to_string();
        let time = now.format(&self.config.time).to_string();

        let filename_stem = if let Some(t) = self.title.as_ref() {
            if t.ends_with(".md")
                || t.ends_with(".txt")
                || t.ends_with(".canvas")
                || t.ends_with(".excalidraw")
            {
                t.strip_suffix(".md")
                    .or_else(|| t.strip_suffix(".txt"))
                    .or_else(|| t.strip_suffix(".canvas"))
                    .or_else(|| t.strip_suffix(".excalidraw"))
                    .unwrap_or(t)
                    .to_string()
            } else {
                slugify(t)
            }
        } else {
            // Sin título: usa date + time para diferenciarse de daily note
            format!("{} {}", date, time)
        };

        let mut vars = BTreeMap::new();
        vars.insert("date".to_string(), date);
        vars.insert("time".to_string(), time);
        vars.insert("title".to_string(), filename_stem);

        Ok(vars)
    }

    fn open_editor_new_file(&self, target_file: &Path) -> anyhow::Result<()> {
        // Use editor_override if provided
        if let Some(ref editor_cmd) = self.editor_override {
            let status = std::process::Command::new(editor_cmd)
                .arg(target_file)
                .status()?;
            if !status.success() {
                eprintln!("Editor exited with non-zero status");
            }
            println!("Creado: {}", target_file.display());
            self.update_tag_cache()?;
        } else {
            let editor_mode = self.config.editor_mode.as_deref().unwrap_or("integrated");

            if editor_mode == "integrated" {
                let saved = editor::open(target_file, &self.vault)?;
                if saved {
                    println!("Creado: {}", target_file.display());
                    self.update_tag_cache()?;
                } else {
                    println!("Edición cancelada sin guardar");
                    let _ = std::fs::remove_file(target_file);
                }
            } else {
                let editor = self.config.editor.as_deref().unwrap_or("vi");
                let status = std::process::Command::new(editor)
                    .arg(target_file)
                    .status()?;
                if !status.success() {
                    eprintln!("Editor exited con non-zero status");
                }
                println!("Creado: {}", target_file.display());
                self.update_tag_cache()?;
            }
        }

        Ok(())
    }

    /// Derive tag from directory path relative to vault
    /// Example: vault/Notas/proyecto/cliente → "proyecto/cliente"
    fn derive_tag_from_dir(&self, dir: &Path) -> anyhow::Result<String> {
        let notes_base = self.vault.join(&self.config.notes_dir);

        // Try to strip notes_dir prefix, then vault prefix
        let relative = dir
            .strip_prefix(&notes_base)
            .or_else(|_| dir.strip_prefix(&self.vault))
            .unwrap_or(dir);

        // Convert path components to slash-separated tag
        let tag = relative
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect::<Vec<_>>()
            .join("/");

        Ok(tag)
    }

    fn update_tag_cache(&self) -> anyhow::Result<()> {
        // Skip cache update - causes issues when vault path has special chars
        // TODO: Fix cache update for paths with spaces
        Ok(())
    }
}
