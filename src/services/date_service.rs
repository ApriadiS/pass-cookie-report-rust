#![allow(dead_code)]

use chrono::NaiveDate;
use tracing::info;

pub struct DateService;

impl DateService {
    /// Flexible date parsing - tries multiple formats
    pub fn parse_date(date_str: &str) -> Result<NaiveDate, chrono::ParseError> {
        // Try different date formats
        let formats = [
            "%d/%m/%Y",    // DD/MM/YYYY
            "%Y-%m-%d",    // YYYY-MM-DD
            "%m/%d/%Y",    // MM/DD/YYYY
            "%d-%m-%Y",    // DD-MM-YYYY
            "%Y/%m/%d",    // YYYY/MM/DD
        ];
        
        for format in &formats {
            if let Ok(date) = NaiveDate::parse_from_str(date_str, format) {
                info!("[DATE] Parsed '{}' using format '{}'", date_str, format);
                return Ok(date);
            }
        }
        
        // If all formats fail, return error from last attempt
        NaiveDate::parse_from_str(date_str, "%d/%m/%Y")
    }

    /// Convert any supported date format to DD/MM/YYYY for API (original format)
    pub fn normalize_date_for_api(date_str: &str) -> Result<String, chrono::ParseError> {
        let date = Self::parse_date(date_str)?;
        let normalized = date.format("%d/%m/%Y").to_string();
        info!("[DATE] Normalized '{}' to '{}' for API", date_str, normalized);
        Ok(normalized)
    }

    pub fn get_date_range(from: &str, to: &str) -> Result<Vec<String>, chrono::ParseError> {
        let start_date = Self::parse_date(from)?;
        let end_date = Self::parse_date(to)?;
        
        let mut dates = Vec::new();
        let mut current = start_date;
        
        while current <= end_date {
            // Keep original format for internal use
            dates.push(current.format("%d/%m/%Y").to_string());
            current = current.succ_opt().unwrap_or(current);
        }
        
        Ok(dates)
    }

    pub fn format_for_api(date_str: &str) -> Result<String, chrono::ParseError> {
        let normalized = Self::normalize_date_for_api(date_str)?;
        Ok(urlencoding::encode(&normalized).to_string())
    }
}