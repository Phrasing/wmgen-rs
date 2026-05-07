use crate::http::PlainHttp;
use crate::types::{CreateError, SmsRental};
use anyhow::Result;
use serde::Deserialize;
use std::time::Duration;

const BASE: &str = "https://beta.pvacodes.com/app/api.php";

#[derive(Clone)]
pub struct Pvacodes {
    client: PlainHttp,
    api_key: String,
    app: String,
}

#[derive(Deserialize)]
struct PvaStatus {
    code: String,
}

#[derive(Deserialize)]
struct PvaNumberResp {
    status: PvaStatus,
    data: Option<String>,
    id: Option<i64>,
}

#[derive(Deserialize)]
struct PvaSmsResp {
    status: PvaStatus,
    data: Option<String>,
}

impl Pvacodes {
    pub fn new(api_key: String, app: String) -> Self {
        Self {
            client: PlainHttp::new(),
            api_key,
            app,
        }
    }

    pub async fn rent(&self) -> Result<SmsRental, CreateError> {
        let url = format!(
            "{BASE}?do=get_number&country=USA&app={}&key={}",
            self.app, self.api_key
        );
        let resp: PvaNumberResp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CreateError::Upstream(anyhow::anyhow!("pvacodes get_number: {e}")))?
            .json()
            .await
            .map_err(|e| {
                CreateError::Upstream(anyhow::anyhow!("pvacodes get_number parse: {e}"))
            })?;

        if resp.status.code != "1000" {
            if resp.status.code == "2000"
                || resp.status.code == "1003"
                || resp.status.code == "1004"
            {
                return Err(CreateError::PhoneOutOfStock);
            }
            return Err(CreateError::Upstream(anyhow::anyhow!(
                "pvacodes get_number error code {}",
                resp.status.code
            )));
        }

        let id = resp.id.ok_or_else(|| {
            CreateError::Upstream(anyhow::anyhow!("pvacodes get_number: missing id"))
        })?;
        let number = resp.data.unwrap_or_default();
        Ok(SmsRental { id, number })
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
                anyhow::bail!("pvacodes poll timed out for number {}", rental.number);
            }
            attempt += 1;
            let url = format!(
                "{BASE}?do=get_sms&country=USA&app={}&number={}&key={}",
                self.app, rental.number, self.api_key
            );
            let resp: PvaSmsResp = self.client.get(&url).send().await?.json().await?;
            if resp.status.code == "1000" {
                if let Some(code) = resp.data.as_deref() {
                    let code = code.trim();
                    if !code.is_empty() {
                        tracing::info!(attempt, "OTP received");
                        return Ok(code.to_string());
                    }
                }
            }
            tokio::time::sleep(interval).await;
        }
    }

    pub async fn mark_completed(&self, _rental: &SmsRental) {}

    pub async fn cancel(&self, rental: &SmsRental) {
        let url = format!(
            "{BASE}?do=cancel_number&number_id={}&key={}",
            rental.id, self.api_key
        );
        if let Ok(resp) = self.client.get(&url).send().await {
            let _: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
        }
    }
}
