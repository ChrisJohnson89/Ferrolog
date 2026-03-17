# Ferrolog

A lightweight TUI log parser built in Rust. Navigate, search, and filter log files directly in your terminal.

![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/ChrisJohnson89/Ferrolog/main/install.sh | bash
```

Installs to `/usr/local/bin/ferrolog` (or `~/.local/bin` if sudo isn't available).

**Supported platforms:**
- Linux x86_64
- macOS arm64 (Apple Silicon)
- macOS x86_64 (Intel)

## Usage

```bash
ferrolog <logfile>
```

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` / `Home` | Go to top |
| `G` / `End` | Go to bottom |
| `PgDn` / `PgUp` | Scroll by 20 |
| `/` | Search |
| `1` | Filter: TRACE |
| `2` | Filter: DEBUG |
| `3` | Filter: INFO |
| `4` | Filter: WARN |
| `5` | Filter: ERROR |
| `6` | Filter: FATAL |
| `c` | Clear all filters |
| `Enter` | Toggle detail view |
| `?` | Toggle help |
| `q` / `Esc` | Quit |

## Log Formats

Ferrolog auto-detects common log formats:
- Standard (`[LEVEL] message`)
- Syslog
- Logfmt-style key=value

## Build from Source

```bash
git clone https://github.com/ChrisJohnson89/Ferrolog.git
cd Ferrolog
cargo build --release
./target/release/ferrolog <logfile>
```
