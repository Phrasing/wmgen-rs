use anyhow::Context;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub getatext_api_key: String,
    pub getatext_service: String,
    pub pvacodes_api_key: Option<String>,
    pub pvacodes_app: String,
    pub sms_provider: String,
    pub no_proxy: bool,
    pub wait_on_oos: bool,
    pub emails_path: PathBuf,
    pub proxies_path: PathBuf,
    pub output_path: PathBuf,
}

impl Config {
    pub fn from_env_and_args(
        emails_path: PathBuf,
        proxies_path: PathBuf,
        output_path: PathBuf,
    ) -> anyhow::Result<Self> {
        fn req(k: &str) -> anyhow::Result<String> {
            std::env::var(k).with_context(|| format!("missing required env var {k}"))
        }
        fn opt(k: &str, default: &str) -> String {
            std::env::var(k).unwrap_or_else(|_| default.to_string())
        }
        fn opt_var(k: &str) -> Option<String> {
            std::env::var(k).ok().filter(|v| !v.is_empty())
        }

        let sms_provider = opt("SMS_PROVIDER", "getatext");
        let pvacodes_api_key = opt_var("PVACODES_API_KEY");
        let no_proxy = std::env::var("SKIP_PROXY")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let wait_on_oos = std::env::var("WAIT_ON_OOS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        if sms_provider == "pvacodes" && pvacodes_api_key.is_none() {
            anyhow::bail!("SMS_PROVIDER=pvacodes requires PVACODES_API_KEY to be set");
        }

        Ok(Self {
            getatext_api_key: req("GETATEXT_API_KEY")?,
            getatext_service: opt("GETATEXT_SERVICE", "walmart"),
            pvacodes_api_key,
            pvacodes_app: opt("PVACODES_APP", "Walmart"),
            sms_provider,
            no_proxy,
            wait_on_oos,
            emails_path,
            proxies_path,
            output_path,
        })
    }
}
