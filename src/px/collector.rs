use crate::USER_AGENT;
use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;

const COLLECTOR_URL: &str = "https://collector-pxu6b0qd2s.px-cloud.net/api/v2/collector";
const INIT_JS_URL: &str = "https://www.walmart.com/px/PXu6b0qd2S/init.js";
const FALLBACK_TAG: &str = "eW5CaD8AUB99Zg==";

#[derive(Deserialize, Debug)]
pub struct CollectorResponse {
    #[serde(default)]
    pub ob: String,
}

pub async fn fetch_tag(client: &wreq::Client) -> Result<String> {
    let resp = client
        .get(INIT_JS_URL)
        .header("user-agent", USER_AGENT)
        .send()
        .await
        .context("fetch init.js")?;

    let body = resp.text().await.context("init.js body")?;
    let re = Regex::new(r#"gv\s*=\s*"([^"]+)""#).unwrap();
    if let Some(cap) = re.captures(&body) {
        let tag = cap[1].to_string();
        tracing::debug!(tag = %tag, "extracted PX tag from init.js");
        return Ok(tag);
    }

    tracing::warn!("could not extract PX tag from init.js, using fallback");
    Ok(FALLBACK_TAG.to_string())
}

pub async fn post_collector(
    client: &wreq::Client,
    fields: Vec<(String, String)>,
    origin: &str,
) -> Result<CollectorResponse> {
    let body = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(fields.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .finish();

    let resp = client
        .post(COLLECTOR_URL)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("origin", origin)
        .header("sec-fetch-site", "cross-site")
        .header("sec-fetch-mode", "cors")
        .header("accept", "*/*")
        .header("user-agent", USER_AGENT)
        .body(body)
        .send()
        .await
        .context("collector POST")?;

    let status = resp.status();
    let raw = resp.text().await.context("collector response body")?;
    tracing::debug!(status = %status, raw = %&raw[..raw.len().min(500)], "collector raw response");
    if !status.is_success() {
        anyhow::bail!(
            "collector returned {status}: {}",
            &raw[..raw.len().min(200)]
        );
    }

    serde_json::from_str::<CollectorResponse>(&raw).context("collector response JSON")
}
