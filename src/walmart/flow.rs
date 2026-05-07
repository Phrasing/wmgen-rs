use crate::config::Config;
use crate::generators;
use crate::px::PxSession;
use crate::sms::{RentalGuard, SmsProvider};
use crate::types::{CreateError, CreatedAccount, Proxy};
use crate::walmart::client::{
    extract_errors, phone_otp_choice_url, signup_page_url, verify_account_url, WalmartSession,
};
use crate::walmart::queries;
use anyhow::{Context, Result};
use rand::Rng;
use serde_json::{json, Value};
use std::time::Duration;

const STATE_PARAM: &str = "/";
const TEMPO_TARGETING: &str =
    "%7B%22identityClientTarget%22%3A%225f3fb121-076a-45f6-9587-249f0bc160ff%22%7D";

pub struct AccountInputs {
    pub email: String,
    pub proxy: Proxy,
}

pub async fn create_account(
    inputs: AccountInputs,
    sms: &SmsProvider,
    _cfg: &Config,
) -> Result<CreatedAccount, CreateError> {
    let email = inputs.email;
    let first_name = generators::random_first_name();
    let last_name = generators::random_last_name();
    let password = generators::random_password();

    let proxy_label = if _cfg.no_proxy {
        "none".to_string()
    } else {
        inputs.proxy.host.clone()
    };
    let _ = proxy_label;

    let proxy_ref = if _cfg.no_proxy {
        None
    } else {
        Some(&inputs.proxy)
    };
    let mut session = WalmartSession::new(proxy_ref).map_err(CreateError::from)?;

    warmup(&mut session).await.map_err(CreateError::from)?;

    tokio::time::sleep(jitter_ms(1500, 3500)).await;

    let px_session = PxSession::new(session.http().clone())
        .await
        .map_err(CreateError::from)?;

    let px_www = px_session
        .solve("https://www.walmart.com/")
        .await
        .map_err(CreateError::from)?;

    let px_identity = px_session
        .solve(&session.page_url)
        .await
        .map_err(CreateError::from)?;

    let px3 = if !px_identity.px3.is_empty() {
        &px_identity.px3
    } else {
        &px_www.px3
    };
    let pxvid = if !px_identity.pxvid.is_empty() {
        &px_identity.pxvid
    } else {
        &px_www.pxvid
    };
    let pxcts = if !px_identity.pxcts.is_empty() {
        &px_identity.pxcts
    } else {
        &px_www.pxcts
    };
    let pxde = if !px_identity.pxde.is_empty() {
        &px_identity.pxde
    } else {
        &px_www.pxde
    };

    session
        .inject_px_cookies_raw(px3, pxvid, pxcts, pxde)
        .map_err(CreateError::from)?;
    let marketing_accepted = rand::thread_rng().gen_bool(0.55);
    let remember_me = rand::thread_rng().gen_bool(0.65);

    let (auth_code, phone10) = run_signup(
        &mut session,
        SignupParams {
            email: &email,
            password: &password,
            first_name: &first_name,
            last_name: &last_name,
            marketing_accepted,
            remember_me,
            sms,
            wait_on_oos: _cfg.wait_on_oos,
        },
    )
    .await?;
    tracing::info!("verifying token");

    verify_token(&mut session, &auth_code)
        .await
        .map_err(CreateError::from)?;

    let cookies = session.cookie_snapshot();
    let auth_cookie = cookies.get("auth").cloned();
    let cid = cookies.get("CID").cloned();
    let spid = cookies.get("SPID").cloned();
    if auth_cookie.is_none() {
        return Err(CreateError::NoAuthCookies);
    }

    Ok(CreatedAccount {
        email,
        password,
        first_name,
        last_name,
        phone: phone10,
        proxy: format!("{}:{}", inputs.proxy.host, inputs.proxy.port),
        auth_cookie,
        cid,
        spid,
        all_cookies: cookies,
        created_at: chrono::Utc::now(),
    })
}

async fn warmup(session: &mut WalmartSession) -> Result<()> {
    let homepage = "https://www.walmart.com/";
    let body = session
        .http()
        .get(homepage)
        .headers(session.page_headers(None))
        .send()
        .await
        .context("warmup homepage")?
        .text()
        .await
        .unwrap_or_default();

    if let Some(ver) = extract_platform_version(&body) {
        tracing::debug!(platform_version = %ver, "parsed platform version from homepage");
        session.platform_version = ver;
    }

    {
        use crate::generators::{correlation_id, span_id, trace_id};
        use crate::walmart::queries;
        use wreq::header::{HeaderMap, HeaderName, HeaderValue};
        let mut h = HeaderMap::new();
        let put = |h: &mut HeaderMap, name: &str, value: String| {
            if let (Ok(n), Ok(v)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(&value),
            ) {
                h.insert(n, v);
            }
        };
        let trace = trace_id();
        let span = span_id();
        let corr = correlation_id();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        put(&mut h, "x-o-mart", "B2C".into());
        put(&mut h, "x-o-gql-query", "query Location".into());
        put(&mut h, "sec-ch-ua-platform", "\"Windows\"".into());
        put(&mut h, "x-o-segment", "oaoh".into());
        put(
            &mut h,
            "sec-ch-ua",
            r#""Google Chrome";v="147", "Not.A/Brand";v="8", "Chromium";v="147""#.into(),
        );
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
        put(&mut h, "x-apollo-operation-name", "Location".into());
        put(&mut h, "tenant-id", queries::TENANT_ID.into());
        put(&mut h, "downlink", "1.4".into());
        put(&mut h, "wm_qos.correlation_id", corr.clone());
        put(&mut h, "x-o-platform", "rweb".into());
        put(
            &mut h,
            "x-o-platform-version",
            session.platform_version.clone(),
        );
        put(&mut h, "accept-language", "en-US".into());
        put(&mut h, "x-o-ccm", "server".into());
        put(&mut h, "x-o-bu", "WALMART-US".into());
        put(&mut h, "dpr", "1.5".into());
        put(&mut h, "user-agent", crate::USER_AGENT.into());
        put(&mut h, "wm_page_url", homepage.into());
        put(&mut h, "x-o-correlation-id", corr);
        put(&mut h, "sec-fetch-site", "same-origin".into());
        put(&mut h, "sec-fetch-mode", "cors".into());
        put(&mut h, "sec-fetch-dest", "empty".into());
        put(&mut h, "referer", homepage.into());
        put(&mut h, "accept-encoding", "gzip, deflate, br, zstd".into());
        put(&mut h, "priority", "u=1, i".into());
        const LOCATION_HASH: &str =
            "26e4ca8c866334908ce825a5749f6aa13f772b98a527b0d0b7fe457818c5b349";
        let _ = session
            .http()
            .get(format!(
                "https://www.walmart.com/orchestra/home/graphql/Location/{LOCATION_HASH}?variables=%7B%7D"
            ))
            .headers(h)
            .send()
            .await;
    }

    tokio::spawn({
        let client = session.http().clone();
        async move {
            let _ = client
                .get("https://b.www.walmart.com/rum.js?bh=beacon.lightest.walmart.com")
                .header("accept", "*/*")
                .header("sec-fetch-site", "same-site")
                .header("sec-fetch-mode", "no-cors")
                .header("sec-fetch-dest", "script")
                .header("referer", "https://www.walmart.com/")
                .header("user-agent", crate::USER_AGENT)
                .send()
                .await;
        }
    });

    tokio::spawn({
        let client = session.http().clone();
        async move {
            crate::px::snare::post_snare(&client, "www.walmart.com", homepage, "2").await;
        }
    });

    let login_url = session.page_url.clone();
    let _ = session
        .http()
        .get(&login_url)
        .headers(session.page_headers(Some(homepage)))
        .send()
        .await
        .context("warmup login page")?
        .text()
        .await
        .ok();

    tokio::spawn({
        let client = session.http().clone();
        let url = login_url.clone();
        async move {
            crate::px::snare::post_snare(&client, "identity.walmart.com", &url, "3").await;
        }
    });

    Ok(())
}

struct SignupParams<'a> {
    email: &'a str,
    password: &'a str,
    first_name: &'a str,
    last_name: &'a str,
    marketing_accepted: bool,
    remember_me: bool,
    sms: &'a SmsProvider,
    wait_on_oos: bool,
}

async fn run_signup(
    session: &mut WalmartSession,
    params: SignupParams<'_>,
) -> Result<(String, String), CreateError> {
    let SignupParams {
        email,
        password,
        first_name,
        last_name,
        marketing_accepted,
        remember_me,
        sms,
        wait_on_oos,
    } = params;
    let challenge = session.challenge.clone();

    tokio::spawn({
        let client = session.http().clone();
        let headers = session.graphql_headers("shippingCountryList", "query shippingCountryList");
        async move {
            let body = json!({ "query": queries::SHIPPING_COUNTRY_LIST, "variables": {} });
            let _ = client
                .post("https://identity.walmart.com/orchestra/idp/graphql")
                .headers(headers)
                .body(serde_json::to_string(&body).unwrap())
                .send()
                .await;
        }
    });

    let login_opts = post_graphql(
        session,
        "GetLoginOptions",
        "query GetLoginOptions",
        queries::GET_LOGIN_OPTIONS,
        json!({
            "input": {
                "loginId": email,
                "loginIdType": "EMAIL",
                "ssoOptions": sso_options(&challenge),
            }
        }),
    )
    .await
    .map_err(CreateError::from)?;

    let pref = login_opts
        .pointer("/data/getLoginOptions/loginOptions/signInPreference")
        .and_then(Value::as_str)
        .unwrap_or("");
    if pref != "CREATE" {
        return Err(CreateError::AccountExists(email.to_string()));
    }

    let signup_url = signup_page_url(&challenge);
    let _ = session
        .http()
        .get(&signup_url)
        .headers(session.page_headers(Some(&session.page_url.clone())))
        .send()
        .await
        .context("navigate to signup page")?;
    session.page_url = signup_url.clone();
    session.device_profile_ref_id = generators::device_profile_ref_id();

    tokio::spawn({
        let client = session.http().clone();
        let headers = session.graphql_headers("getSignUpModule", "query getSignUpModule");
        async move {
            let body = json!({
                "query": queries::GET_SIGNUP_MODULE,
                "variables": {
                    "pageType": "SignUpPage",
                    "tenant": "WM_GLASS",
                    "tempo": { "targeting": TEMPO_TARGETING },
                    "isCustomModuleEnabled": false,
                }
            });
            let _ = client
                .post("https://identity.walmart.com/orchestra/idp/graphql")
                .headers(headers)
                .body(serde_json::to_string(&body).unwrap())
                .send()
                .await;
        }
    });
    tokio::spawn({
        let client = session.http().clone();
        let headers =
            session.graphql_headers("shippingCountryListV2", "query shippingCountryListV2");
        async move {
            let body = json!({
                "query": queries::SHIPPING_COUNTRY_LIST_V2,
                "variables": { "input": { "includeRegions": true } }
            });
            let _ = client
                .post("https://identity.walmart.com/orchestra/idp/graphql")
                .headers(headers)
                .body(serde_json::to_string(&body).unwrap())
                .send()
                .await;
        }
    });

    tokio::time::sleep(jitter_ms(800, 2500)).await;

    let rental = loop {
        match sms.rent().await {
            Ok(r) => break r,
            Err(CreateError::PhoneOutOfStock) if wait_on_oos => {
                tracing::warn!("SMS service out of stock — retrying in 5 s");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => return Err(e),
        }
    };
    let guard = RentalGuard::new(rental, sms.clone());
    let phone10 = strip_us_country(&guard.rental().number);
    let phone_id = phone_login_id(&phone10);
    tracing::info!(phone = %phone10, "rented number");

    let r1 = post_graphql(
        session,
        "SignUp",
        "mutation SignUp",
        queries::SIGN_UP,
        json!({
            "input": {
                "loginId": phone_id,
                "loginIdType": "PHONE",
                "password": password,
                "firstName": first_name,
                "lastName": last_name,
                "rememberMe": remember_me,
                "marketingEmailsAccepted": marketing_accepted,
                "emailId": email,
                "residencyRegion": { "residencyCountryCode": "US", "residencyRegionCode": "US" },
                "ssoOptions": sso_options(&challenge),
            }
        }),
    )
    .await
    .map_err(CreateError::from)?;
    if let Some(e) = extract_errors(&r1, &["signUp"]) {
        return Err(CreateError::WalmartErrors(format!("SignUp #1: {e}")));
    }
    tracing::info!("signup #1 passed");

    let otp_choice_url = phone_otp_choice_url(&challenge);
    let _ = session
        .http()
        .get(&otp_choice_url)
        .headers(session.page_headers(Some(&signup_url)))
        .send()
        .await
        .context("navigate to phone-otp-choice")?;
    session.page_url = otp_choice_url.clone();

    tokio::spawn({
        let client = session.http().clone();
        let headers = session.graphql_headers("getSignInTempoModule", "query getSignInTempoModule");
        async move {
            let body = json!({
                "query": queries::GET_SIGN_IN_TEMPO_MODULE,
                "variables": { "pageType": "SignInPage", "tenant": "WM_GLASS" }
            });
            let _ = client
                .post("https://identity.walmart.com/orchestra/idp/graphql")
                .headers(headers)
                .body(serde_json::to_string(&body).unwrap())
                .send()
                .await;
        }
    });

    let r_otp = post_graphql(
        session,
        "GenerateOtp",
        "mutation GenerateOtp",
        queries::GENERATE_OTP,
        json!({
            "input": {
                "otpChannel": "PHONE_TEXT",
                "loginId": phone_id,
                "emailId": email,
                "loginIdType": "PHONE",
                "nonProfilePhoneNumber": {
                    "number": phone10,
                    "countryCode": "1",
                    "isoCountryCode": "US",
                },
                "otpOperation": "OTP_UNREGISTERED_USER_PHONE_VERIFY",
                "phoneNumber": phone_id,
                "ssoOptions": sso_options(&challenge),
            }
        }),
    )
    .await
    .map_err(CreateError::from)?;
    if let Some(e) = extract_errors(&r_otp, &["generateOTP"]) {
        return Err(CreateError::WalmartErrors(format!("GenerateOtp: {e}")));
    }

    let verify_url = verify_account_url(&challenge);
    let _ = session
        .http()
        .get(&verify_url)
        .headers(session.page_headers(Some(&otp_choice_url)))
        .send()
        .await
        .context("navigate to verifyyouraccount")?;
    session.page_url = verify_url.clone();

    tokio::spawn({
        let client = session.http().clone();
        let headers = session.graphql_headers(
            "getVerifyYourAccountModule",
            "query getVerifyYourAccountModule",
        );
        async move {
            let body = json!({
                "query": queries::GET_VERIFY_YOUR_ACCOUNT_MODULE,
                "variables": {
                    "pageType": "VerifyYourAccountPage",
                    "tempo": { "targeting": TEMPO_TARGETING },
                    "tenant": "WM_GLASS",
                }
            });
            let _ = client
                .post("https://identity.walmart.com/orchestra/idp/graphql")
                .headers(headers)
                .body(serde_json::to_string(&body).unwrap())
                .send()
                .await;
        }
    });

    let otp_timeout = Duration::from_secs(60);
    let spinner = {
        use indicatif::{ProgressBar, ProgressStyle};
        let ts = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.6fZ");

        let template =
            format!("{ts}  \x1b[32mINFO\x1b[0m {{spinner:.cyan}} Waiting for OTP... {{msg}}");
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template(&template)
                .unwrap(),
        );
        pb.enable_steady_tick(Duration::from_millis(100));
        pb
    };

    let rental = guard.rental().clone();
    let deadline = std::time::Instant::now() + otp_timeout;
    let poll_fut = sms.poll_code(&rental, otp_timeout, Duration::from_secs(3));
    tokio::pin!(poll_fut);
    let sms_code = loop {
        tokio::select! {
            biased;
            result = &mut poll_fut => {
                spinner.finish_and_clear();
                break result.map_err(|_| CreateError::SmsOtpTimeout(rental.id))?;
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                spinner.set_message(format!("{}s remaining", remaining.as_secs()));
                spinner.tick();
            }
        }
    };

    let r2 = post_graphql(
        session,
        "SignUp",
        "mutation SignUp",
        queries::SIGN_UP,
        json!({
            "input": {
                "loginId": phone_id,
                "loginIdType": "PHONE",
                "password": password,
                "firstName": first_name,
                "lastName": last_name,
                "rememberMe": remember_me,
                "marketingEmailsAccepted": marketing_accepted,
                "nonProfilePhoneNumber": {
                    "number": phone10,
                    "countryCode": "1",
                    "isoCountryCode": "US",
                },
                "phoneNumber": phone10,
                "emailId": email,
                "stepUpOptions": {
                    "nonProfilePhoneNumber": {
                        "number": phone10,
                        "countryCode": "1",
                        "isoCountryCode": "US",
                    },
                    "phoneNumber": phone10,
                    "otpCode": sms_code,
                    "otpOperation": "OTP_UNREGISTERED_USER_PHONE_VERIFY",
                },
                "residencyRegion": { "residencyCountryCode": "US", "residencyRegionCode": "US" },
                "gepAdditionalInfo": {
                    "disclaimerText": queries::DISCLAIMER_TEXT,
                    "moduleId": queries::DISCLAIMER_MODULE_ID,
                    "moduleVersion": queries::DISCLAIMER_MODULE_VERSION,
                },
                "ssoOptions": sso_options(&challenge),
            }
        }),
    )
    .await
    .map_err(CreateError::from)?;
    if let Some(e) = extract_errors(&r2, &["signUp"]) {
        return Err(CreateError::WalmartErrors(format!("SignUp #2: {e}")));
    }
    let auth_code = r2
        .pointer("/data/signUp/authCode/authCode")
        .and_then(Value::as_str)
        .ok_or_else(|| CreateError::WalmartErrors("missing authCode in SignUp #2".into()))?
        .to_string();

    guard.complete().await;
    Ok((auth_code, phone10))
}

async fn post_graphql(
    session: &WalmartSession,
    op_name: &'static str,
    gql_label: &'static str,
    query: &'static str,
    variables: Value,
) -> Result<Value> {
    let body = json!({ "query": query, "variables": variables });
    let resp = session
        .http()
        .post("https://identity.walmart.com/orchestra/idp/graphql")
        .headers(session.graphql_headers(op_name, gql_label))
        .body(serde_json::to_string(&body)?)
        .send()
        .await
        .with_context(|| format!("graphql {op_name}"))?;
    let status = resp.status();
    let text = resp.text().await.context("graphql body")?;
    if !status.is_success() {
        anyhow::bail!(
            "graphql {op_name} non-2xx {status}: {}",
            &text[..text.len().min(400)]
        );
    }
    let v: Value = serde_json::from_str(&text)
        .with_context(|| format!("graphql {op_name} parse: {}", &text[..text.len().min(400)]))?;
    Ok(v)
}

async fn verify_token(session: &mut WalmartSession, auth_code: &str) -> Result<()> {
    session.jar.add(
        format!(
            "walmart-identity-web-code-verifier={}; Path=/; Domain=.walmart.com; Secure",
            session.pkce_verifier
        )
        .as_str(),
        "https://www.walmart.com/",
    );

    let url = format!(
        "https://www.walmart.com/account/verifyToken\
?state=%2F\
&client_id={}\
&redirect_uri=https%3A%2F%2Fwww.walmart.com%2Faccount%2FverifyToken\
&scope=openid%20email%20offline_access\
&code={}\
&action=SignIn\
&rm=true",
        queries::CLIENT_ID,
        auth_code,
    );
    let resp = session
        .http()
        .get(&url)
        .headers(session.page_headers(Some("https://identity.walmart.com/")))
        .send()
        .await
        .context("verifyToken")?;
    let status = resp.status();
    if !(status.is_redirection() || status.is_success()) {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "verifyToken returned unexpected {status}: {}",
            &body[..body.len().min(200)]
        );
    }
    if let Some(loc) = resp.headers().get("location").and_then(|v| v.to_str().ok()) {
        let next = if loc.starts_with("http") {
            loc.to_string()
        } else {
            format!("https://www.walmart.com{loc}")
        };
        let _ = session
            .http()
            .get(&next)
            .headers(session.page_headers(Some(&url)))
            .send()
            .await;
    }
    Ok(())
}

fn sso_options(challenge: &str) -> Value {
    json!({
        "wasConsentCaptured": true,
        "callbackUrl": queries::CALLBACK_URL,
        "clientId": queries::CLIENT_ID,
        "scope": queries::SCOPE,
        "state": STATE_PARAM,
        "challenge": challenge,
    })
}

fn phone_login_id(phone10: &str) -> String {
    format!("US/+1{phone10}")
}

fn strip_us_country(raw: &str) -> String {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 11 && digits.starts_with('1') {
        digits[1..].to_string()
    } else {
        digits
    }
}

fn jitter_ms(lo: u64, hi: u64) -> Duration {
    Duration::from_millis(rand::thread_rng().gen_range(lo..=hi))
}

fn extract_platform_version(body: &str) -> Option<String> {
    let prefix = "usweb-";
    let start = body.find(prefix)?;
    let rest = &body[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '.'))
        .unwrap_or(rest.len());
    let ver = &rest[..end];
    if ver.len() > prefix.len() + 3 {
        Some(ver.to_string())
    } else {
        None
    }
}
