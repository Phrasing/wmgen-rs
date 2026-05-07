use crate::generators::{correlation_id, device_profile_ref_id, span_id, trace_id};
use crate::types::Proxy;
use crate::walmart::queries;
use crate::USER_AGENT;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use wreq::cookie::{CookieStore, Cookies, Jar};
use wreq::redirect::Policy;
use wreq_util::{Emulation, Profile};

pub struct WalmartSession {
    client: wreq::Client,
    pub jar: Arc<Jar>,

    pub device_profile_ref_id: String,

    pub challenge: String,

    pub pkce_verifier: String,

    pub page_url: String,

    pub platform_version: String,
}

const UA: &str = USER_AGENT;
const SEC_CH_UA: &str = r#""Google Chrome";v="147", "Not.A/Brand";v="8", "Chromium";v="147""#;

impl WalmartSession {
    pub fn new(proxy: Option<&Proxy>) -> Result<Self> {
        let jar = Arc::new(Jar::default());

        let emulation = Emulation::builder()
            .profile(Profile::Chrome147)
            .headers(false)
            .build();
        let mut builder = wreq::Client::builder()
            .emulation(emulation)
            .cookie_provider(jar.clone())
            .redirect(Policy::none());
        if let Some(p) = proxy {
            builder = builder.proxy(wreq::Proxy::all(p.http_url()).context("build proxy")?);
        }
        let client = builder.build().context("build wreq client")?;
        let (pkce_verifier, challenge) = crate::generators::pkce_pair();
        let page_url = login_url(&challenge);
        Ok(Self {
            client,
            jar,
            device_profile_ref_id: device_profile_ref_id(),
            challenge,
            pkce_verifier,
            page_url,
            platform_version: queries::PLATFORM_VERSION.to_string(),
        })
    }

    pub fn http(&self) -> &wreq::Client {
        &self.client
    }

    pub fn inject_px_cookies_raw(
        &self,
        px3: &str,
        pxvid: &str,
        pxcts: &str,
        pxde: &str,
    ) -> Result<()> {
        for url in ["https://www.walmart.com/", "https://identity.walmart.com/"] {
            if !px3.is_empty() {
                self.jar.add(
                    format!("_px3={px3}; Path=/; Domain=.walmart.com").as_str(),
                    url,
                );
            }
            if !pxvid.is_empty() {
                self.jar.add(
                    format!("_pxvid={pxvid}; Path=/; Domain=.walmart.com").as_str(),
                    url,
                );
            }
            if !pxcts.is_empty() {
                self.jar.add(
                    format!("pxcts={pxcts}; Path=/; Domain=.walmart.com").as_str(),
                    url,
                );
            }
            if !pxde.is_empty() {
                self.jar.add(
                    format!("_pxde={pxde}; Path=/; Domain=.walmart.com").as_str(),
                    url,
                );
            }
        }
        Ok(())
    }

    pub fn cookie_snapshot(&self) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for url in ["https://www.walmart.com/", "https://identity.walmart.com/"] {
            let uri: wreq::Uri = match url.parse() {
                Ok(u) => u,
                Err(_) => continue,
            };
            match self.jar.cookies(&uri, wreq::Version::HTTP_2) {
                Cookies::Compressed(hv) => {
                    if let Ok(s) = hv.to_str() {
                        parse_cookie_header_into(s, &mut out);
                    }
                }
                Cookies::Uncompressed(headers) => {
                    for hv in headers {
                        if let Ok(s) = hv.to_str() {
                            parse_cookie_header_into(s, &mut out);
                        }
                    }
                }
                Cookies::Empty => {}
                _ => {}
            }
        }
        out
    }

    pub fn graphql_headers(
        &self,
        op_name: &'static str,
        gql_query_label: &'static str,
    ) -> wreq::header::HeaderMap {
        let cookie_value =
            self.current_cookie_header_for("https://identity.walmart.com/orchestra/idp/graphql");
        build_graphql_headers(
            op_name,
            gql_query_label,
            &self.device_profile_ref_id,
            &self.page_url,
            &self.platform_version,
            cookie_value.as_deref(),
        )
    }

    pub fn current_cookie_header_for(&self, target_url: &str) -> Option<String> {
        let uri: wreq::Uri = target_url.parse().ok()?;
        match self.jar.cookies(&uri, wreq::Version::HTTP_2) {
            Cookies::Compressed(hv) => hv.to_str().ok().map(|s| s.to_string()),
            Cookies::Uncompressed(values) => {
                let parts: Vec<String> = values
                    .iter()
                    .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
                    .collect();
                if parts.is_empty() {
                    None
                } else {
                    Some(parts.join("; "))
                }
            }
            _ => None,
        }
    }

    pub fn page_headers(&self, referer: Option<&str>) -> wreq::header::HeaderMap {
        use wreq::header::{HeaderMap, HeaderName, HeaderValue};
        let mut h = HeaderMap::new();
        let put = |h: &mut HeaderMap, name: &str, value: &str| {
            if let (Ok(n), Ok(v)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                h.insert(n, v);
            }
        };
        put(&mut h, "sec-ch-ua", SEC_CH_UA);
        put(&mut h, "sec-ch-ua-mobile", "?0");
        put(&mut h, "sec-ch-ua-platform", "\"Windows\"");
        put(&mut h, "upgrade-insecure-requests", "1");
        put(&mut h, "user-agent", UA);
        put(&mut h, "accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7");
        put(
            &mut h,
            "sec-fetch-site",
            if referer.is_some() {
                "same-site"
            } else {
                "none"
            },
        );
        put(&mut h, "sec-fetch-mode", "navigate");
        put(&mut h, "sec-fetch-user", "?1");
        put(&mut h, "sec-fetch-dest", "document");
        if let Some(r) = referer {
            put(&mut h, "referer", r);
        }
        put(&mut h, "accept-language", "en-US,en;q=0.9");
        h
    }
}

pub fn build_graphql_headers(
    op_name: &str,
    gql_query_label: &str,
    device_profile_ref_id: &str,
    page_url: &str,
    platform_version: &str,
    cookie_value: Option<&str>,
) -> wreq::header::HeaderMap {
    use wreq::header::{HeaderMap, HeaderName, HeaderValue};
    let mut h = HeaderMap::new();
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let trace = trace_id();
    let span = span_id();
    let corr = correlation_id();

    let put = |h: &mut HeaderMap, name: &str, value: String| {
        if let (Ok(n), Ok(v)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(&value),
        ) {
            h.insert(n, v);
        }
    };

    put(&mut h, "x-o-mart", "B2C".into());
    put(&mut h, "x-o-gql-query", gql_query_label.to_string());
    put(&mut h, "sec-ch-ua-platform", "\"Windows\"".into());
    put(&mut h, "x-o-segment", "oaoh".into());
    put(
        &mut h,
        "device_profile_ref_id",
        device_profile_ref_id.to_string(),
    );
    put(&mut h, "sec-ch-ua", SEC_CH_UA.into());
    put(&mut h, "x-enable-server-timing", "1".into());
    put(&mut h, "sec-ch-ua-mobile", "?0".into());
    put(
        &mut h,
        "baggage",
        format!("requestTs={now_ms},tpid=00-{trace}-{span}-00"),
    );
    put(&mut h, "x-latency-trace", "1".into());
    put(&mut h, "traceparent", format!("00-{trace}-{span}-00"));
    put(&mut h, "wm_mp", "true".into());
    put(&mut h, "accept", "application/json".into());
    put(&mut h, "content-type", "application/json".into());
    put(&mut h, "x-apollo-operation-name", op_name.to_string());
    put(&mut h, "tenant-id", queries::TENANT_ID.into());
    put(&mut h, "downlink", "10".into());
    put(&mut h, "wm_qos.correlation_id", corr.clone());
    put(&mut h, "x-o-platform", "rweb".into());
    put(&mut h, "x-o-platform-version", platform_version.to_string());
    put(&mut h, "accept-language", "en-US".into());
    put(&mut h, "x-o-ccm", "server".into());
    put(&mut h, "x-o-bu", "WALMART-US".into());
    put(&mut h, "dpr", "1.5".into());
    put(&mut h, "user-agent", UA.into());
    put(&mut h, "wm_page_url", page_url.to_string());
    put(&mut h, "x-o-correlation-id", corr);
    put(&mut h, "origin", "https://identity.walmart.com".into());
    put(&mut h, "sec-fetch-site", "same-origin".into());
    put(&mut h, "sec-fetch-mode", "cors".into());
    put(&mut h, "sec-fetch-dest", "empty".into());
    put(&mut h, "referer", page_url.to_string());
    put(&mut h, "accept-encoding", "gzip, deflate, br, zstd".into());

    if let Some(c) = cookie_value {
        put(&mut h, "cookie", c.to_string());
    }
    put(&mut h, "priority", "u=1, i".into());
    h
}

pub fn login_url(challenge: &str) -> String {
    format!(
        "https://identity.walmart.com/account/login?client_id={}&redirect_uri=https%3A%2F%2Fwww.walmart.com%2Faccount%2FverifyToken&scope=openid%20email%20offline_access&tenant_id={}&state=%2F&code_challenge={}",
        queries::CLIENT_ID, queries::TENANT_ID, challenge
    )
}

pub fn signup_page_url(challenge: &str) -> String {
    format!(
        "https://identity.walmart.com/account/signup?scope=openid%20email%20offline_access&redirect_uri=https%3A%2F%2Fwww.walmart.com%2Faccount%2FverifyToken&client_id={}&tenant_id={}&code_challenge={}&state=%2F",
        queries::CLIENT_ID, queries::TENANT_ID, challenge
    )
}

pub fn phone_otp_choice_url(challenge: &str) -> String {
    format!(
        "https://identity.walmart.com/account/phone-otp-choice/sign-up?scope=openid%20email%20offline_access&redirect_uri=https%3A%2F%2Fwww.walmart.com%2Faccount%2FverifyToken&client_id={}&tenant_id={}&code_challenge={}&state=%2F",
        queries::CLIENT_ID, queries::TENANT_ID, challenge
    )
}

pub fn verify_account_url(challenge: &str) -> String {
    format!(
        "https://identity.walmart.com/account/verifyyouraccount?scope=openid%20email%20offline_access&redirect_uri=https%3A%2F%2Fwww.walmart.com%2Faccount%2FverifyToken&client_id={}&tenant_id={}&code_challenge={}&state=%2F",
        queries::CLIENT_ID, queries::TENANT_ID, challenge
    )
}

fn parse_cookie_header_into(header: &str, out: &mut HashMap<String, String>) {
    for kv in header.split(';') {
        if let Some((k, v)) = kv.trim().split_once('=') {
            out.entry(k.trim().to_string())
                .or_insert_with(|| v.trim().to_string());
        }
    }
}

pub fn extract_errors(body: &Value, op_path: &[&str]) -> Option<String> {
    let mut node = body.get("data")?;
    for k in op_path {
        node = node.get(*k)?;
    }
    let errs = node.get("errors")?;
    if let Some(arr) = errs.as_array() {
        if arr.is_empty() {
            return None;
        }
        let s = arr
            .iter()
            .filter_map(|e| {
                let code = e.get("code").and_then(Value::as_str).unwrap_or("");
                let msg = e.get("message").and_then(Value::as_str).unwrap_or("");
                if code.is_empty() && msg.is_empty() {
                    None
                } else {
                    Some(format!("{code}: {msg}"))
                }
            })
            .collect::<Vec<_>>()
            .join("; ");
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    } else {
        None
    }
}
