# Sumo

A TUI app for viewing sumo tournament bouts and results.

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]

[crates-badge]:https://img.shields.io/crates/v/sumo.svg 
[crates-url]: https://crates.io/crates/sumo
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/yu-eric/sumo/blob/main/LICENSE

## Features

- **Daily Matches (Torikumi)**: View match results for a specific day and division
- **Rankings (Banzuke)**: View rikishi rankings for a division
- **Tournament Information**: View basic information about a basho (tournament)
- **Rikishi Details**: View detailed information about individual rikishi including stats, heya, and physical measurements
- **Head-to-Head History**: View match history between two rikishi with win/loss records and technique breakdowns
- **Multiple Divisions**: Support for all sumo divisions (Makuuchi, Juryo, Makushita, Sandanme, Jonidan, Jonokuchi)
- **Interactive Navigation**: Keyboard-driven interface

## Installation

Make sure you have Rust installed, then clone and build the project:

```bash
git clone <repository-url>
cd sumo
cargo build --release
```

## Usage

### Basic Usage

Run the application with default settings (current basho, current day, Makuuchi division):

```bash
cargo run
```

### Command Line Options

```bash
# Specify a basho (tournament) by YYYYMM format
cargo run -- --basho 202401

# Specify a day (1-15)
cargo run -- --day 10

# Specify a division
cargo run -- --division juryo

# Start in banzuke (rankings) view
cargo run -- --banzuke

# Combine options
cargo run -- --basho 202401 --day 5 --division makuuchi
```

### Available Divisions

- `makuuchi` - Top division (default)
- `juryo` - Second division
- `makushita` - Third division
- `sandanme` - Fourth division
- `jonidan` - Fifth division
- `jonokuchi` - Sixth division

## Keyboard Controls

### Navigation
- `↑/↓` or `w/s` - Navigate through lists
- `←/→` or `a/d` - Switch between pages (Torikumi ↔ Banzuke ↔ Basho Info)
- `Enter` or `Space` - View details (rikishi details in banzuke, head-to-head in torikumi)
- `1` - Jump to daily matches (torikumi)
- `2` - Jump to rankings (banzuke)
- `3` - Jump to basho information
- `Esc` - Close popups/help

### Data Controls
- `c` - Change day (1-15)
- `v` - Change division (interactive selector)
- `b` - Change basho (YYYYMM format)

### Other
- `h` or `F1` - Toggle help
- `q` - Quit application
- `Esc` - Close help

## API Data Source

This app uses the Sumo API (https://www.sumo-api.com/) to fetch tournament data.

## Building for Release

To build an optimized release version:

```bash
cargo build --release
```

The binary will be available at `target/release/sumo`.
