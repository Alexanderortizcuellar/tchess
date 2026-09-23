mod app;
mod board;
mod db_mgr;
mod engine;
mod pgn;
mod samples;

use app::{App, CurrentScreen};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    env, fs,
    io::{self, IsTerminal, Read},
    time::Instant,
};

fn print_help() {
    println!(
        r#"tchess - Terminal Chessboard, PGN Database Browser & Search Navigator

USAGE:
    tchess [OPTIONS] [FILE]
    cat game.pgn | tchess [OPTIONS]

ARGS:
    [FILE]                    Path to PGN or SCID database file (or '-' to read from stdin)

OPTIONS:
    -f, --flip                Flip the chessboard (Black's perspective)
    -e, --engine-eval         Enable Stockfish engine evaluation on startup
    -d, --depth <NUM>         Stockfish search depth (default: 12)
        --engine <PATH>       Path to Stockfish executable (default: 'stockfish')
    -c, --show-comments       Display move comments (enabled by default)
        --no-comments         Hide move comments panel
    -a, --show-annotations    Show raw clock and eval annotations in comments
        --hide-annotations    Hide clock and eval annotations (default)
    -h, --help                Print this help information

SCREENS & TABS:
    [1] ♟ Board View          Interactive chessboard, moves list, comments, eval
    [2] 📂 Games Table        Browse all games in database, scroll & press Enter to load
    [3] 🔍 Query & Search     Execute CQL-Lite chess search queries powered by scid-mgr

KEYBINDINGS:
    Tab / Shift+Tab           Cycle through screens / tabs
    1 / 2 / 3                 Directly switch to Screen 1, 2, or 3
    /                         Jump to CQL Query search
    ← / h / p                 Previous move (Board)
    → / l / n                 Next move (Board)
    Home / 0                  First move (starting position)
    End / $                   Last move
    f                         Toggle board flip
    e                         Toggle Stockfish engine evaluation
    + / = / ]                 Increase Stockfish depth
    - / _ / [                 Decrease Stockfish depth
    c                         Toggle comments panel
    a                         Toggle raw clock/eval annotations
    t                         Cycle piece themes (Solid, Classic, Letters)
    Space                     Toggle autoplay
    s                         Cycle sample games
    Enter                     Load selected game on board (in Games/Query screens)
    q / Esc                   Quit / Back
"#
    );
}

fn main() -> io::Result<()> {
    let mut custom_pgn = None;
    let mut flip = false;
    let mut show_comments = true;
    let mut show_raw_annotations = false;
    let mut use_engine_eval = false;
    let mut engine_depth = 12;
    let mut engine_path = None;
    let mut file_arg = None;

    let args: Vec<String> = env::args().skip(1).collect();
    let mut idx = 0;
    while idx < args.len() {
        let arg = &args[idx];
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            "-f" | "--flip" => {
                flip = true;
            }
            "-e" | "--engine-eval" | "--eval" => {
                use_engine_eval = true;
            }
            "-d" | "--depth" => {
                if idx + 1 < args.len() {
                    if let Ok(d) = args[idx + 1].parse::<u32>() {
                        engine_depth = d;
                    }
                    use_engine_eval = true;
                    idx += 1;
                }
            }
            "--engine" => {
                if idx + 1 < args.len() {
                    engine_path = Some(args[idx + 1].clone());
                    use_engine_eval = true;
                    idx += 1;
                }
            }
            "-c" | "--show-comments" => {
                show_comments = true;
            }
            "--no-comments" | "--hide-comments" => {
                show_comments = false;
            }
            "-a" | "--show-annotations" | "--raw-annotations" => {
                show_raw_annotations = true;
            }
            "--hide-annotations" | "--clean-comments" => {
                show_raw_annotations = false;
            }
            "-" => {
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf)?;
                if !buf.trim().is_empty() {
                    custom_pgn = Some(buf);
                }
            }
            other if !other.starts_with('-') => {
                file_arg = Some(other.to_string());
            }
            _ => {}
        }
        idx += 1;
    }

    let db_file_path = file_arg.clone();

    if custom_pgn.is_none() {
        if let Some(path) = &file_arg {
            match fs::read_to_string(path) {
                Ok(content) => custom_pgn = Some(content),
                Err(e) => {
                    eprintln!("Failed to read PGN file '{}': {}", path, e);
                    std::process::exit(1);
                }
            }
        } else if !io::stdin().is_terminal() {
            let mut buf = String::new();
            if io::stdin().read_to_string(&mut buf).is_ok() && !buf.trim().is_empty() {
                custom_pgn = Some(buf);
            }
        }
    }

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(
        custom_pgn,
        flip,
        show_comments,
        show_raw_annotations,
        engine_path,
        use_engine_eval,
        engine_depth,
        db_file_path,
    );

    // If database was opened and has games, load the first game
    if app.db_mgr.is_some() {
        app.load_selected_db_game();
    }

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
        app.poll_engine();
        terminal.draw(|f| app.render(f))?;

        // Autoplay handling (only active in Board screen)
        if app.current_screen == CurrentScreen::Board
            && app.autoplay
            && last_tick.elapsed() >= app.autoplay_interval
        {
            if app.current_ply < app.current_game().total_ply() {
                app.next_move();
            } else {
                app.autoplay = false;
            }
            *last_tick = Instant::now();
        }

        // Poll for events: when autoplay is active, wake up when the next move is due;
        // when paused/idle, wait up to 100ms to yield CPU completely.
        let timeout = if app.autoplay && app.current_screen == CurrentScreen::Board {
            app.autoplay_interval.saturating_sub(last_tick.elapsed())
        } else {
            std::time::Duration::from_millis(100)
        };

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.current_screen {
                        CurrentScreen::Board => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            KeyCode::Tab => app.cycle_screen(),
                            KeyCode::BackTab => app.prev_screen(),
                            KeyCode::Char('1') => app.switch_screen(CurrentScreen::Board),
                            KeyCode::Char('2') => app.switch_screen(CurrentScreen::Database),
                            KeyCode::Char('3') => app.switch_screen(CurrentScreen::Query),
                            KeyCode::Char('/') => app.switch_screen(CurrentScreen::Query),
                            KeyCode::Right
                            | KeyCode::Char('l')
                            | KeyCode::Char('n')
                            | KeyCode::Char('j')
                            | KeyCode::Down => app.next_move(),
                            KeyCode::Left
                            | KeyCode::Char('h')
                            | KeyCode::Char('p')
                            | KeyCode::Char('k')
                            | KeyCode::Up => app.prev_move(),
                            KeyCode::Home | KeyCode::Char('0') => app.first_move(),
                            KeyCode::End | KeyCode::Char('$') => app.last_move(),
                            KeyCode::Char('f') => app.toggle_flip(),
                            KeyCode::Char('e') => app.toggle_engine_eval(),
                            KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char(']') => {
                                app.increase_depth()
                            }
                            KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::Char('[') => {
                                app.decrease_depth()
                            }
                            KeyCode::Char('c') => app.toggle_comments(),
                            KeyCode::Char('a') => app.toggle_annotations(),
                            KeyCode::Char('t') => app.cycle_theme(),
                            KeyCode::Char(' ') => {
                                app.toggle_autoplay();
                                *last_tick = Instant::now();
                            }
                            KeyCode::Char('s') => app.next_game(),
                            _ => {}
                        },
                        CurrentScreen::Database => match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Esc => app.switch_screen(CurrentScreen::Board),
                            KeyCode::Tab => app.cycle_screen(),
                            KeyCode::BackTab => app.prev_screen(),
                            KeyCode::Char('1') => app.switch_screen(CurrentScreen::Board),
                            KeyCode::Char('2') => app.switch_screen(CurrentScreen::Database),
                            KeyCode::Char('3') => app.switch_screen(CurrentScreen::Query),
                            KeyCode::Char('/') => app.switch_screen(CurrentScreen::Query),
                            KeyCode::Up | KeyCode::Char('k') => app.select_prev_db_game(),
                            KeyCode::Down | KeyCode::Char('j') => app.select_next_db_game(),
                            KeyCode::PageUp | KeyCode::Char('u') => app.select_db_page_up(15),
                            KeyCode::PageDown | KeyCode::Char('d') => app.select_db_page_down(15),
                            KeyCode::Home | KeyCode::Char('g') => app.select_first_db_game(),
                            KeyCode::End | KeyCode::Char('G') => app.select_last_db_game(),
                            KeyCode::Enter => app.load_selected_db_game(),
                            _ => {}
                        },
                        CurrentScreen::Query => match key.code {
                            KeyCode::Tab => app.cycle_screen(),
                            KeyCode::BackTab => app.prev_screen(),
                            KeyCode::Esc => {
                                if app.query_focus_results {
                                    app.query_focus_results = false;
                                } else if !app.query_input.is_empty() {
                                    app.query_clear();
                                } else {
                                    app.switch_screen(CurrentScreen::Board);
                                }
                            }
                            KeyCode::Enter => {
                                if app.query_focus_results && !app.query_results.is_empty() {
                                    app.load_selected_query_game();
                                } else {
                                    app.execute_query();
                                }
                            }
                            KeyCode::Up => {
                                if app.query_focus_results {
                                    app.select_prev_query_result();
                                }
                            }
                            KeyCode::Down => {
                                if !app.query_results.is_empty() {
                                    app.query_focus_results = true;
                                    app.select_next_query_result();
                                }
                            }
                            KeyCode::Left => app.query_cursor_left(),
                            KeyCode::Right => app.query_cursor_right(),
                            KeyCode::Backspace => app.query_backspace(),
                            KeyCode::Char(c) => app.query_type_char(c),
                            _ => {}
                        },
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
