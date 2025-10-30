mod api;
mod cli;
mod tui;

use clap::Parser;
use api::SumoApi;
use cli::Args;
use tui::{App, AppView, setup_terminal, restore_terminal};
use crossterm::event::{self, Event};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use chrono::{Datelike, Utc};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    // Initialize API client
    let api = SumoApi::new();
    
    // Determine basho ID
    let basho_id = if let Some(basho) = args.basho {
        basho
    } else {
        api.get_current_basho_id().await
    };
    
    // Determine day
    let day = if let Some(day) = args.day {
        day
    } else {
        api.get_current_day(&basho_id).await.unwrap_or(1)
    };
    
    let division = args.division.to_string();
    
    // Create app
    let mut app = App::new(basho_id.clone(), division.clone(), day);
    
    // Set initial view based on args
    if args.banzuke {
        app.current_view = AppView::Banzuke;
    }
    
    // Load initial data before setting up terminal
    match load_data(&api, &basho_id, &division, day, &mut app, true).await {
        Ok(_) => {
            // Data loaded successfully, continue
        },
        Err(e) => {
            eprintln!("Error loading data: {}", e);
            eprintln!("Please check your internet connection and try again.");
            eprintln!("You can also try specifying a different basho with --basho YYYYMM");
            std::process::exit(1);
        }
    }
    
    // Setup terminal after data is loaded
    let mut terminal = setup_terminal()?;
    
    // Run the app with async support for reloading
    let result = run_app_with_reload(&mut terminal, app, api).await;
    
    // Restore terminal
    restore_terminal(&mut terminal)?;
    
    if let Err(err) = result {
        eprintln!("Error running app: {}", err);
        std::process::exit(1);
    }
    
    Ok(())
}

async fn load_data(
    api: &SumoApi,
    basho_id: &str,
    division: &str,
    day: u8,
    app: &mut App,
    log_to_stderr: bool,
) -> anyhow::Result<()> {
    if log_to_stderr {
        eprintln!(
            "Loading data for basho {} division {} (requested day {})...",
            basho_id,
            division,
            day
        );
    }

    let max_day_allowed = max_day_for_division(division);
    let original_day = day;
    let mut resolved_day = original_day.clamp(1, max_day_allowed);
    let today = Utc::now().date_naive();

    // Clear existing torikumi data to avoid showing stale bouts while reloading
    app.clear_torikumi();

    let mut skip_torikumi = false;

    // Load basho info
    match api.get_basho(basho_id).await {
        Ok(basho) => {
            if log_to_stderr {
                eprintln!("✓ Loaded basho information");
            }

            let start_date = basho.start_date_naive();
            let end_date = basho
                .end_date
                .as_deref()
                .and_then(|s| s.split('T').next())
                .and_then(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok());
            let basho_ym = parse_basho_year_month(basho_id);

            let mut is_future = start_date.map(|s| today < s).unwrap_or(false);
            let mut is_finished = end_date.map(|e| today > e).unwrap_or(false);

            if let Some((by, bm)) = basho_ym {
                if !is_future && !is_finished {
                    let now_tuple = (today.year(), today.month());
                    let basho_tuple = (by, bm);
                    if basho_tuple > now_tuple {
                        is_future = true;
                    } else if basho_tuple < now_tuple {
                        is_finished = true;
                    }
                }
            }

            if is_future {
                skip_torikumi = true;
                if app.basho_changed {
                    resolved_day = 1;
                }
                if log_to_stderr {
                    eprintln!(
                        "ℹ️ Basho {} has not started yet; torikumi will remain empty.",
                        basho_id
                    );
                }
            } else if app.basho_changed && is_finished {
                resolved_day = max_day_allowed;
            }

            app.set_basho(basho);
        },
        Err(e) => {
            if log_to_stderr {
                eprintln!("⚠ Warning: Could not load basho info: {}", e);
            }
        }
    }

    if resolved_day != original_day && log_to_stderr {
        eprintln!(
            "ℹ️ Adjusted requested day {} to {} based on tournament status.",
            original_day,
            resolved_day
        );
    }

    if app.day != resolved_day {
        app.day = resolved_day;
    }

    // Load torikumi (daily matches)
    if skip_torikumi {
        app.set_torikumi(Vec::new());
        if log_to_stderr {
            eprintln!("ℹ️ Skipping torikumi fetch for upcoming basho {}.", basho_id);
        }
    } else {
        match api.get_torikumi(basho_id, division, resolved_day).await {
            Ok(torikumi) => {
                if let Some(matches) = torikumi.torikumi {
                    if log_to_stderr {
                        eprintln!("✓ Loaded {} matches for day {}", matches.len(), resolved_day);
                    }
                    app.set_torikumi(matches);
                } else {
                    if log_to_stderr {
                        eprintln!("⚠ No matches found for day {}", resolved_day);
                    }
                    app.set_torikumi(Vec::new());
                }
            },
            Err(e) => {
                if log_to_stderr {
                    eprintln!("⚠ Warning: Could not load torikumi: {}", e);
                }
                app.set_torikumi(Vec::new());
            }
        }
    }
    
    // Load banzuke (rankings)
    match api.get_banzuke(basho_id, division).await {
        Ok(banzuke_response) => {
            // Sort and interleave east and west wrestlers by rank
            let mut all_entries = Vec::new();
            
            // Group by rank_value
            use std::collections::BTreeMap;
            let mut by_rank: BTreeMap<u32, (Option<api::BanzukeEntry>, Option<api::BanzukeEntry>)> = BTreeMap::new();
            
            for entry in banzuke_response.east {
                let rank = entry.rank_value;
                by_rank.entry(rank).or_insert((None, None)).0 = Some(entry);
            }
            
            for entry in banzuke_response.west {
                let rank = entry.rank_value;
                by_rank.entry(rank).or_insert((None, None)).1 = Some(entry);
            }
            
            // Add entries in order: east first, then west for each rank
            for (_rank_value, (east, west)) in by_rank {
                if let Some(e) = east {
                    all_entries.push(e);
                }
                if let Some(w) = west {
                    all_entries.push(w);
                }
            }
            
            if log_to_stderr {
                eprintln!("✓ Loaded {} wrestlers in banzuke", all_entries.len());
            }
            app.set_banzuke(all_entries);
        },
        Err(e) => {
            if log_to_stderr {
                eprintln!("⚠ Warning: Could not load banzuke: {}", e);
            }
        }
    }
    
    if log_to_stderr {
        eprintln!("Data loading completed. Starting TUI...");
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    app.basho_changed = false;
    Ok(())
}

fn max_day_for_division(division: &str) -> u8 {
    let normalized = division.to_ascii_lowercase();
    match normalized.as_str() {
        "makuuchi" | "juryo" => 15,
        _ => 7,
    }
}

fn parse_basho_year_month(basho_id: &str) -> Option<(i32, u32)> {
    if basho_id.len() < 6 {
        return None;
    }
    let year = basho_id[0..4].parse().ok()?;
    let month = basho_id[4..6].parse().ok()?;
    Some((year, month))
}

async fn run_app_with_reload(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    api: SumoApi,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| tui::ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.on_key(key.code);
            }
        }

        if app.should_quit {
            break;
        }

        // Check if we need to reload data
        if app.needs_reload {
            app.needs_reload = false;
            
            // Store values before borrowing mutably
            let basho_id = app.basho_id.clone();
            let division = app.division.clone();
            let requested_day = app.day;

            app.status_message = None;
            let overlay_message = format!("Reloading data for {} {}...", basho_id, division);
            app.loading_overlay = Some(overlay_message);

            terminal.draw(|f| tui::ui(f, &mut app))?;

            match load_data(&api, &basho_id, &division, requested_day, &mut app, false).await {
                Ok(_) => {
                    let active_day = app.day;
                    if active_day != requested_day {
                        app.status_message = Some(format!(
                            "Reloaded {} {} Day {} (auto-selected)",
                            basho_id, division, active_day
                        ));
                    } else {
                        app.status_message = Some(format!(
                            "Reloaded {} {} Day {}",
                            basho_id, division, active_day
                        ));
                    }
                }
                Err(e) => {
                    let msg = format!("Failed to reload data: {}", e);
                    app.status_message = Some(msg.clone());
                    eprintln!("{}", msg);
                }
            }

            app.loading_overlay = None;
        }

        // Check if we need to load rikishi details
        if let Some(rikishi_id) = app.requested_rikishi_id.take() {
            match api.get_rikishi(rikishi_id).await {
                Ok(details) => {
                    app.rikishi_details = Some(details);
                    app.show_rikishi_details = true;
                },
                Err(e) => {
                    eprintln!("Error loading rikishi details: {}", e);
                }
            }
        }

        // Check if we need to load head-to-head data
        if let Some((rikishi_id, opponent_id)) = app.requested_head_to_head.take() {
            match api.get_head_to_head(rikishi_id, opponent_id).await {
                Ok(h2h) => {
                    app.head_to_head_data = Some(h2h);
                    app.show_head_to_head = true;
                },
                Err(e) => {
                    eprintln!("Error loading head-to-head data: {}", e);
                }
            }
        }
    }

    Ok(())
}
