use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Tabs},
    Frame,
};
use scid_mgr::db::GameSummary;
use shakmaty::{Chess, Color as PieceColor, Position, Role};
use std::time::Duration;

use crate::board::{ChessBoardWidget, PieceTheme};
use crate::db_mgr::DatabaseManager;
use crate::engine::{EngineClient, EvalResult};
use crate::pgn::PgnGame;
use crate::samples::{FISCHER_BYRNE, KASPAROV_IMMORTAL, MORPHY_OPERA};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentScreen {
    Board,
    Database,
    Query,
}

pub struct App {
    pub current_screen: CurrentScreen,
    pub games: Vec<(String, PgnGame)>,
    pub current_game_idx: usize,
    pub current_ply: usize,
    pub flipped: bool,
    pub show_comments: bool,
    pub show_raw_annotations: bool,
    pub engine_path: Option<String>,
    pub engine: Option<EngineClient>,
    pub use_engine_eval: bool,
    pub engine_depth: u32,
    pub current_engine_eval: Option<EvalResult>,
    pub engine_error: Option<String>,
    pub theme: PieceTheme,
    pub autoplay: bool,
    pub autoplay_interval: Duration,
    pub should_quit: bool,

    // Database & Query State
    pub db_mgr: Option<DatabaseManager>,
    pub selected_db_idx: usize,
    pub query_input: String,
    pub query_cursor: usize,
    pub query_results: Vec<(usize, GameSummary)>,
    pub query_selected_idx: usize,
    pub query_error: Option<String>,
    pub query_info: Option<String>,
    pub query_focus_results: bool,
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        custom_pgn: Option<String>,
        flipped: bool,
        show_comments: bool,
        show_raw_annotations: bool,
        engine_path: Option<String>,
        use_engine_eval: bool,
        engine_depth: u32,
        db_file_path: Option<String>,
    ) -> Self {
        let mut games = Vec::new();

        if let Some(pgn) = custom_pgn {
            let parsed = PgnGame::parse(&pgn);
            let title =
                if parsed.headers.contains_key("White") || parsed.headers.contains_key("Black") {
                    format!("{} vs {}", parsed.white_player(), parsed.black_player())
                } else {
                    "Loaded PGN Game".to_string()
                };
            games.push((title, parsed));
        }

        games.push((
            "Morphy's Opera Game (1858)".to_string(),
            PgnGame::parse(MORPHY_OPERA),
        ));
        games.push((
            "Kasparov's Immortal (1999)".to_string(),
            PgnGame::parse(KASPAROV_IMMORTAL),
        ));
        games.push((
            "Game of the Century (1956)".to_string(),
            PgnGame::parse(FISCHER_BYRNE),
        ));

        let db_mgr = db_file_path.and_then(|p| DatabaseManager::open(p).ok());

        let mut app = Self {
            current_screen: CurrentScreen::Board,
            games,
            current_game_idx: 0,
            current_ply: 0,
            flipped,
            show_comments,
            show_raw_annotations,
            engine_path: engine_path.clone(),
            engine: None,
            use_engine_eval,
            engine_depth: engine_depth.clamp(1, 35),
            current_engine_eval: None,
            engine_error: None,
            theme: PieceTheme::Solid,
            autoplay: false,
            autoplay_interval: Duration::from_millis(800),
            should_quit: false,

            db_mgr,
            selected_db_idx: 0,
            query_input: String::new(),
            query_cursor: 0,
            query_results: Vec::new(),
            query_selected_idx: 0,
            query_error: None,
            query_info: None,
            query_focus_results: false,
        };

        if use_engine_eval {
            app.init_engine();
            app.trigger_eval();
        }

        app
    }

    fn init_engine(&mut self) {
        if self.engine.is_none() && self.engine_error.is_none() {
            match EngineClient::new(self.engine_path.as_deref()) {
                Ok(client) => {
                    self.engine = Some(client);
                    self.engine_error = None;
                }
                Err(err) => {
                    self.engine_error = Some(err);
                }
            }
        }
    }

    pub fn poll_engine(&mut self) {
        if let Some(engine) = &self.engine {
            if let Some(res) = engine.try_get_eval() {
                self.current_engine_eval = Some(res);
            }
        }
    }

    pub fn trigger_eval(&mut self) {
        if self.use_engine_eval {
            if self.engine.is_none() && self.engine_error.is_none() {
                self.init_engine();
            }
            if let Some(engine) = &self.engine {
                let pos = &self.current_game().get_node(self.current_ply).pos;
                engine.evaluate(pos, self.engine_depth);
            }
        }
    }

    pub fn toggle_engine_eval(&mut self) {
        self.use_engine_eval = !self.use_engine_eval;
        self.current_engine_eval = None;
        if self.use_engine_eval {
            self.init_engine();
            self.trigger_eval();
        }
    }

    pub fn increase_depth(&mut self) {
        if self.engine_depth < 35 {
            self.engine_depth += 1;
            self.current_engine_eval = None;
            self.trigger_eval();
        }
    }

    pub fn decrease_depth(&mut self) {
        if self.engine_depth > 1 {
            self.engine_depth -= 1;
            self.current_engine_eval = None;
            self.trigger_eval();
        }
    }

    pub fn current_game(&self) -> &PgnGame {
        &self.games[self.current_game_idx].1
    }

    pub fn cycle_theme(&mut self) {
        self.theme = match self.theme {
            PieceTheme::Solid => PieceTheme::Classic,
            PieceTheme::Classic => PieceTheme::Letters,
            PieceTheme::Letters => PieceTheme::Solid,
        };
    }

    pub fn next_move(&mut self) {
        if self.current_ply < self.current_game().total_ply() {
            self.current_ply += 1;
            self.current_engine_eval = None;
            self.trigger_eval();
        }
    }

    pub fn prev_move(&mut self) {
        if self.current_ply > 0 {
            self.current_ply -= 1;
            self.current_engine_eval = None;
            self.trigger_eval();
        }
    }

    pub fn first_move(&mut self) {
        self.current_ply = 0;
        self.current_engine_eval = None;
        self.trigger_eval();
    }

    pub fn last_move(&mut self) {
        self.current_ply = self.current_game().total_ply();
        self.current_engine_eval = None;
        self.trigger_eval();
    }

    pub fn toggle_flip(&mut self) {
        self.flipped = !self.flipped;
    }

    pub fn toggle_comments(&mut self) {
        self.show_comments = !self.show_comments;
    }

    pub fn toggle_annotations(&mut self) {
        self.show_raw_annotations = !self.show_raw_annotations;
    }

    pub fn toggle_autoplay(&mut self) {
        self.autoplay = !self.autoplay;
    }

    pub fn next_game(&mut self) {
        self.current_game_idx = (self.current_game_idx + 1) % self.games.len();
        self.current_ply = 0;
        self.autoplay = false;
        self.current_engine_eval = None;
        self.trigger_eval();
    }

    // Screen Switching
    pub fn switch_screen(&mut self, screen: CurrentScreen) {
        self.current_screen = screen;
    }

    pub fn cycle_screen(&mut self) {
        self.current_screen = match self.current_screen {
            CurrentScreen::Board => CurrentScreen::Database,
            CurrentScreen::Database => CurrentScreen::Query,
            CurrentScreen::Query => CurrentScreen::Board,
        };
    }

    pub fn prev_screen(&mut self) {
        self.current_screen = match self.current_screen {
            CurrentScreen::Board => CurrentScreen::Query,
            CurrentScreen::Database => CurrentScreen::Board,
            CurrentScreen::Query => CurrentScreen::Database,
        };
    }

    // Database Navigation
    pub fn select_next_db_game(&mut self) {
        if let Some(db) = &self.db_mgr {
            let total = db.game_count();
            if total > 0 && self.selected_db_idx + 1 < total {
                self.selected_db_idx += 1;
            }
        }
    }

    pub fn select_prev_db_game(&mut self) {
        if self.selected_db_idx > 0 {
            self.selected_db_idx -= 1;
        }
    }

    pub fn select_db_page_down(&mut self, page: usize) {
        if let Some(db) = &self.db_mgr {
            let total = db.game_count();
            if total > 0 {
                self.selected_db_idx = (self.selected_db_idx + page).min(total.saturating_sub(1));
            }
        }
    }

    pub fn select_db_page_up(&mut self, page: usize) {
        self.selected_db_idx = self.selected_db_idx.saturating_sub(page);
    }

    pub fn select_first_db_game(&mut self) {
        self.selected_db_idx = 0;
    }

    pub fn select_last_db_game(&mut self) {
        if let Some(db) = &self.db_mgr {
            let total = db.game_count();
            if total > 0 {
                self.selected_db_idx = total.saturating_sub(1);
            }
        }
    }

    pub fn load_selected_db_game(&mut self) {
        if let Some(db) = &self.db_mgr {
            if let Ok(pgn_str) = db.get_game_pgn(self.selected_db_idx) {
                let parsed = PgnGame::parse(&pgn_str);
                let title = format!(
                    "[{}] {} vs {}",
                    self.selected_db_idx + 1,
                    parsed.white_player(),
                    parsed.black_player()
                );
                self.games.insert(0, (title, parsed));
                self.current_game_idx = 0;
                self.current_ply = 0;
                self.autoplay = false;
                self.current_engine_eval = None;
                self.trigger_eval();
                self.current_screen = CurrentScreen::Board;
            }
        }
    }

    // Query Screen Handlers
    pub fn query_type_char(&mut self, c: char) {
        self.query_input.insert(self.query_cursor, c);
        self.query_cursor += 1;
        self.query_focus_results = false;
    }

    pub fn query_backspace(&mut self) {
        if self.query_cursor > 0 {
            self.query_cursor -= 1;
            self.query_input.remove(self.query_cursor);
            self.query_focus_results = false;
        }
    }

    pub fn query_cursor_left(&mut self) {
        if self.query_cursor > 0 {
            self.query_cursor -= 1;
        }
    }

    pub fn query_cursor_right(&mut self) {
        if self.query_cursor < self.query_input.len() {
            self.query_cursor += 1;
        }
    }

    pub fn query_clear(&mut self) {
        self.query_input.clear();
        self.query_cursor = 0;
        self.query_error = None;
        self.query_info = None;
        self.query_focus_results = false;
    }

    pub fn execute_query(&mut self) {
        let input = self.query_input.trim();
        if input.is_empty() {
            self.query_error = Some("Enter a search query (e.g. white=\"Kasparov\")".to_string());
            return;
        }

        if let Some(db) = &self.db_mgr {
            match db.execute_cql_query(input, 100) {
                Ok(matches) => {
                    let count = matches.len();
                    self.query_results = matches;
                    self.query_selected_idx = 0;
                    self.query_error = None;
                    self.query_info = Some(format!("Found {} matching game(s)", count));
                    if count > 0 {
                        self.query_focus_results = true;
                    }
                }
                Err(err) => {
                    self.query_error = Some(err);
                    self.query_info = None;
                }
            }
        } else {
            self.query_error = Some("No database loaded to query against".to_string());
        }
    }

    pub fn select_next_query_result(&mut self) {
        if !self.query_results.is_empty() && self.query_selected_idx + 1 < self.query_results.len()
        {
            self.query_selected_idx += 1;
        }
    }

    pub fn select_prev_query_result(&mut self) {
        if self.query_selected_idx > 0 {
            self.query_selected_idx -= 1;
        }
    }

    pub fn load_selected_query_game(&mut self) {
        if let Some((game_id, _)) = self.query_results.get(self.query_selected_idx) {
            if let Some(db) = &self.db_mgr {
                if let Ok(pgn_str) = db.get_game_pgn(*game_id) {
                    let parsed = PgnGame::parse(&pgn_str);
                    let title = format!(
                        "[{}] {} vs {}",
                        game_id + 1,
                        parsed.white_player(),
                        parsed.black_player()
                    );
                    self.games.insert(0, (title, parsed));
                    self.current_game_idx = 0;
                    self.current_ply = 0;
                    self.autoplay = false;
                    self.current_engine_eval = None;
                    self.trigger_eval();
                    self.current_screen = CurrentScreen::Board;
                }
            }
        }
    }

    pub fn calculate_material(&self, pos: &Chess) -> (String, String, i32) {
        let piece_values = [
            (Role::Pawn, 1, "♙", "♟"),
            (Role::Knight, 3, "♘", "♞"),
            (Role::Bishop, 3, "♗", "♝"),
            (Role::Rook, 5, "♖", "♜"),
            (Role::Queen, 9, "♕", "♛"),
        ];

        let mut white_score = 0;
        let mut black_score = 0;
        let mut white_lost = String::new();
        let mut black_lost = String::new();

        let init_counts = [
            (Role::Pawn, 8),
            (Role::Knight, 2),
            (Role::Bishop, 2),
            (Role::Rook, 2),
            (Role::Queen, 1),
        ];

        for (role, init_cnt) in init_counts {
            let w_cnt =
                (pos.board().by_color(PieceColor::White) & pos.board().by_role(role)).count();
            let b_cnt =
                (pos.board().by_color(PieceColor::Black) & pos.board().by_role(role)).count();

            if let Some(&(_, val, w_sym, b_sym)) =
                piece_values.iter().find(|(r, _, _, _)| *r == role)
            {
                white_score += (w_cnt as i32) * val;
                black_score += (b_cnt as i32) * val;

                if init_cnt > w_cnt {
                    white_lost.push_str(&w_sym.repeat(init_cnt - w_cnt));
                }
                if init_cnt > b_cnt {
                    black_lost.push_str(&b_sym.repeat(init_cnt - b_cnt));
                }
            }
        }

        (white_lost, black_lost, white_score - black_score)
    }

    pub fn render(&self, f: &mut Frame) {
        let size = f.area();
        if size.width < 10 || size.height < 6 {
            return;
        }

        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(8)])
            .split(size);

        // Header Navigation Tabs
        let titles = vec![
            " [1] ♟ Board ",
            " [2] 📂 Games Table ",
            " [3] 🔍 Query & Search ",
        ];

        let selected_tab = match self.current_screen {
            CurrentScreen::Board => 0,
            CurrentScreen::Database => 1,
            CurrentScreen::Query => 2,
        };

        let tabs = Tabs::new(titles)
            .select(selected_tab)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" ♞ tchess — Navigation "),
            )
            .style(Style::default().fg(Color::DarkGray))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(tabs, main_layout[0]);

        // Render Active Screen
        match self.current_screen {
            CurrentScreen::Board => self.render_board_view(f, main_layout[1]),
            CurrentScreen::Database => self.render_database_view(f, main_layout[1]),
            CurrentScreen::Query => self.render_query_view(f, main_layout[1]),
        }
    }

    fn render_board_view(&self, f: &mut Frame, area: Rect) {
        let is_desktop = area.width >= 75;
        let main_chunks = if is_desktop {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(40), Constraint::Length(42)])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(area)
        };

        let game = self.current_game();
        let current_node = game.get_node(self.current_ply);

        // Render Chess Board Widget
        let board_widget = ChessBoardWidget::new(&current_node.pos)
            .last_move(current_node.last_move.as_ref())
            .flipped(self.flipped)
            .theme(self.theme);

        f.render_widget(board_widget, main_chunks[0]);

        // Right / Bottom Sidebar
        let (side_chunks, comment_chunk_idx, controls_chunk_idx) = if self.show_comments {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(7), // Info
                    Constraint::Min(5),    // Moves
                    Constraint::Length(6), // Comments
                    Constraint::Length(3), // Controls Footer
                ])
                .split(main_chunks[1]);
            (chunks, Some(2), 3)
        } else {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(7), // Info
                    Constraint::Min(5),    // Moves
                    Constraint::Length(3), // Controls Footer
                ])
                .split(main_chunks[1]);
            (chunks, None, 2)
        };

        // Game Info Block
        let eval_span = if self.use_engine_eval {
            if self.engine_error.is_some() {
                Span::styled("Eval: [SF not found]  ", Style::default().fg(Color::Yellow))
            } else if let Some(eval) = &self.current_engine_eval {
                let color = if let Some(cp) = eval.score_cp {
                    if cp > 20 {
                        Color::Green
                    } else if cp < -20 {
                        Color::Red
                    } else {
                        Color::White
                    }
                } else if eval.display.starts_with('+') {
                    Color::Green
                } else if eval.display.starts_with('-') {
                    Color::Red
                } else {
                    Color::White
                };
                Span::styled(
                    format!("Eval: {} (SF)  ", eval.display),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    "Eval: Thinking... (SF)  ",
                    Style::default().fg(Color::LightBlue),
                )
            }
        } else {
            let (_, _, eval_diff) = self.calculate_material(&current_node.pos);
            let eval_str = if eval_diff > 0 {
                format!("+{}", eval_diff)
            } else if eval_diff < 0 {
                format!("{}", eval_diff)
            } else {
                "=".to_string()
            };
            Span::styled(
                format!("Eval: {} (Mat)  ", eval_str),
                Style::default()
                    .fg(if eval_diff >= 0 {
                        Color::Green
                    } else {
                        Color::Red
                    })
                    .add_modifier(Modifier::BOLD),
            )
        };

        let play_status = if self.autoplay {
            "⏸ Playing"
        } else {
            "▶ Paused"
        };

        let info_text = vec![
            Line::from(vec![Span::styled(
                format!("⚔ {} vs {}", game.white_player(), game.black_player()),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!("🏆 {} ({})", game.event(), game.result()),
                Style::default().fg(Color::DarkGray),
            )]),
            Line::from(vec![
                Span::styled(
                    format!("Move: {}/{}  ", self.current_ply, game.total_ply()),
                    Style::default().fg(Color::Cyan),
                ),
                eval_span,
                Span::styled(play_status, Style::default().fg(Color::Magenta)),
            ]),
            Line::from(vec![Span::styled(
                format!(
                    "Game: [{}] {}",
                    self.current_game_idx + 1,
                    self.games[self.current_game_idx].0
                ),
                Style::default().fg(Color::LightBlue),
            )]),
        ];

        let info_para = Paragraph::new(info_text).block(
            Block::default()
                .title(" ♞ Game Info ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        f.render_widget(info_para, side_chunks[0]);

        // Moves Table
        let mut rows = Vec::new();
        let num_moves = game.moves.len();
        let total_pairs = num_moves.div_ceil(2);

        let active_pair_idx = self.current_ply / 2;
        let visible_rows = side_chunks[1].height.saturating_sub(2) as usize;
        let start_pair = if active_pair_idx >= visible_rows {
            active_pair_idx.saturating_sub(visible_rows / 2)
        } else {
            0
        };

        for pair_idx in start_pair..(start_pair + visible_rows).min(total_pairs) {
            let move_num = pair_idx + 1;
            let w_ply = pair_idx * 2 + 1;
            let b_ply = w_ply + 1;

            let num_str = format!("{:2}.", move_num);
            let w_san = if w_ply < num_moves {
                game.moves[w_ply].san.clone()
            } else {
                "".to_string()
            };
            let b_san = if b_ply < num_moves {
                game.moves[b_ply].san.clone()
            } else {
                "".to_string()
            };

            let w_style = if w_ply == self.current_ply {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let b_style = if b_ply == self.current_ply {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            rows.push(Row::new(vec![
                Span::styled(num_str, Style::default().fg(Color::DarkGray)),
                Span::styled(w_san, w_style),
                Span::styled(b_san, b_style),
            ]));
        }

        let moves_table = Table::new(
            rows,
            [
                Constraint::Length(4),
                Constraint::Length(10),
                Constraint::Length(10),
            ],
        )
        .block(
            Block::default()
                .title(" Moves ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(moves_table, side_chunks[1]);

        // Comments Block
        if let Some(c_idx) = comment_chunk_idx {
            let raw_comment = &current_node.comment;
            let comment_text = if self.show_raw_annotations {
                raw_comment.trim().to_string()
            } else {
                crate::pgn::strip_annotations(raw_comment)
            };

            let comment_para = if comment_text.is_empty() {
                Paragraph::new(Line::from(vec![Span::styled(
                    "No comments on this move",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                )]))
            } else {
                Paragraph::new(comment_text).style(Style::default().fg(Color::White))
            }
            .wrap(ratatui::widgets::Wrap { trim: true })
            .block(
                Block::default()
                    .title(if self.show_raw_annotations {
                        " 💬 Comments [Raw Annotations] "
                    } else {
                        " 💬 Comments "
                    })
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::LightCyan)),
            );
            f.render_widget(comment_para, side_chunks[c_idx]);
        }

        // Controls Footer
        let controls_para = Paragraph::new(
            "[←/→] Move  [F] Flip  [E] Eval  [+/-] Depth  [C] Comment  [Space] Play  [Tab] Screen  [Q] Quit",
        )
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(controls_para, side_chunks[controls_chunk_idx]);
    }

    fn render_database_view(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Info banner
                Constraint::Min(6),    // Games table
                Constraint::Length(3), // Controls footer
            ])
            .split(area);

        // Header / Database Info
        let (db_path_str, game_count) = if let Some(db) = &self.db_mgr {
            (
                db.path.display().to_string(),
                format!(
                    "{} games (Indexed via scid-mgr / .pgn.idx)",
                    db.game_count()
                ),
            )
        } else {
            (
                "No database file opened (using built-in sample games)".to_string(),
                format!("{} games loaded", self.games.len()),
            )
        };

        let banner_text = Line::from(vec![
            Span::styled("📂 Source: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                db_path_str,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  |  Total: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                game_count,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        let banner_para = Paragraph::new(banner_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Database Summary ")
                .border_style(Style::default().fg(Color::Cyan)),
        );
        f.render_widget(banner_para, chunks[0]);

        let is_compact = chunks[1].width < 80;

        if is_compact {
            let available_height = chunks[1].height.saturating_sub(2) as usize;
            let items_per_page = (available_height / 2).max(1);
            let mut list_items = Vec::new();

            if let Some(db) = &self.db_mgr {
                let total = db.game_count();
                let start_idx = if self.selected_db_idx >= items_per_page {
                    self.selected_db_idx.saturating_sub(items_per_page / 2)
                } else {
                    0
                };

                for idx in start_idx..(start_idx + items_per_page).min(total) {
                    if let Some(summary) = db.get_summary(idx) {
                        let is_selected = idx == self.selected_db_idx;
                        let prefix = if is_selected { "▶ " } else { "  " };

                        let w_elo = if summary.white_elo > 0 {
                            format!(" ({})", summary.white_elo)
                        } else {
                            String::new()
                        };
                        let b_elo = if summary.black_elo > 0 {
                            format!(" ({})", summary.black_elo)
                        } else {
                            String::new()
                        };

                        let num_style = if is_selected {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Cyan)
                        };
                        let name_style = if is_selected {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        };
                        let meta_style = if is_selected {
                            Style::default().fg(Color::LightCyan)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };
                        let result_style = if is_selected {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD)
                        };

                        let line1 = Line::from(vec![
                            Span::styled(format!("{}[{:>4}] ", prefix, idx + 1), num_style),
                            Span::styled(format!("{}{}", summary.white, w_elo), name_style),
                            Span::styled(" vs ", Style::default().fg(Color::DarkGray)),
                            Span::styled(format!("{}{}", summary.black, b_elo), name_style),
                            Span::styled(format!("  [{}]", summary.result), result_style),
                        ]);

                        let line2 = Line::from(vec![
                            Span::styled("       ", Style::default()),
                            Span::styled(
                                format!("{} • {} • {}", summary.date, summary.eco, summary.event),
                                meta_style,
                            ),
                        ]);

                        list_items.push(ListItem::new(vec![line1, line2]));
                    }
                }
            } else {
                let start_idx = if self.selected_db_idx >= items_per_page {
                    self.selected_db_idx.saturating_sub(items_per_page / 2)
                } else {
                    0
                };

                for (idx, (title, game)) in self
                    .games
                    .iter()
                    .enumerate()
                    .skip(start_idx)
                    .take(items_per_page)
                {
                    let is_selected = idx == self.selected_db_idx;
                    let prefix = if is_selected { "▶ " } else { "  " };

                    let num_style = if is_selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Cyan)
                    };
                    let name_style = if is_selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let meta_style = if is_selected {
                        Style::default().fg(Color::LightCyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };

                    let line1 = Line::from(vec![
                        Span::styled(format!("{}[{:>4}] ", prefix, idx + 1), num_style),
                        Span::styled(game.white_player().to_string(), name_style),
                        Span::styled(" vs ", Style::default().fg(Color::DarkGray)),
                        Span::styled(game.black_player().to_string(), name_style),
                        Span::styled(
                            format!("  [{}]", game.result()),
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]);

                    let line2 = Line::from(vec![
                        Span::styled("       ", Style::default()),
                        Span::styled(title.clone(), meta_style),
                    ]);

                    list_items.push(ListItem::new(vec![line1, line2]));
                }
            }

            let list = List::new(list_items).block(
                Block::default()
                    .title(" 📂 Games List ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );
            f.render_widget(list, chunks[1]);
        } else {
            // Games Table
            let visible_rows = chunks[1].height.saturating_sub(2) as usize;
            let mut rows = Vec::new();

            if let Some(db) = &self.db_mgr {
                let total = db.game_count();
                let start_idx = if self.selected_db_idx >= visible_rows {
                    self.selected_db_idx.saturating_sub(visible_rows / 2)
                } else {
                    0
                };

                for idx in start_idx..(start_idx + visible_rows).min(total) {
                    if let Some(summary) = db.get_summary(idx) {
                        let is_selected = idx == self.selected_db_idx;
                        let style = if is_selected {
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        };

                        rows.push(Row::new(vec![
                            Span::styled(
                                format!("{:>5}", idx + 1),
                                if is_selected {
                                    style
                                } else {
                                    Style::default().fg(Color::DarkGray)
                                },
                            ),
                            Span::styled(summary.white, style),
                            Span::styled(
                                if summary.white_elo > 0 {
                                    format!("{}", summary.white_elo)
                                } else {
                                    "-".to_string()
                                },
                                style,
                            ),
                            Span::styled(summary.black, style),
                            Span::styled(
                                if summary.black_elo > 0 {
                                    format!("{}", summary.black_elo)
                                } else {
                                    "-".to_string()
                                },
                                style,
                            ),
                            Span::styled(summary.result, style),
                            Span::styled(summary.date, style),
                            Span::styled(summary.eco, style),
                            Span::styled(summary.event, style),
                        ]));
                    }
                }
            } else {
                // Display loaded sample games
                for (idx, (title, game)) in self.games.iter().enumerate() {
                    let is_selected = idx == self.selected_db_idx;
                    let style = if is_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    rows.push(Row::new(vec![
                        Span::styled(
                            format!("{:>5}", idx + 1),
                            if is_selected {
                                style
                            } else {
                                Style::default().fg(Color::DarkGray)
                            },
                        ),
                        Span::styled(game.white_player().to_string(), style),
                        Span::styled("-", style),
                        Span::styled(game.black_player().to_string(), style),
                        Span::styled("-", style),
                        Span::styled(game.result().to_string(), style),
                        Span::styled("-", style),
                        Span::styled("-", style),
                        Span::styled(title.clone(), style),
                    ]));
                }
            }

            let header = Row::new(vec![
                Span::styled(
                    "    #",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "White",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Elo",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Black",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Elo",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Result",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Date",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "ECO",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Event",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]);

            let table = Table::new(
                rows,
                [
                    Constraint::Length(6),
                    Constraint::Percentage(22),
                    Constraint::Length(6),
                    Constraint::Percentage(22),
                    Constraint::Length(6),
                    Constraint::Length(8),
                    Constraint::Length(11),
                    Constraint::Length(6),
                    Constraint::Percentage(25),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .title(" 📂 Games List ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );
            f.render_widget(table, chunks[1]);
        }

        // Controls
        let controls = Paragraph::new(
            "[↑/↓ / j/k] Navigate  [Enter] Load Game on Board  [PgUp/PgDn] Page  [/] Search Query  [Tab] Switch Tab  [Q] Quit",
        )
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(controls, chunks[2]);
    }

    fn render_query_view(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search input
                Constraint::Length(3), // Status / error message
                Constraint::Min(6),    // Results table
                Constraint::Length(3), // Controls footer
            ])
            .split(area);

        // Search Input Box
        let prompt_style = if !self.query_focus_results {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let cursor_char = if !self.query_focus_results { "█" } else { "" };
        let mut display_input = self.query_input.clone();
        if !self.query_focus_results && self.query_cursor <= display_input.len() {
            display_input.insert_str(self.query_cursor, cursor_char);
        }

        let input_para = Paragraph::new(Line::from(vec![
            Span::styled("CQL > ", prompt_style),
            Span::styled(
                display_input,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 🔍 CQL-Lite Search Query (powered by scid-mgr) ")
                .border_style(if !self.query_focus_results {
                    Color::Yellow
                } else {
                    Color::DarkGray
                }),
        );
        f.render_widget(input_para, chunks[0]);

        // Status or Error
        let status_para = if let Some(err) = &self.query_error {
            Paragraph::new(Line::from(vec![
                Span::styled(
                    "❌ Error: ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(err, Style::default().fg(Color::LightRed)),
            ]))
        } else if let Some(info) = &self.query_info {
            Paragraph::new(Line::from(vec![
                Span::styled(
                    "✓ ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(info, Style::default().fg(Color::Green)),
            ]))
        } else {
            Paragraph::new(Line::from(vec![
                Span::styled("💡 Examples: ", Style::default().fg(Color::DarkGray)),
                Span::styled("white=\"Kasparov\"", Style::default().fg(Color::LightCyan)),
                Span::styled(" | ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "result=\"1-0\" and eco=\"B90\"",
                    Style::default().fg(Color::LightCyan),
                ),
                Span::styled(" | ", Style::default().fg(Color::DarkGray)),
                Span::styled("check and Q vs Q", Style::default().fg(Color::LightCyan)),
                Span::styled(" | ", Style::default().fg(Color::DarkGray)),
                Span::styled("moves=[e4, c5, Nf3]", Style::default().fg(Color::LightCyan)),
            ]))
        }
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Query Diagnostics "),
        );
        f.render_widget(status_para, chunks[1]);

        let is_compact = chunks[2].width < 80;

        if is_compact {
            let available_height = chunks[2].height.saturating_sub(2) as usize;
            let items_per_page = (available_height / 2).max(1);
            let mut list_items = Vec::new();

            if !self.query_results.is_empty() {
                let total = self.query_results.len();
                let start_idx = if self.query_selected_idx >= items_per_page {
                    self.query_selected_idx.saturating_sub(items_per_page / 2)
                } else {
                    0
                };

                for idx in start_idx..(start_idx + items_per_page).min(total) {
                    let (game_id, summary) = &self.query_results[idx];
                    let is_selected = idx == self.query_selected_idx && self.query_focus_results;
                    let prefix = if is_selected { "▶ " } else { "  " };

                    let w_elo = if summary.white_elo > 0 {
                        format!(" ({})", summary.white_elo)
                    } else {
                        String::new()
                    };
                    let b_elo = if summary.black_elo > 0 {
                        format!(" ({})", summary.black_elo)
                    } else {
                        String::new()
                    };

                    let num_style = if is_selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Cyan)
                    };
                    let name_style = if is_selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let meta_style = if is_selected {
                        Style::default().fg(Color::LightCyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let result_style = if is_selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    };

                    let line1 = Line::from(vec![
                        Span::styled(format!("{}[{:>4}] ", prefix, game_id + 1), num_style),
                        Span::styled(format!("{}{}", summary.white, w_elo), name_style),
                        Span::styled(" vs ", Style::default().fg(Color::DarkGray)),
                        Span::styled(format!("{}{}", summary.black, b_elo), name_style),
                        Span::styled(format!("  [{}]", summary.result), result_style),
                    ]);

                    let line2 = Line::from(vec![
                        Span::styled("       ", Style::default()),
                        Span::styled(
                            format!("{} • {} • {}", summary.date, summary.eco, summary.event),
                            meta_style,
                        ),
                    ]);

                    list_items.push(ListItem::new(vec![line1, line2]));
                }
            }

            let list = List::new(list_items).block(
                Block::default()
                    .title(format!(" 📂 Matches ({}) ", self.query_results.len()))
                    .borders(Borders::ALL)
                    .border_style(if self.query_focus_results {
                        Color::Yellow
                    } else {
                        Color::DarkGray
                    }),
            );
            f.render_widget(list, chunks[2]);
        } else {
            // Results Table
            let visible_rows = chunks[2].height.saturating_sub(2) as usize;
            let mut rows = Vec::new();

            if !self.query_results.is_empty() {
                let total = self.query_results.len();
                let start_idx = if self.query_selected_idx >= visible_rows {
                    self.query_selected_idx.saturating_sub(visible_rows / 2)
                } else {
                    0
                };

                for idx in start_idx..(start_idx + visible_rows).min(total) {
                    let (game_id, summary) = &self.query_results[idx];
                    let is_selected = idx == self.query_selected_idx && self.query_focus_results;
                    let style = if is_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    rows.push(Row::new(vec![
                        Span::styled(
                            format!("{:>5}", game_id + 1),
                            if is_selected {
                                style
                            } else {
                                Style::default().fg(Color::DarkGray)
                            },
                        ),
                        Span::styled(summary.white.clone(), style),
                        Span::styled(
                            if summary.white_elo > 0 {
                                format!("{}", summary.white_elo)
                            } else {
                                "-".to_string()
                            },
                            style,
                        ),
                        Span::styled(summary.black.clone(), style),
                        Span::styled(
                            if summary.black_elo > 0 {
                                format!("{}", summary.black_elo)
                            } else {
                                "-".to_string()
                            },
                            style,
                        ),
                        Span::styled(summary.result.clone(), style),
                        Span::styled(summary.date.clone(), style),
                        Span::styled(summary.eco.clone(), style),
                        Span::styled(summary.event.clone(), style),
                    ]));
                }
            }

            let header = Row::new(vec![
                Span::styled(
                    "    #",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "White",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Elo",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Black",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Elo",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Result",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Date",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "ECO",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Event",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]);

            let results_table = Table::new(
                rows,
                [
                    Constraint::Length(6),
                    Constraint::Percentage(22),
                    Constraint::Length(6),
                    Constraint::Percentage(22),
                    Constraint::Length(6),
                    Constraint::Length(8),
                    Constraint::Length(11),
                    Constraint::Length(6),
                    Constraint::Percentage(25),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .title(format!(" Matches ({}) ", self.query_results.len()))
                    .borders(Borders::ALL)
                    .border_style(if self.query_focus_results {
                        Color::Yellow
                    } else {
                        Color::DarkGray
                    }),
            );
            f.render_widget(results_table, chunks[2]);
        }

        // Controls
        let controls = Paragraph::new(
            "[Enter] Execute Query / Load Selected Match  [↑/↓] Select Result  [Esc] Clear / Edit  [Tab] Switch Tab  [Q] Quit",
        )
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(controls, chunks[3]);
    }
}
