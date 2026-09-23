# ♞ tchess

A fast, responsive, and modern terminal chessboard widget and PGN game viewer built with **Rust** and **Ratatui**.

Works seamlessly on desktop terminals and mobile devices (via Termux / SSH).

---

## ✨ Features

- **📱 Fully Responsive Square Scaling**: Automatically detects terminal window dimensions and scales cell sizes ($2\times 1$, $4\times 2$, $6\times 3$, $8\times 4$) using a strict 2:1 character cell ratio to maintain a 1:1 square visual aspect ratio.
- **♔ Centered Unicode Pieces**: Crisp Unicode chess pieces (`♚ ♛ ♜ ♝ ♞ ♟` / `♔ ♕ ♖ ♗ ♘ ♙`) with Unicode Variation Selector-15 (`\uFE0E`) to prevent 3D emoji discoloration.
- **🎨 Piece Themes**: Press `t` to cycle between **Solid**, **Classic**, and **Letters** piece themes.
- **📖 PGN Game Navigation**: Step through moves (`←`/`→`, `h`/`l`, `p`/`n`), jump to start/end (`Home`/`End`), or run animated **Autoplay** (`Space`).
- **⚔ Live Evaluation & Captures**: Real-time material differential calculation and captured piece display.
- **🏆 Built-in Famous Games**: Includes Morphy's Opera Game (1858), Kasparov's Immortal (1999), and Fischer's Game of the Century (1956). Press `s` to switch between games.
- **📂 Custom PGN Support**: Load any standard `.pgn` file directly from the command line.
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

# Or view your own PGN file
cargo run --release -- path/to/game.pgn
```

### Install via Cargo
```bash
cargo install --path .
tchess [game.pgn]
```

---

## ⌨️ Keybindings

| Key | Action |
| :--- | :--- |
| `→` / `l` / `n` | Next Move |
| `←` / `h` / `p` | Previous Move |
| `Home` / `0` | Jump to Start of Game |
| `End` / `$` | Jump to End of Game |
| `Space` | Toggle Autoplay (Play / Pause) |
| `f` | Flip Board (White ⇄ Black perspective) |
| `t` | Cycle Piece Theme (Solid ⇄ Classic ⇄ Letters) |
| `s` | Switch Game (Cycle through built-in games) |
| `q` / `Esc` | Quit |

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
