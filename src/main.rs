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
use std::{env, fs, io, io::Read, path::Path, process};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let mut follow_mode = false;
    let mut file_arg: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                process::exit(0);
            }
            "--version" | "-V" => {
                println!("ferrolog {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "-f" => {
                follow_mode = true;
            }
            arg if !arg.starts_with('-') || arg == "-" => {
                file_arg = Some(arg.to_string());
            }
            arg => {
                eprintln!("Unknown flag: {}", arg);
                eprintln!("Run with --help for usage.");
                process::exit(1);
            }
        }
        i += 1;
    }

    let (content, filename, filepath) = if let Some(ref path_str) = file_arg {
        if path_str == "-" {
            let mut content = String::new();
            io::stdin().read_to_string(&mut content)?;
            (content, "stdin".to_string(), None)
        } else {
            let path = Path::new(path_str);
            if !path.exists() {
                eprintln!("Error: file not found: {}", path_str);
                process::exit(1);
            }
            let content = fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("Error reading file: {}", e);
                process::exit(1);
            });
            let fname = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path_str.clone());
            (content, fname, Some(path_str.clone()))
        }
    } else {
        // No file argument: read from stdin
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)?;
        if content.is_empty() {
            print_help();
            process::exit(1);
        }
        (content, "stdin".to_string(), None)
    };

    let parser = LogParser::new();
    let entries = parser.parse_file(&content);

    if entries.is_empty() {
        eprintln!("No log entries found");
        process::exit(1);
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(entries, filename, follow_mode, filepath);

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

fn print_help() {
    eprintln!("Usage: ferrolog [-f] [logfile|-]");
    eprintln!();
    eprintln!("  -f          Follow mode: watch file and stream new lines");
    eprintln!("  logfile     Path to log file (omit or use - to read from stdin)");
    eprintln!();
    eprintln!("A lightweight TUI log viewer");
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        app.handle_events()?;
        app.check_follow()?;
        if app.should_quit {
            return Ok(());
        }
    }
}
