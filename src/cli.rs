use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Basho ID in YYYYMM format (e.g., 202401 for January 2024)
    #[arg(short, long)]
    pub basho: Option<String>,

    /// Day of the basho (1-15)
    #[arg(short, long)]
    pub day: Option<u8>,

    /// Division to show
    #[arg(long, default_value = "makuuchi")]
    pub division: Division,

    /// Show banzuke instead of daily results
    #[arg(long)]
    pub banzuke: bool,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum Division {
    Makuuchi,
    Juryo,
    Makushita,
    Sandanme,
    Jonidan,
    Jonokuchi,
}

impl std::fmt::Display for Division {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Division::Makuuchi => write!(f, "Makuuchi"),
            Division::Juryo => write!(f, "Juryo"),
            Division::Makushita => write!(f, "Makushita"),
            Division::Sandanme => write!(f, "Sandanme"),
            Division::Jonidan => write!(f, "Jonidan"),
            Division::Jonokuchi => write!(f, "Jonokuchi"),
        }
    }
}
