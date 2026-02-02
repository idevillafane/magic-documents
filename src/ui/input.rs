use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{self, Write};

/// Input text with ESC support
/// Returns Ok(None) if user pressed ESC or Ctrl+C, Ok(Some(input)) on Enter
pub fn input_with_esc(prompt: &str) -> anyhow::Result<Option<String>> {
    print!("{}: ", prompt);
    io::stdout().flush()?;

    enable_raw_mode()?;
    let mut input = String::new();

    loop {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => {
                    disable_raw_mode()?;
                    println!();
                    return Ok(None);
                }
                KeyCode::Enter => {
                    disable_raw_mode()?;
                    println!();
                    return Ok(Some(input));
                }
                KeyCode::Char(c) if key.modifiers == KeyModifiers::CONTROL && c == 'c' => {
                    disable_raw_mode()?;
                    println!();
                    return Ok(None);
                }
                KeyCode::Backspace => {
                    if !input.is_empty() {
                        input.pop();
                        print!("\x08 \x08");
                        io::stdout().flush()?;
                    }
                }
                KeyCode::Char(c)
                    if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
                {
                    input.push(c);
                    print!("{}", c);
                    io::stdout().flush()?;
                }
                _ => {}
            }
        }
    }
}
