use crate::http::PlainHttp;
use crate::types::{CreateError, SmsRental};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const BASE: &str = "https://getatext.com/api/v1";

#[derive(Clone)]
pub struct Getatext {
    client: PlainHttp,
    api_key: String,
    service: String,
}

#[derive(Serialize)]
struct RentReq<'a> {
    service: &'a str,
}

#[derive(Serialize)]
struct IdReq {
    id: i64,
}

#[derive(Deserialize, Debug, Clone)]
struct RentResponse {
    #[serde(default)]
    id: Option<i64>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    errors: Option<serde_json::Value>,
    #[serde(default)]
    number: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    price: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
struct StatusResponse {
    #[serde(default)]
    code: Option<String>,
}

impl Getatext {
    pub fn new(api_key: String, service: String) -> Self {
        Self {
            client: PlainHttp::new(),
            api_key,
            service,
        }
    }

    pub async fn rent(&self) -> Result<SmsRental, CreateError> {
        let resp: RentResponse = self
            .client
            .post(&format!("{BASE}/rent-a-number"))
            .header("Auth", &self.api_key)
            .json(&RentReq {
                service: &self.service,
            })
            .send()
            .await
            .context("getatext /rent-a-number")
            .map_err(CreateError::Upstream)?
            .json()
            .await
            .context("getatext /rent-a-number parse")
            .map_err(CreateError::Upstream)?;
        if resp.id.is_none() {
            let err_str = resp
                .errors
                .as_ref()
                .and_then(|e| e.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if err_str.contains("out of stock") {
                return Err(CreateError::PhoneOutOfStock);
            }
            if err_str.contains("insufficient funds") {
                return Err(CreateError::InsufficientFunds);
            }
            return Err(CreateError::Upstream(anyhow::anyhow!(
                "getatext rent failed: id=None message={:?} errors={:?}",
                resp.message,
                resp.errors
            )));
        }
        Ok(SmsRental {
            id: resp.id.unwrap(),
            number: resp.number.unwrap_or_default(),
        })
    }

    pub async fn poll_code(
        &self,
        rental: &SmsRental,
        timeout: Duration,
        interval: Duration,
    ) -> Result<String> {
        let deadline = std::time::Instant::now() + timeout;
        let mut attempt = 0u32;
        loop {
            if std::time::Instant::now() > deadline {
                anyhow::bail!("getatext poll timed out for rental {}", rental.id);
            }
            attempt += 1;
            let resp: StatusResponse = self
                .client
                .post(&format!("{BASE}/rental-status"))
                .header("Auth", &self.api_key)
                .json(&IdReq { id: rental.id })
                .send()
                .await?
                .json()
                .await?;
            if let Some(code) = resp.code.as_deref() {
                if !code.trim().is_empty() {
                    tracing::info!(attempt, "OTP received");
                    return Ok(code.trim().to_string());
                }
            }
            tokio::time::sleep(interval).await;
        }
    }

    pub async fn mark_completed(&self, rental: &SmsRental) {
        if let Ok(resp) = self
            .client
            .post(&format!("{BASE}/rental-status/{}/completed", rental.id))
            .header("Auth", &self.api_key)
            .json(&serde_json::json!({}))
            .send()
            .await
        {
            let _: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
        }
    }

    pub async fn cancel(&self, rental: &SmsRental) {
        if let Ok(resp) = self
            .client
            .post(&format!("{BASE}/cancel-rental"))
            .header("Auth", &self.api_key)
            .json(&IdReq { id: rental.id })
            .send()
            .await
        {
            let _: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
        }
    }
}
