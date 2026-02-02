use crate::core::config::Config;
use crate::core::note::NoteBuilder;
use std::path::PathBuf;

pub fn run(
    config: Config,
    vault: PathBuf,
    title: Option<String>,
    target_dir: Option<PathBuf>,
    editor: Option<String>,
) -> anyhow::Result<()> {
    let mut builder = NoteBuilder::new(vault, config)
        .title(title)
        .hierarchical_tags(true)
        .editor(editor);

    if let Some(dir) = target_dir {
        builder = builder.target_directory(dir);
    }

    builder.create()?;

    Ok(())
}
