use crate::core::config::Config;
use crate::core::note::NoteBuilder;
use crate::core::template;
use chrono::Local;
use std::fs;
use std::path::PathBuf;

pub fn run(config: Config, vault: PathBuf, editor: Option<String>) -> anyhow::Result<()> {
    let today = Local::now().format("%Y-%m-%d").to_string();

    // Daily notes go in configured diary directory
    let diario_dir = vault.join(&config.diary_dir);
    fs::create_dir_all(&diario_dir)?;

    let daily_file = diario_dir.join(format!("{}.md", today));

    // If daily note exists, open it with timestamp
    if daily_file.exists() {
        println!("Abriendo daily note: {}", today);
        NoteBuilder::add_timestamp_and_open(&daily_file, &vault, &config, editor)?;
        return Ok(());
    }

    // Create new daily note
    println!("Creando daily note: {}", today);

    // Try to load template from centralized Templates directory first
    let centralized_template = vault
        .join(&config.templates_dir)
        .join(format!("{}.md", &config.diary_dir));
    let local_template = diario_dir.join("template.txt");

    let template_path = if centralized_template.exists() {
        centralized_template
    } else {
        local_template
    };

    let content = if template_path.exists() {
        // Load and render template
        let (frontmatter_map, body) = template::read(&template_path)?;

        // Build variables for template rendering
        let mut vars = std::collections::BTreeMap::new();
        let now = Local::now();
        vars.insert("date".to_string(), now.format(&config.date).to_string());
        vars.insert("time".to_string(), now.format(&config.time).to_string());
        vars.insert("title".to_string(), today.clone());

        // Render frontmatter and body
        let rendered_fm = crate::core::frontmatter::render(frontmatter_map, &vars);
        let rendered_body = template::render_body(&body, &vars);

        format!(
            "---\n{}---\n{}",
            serde_yaml::to_string(&rendered_fm)?,
            rendered_body
        )
    } else {
        // Create generic daily note
        let now = Local::now();
        format!(
            "---\ndate: {}\ntime: {}\n---\n\n# {}\n\n",
            now.format(&config.date),
            now.format(&config.time),
            today
        )
    };

    // Write the file
    fs::write(&daily_file, content)?;

    // Open in editor (as new file, no timestamp)
    if let Some(editor_cmd) = editor {
        std::process::Command::new(editor_cmd)
            .arg(&daily_file)
            .status()?;
    } else {
        use crate::ui::editor;
        editor::open(&daily_file, &vault)?;
    }

    Ok(())
}
