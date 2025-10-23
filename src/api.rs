use serde::{Deserialize, Serialize};
use chrono::Datelike;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Basho {
    pub date: String,
    pub location: String,
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "endDate")]
    pub end_date: String,
    pub yusho: Option<Vec<YushoEntry>>,
    pub sansho: Option<Vec<SanshoEntry>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct YushoEntry {
    #[serde(rename = "type")]
    pub division: String,
    #[serde(rename = "rikishiId")]
    pub rikishi_id: u32,
    #[serde(rename = "shikonaEn")]
    pub shikona_en: String,
    #[serde(rename = "shikonaJp")]
    pub shikona_jp: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SanshoEntry {
    #[serde(rename = "type")]
    pub award_type: String,
    #[serde(rename = "rikishiId")]
    pub rikishi_id: u32,
    #[serde(rename = "shikonaEn")]
    pub shikona_en: String,
    #[serde(rename = "shikonaJp")]
    pub shikona_jp: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BanzukeResponse {
    #[serde(rename = "bashoId")]
    pub basho_id: String,
    pub division: String,
    pub east: Vec<BanzukeEntry>,
    pub west: Vec<BanzukeEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BanzukeEntry {
    pub side: String,
    #[serde(rename = "rikishiID")]
    pub rikishi_id: u32,
    #[serde(rename = "shikonaEn")]
    pub shikona_en: String,
    #[serde(rename = "rankValue")]
    pub rank_value: u32,
    pub rank: String,
    pub record: Option<Vec<MatchRecord>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MatchRecord {
    pub result: String,
    #[serde(rename = "opponentShikonaEn")]
    pub opponent_shikona_en: String,
    #[serde(rename = "opponentShikonaJp")]
    pub opponent_shikona_jp: String,
    pub kimarite: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TorikumiResponse {
    pub date: String,
    pub location: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "endDate")]
    pub end_date: String,
    pub torikumi: Option<Vec<TorikumiEntry>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TorikumiEntry {
    pub id: String,
    #[serde(rename = "bashoId")]
    pub basho_id: String,
    pub division: String,
    pub day: u8,
    #[serde(rename = "matchNo")]
    pub match_no: u8,
    #[serde(rename = "eastId")]
    pub east_id: u32,
    #[serde(rename = "eastShikona")]
    pub east_shikona: String,
    #[serde(rename = "eastRank")]
    pub east_rank: String,
    #[serde(rename = "westId")]
    pub west_id: u32,
    #[serde(rename = "westShikona")]
    pub west_shikona: String,
    #[serde(rename = "westRank")]
    pub west_rank: String,
    pub kimarite: Option<String>,
    #[serde(rename = "winnerId")]
    pub winner_id: Option<u32>,
    #[serde(rename = "winnerEn")]
    pub winner_en: Option<String>,
    #[serde(rename = "winnerJp")]
    pub winner_jp: Option<String>,
}

pub struct SumoApi {
    client: reqwest::Client,
    base_url: String,
}

impl SumoApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://www.sumo-api.com".to_string(),
        }
    }

    pub async fn get_basho(&self, basho_id: &str) -> anyhow::Result<Basho> {
        let url = format!("{}/api/basho/{}", self.base_url, basho_id);
        let response = self.client.get(&url).send().await?;
        let basho = response.json::<Basho>().await?;
        Ok(basho)
    }

    pub async fn get_banzuke(&self, basho_id: &str, division: &str) -> anyhow::Result<BanzukeResponse> {
        let url = format!("{}/api/basho/{}/banzuke/{}", self.base_url, basho_id, division);
        let response = self.client.get(&url).send().await?;
        let banzuke = response.json::<BanzukeResponse>().await?;
        Ok(banzuke)
    }

    pub async fn get_torikumi(&self, basho_id: &str, division: &str, day: u8) -> anyhow::Result<TorikumiResponse> {
        let url = format!("{}/api/basho/{}/torikumi/{}/{}", self.base_url, basho_id, division, day);
        let response = self.client.get(&url).send().await?;
        let torikumi = response.json::<TorikumiResponse>().await?;
        Ok(torikumi)
    }

    /// Get the current basho ID based on today's date
    pub async fn get_current_basho_id(&self) -> String {
        let now = chrono::Utc::now();
        let mut year = now.year();
        let current_month = now.month();
        
        // Sumo basho months: January (01), March (03), May (05), July (07), September (09), November (11)
        let basho_months = [11, 9, 7, 5, 3, 1];
        
        // Find the most recent basho month (go back to the most recent odd month)
        let basho_month = if basho_months.contains(&current_month) {
            // Current month is a basho month
            current_month
        } else {
            // Find the most recent basho month before current month
            if let Some(&month) = basho_months.iter().find(|&&m| m < current_month) {
                month
            } else {
                // If no basho month found before current month, use November of previous year
                year -= 1;
                11
            }
        };
        
        // Try the calculated basho
        let basho_id = format!("{}{:02}", year, basho_month);
        
        // Verify it exists
        match self.get_basho(&basho_id).await {
            Ok(basho) => {
                if !basho.date.is_empty() && basho.date != "0001-01-01T00:00:00Z" && basho.date != "" {
                    return basho_id;
                }
            }
            Err(_) => {}
        }
        
        // If it doesn't exist, try previous bashos from current year
        for &month in basho_months.iter() {
            let test_id = format!("{}{:02}", year, month);
            if test_id == basho_id {
                continue;
            }
            
            match self.get_basho(&test_id).await {
                Ok(basho) => {
                    if !basho.date.is_empty() && basho.date != "0001-01-01T00:00:00Z" && basho.date != "" {
                        return test_id;
                    }
                }
                Err(_) => continue,
            }
        }
        
        // Try previous year
        year -= 1;
        for &month in basho_months.iter() {
            let test_id = format!("{}{:02}", year, month);
            
            match self.get_basho(&test_id).await {
                Ok(basho) => {
                    if !basho.date.is_empty() && basho.date != "0001-01-01T00:00:00Z" && basho.date != "" {
                        return test_id;
                    }
                }
                Err(_) => continue,
            }
        }
        
        // Fallback to a known good basho
        "202509".to_string()
    }

    /// Get the basho name from the month
    pub fn get_basho_name(month: u32) -> &'static str {
        match month {
            1 => "Hatsu Basho",
            3 => "Haru Basho", 
            5 => "Natsu Basho",
            7 => "Nagoya Basho",
            9 => "Aki Basho",
            11 => "Kyushu Basho",
            _ => "Unknown Basho",
        }
    }

    /// Format basho ID as human readable date
    pub fn format_basho_date(basho_id: &str) -> String {
        if basho_id.len() != 6 {
            return basho_id.to_string();
        }
        
        let year: u32 = basho_id[0..4].parse().unwrap_or(0);
        let month: u32 = basho_id[4..6].parse().unwrap_or(0);
        
        let month_name = match month {
            1 => "January",
            3 => "March",
            5 => "May", 
            7 => "July",
            9 => "September",
            11 => "November",
            _ => "Unknown",
        };
        
        format!("{} {}", month_name, year)
    }

    /// Get the current day of the basho (1-15)
    pub async fn get_current_day(&self, basho_id: &str) -> anyhow::Result<u8> {
        let basho = self.get_basho(basho_id).await?;
        let start_date = chrono::NaiveDate::parse_from_str(&basho.start_date[..10], "%Y-%m-%d")?;
        let now = chrono::Utc::now().naive_utc().date();
        
        let days_since_start = (now - start_date).num_days();
        
        // Basho runs for 15 days
        let day = if days_since_start < 0 {
            1
        } else if days_since_start > 14 {
            15
        } else {
            (days_since_start + 1) as u8
        };
        
        Ok(day)
    }
}
