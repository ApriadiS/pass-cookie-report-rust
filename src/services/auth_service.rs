use crate::errors::DebugAppError;
use reqwest::header::{COOKIE, SET_COOKIE};
use scraper::{Html, Selector};
use std::env;
use tracing::info;

pub struct AuthService;

impl AuthService {
    pub async fn perform_login(username: &str, password: &str) -> Result<String, DebugAppError> {
        let base_url = env::var("API_BASE_URL")
            .map_err(|_| DebugAppError::DateParse("API_BASE_URL not set".to_string()))?;

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        // Step 1: GET login page
        let login_url = format!("{}/login", base_url);
        info!("[AUTH] Getting login page: {}", login_url);
        
        let response = client.get(&login_url).send().await?;
        
        // Extract cookies before consuming response
        let headers = response.headers().clone();
        let html_content = response.text().await?;

        // Extract cookies from Set-Cookie headers
        let mut xsrf_token = String::new();
        let mut session_token = String::new();
        
        for cookie_header in headers.get_all(SET_COOKIE) {
            let cookie_str = cookie_header.to_str().unwrap_or("");
            if cookie_str.contains("XSRF-TOKEN=") {
                xsrf_token = Self::extract_cookie_value(cookie_str, "XSRF-TOKEN");
            } else if cookie_str.contains("new_kasir_v2_session=") {
                session_token = Self::extract_cookie_value(cookie_str, "new_kasir_v2_session");
            }
        }

        info!("[AUTH] Extracted XSRF-TOKEN and new_kasir_v2_session");
        info!("[AUTH] Extracted XSRF-TOKEN: {}",&xsrf_token);
        info!("[AUTH] Extracted new_kasir_v2_session: {}",&session_token);

        // Step 2: Parse HTML to get _token
        let csrf_token = {
            let document = Html::parse_document(&html_content);
            let token_selector = Selector::parse("input[name='_token']")
                .map_err(|_| DebugAppError::Serialization("Invalid CSS selector".to_string()))?;
            
            document
                .select(&token_selector)
                .next()
                .and_then(|element| element.value().attr("value"))
                .map(|s| s.to_string())
                .ok_or_else(|| DebugAppError::Serialization("CSRF token not found".to_string()))?
        };

        info!("[AUTH] Extracted CSRF token and cookies");
        info!("[AUTH] _token: {}",&csrf_token.as_str());

        // Step 3: POST login
        let login_data = [
            ("_token", csrf_token.as_str()),
            ("username", username),
            ("password", password),
        ];


        let cookie_header = format!("XSRF-TOKEN={}; new_kasir_v2_session={}", xsrf_token, session_token);
        
        let login_response = client
            .post(&login_url)
            .header(COOKIE, cookie_header)
            .form(&login_data)
            .send()
            .await?;

        // Step 4: Extract final cookies from login response
        let login_headers = login_response.headers();
        let mut final_cookies = Vec::new();

        info!("Header: {:?}",login_headers);
        
        for cookie_header in login_headers.get_all(SET_COOKIE) {
            let cookie_str = cookie_header.to_str().unwrap_or("");
            if cookie_str.contains("remember_web_") || 
               cookie_str.contains("XSRF-TOKEN=") || 
               cookie_str.contains("new_kasir_v2_session=") {
                let cookie_part = cookie_str.split(';').next().unwrap_or("");
                if !cookie_part.is_empty() {
                    final_cookies.push(cookie_part.to_string());
                }
            }
        }

        if final_cookies.is_empty() {
            return Err(DebugAppError::Unauthorized("Login failed - no cookies received".to_string()));
        }

        let final_cookie = final_cookies.join("; ");
        info!("[AUTH] Login successful, got {} cookies", final_cookies.len());
        info!("[AUTH] Final cookies: {}", &final_cookie);
        
        Ok(final_cookie)
    }

    fn extract_cookie_value(cookie_str: &str, cookie_name: &str) -> String {
        cookie_str
            .split(';')
            .find(|part| part.trim().starts_with(cookie_name))
            .and_then(|part| part.split('=').nth(1))
            .unwrap_or("")
            .to_string()
    }
}