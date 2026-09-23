# ♞ tchess

A fast, responsive, and modern terminal chessboard widget and PGN game viewer built with **Rust** and **Ratatui**.

Works seamlessly on desktop terminals and mobile devices (via Termux / SSH).

---

## ✨ Features

- **📑 Multi-Screen Tabs**: Easily switch between **[1] Board**, **[2] Games Table**, and **[3] Query & Search** using `Tab` or `1`/`2`/`3`.
- **🗄 Fast PGN Database Browser**: Integrated with [`scid-mgr`](https://github.com/Alexanderortizcuellar/scid-mgr) for binary indexed (`.idx`) zero-copy database lookups. Scroll through thousands of games and press `Enter` to instantly load any game onto the board.
- **🔍 CQL-Lite Game Query Engine**: Run SQL-like queries against indexed databases (e.g. `ECO = "B90" AND Result = "1-0"`, `WhiteElo > 2700`, `White = "Kasparov"`, `Event LIKE "World Championship"`) and immediately browse the matching games.
- **📱 Fully Responsive Square Scaling**: Automatically detects terminal window dimensions and scales cell sizes ($2\times 1$, $4\times 2$, $6\times 3$, $8\times 4$) using a strict 2:1 character cell ratio to maintain a 1:1 square visual aspect ratio.
- **♔ Centered Unicode Pieces**: Crisp Unicode chess pieces (`♚ ♛ ♜ ♝ ♞ ♟` / `♔ ♕ ♖ ♗ ♘ ♙`) with Unicode Variation Selector-15 (`\uFE0E`) to prevent 3D emoji discoloration.
- **🎨 Piece Themes**: Press `t` to cycle between **Solid**, **Classic**, and **Letters** piece themes.
- **📖 PGN Game Navigation**: Step through moves (`←`/`→`, `h`/`l`, `p`/`n`), jump to start/end (`Home`/`End`), or run animated **Autoplay** (`Space`).
- **🤖 Live Stockfish Engine Evaluation**: Press `e` (or `--engine-eval`) to toggle real-time Stockfish UCI evaluation (`+0.45 (d12)`, `-M2`) right in the Game Info panel alongside material differentials.
- **💬 Move Comments & Clean Annotations**: View human explanations for each move. Engine evaluation (`[%eval ...]`) and clock (`[%clk ...]`) metadata are automatically filtered by default for a clean reading experience, with toggle options to view raw annotations.
- **🔄 Pipe PGN via Stdin**: Pipe PGN streams or files directly into `tchess` (`cat game.pgn | tchess` or `curl ... | tchess`).
- **⚔ Live Material & Captures**: Real-time material differential calculation and captured piece display.
- **🏆 Built-in Famous Games**: Includes Morphy's Opera Game (1858), Kasparov's Immortal (1999), and Fischer's Game of the Century (1956). Press `s` to switch between games.
- **📂 Custom PGN Support**: Load any standard `.pgn` file or pipe from stdin.
- **⚡ Single Zero-Dependency Binary**: Instant startup and tiny memory footprint.

---

## 🚀 Installation & Running

### Build from Source

Ensure you have Rust installed (1.75+):

```bash
# Clone the repository
git clone https://github.com/Alexanderortizcuellar/tchess.git
cd tchess

# Run with built-in famous games
cargo run --release

# View your own PGN file or database with Stockfish eval enabled
cargo run --release -- -e path/to/database.pgn

# Or pipe PGN from stdin directly
cat game.pgn | cargo run --release
curl -sL https://lichess.org/game/export/ID | cargo run --release

# Start flipped (Black's perspective)
cargo run --release -- --flip game.pgn
```

### CLI Options

| Flag | Description |
| :--- | :--- |
| `-f`, `--flip` | Start with board flipped (Black's perspective) |
| `-e`, `--engine-eval` | Enable Stockfish engine evaluation on startup |
| `-d`, `--depth <NUM>` | Stockfish search depth (default: `12`) |
| `--engine <PATH>` | Custom path to Stockfish binary (default: `stockfish`) |
| `-c`, `--show-comments` | Show move comments panel (default) |
| `--no-comments` | Hide move comments panel |
| `-a`, `--show-annotations` | Show raw clock (`[%clk]`) and eval (`[%eval]`) annotations |
| `--hide-annotations` | Clean comments by filtering clock and eval annotations (default) |
| `-h`, `--help` | Print help and usage information |

---

## ⌨️ Keybindings

### Global & Navigation
| Key | Action |
| :--- | :--- |
| `Tab` / `Shift+Tab` | Switch between screens (**Board** ⇄ **Games Table** ⇄ **Query**) |
| `1` / `2` / `3` | Directly switch to Screen 1, 2, or 3 |
| `q` / `Esc` | Quit Application |

### Board Screen (`[1] Board`)
| Key | Action |
| :--- | :--- |
| `→` / `l` / `n` | Next Move |
| `←` / `h` / `p` | Previous Move |
| `Home` / `0` | Jump to Start of Game |
| `End` / `$` | Jump to End of Game |
| `Space` | Toggle Autoplay (Play / Pause) |
| `f` | Flip Board (White ⇄ Black perspective) |
| `e` | Toggle Stockfish Engine Evaluation (Material ⇄ Stockfish) |
| `+` / `=` / `]` | Increase Stockfish Search Depth |
| `-` / `_` / `[` | Decrease Stockfish Search Depth |
| `c` | Toggle Comments Panel |
| `a` | Toggle Raw Annotations (Clock & Eval) |
| `t` | Cycle Piece Theme (Solid ⇄ Classic ⇄ Letters) |
| `s` | Switch Game (Cycle through loaded games) |

### Games Table Screen (`[2] Games Table`)
| Key | Action |
| :--- | :--- |
| `↓` / `j` | Select Next Game |
| `↑` / `k` | Select Previous Game |
| `PageDown` / `PageUp` | Jump 10 Games Down / Up |
| `Home` / `End` | Jump to First / Last Game in Database |
| `Enter` | Load Selected Game onto Board |

### Query & Search Screen (`[3] Query & Search`)
| Key | Action |
| :--- | :--- |
| `i` / `/` | Enter Query Text Input Mode |
| `Enter` (editing) | Execute CQL-Lite Query |
| `Esc` (editing) | Exit Text Input Mode |
| `↓` / `j` | Select Next Matching Game |
| `↑` / `k` | Select Previous Matching Game |
| `Enter` (browsing) | Load Selected Search Result onto Board |

---

## 📱 Mobile / Termux Usage

`tchess` compiles directly to a standalone native binary on Android with **Termux**:

```bash
pkg install rust git
git clone https://github.com/Alexanderortizcuellar/tchess.git
cd tchess
cargo build --release
./target/release/tchess
```

---

## 📜 License

MIT License.
