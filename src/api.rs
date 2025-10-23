use serde::{Deserialize, Serialize};
use chrono::Datelike;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Basho {
    pub date: Option<String>,
    pub location: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
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

    /// Get the current basho ID based on today's date.
    ///
    /// This is deterministic and does not probe the network. It selects the most
    /// recent scheduled basho month relative to the current month using the
    /// standard basho months: Jan, Mar, May, Jul, Sep, Nov.
    pub async fn get_current_basho_id(&self) -> String {
        let now = chrono::Utc::now();
        let (year, month) = (now.year(), now.month());
        let (by, bm) = most_recent_basho_ym(year, month);
        format!("{}{:02}", by, bm)
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
        // Parse basho year and month from basho_id (YYYYMM)
        let now = chrono::Utc::now().naive_utc().date();
        let (ny, nm) = (now.year(), now.month());

        let (by, bm) = if basho_id.len() >= 6 {
            let y = basho_id[0..4].parse::<i32>().unwrap_or(ny);
            let m = basho_id[4..6].parse::<u32>().unwrap_or(nm);
            (y, m)
        } else {
            (ny, nm)
        };

        // If the selected basho month is in the past relative to 'now', it's finished => day 15.
        if (by, bm) < (ny, nm) {
            return Ok(15);
        }

        // If the selected basho month is in the future, it's not started => day 1.
        if (by, bm) > (ny, nm) {
            return Ok(1);
        }

        // Same month: try to use API start date; if that fails, approximate as second Sunday.
        match self.get_basho(basho_id).await {
            Ok(basho) => {
                if let Some(s) = basho.start_date.as_deref() {
                    if s.len() >= 10 {
                        if let Ok(start_date) = chrono::NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d") {
                            let days_since_start = (now - start_date).num_days();
                            let day = if days_since_start < 0 {
                                1
                            } else if days_since_start > 14 {
                                15
                            } else {
                                (days_since_start + 1) as u8
                            };
                            return Ok(day);
                        }
                    }
                }
                // Fall through to approximation if parsing failed or missing data
            }
            Err(_) => {
                // Fall through to approximation on API failure
            }
        }

        // Approximate: basho typically starts on the second Sunday of the month and lasts 15 days.
        let approx_start = approximate_basho_start(by, bm).unwrap_or_else(|| {
            // Fallback: if approximation somehow fails, use the 10th as a rough midpoint
            chrono::NaiveDate::from_ymd_opt(by, bm, 10).unwrap()
        });
        let days_since_start = (now - approx_start).num_days();
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

/// Compute the most recent basho (year, month) for a given year and month.
/// Basho months are fixed: 1, 3, 5, 7, 9, 11.
fn most_recent_basho_ym(year: i32, month: u32) -> (i32, u32) {
    // Fast path when month is one of the basho months
    match month {
        1 | 3 | 5 | 7 | 9 | 11 => return (year, month),
        _ => {}
    }

    // Otherwise, pick the greatest basho month <= current month
    let candidates = [1u32, 3, 5, 7, 9, 11];
    if let Some(&m) = candidates.iter().filter(|&&m| m <= month).max() {
        (year, m)
    } else {
        // This should never happen for real calendar months, but keep a safe fallback
        (year - 1, 11)
    }
}

/// Approximate the basho start date as the second Sunday of a given month.
fn approximate_basho_start(year: i32, month: u32) -> Option<chrono::NaiveDate> {
    let first = chrono::NaiveDate::from_ymd_opt(year, month, 1)?;
    let first_weekday_from_sun = first.weekday().num_days_from_sunday(); // 0..=6
    let days_to_first_sunday = (7 - first_weekday_from_sun) % 7; // 0..=6
    let first_sunday_day = 1 + days_to_first_sunday as u32;
    let second_sunday_day = first_sunday_day + 7;
    chrono::NaiveDate::from_ymd_opt(year, month, second_sunday_day)
}

#[cfg(test)]
mod tests {
    use super::{most_recent_basho_ym, approximate_basho_start};

    #[test]
    fn october_maps_to_september() {
        assert_eq!(most_recent_basho_ym(2025, 10), (2025, 9));
    }

    #[test]
    fn december_maps_to_november() {
        assert_eq!(most_recent_basho_ym(2025, 12), (2025, 11));
    }

    #[test]
    fn february_maps_to_january() {
        assert_eq!(most_recent_basho_ym(2025, 2), (2025, 1));
    }

    #[test]
    fn january_stays_january() {
        assert_eq!(most_recent_basho_ym(2025, 1), (2025, 1));
    }

    #[test]
    fn march_stays_march() {
        assert_eq!(most_recent_basho_ym(2025, 3), (2025, 3));
    }

    #[test]
    fn approximate_second_sunday() {
        // For September 2025, the first is Monday (2025-09-01), Sundays are 7,14,21,28 -> second is 14
        let d = approximate_basho_start(2025, 9).unwrap();
        assert_eq!(d.to_string(), "2025-09-14");
    }
}
