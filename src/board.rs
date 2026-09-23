use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use shakmaty::{Chess, Color as PieceColor, File, Move, Piece, Position, Rank, Role, Square};

const VS15: &str = "\u{FE0E}";

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PieceTheme {
    Solid,
    Classic,
    Letters,
}

pub struct ChessBoardWidget<'a> {
    pub position: &'a Chess,
    pub last_move: Option<&'a Move>,
    pub flipped: bool,
    pub show_coordinates: bool,
    pub theme: PieceTheme,
}

impl<'a> ChessBoardWidget<'a> {
    pub fn new(position: &'a Chess) -> Self {
        Self {
            position,
            last_move: None,
            flipped: false,
            show_coordinates: true,
            theme: PieceTheme::Solid,
        }
    }

    pub fn last_move(mut self, m: Option<&'a Move>) -> Self {
        self.last_move = m;
        self
    }

    pub fn flipped(mut self, flipped: bool) -> Self {
        self.flipped = flipped;
        self
    }

    pub fn theme(mut self, theme: PieceTheme) -> Self {
        self.theme = theme;
        self
    }

    fn piece_glyph(&self, piece: Piece) -> String {
        match self.theme {
            PieceTheme::Solid => {
                let s = match piece.role {
                    Role::Pawn => "♟",
                    Role::Knight => "♞",
                    Role::Bishop => "♝",
                    Role::Rook => "♜",
                    Role::Queen => "♛",
                    Role::King => "♚",
                };
                format!("{}{}", s, VS15)
            }
            PieceTheme::Classic => {
                let s = match (piece.color, piece.role) {
                    (PieceColor::White, Role::Pawn) => "♙",
                    (PieceColor::White, Role::Knight) => "♘",
                    (PieceColor::White, Role::Bishop) => "♗",
                    (PieceColor::White, Role::Rook) => "♖",
                    (PieceColor::White, Role::Queen) => "♕",
                    (PieceColor::White, Role::King) => "♔",
                    (PieceColor::Black, Role::Pawn) => "♟",
                    (PieceColor::Black, Role::Knight) => "♞",
                    (PieceColor::Black, Role::Bishop) => "♝",
                    (PieceColor::Black, Role::Rook) => "♜",
                    (PieceColor::Black, Role::Queen) => "♛",
                    (PieceColor::Black, Role::King) => "♚",
                };
                format!("{}{}", s, VS15)
            }
            PieceTheme::Letters => {
                let s = match (piece.color, piece.role) {
                    (PieceColor::White, Role::Pawn) => "P",
                    (PieceColor::White, Role::Knight) => "N",
                    (PieceColor::White, Role::Bishop) => "B",
                    (PieceColor::White, Role::Rook) => "R",
                    (PieceColor::White, Role::Queen) => "Q",
                    (PieceColor::White, Role::King) => "K",
                    (PieceColor::Black, Role::Pawn) => "p",
                    (PieceColor::Black, Role::Knight) => "n",
                    (PieceColor::Black, Role::Bishop) => "b",
                    (PieceColor::Black, Role::Rook) => "r",
                    (PieceColor::Black, Role::Queen) => "q",
                    (PieceColor::Black, Role::King) => "k",
                };
                s.to_string()
            }
        }
    }
}

impl<'a> Widget for ChessBoardWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 8 {
            return;
        }

        let coord_w: u16 = if self.show_coordinates { 2 } else { 0 };
        let coord_h: u16 = if self.show_coordinates { 1 } else { 0 };

        let max_scale_w = (area.width.saturating_sub(coord_w)) / 16;
        let max_scale_h = (area.height.saturating_sub(coord_h)) / 8;

        let scale = max_scale_w.min(max_scale_h).max(1);
        let sq_h = scale;
        let sq_w = 2 * scale;

        let total_w = 8 * sq_w + coord_w;
        let total_h = 8 * sq_h + coord_h;

        let offset_x = area.x + (area.width.saturating_sub(total_w)) / 2;
        let offset_y = area.y + (area.height.saturating_sub(total_h)) / 2;

        let light_bg = Color::Rgb(240, 217, 181);
        let dark_bg = Color::Rgb(181, 136, 99);
        let light_last_move = Color::Rgb(205, 210, 106);
        let dark_last_move = Color::Rgb(170, 162, 58);
        let check_bg = Color::Rgb(220, 60, 60);

        let white_fg = Color::Rgb(255, 255, 255);
        let black_fg = Color::Rgb(25, 25, 25);
        let border_fg = Color::Rgb(160, 160, 160);

        let is_check = self.position.is_check();
        let check_king_sq = if is_check {
            self.position.board().king_of(self.position.turn())
        } else {
            None
        };

        let last_from = self.last_move.and_then(|m| m.from());
        let last_to = self.last_move.map(|m| m.to());

        let ranks: Vec<u8> = if !self.flipped {
            (0..8).rev().collect()
        } else {
            (0..8).collect()
        };
        let files: Vec<u8> = if !self.flipped {
            (0..8).collect()
        } else {
            (0..8).rev().collect()
        };

        for (rank_idx, &rank_num) in ranks.iter().enumerate() {
            let rank = Rank::new(rank_num as u32);
            let rank_char = (b'1' + rank_num) as char;

            for sub_y in 0..sq_h {
                let curr_y = offset_y + (rank_idx as u16) * sq_h + sub_y;

                if self.show_coordinates && sub_y == sq_h / 2 {
                    let rank_str = format!("{} ", rank_char);
                    buf.set_string(
                        offset_x,
                        curr_y,
                        rank_str,
                        Style::default().fg(border_fg).add_modifier(Modifier::BOLD),
                    );
                }

                for (file_idx, &file_num) in files.iter().enumerate() {
                    let file = File::new(file_num as u32);
                    let sq = Square::from_coords(file, rank);
                    let curr_x = offset_x + coord_w + (file_idx as u16) * sq_w;

                    let is_light = (rank_num + file_num) % 2 != 0;
                    let is_last_move = last_from == Some(sq) || last_to == Some(sq);
                    let is_king_in_check = Some(sq) == check_king_sq;

                    let bg = if is_king_in_check {
                        check_bg
                    } else if is_last_move {
                        if is_light {
                            light_last_move
                        } else {
                            dark_last_move
                        }
                    } else {
                        if is_light {
                            light_bg
                        } else {
                            dark_bg
                        }
                    };

                    let piece = self.position.board().piece_at(sq);

                    // Fill square background
                    for dx in 0..sq_w {
                        buf.set_string(curr_x + dx, curr_y, " ", Style::default().bg(bg));
                    }

                    // Place piece in middle sub-row
                    if sub_y == sq_h / 2 {
                        if let Some(p) = piece {
                            let glyph = self.piece_glyph(p);
                            let fg = if p.color == PieceColor::White {
                                white_fg
                            } else {
                                black_fg
                            };
                            let left_pad = (sq_w.saturating_sub(1)) / 2;
                            buf.set_string(
                                curr_x + left_pad,
                                curr_y,
                                glyph,
                                Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
                            );
                        }
                    }
                }
            }
        }

        // Render bottom file coordinates
        if self.show_coordinates {
            let label_y = offset_y + 8 * sq_h;
            for (file_idx, &file_num) in files.iter().enumerate() {
                let file_char = (b'a' + file_num) as char;
                let curr_x = offset_x + coord_w + (file_idx as u16) * sq_w;
                let left_pad = (sq_w.saturating_sub(1)) / 2;
                buf.set_string(
                    curr_x + left_pad,
                    label_y,
                    file_char.to_string(),
                    Style::default().fg(border_fg).add_modifier(Modifier::BOLD),
                );
            }
        }
    }
}
