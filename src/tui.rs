use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Table, Row, Cell},
    Frame, Terminal,
};
use std::io;
use crate::api::{Basho, BanzukeEntry, TorikumiEntry, RikishiDetails, HeadToHeadResponse};
use std::collections::HashMap;

const DIVISIONS: &[&str] = &["Makuuchi", "Juryo", "Makushita", "Sandanme", "Jonidan", "Jonokuchi"];

#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    EditingDay,
    SelectingDivision,
    EditingBasho,
}

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
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub needs_reload: bool,
    pub division_selector_index: usize,
    pub show_rikishi_details: bool,
    pub rikishi_details: Option<RikishiDetails>,
    pub requested_rikishi_id: Option<u32>,
    pub show_head_to_head: bool,
    pub head_to_head_data: Option<HeadToHeadResponse>,
    pub requested_head_to_head: Option<(u32, u32)>, // (rikishi_id, opponent_id)
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
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            needs_reload: false,
            division_selector_index: 0,
            show_rikishi_details: false,
            rikishi_details: None,
            requested_rikishi_id: None,
            show_head_to_head: false,
            head_to_head_data: None,
            requested_head_to_head: None,
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
        // Handle input mode first
        match self.input_mode {
            InputMode::Normal => {
                match key {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char('h') | KeyCode::F(1) => self.show_help = !self.show_help,
                    KeyCode::Char('c') => {
                        self.input_mode = InputMode::EditingDay;
                        self.input_buffer.clear();
                    },
                    KeyCode::Char('v') => {
                        self.input_mode = InputMode::SelectingDivision;
                        // Find current division index
                        self.division_selector_index = DIVISIONS.iter()
                            .position(|&d| d == self.division)
                            .unwrap_or(0);
                    },
                    KeyCode::Char('b') => {
                        self.input_mode = InputMode::EditingBasho;
                        self.input_buffer.clear();
                    },
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
                    // Page navigation with a/d and left/right arrows
                    KeyCode::Char('a') | KeyCode::Left => {
                        match self.current_view {
                            AppView::Torikumi => {
                                // Already at first page, do nothing
                            },
                            AppView::Banzuke => {
                                self.current_view = AppView::Torikumi;
                                self.selected_index = 0;
                                self.scroll_offset = 0;
                            },
                            AppView::BashoInfo => {
                                self.current_view = AppView::Banzuke;
                                self.selected_index = 0;
                                self.scroll_offset = 0;
                            },
                        }
                    },
                    KeyCode::Char('d') | KeyCode::Right => {
                        match self.current_view {
                            AppView::Torikumi => {
                                self.current_view = AppView::Banzuke;
                                self.selected_index = 0;
                                self.scroll_offset = 0;
                            },
                            AppView::Banzuke => {
                                self.current_view = AppView::BashoInfo;
                                self.selected_index = 0;
                                self.scroll_offset = 0;
                            },
                            AppView::BashoInfo => {
                                // Already at last page, do nothing
                            },
                        }
                    },
                    // WASD navigation
                    KeyCode::Char('w') | KeyCode::Up => {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                            if self.selected_index < self.scroll_offset {
                                self.scroll_offset = self.selected_index;
                            }
                        }
                    }
                    KeyCode::Char('s') | KeyCode::Down => {
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
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        // If in banzuke view, show rikishi details
                        if self.current_view == AppView::Banzuke {
                            if let Some(banzuke) = &self.banzuke {
                                if self.selected_index < banzuke.len() {
                                    let rikishi_id = banzuke[self.selected_index].rikishi_id;
                                    self.requested_rikishi_id = Some(rikishi_id);
                                }
                            }
                        }
                        // If in torikumi view, show head-to-head
                        else if self.current_view == AppView::Torikumi {
                            if let Some(torikumi) = &self.torikumi {
                                if self.selected_index < torikumi.len() {
                                    let match_entry = &torikumi[self.selected_index];
                                    let east_id = match_entry.east_id;
                                    let west_id = match_entry.west_id;
                                    self.requested_head_to_head = Some((east_id, west_id));
                                }
                            }
                        }
                    }
                    KeyCode::Esc => {
                        if self.show_rikishi_details {
                            self.show_rikishi_details = false;
                            self.rikishi_details = None;
                        } else if self.show_head_to_head {
                            self.show_head_to_head = false;
                            self.head_to_head_data = None;
                        } else {
                            self.show_help = false;
                        }
                    }
                    _ => {}
                }
            },
            InputMode::EditingDay => {
                match key {
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        self.input_buffer.push(c);
                    },
                    KeyCode::Backspace => {
                        self.input_buffer.pop();
                    },
                    KeyCode::Enter => {
                        if let Ok(day) = self.input_buffer.parse::<u8>() {
                            if day >= 1 && day <= 15 {
                                self.day = day;
                                self.needs_reload = true;
                            }
                        }
                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                    },
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                    },
                    _ => {}
                }
            },
            InputMode::SelectingDivision => {
                match key {
                    KeyCode::Up => {
                        if self.division_selector_index > 0 {
                            self.division_selector_index -= 1;
                        }
                    },
                    KeyCode::Down => {
                        if self.division_selector_index + 1 < DIVISIONS.len() {
                            self.division_selector_index += 1;
                        }
                    },
                    KeyCode::Enter => {
                        self.division = DIVISIONS[self.division_selector_index].to_string();
                        self.needs_reload = true;
                        self.input_mode = InputMode::Normal;
                    },
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                    },
                    _ => {}
                }
            },
            InputMode::EditingBasho => {
                match key {
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        if self.input_buffer.len() < 6 {
                            self.input_buffer.push(c);
                        }
                    },
                    KeyCode::Backspace => {
                        self.input_buffer.pop();
                    },
                    KeyCode::Enter => {
                        if self.input_buffer.len() == 6 {
                            if let Ok(year) = self.input_buffer[0..4].parse::<i32>() {
                                if let Ok(month) = self.input_buffer[4..6].parse::<u32>() {
                                    // Valid basho months are 1, 3, 5, 7, 9, 11
                                    if year >= 2000 && year <= 2100 && [1, 3, 5, 7, 9, 11].contains(&month) {
                                        self.basho_id = self.input_buffer.clone();
                                        self.needs_reload = true;
                                    }
                                }
                            }
                        }
                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                    },
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                    },
                    _ => {}
                }
            },
        }
    }
}

pub fn ui(f: &mut Frame, app: &mut App) {
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
    let footer_text = "q: Quit | 1: Torikumi | 2: Banzuke | 3: Info | c: Day | v: Division | b: Basho | h: Help";
    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(footer, chunks[2]);

    // Help popup
    if app.show_help {
        render_help_popup(f);
    }
    
    // Input popups
    match app.input_mode {
        InputMode::EditingDay => render_input_popup(f, "Day (1-15)", &app.input_buffer),
        InputMode::SelectingDivision => render_division_selector(f, app.division_selector_index),
        InputMode::EditingBasho => render_input_popup(f, "Basho (YYYYMM, e.g., 202501)", &app.input_buffer),
        InputMode::Normal => {},
    }
    
    // Rikishi details popup
    if app.show_rikishi_details {
        if let Some(details) = &app.rikishi_details {
            render_rikishi_details(f, details);
        }
    }
    
    // Head-to-head popup
    if app.show_head_to_head {
        if let Some(h2h) = &app.head_to_head_data {
            render_head_to_head(f, h2h);
        }
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
            // Line::from(vec![
            //     Span::styled("Location: ", Style::default().fg(Color::Yellow)),
            //     Span::raw(basho.location.as_deref().unwrap_or("Unknown")),
            // ]), TODO: Fix unknown location
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
    let area = centered_rect(70, 60, f.area());
    f.render_widget(Clear, area);

    let help_text = vec![
        Line::from("Sumo TUI Help"),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  ↑/↓/w/s     - Navigate lists"),
        Line::from("  ←/→/a/d     - Switch between pages"),
        Line::from("  Enter       - View details (rikishi in banzuke, head-to-head in torikumi)"),
        Line::from("  1           - View daily matches (torikumi)"),
        Line::from("  2           - View rankings (banzuke)"),
        Line::from("  3           - View basho information"),
        Line::from(""),
        Line::from("Switch Data:"),
        Line::from("  c       - Change day (1-15)"),
        Line::from("  v       - Change division"),
        Line::from("  b       - Change basho (YYYYMM format)"),
        Line::from(""),
        Line::from("Other:"),
        Line::from("  h/F1    - Toggle this help"),
        Line::from("  q       - Quit application"),
        Line::from("  Esc     - Close help/cancel input/close details"),
        Line::from(""),
        Line::from("Divisions: Makuuchi, Juryo, Makushita, Sandanme, Jonidan, Jonokuchi"),
        Line::from("Basho months: 01, 03, 05, 07, 09, 11"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_input_popup(f: &mut Frame, prompt: &str, input: &str) {
    let area = centered_rect(50, 20, f.area());
    f.render_widget(Clear, area);

    let text = vec![
        Line::from(prompt),
        Line::from(""),
        Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Green)),
            Span::raw(input),
            Span::styled("_", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from("Press Enter to confirm, Esc to cancel"),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Input"))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn render_division_selector(f: &mut Frame, selected_index: usize) {
    let area = centered_rect(50, 50, f.area());
    f.render_widget(Clear, area);

    let mut text = vec![
        Line::from("Select Division"),
        Line::from(""),
    ];

    for (i, division) in DIVISIONS.iter().enumerate() {
        let line = if i == selected_index {
            Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(*division, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ])
        } else {
            Line::from(vec![
                Span::raw("  "),
                Span::raw(*division),
            ])
        };
        text.push(line);
    }

    text.push(Line::from(""));
    text.push(Line::from("Use ↑↓ to select, Enter to confirm, Esc to cancel"));

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Division"))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn render_rikishi_details(f: &mut Frame, details: &RikishiDetails) {
    let area = centered_rect(70, 70, f.area());
    f.render_widget(Clear, area);

    // Helper function to format date
    let format_date = |date_str: &str| -> String {
        if let Some(date_part) = date_str.split('T').next() {
            date_part.to_string()
        } else {
            date_str.to_string()
        }
    };

    // Calculate age from birth date
    let age_str = if let Some(birth_date) = &details.birth_date {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(&birth_date[..10], "%Y-%m-%d") {
            let now = chrono::Utc::now().date_naive();
            let age = now.years_since(date).unwrap_or(0);
            format!(" (Age: {})", age)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let mut text = vec![
        Line::from(vec![
            Span::styled("Rikishi Details", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Shikona (English): ", Style::default().fg(Color::Green)),
            Span::raw(&details.shikona_en),
        ]),
        Line::from(vec![
            Span::styled("Shikona (Japanese): ", Style::default().fg(Color::Green)),
            Span::raw(&details.shikona_jp),
        ]),
        Line::from(""),
    ];

    if let Some(rank) = &details.current_rank {
        text.push(Line::from(vec![
            Span::styled("Current Rank: ", Style::default().fg(Color::Cyan)),
            Span::raw(rank),
        ]));
    }

    if let Some(heya) = &details.heya {
        text.push(Line::from(vec![
            Span::styled("Heya: ", Style::default().fg(Color::Cyan)),
            Span::raw(heya),
        ]));
    }

    text.push(Line::from(""));

    if let Some(birth_date) = &details.birth_date {
        text.push(Line::from(vec![
            Span::styled("Birth Date: ", Style::default().fg(Color::Magenta)),
            Span::raw(format_date(birth_date)),
            Span::raw(age_str),
        ]));
    }

    if let Some(shusshin) = &details.shusshin {
        text.push(Line::from(vec![
            Span::styled("Birthplace: ", Style::default().fg(Color::Magenta)),
            Span::raw(shusshin),
        ]));
    }

    text.push(Line::from(""));

    if let Some(height) = details.height {
        // Convert cm to feet and inches
        let total_inches = (height as f64) / 2.54;
        let feet = (total_inches / 12.0).floor() as u32;
        let inches = (total_inches % 12.0).round() as u32;
        
        text.push(Line::from(vec![
            Span::styled("Height: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{} cm ({}' {}\")", height, feet, inches)),
        ]));
    }

    if let Some(weight) = details.weight {
        // Convert kg to lbs
        let lbs = ((weight as f64) * 2.20462).round() as u32;
        
        text.push(Line::from(vec![
            Span::styled("Weight: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{} kg ({} lbs)", weight, lbs)),
        ]));
    }

    text.push(Line::from(""));

    if let Some(debut) = &details.debut {
        // Format debut as YYYY-MM
        let debut_formatted = if debut.len() == 6 {
            format!("{}-{}", &debut[0..4], &debut[4..6])
        } else {
            debut.clone()
        };
        text.push(Line::from(vec![
            Span::styled("Debut: ", Style::default().fg(Color::Green)),
            Span::raw(debut_formatted),
        ]));
    }

    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("Press Esc to close", Style::default().fg(Color::Cyan).add_modifier(Modifier::ITALIC)),
    ]));

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Rikishi Information"))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_head_to_head(f: &mut Frame, h2h: &HeadToHeadResponse) {
    let area = centered_rect(80, 80, f.area());
    f.render_widget(Clear, area);

    let mut text = vec![
        Line::from(vec![
            Span::styled("Head-to-Head Record", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];

    // Overall record
    if !h2h.matches.is_empty() {
        let first_match = &h2h.matches[0];
        let rikishi_name = if first_match.winner_id == Some(first_match.east_id) || first_match.east_id != first_match.east_id {
            &first_match.east_shikona
        } else {
            &first_match.east_shikona
        };
        let opponent_name = if first_match.east_id == first_match.east_id {
            &first_match.west_shikona
        } else {
            &first_match.west_shikona
        };

        text.push(Line::from(vec![
            Span::styled("Total Matches: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", h2h.total)),
        ]));
        text.push(Line::from(vec![
            Span::styled(format!("{} Wins: ", rikishi_name), Style::default().fg(Color::Green)),
            Span::raw(format!("{}", h2h.rikishi_wins)),
        ]));
        text.push(Line::from(vec![
            Span::styled(format!("{} Wins: ", opponent_name), Style::default().fg(Color::Red)),
            Span::raw(format!("{}", h2h.opponent_wins)),
        ]));
        text.push(Line::from(""));
    }

    // Kimarite wins
    if let Some(wins) = &h2h.kimarite_wins {
        if !wins.is_empty() {
            text.push(Line::from(vec![
                Span::styled("Winning Techniques:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]));
            for (technique, count) in wins {
                // Capitalize first letter
                let capitalized = if !technique.is_empty() {
                    let mut chars: Vec<char> = technique.chars().collect();
                    chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                    chars.into_iter().collect()
                } else {
                    technique.clone()
                };
                
                text.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(capitalized, Style::default().fg(Color::Green)),
                    Span::raw(format!(": {}", count)),
                ]));
            }
            text.push(Line::from(""));
        }
    }

    // Kimarite losses
    if let Some(losses) = &h2h.kimarite_losses {
        if !losses.is_empty() {
            text.push(Line::from(vec![
                Span::styled("Losing Techniques:", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            ]));
            for (technique, count) in losses {
                // Capitalize first letter
                let capitalized = if !technique.is_empty() {
                    let mut chars: Vec<char> = technique.chars().collect();
                    chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                    chars.into_iter().collect()
                } else {
                    technique.clone()
                };
                
                text.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(capitalized, Style::default().fg(Color::Red)),
                    Span::raw(format!(": {}", count)),
                ]));
            }
            text.push(Line::from(""));
        }
    }

    // Match history (show most recent 10)
    text.push(Line::from(vec![
        Span::styled("Recent Matches:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]));
    text.push(Line::from(""));

    for (i, match_entry) in h2h.matches.iter().take(10).enumerate() {
        let basho_date = crate::api::SumoApi::format_basho_date(&match_entry.basho_id);
        let winner = match_entry.winner_en.as_deref().unwrap_or("N/A");
        let kimarite_raw = match_entry.kimarite.as_deref().unwrap_or("N/A");
        
        // Capitalize first letter of kimarite
        let kimarite = if !kimarite_raw.is_empty() {
            let mut chars: Vec<char> = kimarite_raw.chars().collect();
            chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
            chars.into_iter().collect()
        } else {
            kimarite_raw.to_string()
        };

        text.push(Line::from(vec![
            Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{} Day {}: ", basho_date, match_entry.day)),
            Span::styled(winner, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" by "),
            Span::styled(kimarite, Style::default().fg(Color::Cyan)),
        ]));
    }

    if h2h.matches.len() > 10 {
        text.push(Line::from(""));
        text.push(Line::from(vec![
            Span::styled(format!("... and {} more", h2h.matches.len() - 10), Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
    }

    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("Press Esc to close", Style::default().fg(Color::Cyan).add_modifier(Modifier::ITALIC)),
    ]));

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Match History"))
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
