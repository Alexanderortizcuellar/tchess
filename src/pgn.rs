use pgn_reader::{BufferedReader, SanPlus, Visitor};
use shakmaty::san::San;
use shakmaty::{Chess, Move, Position};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct MoveNode {
    #[allow(dead_code)]
    pub ply: usize,
    pub san: String,
    pub pos: Chess,
    pub last_move: Option<Move>,
    pub comment: String,
}

#[derive(Clone, Debug)]
pub struct PgnGame {
    pub headers: HashMap<String, String>,
    pub moves: Vec<MoveNode>,
}

pub fn strip_annotations(comment: &str) -> String {
    let mut result = String::with_capacity(comment.len());
    let mut chars = comment.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '[' && chars.peek() == Some(&'%') {
            for c in chars.by_ref() {
                if c == ']' {
                    break;
                }
            }
            continue;
        }
        result.push(ch);
    }

    let words: Vec<&str> = result.split_whitespace().collect();
    words.join(" ")
}

impl PgnGame {
    pub fn new() -> Self {
        let start_pos = Chess::default();
        Self {
            headers: HashMap::new(),
            moves: vec![MoveNode {
                ply: 0,
                san: "Start".to_string(),
                pos: start_pos,
                last_move: None,
                comment: String::new(),
            }],
        }
    }

    pub fn parse(pgn: &str) -> Self {
        struct GameCollector {
            game: PgnGame,
            current_pos: Chess,
        }

        impl Visitor for GameCollector {
            type Result = PgnGame;

            fn begin_game(&mut self) {}

            fn san(&mut self, san_plus: SanPlus) {
                if let Ok(san) = san_plus.san.to_string().parse::<San>() {
                    if let Ok(m) = san.to_move(&self.current_pos) {
                        let mut next_pos = self.current_pos.clone();
                        next_pos.play_unchecked(&m);
                        let san_str = san_plus.to_string();
                        let ply = self.game.moves.len();
                        self.game.moves.push(MoveNode {
                            ply,
                            san: san_str,
                            pos: next_pos.clone(),
                            last_move: Some(m),
                            comment: String::new(),
                        });
                        self.current_pos = next_pos;
                    }
                }
            }

            fn comment(&mut self, comment: pgn_reader::RawComment<'_>) {
                if let Ok(comment_str) = std::str::from_utf8(comment.as_bytes()) {
                    if let Some(last_node) = self.game.moves.last_mut() {
                        if !last_node.comment.is_empty() {
                            last_node.comment.push(' ');
                        }
                        last_node.comment.push_str(comment_str.trim());
                    }
                }
            }

            fn end_game(&mut self) -> Self::Result {
                self.game.clone()
            }
        }

        let mut headers = HashMap::new();
        for line in pgn.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let inner = &trimmed[1..trimmed.len() - 1];
                if let Some(space_idx) = inner.find(' ') {
                    let key = inner[..space_idx].trim().to_string();
                    let val = inner[space_idx..].trim().trim_matches('"').to_string();
                    headers.insert(key, val);
                }
            }
        }

        let mut reader = BufferedReader::new(pgn.as_bytes());
        let mut collector = GameCollector {
            game: PgnGame {
                headers,
                moves: vec![MoveNode {
                    ply: 0,
                    san: "Start".to_string(),
                    pos: Chess::default(),
                    last_move: None,
                    comment: String::new(),
                }],
            },
            current_pos: Chess::default(),
        };

        match reader.read_game(&mut collector) {
            Ok(Some(game)) => game,
            _ => PgnGame::new(),
        }
    }

    pub fn white_player(&self) -> &str {
        self.headers
            .get("White")
            .map(|s| s.as_str())
            .unwrap_or("White")
    }

    pub fn black_player(&self) -> &str {
        self.headers
            .get("Black")
            .map(|s| s.as_str())
            .unwrap_or("Black")
    }

    pub fn event(&self) -> &str {
        self.headers
            .get("Event")
            .map(|s| s.as_str())
            .unwrap_or("Casual Game")
    }

    pub fn result(&self) -> &str {
        self.headers
            .get("Result")
            .map(|s| s.as_str())
            .unwrap_or("*")
    }

    pub fn total_ply(&self) -> usize {
        self.moves.len().saturating_sub(1)
    }

    pub fn get_node(&self, ply: usize) -> &MoveNode {
        let idx = ply.min(self.total_ply());
        &self.moves[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::samples::MORPHY_OPERA;

    #[test]
    fn test_parse_opera_game() {
        let game = PgnGame::parse(MORPHY_OPERA);
        assert_eq!(game.white_player(), "Paul Morphy");
        assert_eq!(game.total_ply(), 33);
    }

    #[test]
    fn test_comments_and_annotation_stripping() {
        let pgn = r#"[Event "Test Game"]
[White "Player1"]
[Black "Player2"]

1. e4 { [%clk 0:05:00] [%eval 0.25] Best by test. } 1... e5 { [%clk 0:04:58] [%eval 0.20] Standard reply. }
2. Nf3 { [%eval #3] White threatens e5 } *
"#;
        let game = PgnGame::parse(pgn);
        assert_eq!(game.total_ply(), 3);
        let node1 = game.get_node(1);
        assert_eq!(node1.san, "e4");
        assert_eq!(node1.comment, "[%clk 0:05:00] [%eval 0.25] Best by test.");
        assert_eq!(strip_annotations(&node1.comment), "Best by test.");

        let node2 = game.get_node(2);
        assert_eq!(node2.san, "e5");
        assert_eq!(strip_annotations(&node2.comment), "Standard reply.");

        let node3 = game.get_node(3);
        assert_eq!(node3.san, "Nf3");
        assert_eq!(strip_annotations(&node3.comment), "White threatens e5");
    }
}
