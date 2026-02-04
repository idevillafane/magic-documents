use chrono::{Duration, NaiveDateTime};
use std::fs;
use std::path::{Path, PathBuf};

/// Config mínima de un calendario rcal (solo lo que necesitamos)
#[derive(Debug, Clone)]
pub struct RcalCalendar {
    pub name: String,
    pub path: PathBuf,
}

/// Config completa de rcal que usa mad (calendarios + ventana de tiempo)
#[derive(Debug, Clone)]
pub struct RcalConfig {
    pub calendars: Vec<RcalCalendar>,
    pub time_backward: Duration,
    pub time_forward: Duration,
}

/// Tarea extraída de un archivo .ics
#[derive(Debug, Clone)]
pub struct IcalTask {
    pub summary: String,
    pub start: Option<NaiveDateTime>,
    pub completed: bool,
    pub file_path: PathBuf,
    pub calendar_name: String,
}

/// Busca config de rcal.
/// Orden: mad config.rcal_config → ~/.config/rcal/config.toml → ~/.rcal/config.toml
pub fn find_rcal_config(mad_override: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = mad_override {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    let home = dirs::home_dir()?;

    let xdg = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join(".config"));

    let candidate = xdg.join("rcal").join("config.toml");
    if candidate.exists() {
        return Some(candidate);
    }

    let candidate = home.join(".rcal").join("config.toml");
    if candidate.exists() {
        return Some(candidate);
    }

    None
}

/// Parse config de rcal → RcalConfig (calendarios + ventana de tiempo).
pub fn read_rcal_config(config_path: &Path) -> anyhow::Result<RcalConfig> {
    let content = fs::read_to_string(config_path)?;
    let config: toml::Value = toml::from_str(&content)?;

    let mut calendars = Vec::new();

    if let Some(cals) = config.get("calendars").and_then(|v| v.as_table()) {
        for (name, cal) in cals {
            if let Some(path_str) = cal.get("path").and_then(|v| v.as_str()) {
                let path = PathBuf::from(path_str);
                if path.exists() {
                    calendars.push(RcalCalendar {
                        name: name.clone(),
                        path,
                    });
                }
            }
        }
    }

    // Parsear ventana de tiempo de [default]
    let defaults = config.get("default");
    let time_backward = defaults
        .and_then(|d| d.get("timebackward"))
        .and_then(|v| v.as_str())
        .map(parse_duration)
        .unwrap_or(Duration::days(2));
    let time_forward = defaults
        .and_then(|d| d.get("timeforward"))
        .and_then(|v| v.as_str())
        .map(parse_duration)
        .unwrap_or(Duration::days(7));

    Ok(RcalConfig {
        calendars,
        time_backward,
        time_forward,
    })
}

/// Parsea string de duración de rcal: "2d", "7d", "60m", "3h"
fn parse_duration(s: &str) -> Duration {
    let s = s.trim();
    if let Some(n) = s.strip_suffix('d') {
        if let Ok(days) = n.parse::<i64>() {
            return Duration::days(days);
        }
    }
    if let Some(n) = s.strip_suffix('h') {
        if let Ok(hours) = n.parse::<i64>() {
            return Duration::hours(hours);
        }
    }
    if let Some(n) = s.strip_suffix('m') {
        if let Ok(mins) = n.parse::<i64>() {
            return Duration::minutes(mins);
        }
    }
    Duration::zero()
}

/// Escanea todos los calendarios y retorna tareas #TODO dentro de la ventana de tiempo de rcal.
/// Tareas sin DTSTART se incluyen siempre.
pub fn read_pending_tasks(rcal_cfg: &RcalConfig) -> anyhow::Result<Vec<IcalTask>> {
    let now = chrono::Local::now().naive_local();
    let window_start = now - rcal_cfg.time_backward;
    let window_end = now + rcal_cfg.time_forward;

    let mut tasks = Vec::new();

    for cal in &rcal_cfg.calendars {
        if !cal.path.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&cal.path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("ics") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(task) = parse_ics_task(&content, &path, &cal.name) {
                    if task.completed {
                        continue;
                    }
                    // Filtro de ventana: sin DTSTART → incluir siempre
                    if let Some(start) = task.start {
                        if start < window_start || start > window_end {
                            continue;
                        }
                    }
                    tasks.push(task);
                }
            }
        }
    }

    // Ordenar por fecha de inicio (None al final)
    tasks.sort_by(|a, b| match (&a.start, &b.start) {
        (Some(a_start), Some(b_start)) => a_start.cmp(b_start),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    Ok(tasks)
}

/// Toggle #TODO ↔ #DONE en un archivo .ics
pub fn toggle_task(file_path: &Path) -> anyhow::Result<()> {
    let content = fs::read_to_string(file_path)?;
    let new_content = if content.contains("DESCRIPTION:#TODO") {
        content.replacen("DESCRIPTION:#TODO", "DESCRIPTION:#DONE", 1)
    } else if content.contains("DESCRIPTION:#DONE") {
        content.replacen("DESCRIPTION:#DONE", "DESCRIPTION:#TODO", 1)
    } else {
        return Err(anyhow::anyhow!(
            "No se encontró #TODO ni #DONE en {}",
            file_path.display()
        ));
    };
    fs::write(file_path, new_content)?;
    Ok(())
}

/// Parse un archivo .ics y retorna un IcalTask si contiene DESCRIPTION:#TODO o #DONE
fn parse_ics_task(content: &str, file_path: &Path, calendar_name: &str) -> Option<IcalTask> {
    let mut summary: Option<String> = None;
    let mut is_task = false;
    let mut completed = false;
    let mut start: Option<NaiveDateTime> = None;

    for line in content.lines() {
        if line.starts_with("SUMMARY:") {
            summary = Some(line.strip_prefix("SUMMARY:")?.to_string());
        } else if line == "DESCRIPTION:#TODO" {
            is_task = true;
            completed = false;
        } else if line == "DESCRIPTION:#DONE" {
            is_task = true;
            completed = true;
        } else if line.starts_with("DTSTART") {
            start = parse_dtstart(line);
        }
    }

    if !is_task {
        return None;
    }

    Some(IcalTask {
        summary: summary.unwrap_or_else(|| "(sin título)".to_string()),
        start,
        completed,
        file_path: file_path.to_path_buf(),
        calendar_name: calendar_name.to_string(),
    })
}

/// Parsea línea DTSTART con formatos:
/// DTSTART:YYYYMMDDTHHmmss
/// DTSTART;VALUE=DATE-TIME:YYYYMMDDTHHmmss
/// DTSTART;VALUE=DATE:YYYYMMDD
/// DTSTART;TZID=...:YYYYMMDDTHHmmss
fn parse_dtstart(line: &str) -> Option<NaiveDateTime> {
    // Obtener el valor después del último ':'
    let value = line.rsplit(':').next()?.trim();

    // Formato datetime: YYYYMMDDTHHmmss
    if value.len() == 15 && value.contains('T') {
        return NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").ok();
    }

    // Formato date: YYYYMMDD → convertir a datetime en medianoche
    if value.len() == 8 && value.chars().all(|c| c.is_ascii_digit()) {
        let date = chrono::NaiveDate::parse_from_str(value, "%Y%m%d").ok()?;
        return Some(date.and_hms_opt(0, 0, 0).unwrap());
    }

    None
}

/// Verifica que el binario `rcal` esté disponible en PATH
pub fn rcal_available() -> bool {
    std::process::Command::new("rcal")
        .arg("--help")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Ejecuta `rcal todo` con título y flags opcionales.
/// Retorna Ok(()) si el proceso terminó con exit code 0.
pub fn run_rcal_todo(
    title: &str,
    calendar: Option<&str>,
    date: Option<&str>,
    time: Option<&str>,
    duration: Option<&str>,
) -> anyhow::Result<()> {
    let mut cmd = std::process::Command::new("rcal");
    cmd.arg("todo").arg(title);

    if let Some(c) = calendar {
        cmd.arg("-c").arg(c);
    }
    if let Some(d) = date {
        cmd.arg("-f").arg(d);
    }
    if let Some(t) = time {
        cmd.arg("-t").arg(t);
    }
    if let Some(dur) = duration {
        cmd.arg("-d").arg(dur);
    }

    let status = cmd
        .stdout(std::process::Stdio::null())
        .status()
        .map_err(|e| anyhow::anyhow!("No se pudo ejecutar `rcal`: {}", e))?;

    if !status.success() {
        return Err(anyhow::anyhow!(
            "`rcal todo` terminó con código de salida {}",
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dtstart_datetime() {
        let line = "DTSTART:20250215T143000";
        let result = parse_dtstart(line);
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2025-02-15 14:30:00");
    }

    #[test]
    fn test_parse_dtstart_date_only() {
        let line = "DTSTART;VALUE=DATE:20250215";
        let result = parse_dtstart(line);
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2025-02-15");
    }

    #[test]
    fn test_parse_dtstart_with_tzid() {
        let line = "DTSTART;TZID=America/Mexico_City:20250215T143000";
        let result = parse_dtstart(line);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_ics_task_todo() {
        let ics = "BEGIN:VEVENT\nSUMMARY:Test task\nDESCRIPTION:#TODO\nDTSTART:20250215T100000\nEND:VEVENT";
        let result = parse_ics_task(ics, std::path::Path::new("/tmp/test.ics"), "default");
        assert!(result.is_some());
        let task = result.unwrap();
        assert_eq!(task.summary, "Test task");
        assert!(!task.completed);
        assert!(task.start.is_some());
    }

    #[test]
    fn test_parse_ics_task_done() {
        let ics = "BEGIN:VEVENT\nSUMMARY:Done task\nDESCRIPTION:#DONE\nEND:VEVENT";
        let result = parse_ics_task(ics, std::path::Path::new("/tmp/test.ics"), "cal");
        assert!(result.is_some());
        let task = result.unwrap();
        assert!(task.completed);
    }

    #[test]
    fn test_parse_ics_not_a_task() {
        let ics = "BEGIN:VEVENT\nSUMMARY:Regular event\nDESCRIPTION:Just an event\nEND:VEVENT";
        let result = parse_ics_task(ics, std::path::Path::new("/tmp/test.ics"), "cal");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_rcal_config_nonexistent() {
        let result = find_rcal_config(Some("/nonexistent/path/config.toml"));
        // No debe retornar ese path porque no existe; puede retornar None o un default existente
        if let Some(p) = &result {
            assert!(p.exists());
        }
    }

    #[test]
    fn test_parse_duration_days() {
        assert_eq!(parse_duration("2d"), Duration::days(2));
        assert_eq!(parse_duration("7d"), Duration::days(7));
    }

    #[test]
    fn test_parse_duration_hours() {
        assert_eq!(parse_duration("3h"), Duration::hours(3));
    }

    #[test]
    fn test_parse_duration_minutes() {
        assert_eq!(parse_duration("60m"), Duration::minutes(60));
        assert_eq!(parse_duration("25m"), Duration::minutes(25));
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert_eq!(parse_duration("abc"), Duration::zero());
        assert_eq!(parse_duration(""), Duration::zero());
    }
}
