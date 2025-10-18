#![allow(dead_code)]

use chrono::NaiveDate;

pub struct DateService;

impl DateService {
    pub fn parse_date(date_str: &str) -> Result<NaiveDate, chrono::ParseError> {
        // Parse DD/MM/YYYY format
        NaiveDate::parse_from_str(date_str, "%d/%m/%Y")
    }

    pub fn get_date_range(from: &str, to: &str) -> Result<Vec<String>, chrono::ParseError> {
        let start_date = Self::parse_date(from)?;
        let end_date = Self::parse_date(to)?;
        
        let mut dates = Vec::new();
        let mut current = start_date;
        
        while current <= end_date {
            dates.push(current.format("%d/%m/%Y").to_string());
            current = current.succ_opt().unwrap_or(current);
        }
        
        Ok(dates)
    }

    pub fn format_for_api(date_str: &str) -> Result<String, chrono::ParseError> {
        let date = Self::parse_date(date_str)?;
        Ok(urlencoding::encode(&date.format("%d/%m/%Y").to_string()).to_string())
    }
}