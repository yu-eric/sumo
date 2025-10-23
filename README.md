# Sumo TUI

A Terminal User Interface (TUI) application for viewing sumo tournament scores and information. This application fetches data from the Sumo API (https://www.sumo-api.com/) and displays it in an interactive terminal interface.

## Features

- **Daily Matches (Torikumi)**: View match results for a specific day and division
- **Rankings (Banzuke)**: View wrestler rankings for a division
- **Tournament Information**: View basic information about a basho (tournament)
- **Multiple Divisions**: Support for all sumo divisions (Makuuchi, Juryo, Makushita, Sandanme, Jonidan, Jonokuchi)
- **Interactive Navigation**: Keyboard-driven interface with help system

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
- `↑/↓` - Navigate through lists
- `1` - View daily matches (torikumi)
- `2` - View rankings (banzuke)
- `3` - View basho information
- `h` or `F1` - Toggle help
- `q` - Quit application
- `Esc` - Close help

## Default Behavior

When run without arguments, the application will:
1. Determine the current basho based on today's date
2. Calculate the current day of the basho (or use the last day if the basho has ended)
3. Show Makuuchi division results
4. Start in the daily matches (torikumi) view

## Basho Schedule

Sumo tournaments (basho) are held six times per year:
- January (Hatsu basho)
- March (Haru basho)
- May (Natsu basho)
- July (Nagoya basho)
- September (Aki basho)
- November (Kyushu basho)

Each basho runs for 15 days.

## API Data Source

This application uses the Sumo API (https://www.sumo-api.com/) to fetch tournament data. The API provides:
- Basho information including yusho (championship) and sansho (special prizes)
- Banzuke (rankings) for each division
- Torikumi (daily match results) for each day and division

## Dependencies

- `ratatui` - Terminal UI library
- `crossterm` - Cross-platform terminal manipulation
- `tokio` - Async runtime
- `reqwest` - HTTP client for API requests
- `serde` - Serialization/deserialization
- `chrono` - Date and time handling
- `clap` - Command line argument parsing
- `anyhow` - Error handling

## Error Handling

The application gracefully handles various error conditions:
- Network connectivity issues
- Missing or invalid tournament data
- Invalid command line arguments
- Terminal display errors

If data cannot be loaded, appropriate warning messages are displayed, and the application continues to run with the available data.

## Building for Release

To build an optimized release version:

```bash
cargo build --release
```

The binary will be available at `target/release/sumo`.
