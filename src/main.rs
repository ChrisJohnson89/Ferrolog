mod app;
mod parser;

use app::{ui, App};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use parser::LogParser;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{env, fs, io, path::Path, process};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        eprintln!("Usage: ferrolog <logfile>");
        eprintln!();
        eprintln!("A lightweight TUI log viewer");
        process::exit(1);
    }

    if args[1] == "--version" || args[1] == "-V" {
        println!("ferrolog {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let filepath = &args[1];
    let path = Path::new(filepath);

    if !path.exists() {
        eprintln!("Error: file not found: {}", filepath);
        process::exit(1);
    }

    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error reading file: {}", e);
        process::exit(1);
    });

    let parser = LogParser::new();
    let entries = parser.parse_file(&content);

    if entries.is_empty() {
        eprintln!("No log entries found in {}", filepath);
        process::exit(1);
    }

    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| filepath.to_string());

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(entries, filename);

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
        process::exit(1);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        app.handle_events()?;
        if app.should_quit {
            return Ok(());
        }
    }
}
