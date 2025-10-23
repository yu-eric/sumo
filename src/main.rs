mod api;
mod cli;
mod tui;

use clap::Parser;
use api::SumoApi;
use cli::Args;
use tui::{App, AppView, setup_terminal, restore_terminal, run_app};

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
    match load_data(&api, &basho_id, &division, day, &mut app).await {
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
    
    // Run the app
    let result = run_app(&mut terminal, app);
    
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
) -> anyhow::Result<()> {
    // Print to stderr so it doesn't interfere with TUI
    eprintln!("Loading data for basho {} division {} day {}...", basho_id, division, day);
    
    // Load basho info
    match api.get_basho(basho_id).await {
        Ok(basho) => {
            eprintln!("✓ Loaded basho information");
            app.set_basho(basho);
        },
        Err(e) => {
            eprintln!("⚠ Warning: Could not load basho info: {}", e);
        }
    }
    
    // Load torikumi (daily matches)
    match api.get_torikumi(basho_id, division, day).await {
        Ok(torikumi) => {
            if let Some(matches) = torikumi.torikumi {
                eprintln!("✓ Loaded {} matches for day {}", matches.len(), day);
                app.set_torikumi(matches);
            } else {
                eprintln!("⚠ No matches found for day {}", day);
            }
        },
        Err(e) => {
            eprintln!("⚠ Warning: Could not load torikumi: {}", e);
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
            
            eprintln!("✓ Loaded {} wrestlers in banzuke", all_entries.len());
            app.set_banzuke(all_entries);
        },
        Err(e) => {
            eprintln!("⚠ Warning: Could not load banzuke: {}", e);
        }
    }
    
    eprintln!("Data loading completed. Starting TUI...");
    // Give the terminal a moment to clear
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    Ok(())
}
