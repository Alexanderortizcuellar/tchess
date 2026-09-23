use shakmaty::fen::Fen;
use shakmaty::{Chess, Color, EnPassantMode, Position};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

pub fn position_to_fen(pos: &Chess) -> String {
    Fen::from_position(pos.clone(), EnPassantMode::Legal).to_string()
}

#[derive(Debug, Clone)]
pub struct EvalResult {
    #[allow(dead_code)]
    pub fen: String,
    pub display: String,
    pub score_cp: Option<i32>,
    #[allow(dead_code)]
    pub depth: u32,
}

enum EngineCommand {
    Evaluate {
        fen: String,
        is_black: bool,
        depth: u32,
    },
    Quit,
}

pub struct EngineClient {
    cmd_sender: Sender<EngineCommand>,
    result_receiver: Receiver<EvalResult>,
    _worker: thread::JoinHandle<()>,
}

impl EngineClient {
    pub fn new(engine_path: Option<&str>) -> Result<Self, String> {
        let path = engine_path.unwrap_or("stockfish");

        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn '{}': {}", path, e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to capture engine stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture engine stdout".to_string())?;

        let (cmd_tx, cmd_rx) = mpsc::channel::<EngineCommand>();
        let (res_tx, res_rx) = mpsc::channel::<EvalResult>();

        let worker = thread::spawn(move || {
            engine_worker_loop(child, stdin, stdout, cmd_rx, res_tx);
        });

        Ok(Self {
            cmd_sender: cmd_tx,
            result_receiver: res_rx,
            _worker: worker,
        })
    }

    pub fn evaluate(&self, pos: &Chess, depth: u32) {
        let is_black = pos.turn() == Color::Black;
        let fen = position_to_fen(pos);
        let _ = self.cmd_sender.send(EngineCommand::Evaluate {
            fen,
            is_black,
            depth,
        });
    }

    pub fn try_get_eval(&self) -> Option<EvalResult> {
        let mut latest = None;
        loop {
            match self.result_receiver.try_recv() {
                Ok(res) => latest = Some(res),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        latest
    }
}

impl Drop for EngineClient {
    fn drop(&mut self) {
        let _ = self.cmd_sender.send(EngineCommand::Quit);
    }
}

fn engine_worker_loop(
    mut child: Child,
    mut stdin: ChildStdin,
    stdout: std::process::ChildStdout,
    cmd_rx: Receiver<EngineCommand>,
    res_tx: Sender<EvalResult>,
) {
    let mut reader = BufReader::new(stdout);

    // Initial UCI setup
    if writeln!(stdin, "uci").is_err() {
        return;
    }
    let mut line = String::new();
    while let Ok(n) = reader.read_line(&mut line) {
        if n == 0 || line.trim() == "uciok" {
            break;
        }
        line.clear();
    }

    if writeln!(stdin, "isready").is_err() {
        return;
    }
    line.clear();
    while let Ok(n) = reader.read_line(&mut line) {
        if n == 0 || line.trim() == "readyok" {
            break;
        }
        line.clear();
    }

    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            EngineCommand::Quit => {
                let _ = writeln!(stdin, "quit");
                let _ = child.kill();
                break;
            }
            EngineCommand::Evaluate {
                fen,
                is_black,
                depth,
            } => {
                // Drain any pending commands to get the newest position
                let mut latest_fen = fen;
                let mut latest_is_black = is_black;
                let mut latest_depth = depth;
                while let Ok(newer_cmd) = cmd_rx.try_recv() {
                    match newer_cmd {
                        EngineCommand::Quit => {
                            let _ = writeln!(stdin, "quit");
                            let _ = child.kill();
                            return;
                        }
                        EngineCommand::Evaluate {
                            fen: n_fen,
                            is_black: n_is_black,
                            depth: n_depth,
                        } => {
                            latest_fen = n_fen;
                            latest_is_black = n_is_black;
                            latest_depth = n_depth;
                        }
                    }
                }

                let _ = writeln!(stdin, "position fen {}", latest_fen);
                let _ = writeln!(stdin, "go depth {}", latest_depth);

                line.clear();
                let mut best_eval: Option<EvalResult> = None;

                while let Ok(n) = reader.read_line(&mut line) {
                    if n == 0 {
                        break;
                    }
                    let trimmed = line.trim();

                    if trimmed.starts_with("info ") && trimmed.contains(" score ") {
                        if let Some(eval) = parse_info_line(trimmed, &latest_fen, latest_is_black) {
                            best_eval = Some(eval.clone());
                            let _ = res_tx.send(eval);
                        }
                    }

                    if trimmed.starts_with("bestmove") {
                        break;
                    }
                    line.clear();
                }

                if let Some(eval) = best_eval {
                    let _ = res_tx.send(eval);
                }
            }
        }
    }
}

fn parse_info_line(line: &str, fen: &str, is_black: bool) -> Option<EvalResult> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    let mut depth: u32 = 0;
    let mut score_cp: Option<i32> = None;
    let mut score_mate: Option<i32> = None;

    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            "depth" if i + 1 < parts.len() => {
                depth = parts[i + 1].parse().unwrap_or(0);
                i += 1;
            }
            "score" if i + 2 < parts.len() => {
                match parts[i + 1] {
                    "cp" => {
                        score_cp = parts[i + 2].parse().ok();
                    }
                    "mate" => {
                        score_mate = parts[i + 2].parse().ok();
                    }
                    _ => {}
                }
                i += 2;
            }
            _ => {}
        }
        i += 1;
    }

    if let Some(mate) = score_mate {
        let white_mate = if is_black { -mate } else { mate };
        let display = if white_mate > 0 {
            format!("+M{} (d{})", white_mate, depth)
        } else {
            format!("-M{} (d{})", white_mate.abs(), depth)
        };
        Some(EvalResult {
            fen: fen.to_string(),
            display,
            score_cp: None,
            depth,
        })
    } else if let Some(cp) = score_cp {
        let white_cp = if is_black { -cp } else { cp };
        let pawns = (white_cp as f64) / 100.0;
        let display = if white_cp > 0 {
            format!("+{:0.2} (d{})", pawns, depth)
        } else if white_cp < 0 {
            format!("{:0.2} (d{})", pawns, depth)
        } else {
            format!("0.00 (d{})", depth)
        };
        Some(EvalResult {
            fen: fen.to_string(),
            display,
            score_cp: Some(white_cp),
            depth,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::Chess;

    #[test]
    fn test_position_to_fen() {
        let start_pos = Chess::default();
        let fen = position_to_fen(&start_pos);
        assert_eq!(
            fen,
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        );
    }

    #[test]
    fn test_parse_info_cp() {
        let line = "info depth 12 seldepth 18 score cp 45 nodes 3012 nps 200000 time 15 pv e2e4";
        let eval = parse_info_line(line, "fen", false).unwrap();
        assert_eq!(eval.display, "+0.45 (d12)");
        assert_eq!(eval.score_cp, Some(45));
        assert_eq!(eval.depth, 12);

        // When black is to move, +45 cp for black is -0.45 for white
        let eval_black = parse_info_line(line, "fen", true).unwrap();
        assert_eq!(eval_black.display, "-0.45 (d12)");
        assert_eq!(eval_black.score_cp, Some(-45));
    }

    #[test]
    fn test_parse_info_mate() {
        let line = "info depth 10 score mate 3 nodes 1234 pv e2e4";
        let eval = parse_info_line(line, "fen", false).unwrap();
        assert_eq!(eval.display, "+M3 (d10)");

        let eval_black = parse_info_line(line, "fen", true).unwrap();
        assert_eq!(eval_black.display, "-M3 (d10)");
    }
}
