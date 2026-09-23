use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};
use shakmaty::{Chess, Color as PieceColor, Position, Role};
use std::time::Duration;

use crate::board::{ChessBoardWidget, PieceTheme};
use crate::pgn::PgnGame;
use crate::samples::{FISCHER_BYRNE, KASPAROV_IMMORTAL, MORPHY_OPERA};

pub struct App {
    pub games: Vec<(&'static str, PgnGame)>,
    pub current_game_idx: usize,
    pub current_ply: usize,
    pub flipped: bool,
    pub theme: PieceTheme,
    pub autoplay: bool,
    pub autoplay_interval: Duration,
    pub should_quit: bool,
}

impl App {
    pub fn new(custom_pgn: Option<String>) -> Self {
        let mut games = Vec::new();

        if let Some(pgn) = custom_pgn {
            games.push(("Loaded PGN File", PgnGame::parse(&pgn)));
        }

        games.push(("Morphy's Opera Game (1858)", PgnGame::parse(MORPHY_OPERA)));
        games.push((
            "Kasparov's Immortal (1999)",
            PgnGame::parse(KASPAROV_IMMORTAL),
        ));
        games.push(("Game of the Century (1956)", PgnGame::parse(FISCHER_BYRNE)));

        Self {
            games,
            current_game_idx: 0,
            current_ply: 0,
            flipped: false,
            theme: PieceTheme::Solid,
            autoplay: false,
            autoplay_interval: Duration::from_millis(800),
            should_quit: false,
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
        }
    }

    pub fn prev_move(&mut self) {
        if self.current_ply > 0 {
            self.current_ply -= 1;
        }
    }

    pub fn first_move(&mut self) {
        self.current_ply = 0;
    }

    pub fn last_move(&mut self) {
        self.current_ply = self.current_game().total_ply();
    }

    pub fn toggle_flip(&mut self) {
        self.flipped = !self.flipped;
    }

    pub fn toggle_autoplay(&mut self) {
        self.autoplay = !self.autoplay;
    }

    pub fn next_game(&mut self) {
        self.current_game_idx = (self.current_game_idx + 1) % self.games.len();
        self.current_ply = 0;
        self.autoplay = false;
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

        let is_desktop = size.width >= 75;
        let main_chunks = if is_desktop {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(40), Constraint::Length(42)])
                .split(size)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(size)
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
        let side_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7), // Info
                Constraint::Min(6),    // Moves
                Constraint::Length(3), // Controls Footer
            ])
            .split(main_chunks[1]);

        // Game Info Block
        let (_, _, eval_diff) = self.calculate_material(&current_node.pos);
        let eval_str = if eval_diff > 0 {
            format!("+{}", eval_diff)
        } else if eval_diff < 0 {
            format!("{}", eval_diff)
        } else {
            "=".to_string()
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
                Span::styled(
                    format!("Eval: {}  ", eval_str),
                    Style::default()
                        .fg(if eval_diff >= 0 {
                            Color::Green
                        } else {
                            Color::Red
                        })
                        .add_modifier(Modifier::BOLD),
                ),
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

        // Auto-scroll window for moves
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

        // Controls Footer
        let controls_para = Paragraph::new(
            "[←/→] Move  [Home/End] Start/End  [F] Flip  [Space] Play  [S] Game  [Q] Quit",
        )
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(controls_para, side_chunks[2]);
    }
}
