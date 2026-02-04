use crate::commands::rcal_tasks;
use crate::core::config::Config;
use crate::utils::vault::VaultWalker;
use chrono::{DateTime, Local, NaiveDate};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ─── Structs unificados ─────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum TaskSource {
    Markdown {
        path: PathBuf,
        line_number: usize,
    },
    Ical {
        file_path: PathBuf,
    },
}

#[derive(Clone, Debug)]
struct Task {
    title: String,
    source: TaskSource,
    meta_date: String,
    meta_label: String,
}

/// Acción retornada por el TUI
enum Action {
    MarkDone(usize),
    Migrate(usize),
    CreateNew,
    Quit,
}

// ─── Entry point ─────────────────────────────────────────────────────────────

pub fn run(vault: PathBuf, config: Config, mark_all: bool, full: bool) -> anyhow::Result<()> {
    if mark_all {
        return run_mark_all(&vault, &config);
    }
    if full {
        return run_tui(&vault, &config);
    }
    run_simple(&vault, &config)
}

// ─── Path mark_all: dialoguer original, solo tareas md ──────────────────────

fn run_mark_all(vault: &Path, config: &Config) -> anyhow::Result<()> {
    let md_tasks = collect_md_tasks(vault, config)?;

    if md_tasks.is_empty() {
        println!("No se encontraron tareas pendientes en el vault.");
        return Ok(());
    }

    let mark = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("¿Quieres marcar TODAS las tareas como listas?")
        .default(false)
        .interact()?;

    if !mark {
        return Ok(());
    }

    let mut by_file: HashMap<PathBuf, Vec<usize>> = HashMap::new();
    for task in &md_tasks {
        if let TaskSource::Markdown { path, line_number } = &task.source {
            by_file.entry(path.clone()).or_default().push(*line_number);
        }
    }

    let mut updated = 0usize;
    for (path, line_numbers) in by_file {
        updated += mark_tasks_in_file(&path, &line_numbers, "- [x] ")?;
    }

    println!("✅ Tareas marcadas como listas: {}", updated);
    Ok(())
}

// ─── Path simple: dialoguer Select, md + ical ───────────────────────────────

fn run_simple(vault: &Path, config: &Config) -> anyhow::Result<()> {
    loop {
        let tasks = collect_all_tasks(vault, config)?;

        if tasks.is_empty() {
            println!("No se encontraron tareas pendientes.");
            return Ok(());
        }

        let (term_width, _) = crossterm::terminal::size().unwrap_or((80, 24));
        let term_width = term_width as usize;

        let items: Vec<String> = tasks
            .iter()
            .map(|task| {
                let meta = if task.meta_label.is_empty() {
                    format!("({})", task.meta_date)
                } else {
                    format!("({} {})", task.meta_date, task.meta_label)
                };
                let meta_len = meta.chars().count();
                let checkbox_width = 4; // "[ ] "
                let available = term_width.saturating_sub(checkbox_width + meta_len + 2);

                let title: String = if task.title.chars().count() > available {
                    let truncated: String =
                        task.title.chars().take(available.saturating_sub(3)).collect();
                    format!("{}...", truncated)
                } else {
                    task.title.clone()
                };

                let padding = available.saturating_sub(title.chars().count());
                format!("[ ] {}{:width$}{}", title, "", meta, width = padding)
            })
            .collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .items(&items)
            .default(0)
            .interact_opt()?;

        let Some(idx) = selection else {
            return Ok(());
        };

        let task = &tasks[idx];
        match &task.source {
            TaskSource::Ical { file_path } => {
                rcal_tasks::toggle_task(file_path)?;
                println!("✓ Tarea marcada como lista: {}", task.title);
            }
            TaskSource::Markdown { path, line_number } => {
                if rcal_tasks::rcal_available() {
                    let migrate = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("¿Migrar a rcal?")
                        .default(true)
                        .interact()?;

                    if migrate {
                        mark_tasks_in_file(path, &[*line_number], "- [M] ")?;
                        match rcal_tasks::run_rcal_todo(&task.title, None, None, None, None) {
                            Ok(()) => println!("✓ Tarea migrada a rcal: {}", task.title),
                            Err(e) => {
                                let _ = rollback_migrate(path, *line_number);
                                eprintln!("✗ Error en rcal todo: {}", e);
                            }
                        }
                    } else {
                        mark_tasks_in_file(path, &[*line_number], "- [x] ")?;
                        println!("✓ Tarea marcada como lista: {}", task.title);
                    }
                } else {
                    mark_tasks_in_file(path, &[*line_number], "- [x] ")?;
                    println!("✓ Tarea marcada como lista: {}", task.title);
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(600));
    }
}

// ─── Path TUI: ratatui con tareas md + ical ─────────────────────────────────

fn run_tui(vault: &Path, config: &Config) -> anyhow::Result<()> {
    loop {
        let tasks = collect_all_tasks(vault, config)?;

        if tasks.is_empty() {
            println!("No se encontraron tareas pendientes.");
            return Ok(());
        }

        let action = run_task_tui(&tasks)?;

        match action {
            Action::Quit => return Ok(()),

            Action::MarkDone(idx) => {
                let task = &tasks[idx];
                match &task.source {
                    TaskSource::Markdown { path, line_number } => {
                        mark_tasks_in_file(path, &[*line_number], "- [x] ")?;
                        println!("✓ Tarea marcada como lista: {}", task.title);
                        std::thread::sleep(std::time::Duration::from_millis(600));
                    }
                    TaskSource::Ical { file_path } => {
                        rcal_tasks::toggle_task(file_path)?;
                        println!("✓ Tarea marcada como lista: {}", task.title);
                        std::thread::sleep(std::time::Duration::from_millis(600));
                    }
                }
            }

            Action::Migrate(idx) => {
                let task = &tasks[idx];
                let TaskSource::Markdown { path, line_number } = &task.source else {
                    continue;
                };

                if !rcal_tasks::rcal_available() {
                    println!("✗ `rcal` no encontrado en PATH. No se puede migrar.");
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    continue;
                }

                // Marcar [M] primero
                mark_tasks_in_file(path, &[*line_number], "- [M] ")?;

                // Prompts fuera del TUI
                match prompt_rcal_flags(Some(&task.title))? {
                    Some((title, cal, date, time, dur)) => {
                        match rcal_tasks::run_rcal_todo(
                            &title,
                            cal.as_deref(),
                            date.as_deref(),
                            time.as_deref(),
                            dur.as_deref(),
                        ) {
                            Ok(()) => {
                                println!("✓ Tarea migrada a rcal: {}", title);
                            }
                            Err(e) => {
                                // Rollback: restaurar [ ]
                                let _ = rollback_migrate(path, *line_number);
                                eprintln!("✗ Error en rcal todo: {}", e);
                            }
                        }
                    }
                    None => {
                        // Cancelado → rollback
                        let _ = rollback_migrate(path, *line_number);
                        println!("Migración cancelada.");
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(600));
            }

            Action::CreateNew => {
                if !rcal_tasks::rcal_available() {
                    println!("✗ `rcal` no encontrado en PATH. No se puede crear tarea.");
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    continue;
                }

                match prompt_rcal_flags(None)? {
                    Some((title, cal, date, time, dur)) => {
                        match rcal_tasks::run_rcal_todo(
                            &title,
                            cal.as_deref(),
                            date.as_deref(),
                            time.as_deref(),
                            dur.as_deref(),
                        ) {
                            Ok(()) => println!("✓ Tarea creada en rcal: {}", title),
                            Err(e) => eprintln!("✗ Error en rcal todo: {}", e),
                        }
                    }
                    None => println!("Creación cancelada."),
                }
                std::thread::sleep(std::time::Duration::from_millis(600));
            }
        }
    }
}

// ─── TUI ratatui ─────────────────────────────────────────────────────────────

fn run_task_tui(tasks: &[Task]) -> anyhow::Result<Action> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Calcular índice del separador (si hay ambos tipos)
    let separator_idx = find_separator_index(tasks);

    // Estado de la lista: la lista renderizada puede tener un separador extra
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    let action = loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(3)])
                .split(f.area());

            // Construir items de la lista
            let items = build_list_items(tasks, separator_idx, list_state.selected());
            let item_count = items.len();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Tareas pendientes ")
                        .style(Style::default().fg(Color::Cyan)),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::Cyan)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("› ");

            f.render_stateful_widget(list, chunks[0], &mut list_state);

            let hints = if item_count > 0 {
                " ↑↓ Navegar | Enter: Marcar lista | n: Nueva | c: Migrar a rcal | ESC: Salir "
            } else {
                " ESC: Salir "
            };
            let status = Paragraph::new(hints)
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(status, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => {
                    break Action::Quit;
                }
                KeyCode::Down => {
                    let max = visible_item_count(tasks, separator_idx);
                    move_selection(&mut list_state, max, separator_idx, true);
                }
                KeyCode::Up => {
                    let max = visible_item_count(tasks, separator_idx);
                    move_selection(&mut list_state, max, separator_idx, false);
                }
                KeyCode::Enter => {
                    if let Some(sel) = list_state.selected() {
                        if let Some(task_idx) = visible_to_task_idx(sel, separator_idx) {
                            break Action::MarkDone(task_idx);
                        }
                    }
                }
                KeyCode::Char('n') if key.modifiers.is_empty() => {
                    break Action::CreateNew;
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    if let Some(sel) = list_state.selected() {
                        if let Some(task_idx) = visible_to_task_idx(sel, separator_idx) {
                            // Solo permitir migrar tareas md
                            if matches!(tasks[task_idx].source, TaskSource::Markdown { .. }) {
                                break Action::Migrate(task_idx);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(action)
}

/// Construye los ListItems, insertando un separador visual entre vault y rcal
fn build_list_items(
    tasks: &[Task],
    separator_idx: Option<usize>,
    selected: Option<usize>,
) -> Vec<ListItem<'static>> {
    let mut items: Vec<ListItem> = Vec::new();

    for (i, task) in tasks.iter().enumerate() {
        // Insertar separador antes del primer task ical
        if separator_idx == Some(i) {
            let sep = ListItem::new(
                ratatui::text::Line::from("─── rcal ────────────────────────────────────────")
                    .style(Style::default().fg(Color::DarkGray)),
            );
            items.push(sep);
        }

        let visible_idx = if let Some(sep) = separator_idx {
            if i >= sep { i + 1 } else { i }
        } else {
            i
        };

        let is_selected = selected == Some(visible_idx);
        let prefix = if is_selected { "" } else { "[ ] " };

        let label = format!(
            "{}{}",
            prefix,
            task.title
        );

        let meta = format!("({} {})", task.meta_date, task.meta_label);
        let line = ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(label, Style::default().fg(Color::White)),
            ratatui::text::Span::raw("  "),
            ratatui::text::Span::styled(meta, Style::default().fg(Color::DarkGray)),
        ]);

        items.push(ListItem::new(line));
    }

    items
}

// ─── Helpers de navegación ───────────────────────────────────────────────────

/// Retorna el índice en tasks[] donde comienzan las tareas ical (si existen ambos tipos)
fn find_separator_index(tasks: &[Task]) -> Option<usize> {
    let has_md = tasks.iter().any(|t| matches!(t.source, TaskSource::Markdown { .. }));
    let first_ical = tasks
        .iter()
        .position(|t| matches!(t.source, TaskSource::Ical { .. }));

    if has_md && first_ical.is_some() {
        first_ical
    } else {
        None
    }
}

/// Cantidad total de items visibles (tasks + separador si existe)
fn visible_item_count(tasks: &[Task], separator_idx: Option<usize>) -> usize {
    tasks.len() + if separator_idx.is_some() { 1 } else { 0 }
}

/// Convierte índice visible → índice en tasks[], saltando separador
fn visible_to_task_idx(visible: usize, separator_idx: Option<usize>) -> Option<usize> {
    match separator_idx {
        Some(sep) => {
            if visible == sep {
                None // Es el separador
            } else if visible > sep {
                Some(visible - 1)
            } else {
                Some(visible)
            }
        }
        None => Some(visible),
    }
}

/// Mueve la selección, saltando el separador
fn move_selection(
    state: &mut ListState,
    max: usize,
    separator_idx: Option<usize>,
    down: bool,
) {
    let current = state.selected().unwrap_or(0);
    let next = if down {
        if current + 1 >= max { 0 } else { current + 1 }
    } else if current == 0 {
        max - 1
    } else {
        current - 1
    };

    // Si el siguiente es el separador, saltar uno más
    let next = if Some(next) == separator_idx {
        if down {
            if next + 1 >= max { 0 } else { next + 1 }
        } else if next == 0 {
            max - 1
        } else {
            next - 1
        }
    } else {
        next
    };

    state.select(Some(next));
}

// ─── Prompts de flags (dialoguer, fuera del TUI) ────────────────────────────

/// (título, calendario, fecha, hora, duración)
type RcalFlags = (String, Option<String>, Option<String>, Option<String>, Option<String>);

/// Muestra prompts para crear/migrar tarea en rcal.
/// Si `prefill_title` es Some, se usa como default del campo título.
/// Retorna None si el usuario cancela (ESC / Ctrl+C en dialoguer).
fn prompt_rcal_flags(prefill_title: Option<&str>) -> anyhow::Result<Option<RcalFlags>> {

    let title: String = if let Some(default) = prefill_title {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Título")
            .default(default.to_string())
            .interact()?
    } else {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Título")
            .interact()?
    };

    if title.trim().is_empty() {
        return Ok(None);
    }

    let calendar: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Calendario (vacío = default)")
        .default(String::new())
        .interact()?;

    let date: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Fecha (vacío = hoy)")
        .default(String::new())
        .interact()?;

    let time: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Hora (vacío = auto)")
        .default(String::new())
        .interact()?;

    let duration: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Duración (vacío = default)")
        .default(String::new())
        .interact()?;

    Ok(Some((
        title,
        if calendar.is_empty() { None } else { Some(calendar) },
        if date.is_empty() { None } else { Some(date) },
        if time.is_empty() { None } else { Some(time) },
        if duration.is_empty() { None } else { Some(duration) },
    )))
}

// ─── Rollback migración ──────────────────────────────────────────────────────

fn rollback_migrate(path: &Path, line_number: usize) -> anyhow::Result<()> {
    let content = fs::read_to_string(path)?;
    let mut lines: Vec<String> = content.split('\n').map(|s| s.to_string()).collect();

    let idx = line_number - 1;
    if idx < lines.len() && lines[idx].starts_with("- [M] ") {
        lines[idx] = lines[idx].replacen("- [M] ", "- [ ] ", 1);
        fs::write(path, lines.join("\n"))?;
    }
    Ok(())
}

// ─── Recolección de tareas ───────────────────────────────────────────────────

/// Recolecta tareas md + ical. Degradación gracil: si no hay config rcal, solo md.
fn collect_all_tasks(vault: &Path, config: &Config) -> anyhow::Result<Vec<Task>> {
    let mut tasks = collect_md_tasks(vault, config)?;

    // Intentar cargar tareas ical (degradación gracil)
    if let Some(rcal_cfg_path) = rcal_tasks::find_rcal_config(config.rcal_config.as_deref()) {
        if let Ok(rcal_cfg) = rcal_tasks::read_rcal_config(&rcal_cfg_path) {
            if let Ok(ical_tasks) = rcal_tasks::read_pending_tasks(&rcal_cfg) {
                for it in ical_tasks {
                    tasks.push(Task {
                        title: it.summary,
                        source: TaskSource::Ical {
                            file_path: it.file_path,
                        },
                        meta_date: it
                            .start
                            .map(|dt| dt.format("%d/%m %H:%M").to_string())
                            .unwrap_or_else(|| "--/-- --:--".to_string()),
                        meta_label: String::new(),
                    });
                }
            }
        }
    }

    Ok(tasks)
}

/// Recolecta solo tareas markdown del vault
fn collect_md_tasks(vault: &Path, config: &Config) -> anyhow::Result<Vec<Task>> {
    let templates_path = vault.join(&config.templates_dir);
    let diario_dir = vault.join(&config.diary_dir);
    let mut tasks = Vec::new();

    VaultWalker::new(vault)
        .exclude_templates(&templates_path)
        .walk(|path, content| {
            let mut in_code_block = false;
            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("sin-titulo")
                .to_string();
            let meta_label = if path.starts_with(&diario_dir) {
                "diario".to_string()
            } else {
                file_stem
            };
            let meta_date = task_meta_date(path, &diario_dir)?;

            for (idx, line) in content.split('\n').enumerate() {
                let trimmed = line.trim_start();

                if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                    in_code_block = !in_code_block;
                    continue;
                }

                if !in_code_block && line.starts_with("- [ ] ") {
                    let title = line
                        .trim_start()
                        .strip_prefix("- [ ] ")
                        .unwrap_or(line)
                        .trim_end()
                        .to_string();

                    tasks.push(Task {
                        title,
                        source: TaskSource::Markdown {
                            path: path.to_path_buf(),
                            line_number: idx + 1,
                        },
                        meta_date: meta_date.clone(),
                        meta_label: meta_label.clone(),
                    });
                }
            }
            Ok(())
        })?;

    Ok(tasks)
}

// ─── Utilidades de archivo ───────────────────────────────────────────────────

fn task_meta_date(path: &Path, diario_dir: &Path) -> anyhow::Result<String> {
    if path.starts_with(diario_dir) {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if let Ok(date) = NaiveDate::parse_from_str(stem, "%Y-%m-%d") {
                return Ok(date.format("%d/%m").to_string());
            }
        }
    }

    let metadata = fs::metadata(path)?;
    let modified = metadata.modified()?;
    let datetime: DateTime<Local> = modified.into();
    Ok(datetime.format("%d/%m").to_string())
}

/// Marca líneas en un archivo md reemplazando `- [ ] ` por `replacement`.
/// Retorna la cantidad de líneas reemplazadas.
fn mark_tasks_in_file(path: &Path, line_numbers: &[usize], replacement: &str) -> anyhow::Result<usize> {
    let content = fs::read_to_string(path)?;
    let mut lines: Vec<String> = content.split('\n').map(|s| s.to_string()).collect();

    let mut updated = 0usize;
    for &line_number in line_numbers {
        if line_number == 0 {
            continue;
        }
        let idx = line_number - 1;
        if idx >= lines.len() {
            continue;
        }
        let line = &lines[idx];
        if line.starts_with("- [ ] ") {
            lines[idx] = line.replacen("- [ ] ", replacement, 1);
            updated += 1;
        }
    }

    if updated > 0 {
        fs::write(path, lines.join("\n"))?;
    }

    Ok(updated)
}
