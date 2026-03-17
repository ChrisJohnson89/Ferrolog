# Ferrolog

A fast, lightweight TUI log viewer built in Rust. Open any log file and navigate, search, and filter entries without leaving your terminal.

![Rust](https://img.shields.io/badge/rust-stable-orange)
[![Release](https://img.shields.io/github/v/release/ChrisJohnson89/Ferrolog)](https://github.com/ChrisJohnson89/Ferrolog/releases/latest)
![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macOS-lightgrey)

```
┌ Ferrolog ──────────────────────────────────────────────────────────────────┐
│ ferrolog  staging.enterasource.com-error_log  [1216/3208]  Filter: ERROR   │
└────────────────────────────────────────────────────────────────────────────┘
  #     Timestamp              Level    Source         Message
  150   2026/01/15 01:29:57    ERROR                   FastCGI sent in stderr: PHP Fatal error...
  157   2026/01/15 01:29:57    ERROR                   FastCGI sent in stderr: PHP Fatal error...
>> 243  2026/01/15 01:40:43    ERROR                   FastCGI sent in stderr: PHP Warning: require...

┌ Detail ─────────────────────────────────────────────────────────────────────┐
│ Line:  243                                                                   │
│ Level: ERROR                                                                 │
│ Time:  2026/01/15 01:40:43                                                   │
│                                                                              │
│ Raw:                                                                         │
│ [error] 1843380#1843380: *2955 FastCGI sent in stderr: "PHP message:...      │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/ChrisJohnson89/Ferrolog/main/install.sh | bash
```

Installs to `/usr/local/bin/ferrolog` (falls back to `~/.local/bin` without sudo).

**Supported platforms:**
| Platform | Target |
|----------|--------|
| Linux x86_64 | `x86_64-unknown-linux-musl` (static) |
| macOS Apple Silicon | `aarch64-apple-darwin` |
| macOS Intel | `x86_64-apple-darwin` |

## Usage

```bash
ferrolog <logfile>

# Examples
ferrolog /var/log/nginx/error.log
ferrolog /var/log/syslog
ferrolog app.log
```

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` / `Home` | Jump to top |
| `G` / `End` | Jump to bottom |
| `PgDn` / `PgUp` | Scroll by 20 |
| `Enter` | Toggle detail view |
| `/` | Search (live filter) |
| `Esc` | Exit search |
| `1` | Filter: TRACE |
| `2` | Filter: DEBUG |
| `3` | Filter: INFO |
| `4` | Filter: WARN |
| `5` | Filter: ERROR |
| `6` | Filter: FATAL |
| `c` | Clear all filters |
| `?` | Toggle help popup |
| `q` / `Esc` | Quit |

## Log Formats

Ferrolog auto-detects common formats:
- **Nginx / Apache** — `[error] ... "message"`
- **Standard** — `[LEVEL] message`, `LEVEL: message`
- **Syslog** — `Jan 15 01:29:57 host service[pid]: message`
- **Logfmt** — `level=error msg="something happened"`
- **Unrecognised lines** — shown as `UNKNOWN` and still navigable

## Build from Source

Requires Rust stable.

```bash
git clone https://github.com/ChrisJohnson89/Ferrolog.git
cd Ferrolog
cargo build --release
./target/release/ferrolog <logfile>
```

## Related

- [Ferromon](https://github.com/ChrisJohnson89/Ferromon) — system resource monitor TUI
