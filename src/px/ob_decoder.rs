use base64::{engine::general_purpose::STANDARD, Engine};
use regex::Regex;
use std::collections::HashMap;

pub fn decode_ob(ob: &str, xor_key: u8) -> anyhow::Result<String> {
    let pad = (4 - ob.len() % 4) % 4;
    let padded = format!("{}{}", ob, "=".repeat(pad));
    let raw = STANDARD.decode(&padded)?;
    let plain: Vec<u8> = raw.iter().map(|b| b ^ xor_key).collect();
    Ok(String::from_utf8_lossy(&plain).into_owned())
}

pub fn parse_commands(decoded: &str) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    for segment in decoded.split("~~~~") {
        let parts: Vec<&str> = segment.splitn(64, '|').collect();
        if parts.is_empty() || parts[0].is_empty() {
            continue;
        }
        let label = parts[0].to_string();
        let params: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
        map.insert(label, params);
    }
    map
}

pub fn extract_cookies(decoded: &str) -> HashMap<String, String> {
    let mut cookies = HashMap::new();
    for segment in decoded.split("~~~~") {
        let parts: Vec<&str> = segment.splitn(8, '|').collect();
        if parts.len() < 4 {
            continue;
        }
        let name = parts[1];
        let value = parts[3];
        if name.starts_with("_px") || name == "pxcts" {
            cookies.insert(name.to_string(), value.to_string());
        }
    }
    cookies
}

pub const ROLE_SID: &str = "sid";
pub const ROLE_VID: &str = "vid";
pub const ROLE_CTS: &str = "cts";
pub const ROLE_CS: &str = "cs";
pub const ROLE_TIMESTAMP: &str = "timestamp";
pub const ROLE_TOKEN: &str = "token";

pub fn auto_map(decoded: &str) -> HashMap<&'static str, String> {
    let cmds = parse_commands(decoded);
    let mut result: HashMap<&'static str, String> = HashMap::new();

    let uuid_re =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();
    let hex64_re = Regex::new(r"^[0-9a-f]{64}$").unwrap();
    let digits20_re = Regex::new(r"^\d{18,22}$").unwrap();
    let digits13_re = Regex::new(r"^\d{12,14}$").unwrap();
    let token_re = Regex::new(r"^[a-z0-9]{15,25}$").unwrap();

    for params in cmds.values() {
        if params.is_empty() {
            continue;
        }
        let val = &params[0];

        if val.starts_with("_px") || val == "pxcts" {
            continue;
        }

        if uuid_re.is_match(val) {
            if params.len() >= 2 && (params[1] == "false" || params[1] == "true") {
                result.entry(ROLE_CTS).or_insert_with(|| val.clone());
            } else if params.len() >= 2
                && (params[1] == "31536000"
                    || params[1].contains("3153")
                    || params[1]
                        .parse::<u64>()
                        .map(|n| n <= 31536000)
                        .unwrap_or(false))
            {
                result.entry(ROLE_VID).or_insert_with(|| val.clone());
            } else if params.len() == 1 {
                result.entry(ROLE_SID).or_insert_with(|| val.clone());
            } else {
                result.entry(ROLE_SID).or_insert_with(|| val.clone());
            }
            continue;
        }

        if hex64_re.is_match(val) {
            result.entry(ROLE_CS).or_insert_with(|| val.clone());
            continue;
        }

        if digits13_re.is_match(val) && params.len() == 1 {
            result.entry(ROLE_TIMESTAMP).or_insert_with(|| val.clone());
            continue;
        }

        if digits20_re.is_match(val) {
            continue;
        }

        if params.len() == 1 && token_re.is_match(val) {
            result.entry(ROLE_TOKEN).or_insert_with(|| val.clone());
        }
    }

    result
}
