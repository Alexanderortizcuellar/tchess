mod app;
mod board;
mod pgn;
mod samples;

use app::App;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{env, fs, io, time::Instant};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let custom_pgn = if args.len() > 1 {
        match fs::read_to_string(&args[1]) {
            Ok(content) => Some(content),
            Err(e) => {
                eprintln!("Failed to read PGN file '{}': {}", args[1], e);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(custom_pgn);
    let mut last_tick = Instant::now();

    let res = run_app(&mut terminal, &mut app, &mut last_tick);

    // Terminal restoration
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Application error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    last_tick: &mut Instant,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| app.render(f))?;

        // Autoplay handling
        if app.autoplay && last_tick.elapsed() >= app.autoplay_interval {
            if app.current_ply < app.current_game().total_ply() {
                app.next_move();
            } else {
                app.autoplay = false;
            }
            *last_tick = Instant::now();
        }

        // Poll for events with timeout for smooth autoplay
        let timeout = app.autoplay_interval.saturating_sub(last_tick.elapsed());
        if event::poll(timeout.min(std::time::Duration::from_millis(50)))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('n') => app.next_move(),
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('p') => app.prev_move(),
                        KeyCode::Home | KeyCode::Char('0') => app.first_move(),
                        KeyCode::End | KeyCode::Char('$') => app.last_move(),
                        KeyCode::Char('f') => app.toggle_flip(),
                        KeyCode::Char('t') => app.cycle_theme(),
                        KeyCode::Char(' ') => app.toggle_autoplay(),
                        KeyCode::Char('s') => app.next_game(),
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
