use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Table, Row, Cell},
    Frame, Terminal,
};
use std::io;
use crate::api::{Basho, BanzukeEntry, TorikumiEntry};
use std::collections::HashMap;

pub struct App {
    pub should_quit: bool,
    pub basho: Option<Basho>,
    pub banzuke: Option<Vec<BanzukeEntry>>,
    pub torikumi: Option<Vec<TorikumiEntry>>,
    pub current_view: AppView,
    pub selected_index: usize,
    pub division: String,
    pub day: u8,
    pub basho_id: String,
    pub show_help: bool,
    pub scroll_offset: usize,
    // Map rikishi id -> (wins, losses)
    pub record_map: HashMap<u32, (u8, u8)>,
}

#[derive(Clone, PartialEq)]
pub enum AppView {
    Torikumi,
    Banzuke,
    BashoInfo,
}

impl App {
    pub fn new(basho_id: String, division: String, day: u8) -> Self {
        Self {
            should_quit: false,
            basho: None,
            banzuke: None,
            torikumi: None,
            current_view: AppView::Torikumi,
            selected_index: 0,
            division,
            day,
            basho_id,
            show_help: false,
            scroll_offset: 0,
            record_map: HashMap::new(),
        }
    }

    pub fn set_basho(&mut self, basho: Basho) {
        self.basho = Some(basho);
    }

    pub fn set_banzuke(&mut self, banzuke: Vec<BanzukeEntry>) {
        // Store banzuke
        self.banzuke = Some(banzuke);
        // Recompute records map
        self.recompute_records();
    }

    pub fn set_torikumi(&mut self, torikumi: Vec<TorikumiEntry>) {
        self.torikumi = Some(torikumi);
    }

    fn recompute_records(&mut self) {
        self.record_map.clear();
        if let Some(list) = &self.banzuke {
            for entry in list {
                let mut wins: u8 = 0;
                let mut losses: u8 = 0;
                if let Some(records) = &entry.record {
                    for r in records {
                        let s = r.result.trim();
                        let sl = s.to_lowercase();
                        // Heuristics: support common encodings of results
                        if sl == "w" || sl == "win" || sl.contains("win") || s == "○" {
                            wins = wins.saturating_add(1);
                        } else if sl == "l" || sl == "loss" || sl.contains("loss") || s == "●" {
                            losses = losses.saturating_add(1);
                        }
                    }
                }
                self.record_map.insert(entry.rikishi_id, (wins, losses));
            }
        }
    }

    pub fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('h') | KeyCode::F(1) => self.show_help = !self.show_help,
            KeyCode::Char('1') => {
                self.current_view = AppView::Torikumi;
                self.selected_index = 0;
                self.scroll_offset = 0;
            },
            KeyCode::Char('2') => {
                self.current_view = AppView::Banzuke;
                self.selected_index = 0;
                self.scroll_offset = 0;
            },
            KeyCode::Char('3') => {
                self.current_view = AppView::BashoInfo;
                self.selected_index = 0;
                self.scroll_offset = 0;
            },
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    if self.selected_index < self.scroll_offset {
                        self.scroll_offset = self.selected_index;
                    }
                }
            }
            KeyCode::Down => {
                let max_index = match self.current_view {
                    AppView::Torikumi => self.torikumi.as_ref().map(|t| t.len()).unwrap_or(0),
                    AppView::Banzuke => self.banzuke.as_ref().map(|b| b.len()).unwrap_or(0),
                    AppView::BashoInfo => 0,
                };
                if self.selected_index + 1 < max_index {
                    self.selected_index += 1;
                    // Adjust scroll if selection goes beyond visible area (assume 10 visible items)
                    let visible_items = 10;
                    if self.selected_index >= self.scroll_offset + visible_items {
                        self.scroll_offset = self.selected_index - visible_items + 1;
                    }
                }
            }
            KeyCode::Esc => self.show_help = false,
            _ => {}
        }
    }
}

pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            app.on_key(key.code);
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    // Header
    let basho_date = crate::api::SumoApi::format_basho_date(&app.basho_id);
    let basho_month: u32 = app.basho_id[4..6].parse().unwrap_or(9);
    let basho_name = crate::api::SumoApi::get_basho_name(basho_month);
    
    let header = Paragraph::new(format!(
        "{} Results - {} {} - Day {}",
        basho_name, basho_date, app.division, app.day
    ))
    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).title("Sumo TUI"));

    f.render_widget(header, chunks[0]);

    // Main content
    match app.current_view {
        AppView::Torikumi => render_torikumi(f, chunks[1], app),
        AppView::Banzuke => render_banzuke(f, chunks[1], app),
        AppView::BashoInfo => render_basho_info(f, chunks[1], app),
    }

    // Footer
    let footer_text = "Press 'q' to quit | '1' Torikumi | '2' Banzuke | '3' Basho Info | 'h' Help | ↑↓ Navigate";
    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(footer, chunks[2]);

    // Help popup
    if app.show_help {
        render_help_popup(f);
    }
}

fn render_torikumi(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    if let Some(torikumi) = &app.torikumi {
        let visible_height = area.height.saturating_sub(3) as usize; // Account for borders and header
        let start_index = app.scroll_offset;
        let end_index = (start_index + visible_height).min(torikumi.len());
        
        let rows: Vec<Row> = torikumi
            .iter()
            .enumerate()
            .skip(start_index)
            .take(end_index - start_index)
            .map(|(i, match_entry)| {
                let style = if i == app.selected_index {
                    Style::default().bg(Color::Yellow).fg(Color::Black)
                } else {
                    Style::default()
                };

                let east_name = match_entry.east_shikona.clone();
                let west_name = match_entry.west_shikona.clone();
                let winner_opt = match_entry.winner_en.as_ref();
                let kimarite = match_entry.kimarite.as_ref().unwrap_or(&"N/A".to_string()).to_string();
                // Capitalize first letter of kimarite
                let kimarite = if !kimarite.is_empty() {
                    let mut chars: Vec<char> = kimarite.chars().collect();
                    chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                    chars.into_iter().collect()
                } else {
                    kimarite
                };

                // Compose "Name (Rank) (W-L)"
                let (ew, el) = app.record_map.get(&match_entry.east_id).copied().unwrap_or((0, 0));
                let (ww, wl) = app.record_map.get(&match_entry.west_id).copied().unwrap_or((0, 0));
                let east_text = format!("{} ({}) ({}-{})", east_name, abbr_rank(&match_entry.east_rank), ew, el);
                let west_text = format!("{} ({}) ({}-{})", west_name, abbr_rank(&match_entry.west_rank), ww, wl);

                // Bold the winner if present
                let (east_span, west_span) = if let Some(winner) = winner_opt {
                    let east_is_winner = winner == &east_name;
                    let west_is_winner = winner == &west_name;

                    let win_style = Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD);
                    let east_span = if east_is_winner {
                        Span::styled(east_text, win_style)
                    } else {
                        Span::raw(east_text)
                    };
                    let west_span = if west_is_winner {
                        Span::styled(west_text, win_style)
                    } else {
                        Span::raw(west_text)
                    };
                    (east_span, west_span)
                } else {
                    (Span::raw(east_text), Span::raw(west_text))
                };

                Row::new(vec![
                    Cell::from(Line::from(vec![east_span])),
                    Cell::from(Line::from(vec![west_span])),
                    Cell::from(kimarite),
                ]).style(style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(40), // East
                Constraint::Percentage(40), // West
                Constraint::Percentage(20), // Kimarite
            ],
        )
        .header(
            Row::new(vec!["East", "West", "Kimarite"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        )
        .block(Block::default().borders(Borders::ALL).title("Daily Matches"));

        f.render_widget(table, area);
    } else {
        let paragraph = Paragraph::new("Loading torikumi data...")
            .block(Block::default().borders(Borders::ALL).title("Daily Matches"))
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    }
}

// Convert a rank string to a compact abbreviation, e.g.:
// "Maegashira 7 East" -> "M7", "M7e" -> "M7", "Ozeki" -> "O", "Yokozuna" -> "Y"
fn abbr_rank(rank: &str) -> String {
    let r = rank.trim();
    let l = r.to_lowercase();
    let digits: String = r.chars().filter(|c| c.is_ascii_digit()).collect();

    if l.contains("yokozuna") || r.starts_with('Y') { return "Y".to_string(); }
    if l.contains("ozeki") || r.starts_with('O') { return "O".to_string(); }
    if l.contains("sekiwake") || r.starts_with('S') { return "S".to_string(); }
    if l.contains("komusubi") || r.starts_with('K') { return "K".to_string(); }
    if l.contains("maegashira") || r.starts_with('M') || r.starts_with('m') {
        return if digits.is_empty() { "M".to_string() } else { format!("M{}", digits) };
    }
    if l.contains("juryo") || r.starts_with('J') { return if digits.is_empty() { "J".to_string() } else { format!("J{}", digits) }; }

    // Generic fallback: take first alpha (uppercased) + digits if any, else original
    let first_alpha = r.chars().find(|c| c.is_ascii_alphabetic()).map(|c| c.to_ascii_uppercase());
    if let Some(ch) = first_alpha {
        if digits.is_empty() { ch.to_string() } else { format!("{}{}", ch, digits) }
    } else {
        r.to_string()
    }
}

fn render_banzuke(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    if let Some(banzuke) = &app.banzuke {
        let visible_height = area.height.saturating_sub(3) as usize; // Account for borders and header
        let start_index = app.scroll_offset;
        let end_index = (start_index + visible_height).min(banzuke.len());
        
        // Determine total days based on division
        // Makuuchi and Juryo have 15 days, Makushita and below have 7 days
        let total_days = if app.division.to_lowercase().contains("makuuchi") 
            || app.division.to_lowercase().contains("juryo") {
            15u8
        } else {
            7u8
        };
        
        let rows: Vec<Row> = banzuke
            .iter()
            .enumerate()
            .skip(start_index)
            .take(end_index - start_index)
            .map(|(i, entry)| {
                let style = if i == app.selected_index {
                    Style::default().bg(Color::Yellow).fg(Color::Black)
                } else {
                    Style::default()
                };

                // Calculate W-L-Absent from the record
                let (wins, losses, absent) = if let Some(records) = &entry.record {
                    let mut w = 0;
                    let mut l = 0;
                    for r in records {
                        match r.result.as_str() {
                            "win" => w += 1,
                            "loss" => l += 1,
                            _ => {}, // fusen-loss, fusen-win, or other - don't count as absent
                        }
                    }
                    // Calculate absent as total days minus wins and losses
                    let a = total_days.saturating_sub(w).saturating_sub(l);
                    (w, l, a)
                } else {
                    (0, 0, 0)
                };
                
                let result_str = format!("{}-{}-{}", wins, losses, absent);

                Row::new(vec![
                    Cell::from(entry.rank.clone()),
                    Cell::from(entry.shikona_en.clone()),
                    Cell::from(result_str),
                ]).style(style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(40),  // Rank
                Constraint::Percentage(40),  // Wrestler name
                Constraint::Percentage(20),  // Result (W-L-A)
            ],
        )
        .header(
            Row::new(vec!["Rank", "Wrestler", "Result"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        )
        .block(Block::default().borders(Borders::ALL).title("Banzuke"));

        f.render_widget(table, area);
    } else {
        let paragraph = Paragraph::new("Loading banzuke data...")
            .block(Block::default().borders(Borders::ALL).title("Banzuke"))
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    }
}

fn render_basho_info(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    if let Some(basho) = &app.basho {
        // Helper function to format date without timestamp
        let format_date = |date_str: &str| -> String {
            if let Some(date_part) = date_str.split('T').next() {
                date_part.to_string()
            } else {
                date_str.to_string()
            }
        };

        let mut text = vec![
            Line::from(vec![
                Span::styled("Location: ", Style::default().fg(Color::Yellow)),
                Span::raw(basho.location.as_deref().unwrap_or("Unknown")),
            ]),
            Line::from(vec![
                Span::styled("Start Date: ", Style::default().fg(Color::Yellow)),
                Span::raw(basho.start_date.as_deref().map(format_date).unwrap_or_else(|| "Unknown".to_string())),
            ]),
            Line::from(vec![
                Span::styled("End Date: ", Style::default().fg(Color::Yellow)),
                Span::raw(basho.end_date.as_deref().map(format_date).unwrap_or_else(|| "Unknown".to_string())),
            ]),
        ];

        if let Some(yusho_list) = &basho.yusho {
            text.push(Line::from(""));
            text.push(Line::from(vec![
                Span::styled("Yusho Winners:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]));
            
            for yusho in yusho_list {
                text.push(Line::from(vec![
                    Span::styled("  Division: ", Style::default().fg(Color::Green)),
                    Span::raw(&yusho.division),
                ]));
                text.push(Line::from(vec![
                    Span::styled("  Winner: ", Style::default().fg(Color::Green)),
                    Span::raw(&yusho.shikona_en),
                ]));
                text.push(Line::from(""));
            }
        }

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Basho Information"))
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new("Loading basho information...")
            .block(Block::default().borders(Borders::ALL).title("Basho Information"))
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    }
}

fn render_help_popup(f: &mut Frame) {
    let area = centered_rect(60, 50, f.area());
    f.render_widget(Clear, area);

    let help_text = vec![
        Line::from("Sumo TUI Help"),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  ↑/↓     - Navigate lists"),
        Line::from("  1       - View daily matches (torikumi)"),
        Line::from("  2       - View rankings (banzuke)"),
        Line::from("  3       - View basho information"),
        Line::from("  h/F1    - Toggle this help"),
        Line::from("  q       - Quit application"),
        Line::from("  Esc     - Close help"),
        Line::from(""),
        Line::from("Command line options:"),
        Line::from("  --basho YYYYMM    - Specify basho"),
        Line::from("  --day N           - Specify day (1-15)"),
        Line::from("  --division DIV    - Specify division"),
        Line::from("  --banzuke         - Start in banzuke view"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    Ok(terminal)
}

pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
