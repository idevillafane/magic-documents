use crate::core::frontmatter;
use crate::tags;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use serde_yaml::Value;
use std::fs;
use std::path::Path;
use tui_textarea::TextArea;

/// Opens integrated text editor with ratatui
/// Returns true if saved, false if cancelled with ESC
pub fn open(file_path: &Path, vault_root: &Path) -> anyhow::Result<bool> {
    // Save as last opened note
    let _ = crate::commands::recent::save_last_note(vault_root, file_path);
    open_impl(file_path, vault_root, vault_root, None)
}

/// Opens integrated text editor with specified editor override
pub fn open_with_editor(
    file_path: &Path,
    vault_root: &Path,
    editor: Option<String>,
) -> anyhow::Result<bool> {
    // Save as last opened note
    let _ = crate::commands::recent::save_last_note(vault_root, file_path);
    open_impl(file_path, vault_root, vault_root, editor)
}

fn open_impl(
    file_path: &Path,
    vault_root: &Path,
    vault_for_tags: &Path,
    editor_override: Option<String>,
) -> anyhow::Result<bool> {
    let content = if file_path.exists() {
        fs::read_to_string(file_path)
            .map_err(|e| anyhow::anyhow!("Error leyendo {}: {}", file_path.display(), e))?
    } else {
        anyhow::bail!("Archivo no existe: {}", file_path.display());
    };

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut textarea = TextArea::new(content.lines().map(|s| s.to_string()).collect());

    let num_lines = textarea.lines().len();
    if num_lines > 0 {
        textarea.move_cursor(tui_textarea::CursorMove::Jump(num_lines as u16 - 1, 0));
        textarea.move_cursor(tui_textarea::CursorMove::End);
    }

    let display_path = file_path
        .strip_prefix(vault_root)
        .unwrap_or(file_path)
        .display()
        .to_string();

    let saved;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(3)])
                .split(f.area());

            let title = format!(" Vault → {} ", display_path);
            let editor_block = Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(Style::default().fg(Color::Cyan));

            let inner = editor_block.inner(chunks[0]);
            f.render_widget(editor_block, chunks[0]);
            f.render_widget(&textarea, inner);

            let (row, col) = textarea.cursor();
            let status = format!(
                " Line {}, Col {} | Ctrl+S: Save | Ctrl+T: Tags | Ctrl+G: Editor Alt | Ctrl+R: Rename | Ctrl+D: Delete | ESC: Exit ",
                row + 1,
                col + 1
            );
            let status_widget = Paragraph::new(status)
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(status_widget, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                    let text = textarea.lines().join("\n");
                    fs::write(file_path, text)?;
                    saved = true;
                    break;
                }
                (KeyCode::Esc, _) => {
                    saved = false;
                    break;
                }
                (KeyCode::Char('t'), KeyModifiers::CONTROL) => {
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    terminal.show_cursor()?;

                    println!("\nSelecciona tags para agregar:");
                    let new_tag = tags::selector::select_with_fuzzy(vault_for_tags)?;

                    if !new_tag.is_empty() {
                        let current_text = textarea.lines().join("\n");
                        if let Ok((mut fm, body)) = frontmatter::extract(&current_text) {
                            let mut existing_tags: Vec<String> = Vec::new();
                            for key in ["tags", "tag", "Tags", "Tag"] {
                                if let Some(Value::Sequence(tag_list)) =
                                    fm.get(&Value::String(key.to_string()))
                                {
                                    for tag in tag_list {
                                        if let Value::String(t) = tag {
                                            existing_tags.push(t.clone());
                                        }
                                    }
                                    break;
                                }
                            }

                            // Add new tag if not already present (slash-separated format)
                            if !existing_tags.contains(&new_tag) {
                                existing_tags.push(new_tag);
                            }

                            let tags_value = Value::Sequence(
                                existing_tags
                                    .iter()
                                    .map(|t| Value::String(t.clone()))
                                    .collect(),
                            );
                            fm.insert(Value::String("tags".to_string()), tags_value);

                            let new_content =
                                format!("---\n{}---{}", serde_yaml::to_string(&fm)?, body);

                            let new_lines: Vec<String> =
                                new_content.lines().map(|s| s.to_string()).collect();
                            textarea = TextArea::new(new_lines);

                            let num_lines = textarea.lines().len();
                            if num_lines > 0 {
                                textarea.move_cursor(tui_textarea::CursorMove::Jump(
                                    num_lines as u16 - 1,
                                    0,
                                ));
                                textarea.move_cursor(tui_textarea::CursorMove::End);
                            }
                        }
                    }

                    execute!(std::io::stdout(), EnterAlternateScreen)?;
                    enable_raw_mode()?;
                }
                (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                    // Delete file
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    terminal.show_cursor()?;

                    use dialoguer::Confirm;
                    let confirm = Confirm::new()
                        .with_prompt(format!("¿Eliminar permanentemente '{}'?", display_path))
                        .default(false)
                        .interact_opt()?;

                    if confirm.unwrap_or(false) {
                        fs::remove_file(file_path)?;
                        println!("\n✓ Archivo eliminado");
                        return Ok(false);
                    }

                    execute!(std::io::stdout(), EnterAlternateScreen)?;
                    enable_raw_mode()?;
                }
                (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                    // Rename file
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    terminal.show_cursor()?;

                    println!("\nRenombrar archivo");
                    println!("Nombre actual: {}", display_path);

                    use crate::ui::input::input_with_esc;

                    match input_with_esc("Nuevo nombre (sin extensión)")? {
                        Some(new_name) if !new_name.trim().is_empty() => {
                            let parent = file_path.parent().unwrap();
                            let new_path = parent.join(format!("{}.md", new_name.trim()));

                            if new_path.exists() {
                                println!("\n✗ Error: Ya existe un archivo con ese nombre");
                            } else {
                                fs::rename(file_path, &new_path)?;
                                println!("\n✓ Archivo renombrado");

                                // Update last note reference
                                let _ =
                                    crate::commands::recent::save_last_note(vault_root, &new_path);

                                // Update file_path for the rest of the session
                                disable_raw_mode()?;
                                return open_impl(
                                    &new_path,
                                    vault_root,
                                    vault_for_tags,
                                    editor_override,
                                );
                            }
                        }
                        _ => {
                            println!("\nRenombrado cancelado");
                        }
                    }

                    execute!(std::io::stdout(), EnterAlternateScreen)?;
                    enable_raw_mode()?;
                }
                (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
                    // Open in external editor and exit TUI
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    terminal.show_cursor()?;

                    // Save current content before opening external editor
                    let text = textarea.lines().join("\n");
                    fs::write(file_path, &text)?;

                    // Determine which editor to use: editor_override or config.editor
                    let editor_to_use = if let Some(ref e) = editor_override {
                        Some(e.clone())
                    } else if let Ok(config) = crate::core::config::Config::load_default() {
                        config.editor.clone().or(Some("vi".to_string()))
                    } else {
                        Some("vi".to_string())
                    };

                    if let Some(editor_cmd) = editor_to_use {
                        println!("\nAbriendo en {}...", editor_cmd);
                        let status = std::process::Command::new(&editor_cmd)
                            .arg(file_path)
                            .status();

                        if let Err(e) = status {
                            eprintln!("Error al abrir {}: {}", editor_cmd, e);
                        }
                    } else {
                        eprintln!("\n⚠️  No se pudo determinar qué editor usar");
                    }

                    // Exit TUI completely (don't return to it)
                    saved = true;
                    break;
                }
                (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                    textarea.undo();
                }
                (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                    textarea.redo();
                }
                _ => {
                    textarea.input(key);
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(saved)
}
