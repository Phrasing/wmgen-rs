use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Proxy {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
}

impl Proxy {
    pub fn parse_line(line: &str) -> anyhow::Result<Self> {
        let line = line.trim();
        if let Some(at) = line.rfind('@') {
            let credentials = &line[..at];
            let hostport = &line[at + 1..];
            let (user, pass) = credentials.split_once(':').ok_or_else(|| {
                anyhow::anyhow!("expected user:pass before @, got {credentials:?}")
            })?;
            let (host, port_str) = hostport
                .rsplit_once(':')
                .ok_or_else(|| anyhow::anyhow!("expected host:port after @, got {hostport:?}"))?;
            Ok(Self {
                host: host.to_string(),
                port: port_str.parse()?,
                user: user.to_string(),
                pass: pass.to_string(),
            })
        } else {
            let parts: Vec<&str> = line.splitn(4, ':').collect();
            if parts.len() != 4 {
                anyhow::bail!("expected host:port:user:pass or user:pass@host:port, got {line:?}");
            }
            Ok(Self {
                host: parts[0].to_string(),
                port: parts[1].parse()?,
                user: parts[2].to_string(),
                pass: parts[3].to_string(),
            })
        }
    }

    pub fn http_url(&self) -> String {
        format!(
            "http://{}:{}@{}:{}",
            self.user, self.pass, self.host, self.port
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedAccount {
    pub email: String,
    pub password: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub proxy: String,
    pub auth_cookie: Option<String>,
    pub cid: Option<String>,
    pub spid: Option<String>,
    pub all_cookies: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct SmsRental {
    pub id: i64,

    pub number: String,
}

#[derive(thiserror::Error, Debug)]
pub enum CreateError {
    #[error("account already exists for {0}")]
    AccountExists(String),
    #[error("Walmart returned errors: {0}")]
    WalmartErrors(String),
    #[error("PX block / 403 from Walmart on {endpoint}")]
    PxBlocked { endpoint: String },
    #[error("email OTP timeout for {0}")]
    EmailOtpTimeout(String),
    #[error("SMS OTP timeout for rental {0}")]
    SmsOtpTimeout(i64),
    #[error("verifyToken redirect missing auth cookies")]
    NoAuthCookies,
    #[error("phone number service out of stock")]
    PhoneOutOfStock,
    #[error("SMS provider has insufficient funds")]
    InsufficientFunds,
    #[error("upstream: {0}")]
    Upstream(#[from] anyhow::Error),
}
