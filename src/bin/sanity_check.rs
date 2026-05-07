use anyhow::Result;
use wmgen_rs::echo_server::{CapturedRequest, EchoServer};
use wmgen_rs::generators::{device_profile_ref_id, pkce_pair};
use wmgen_rs::walmart::client::build_graphql_headers;
use wmgen_rs::walmart::queries;
use wreq_util::{Emulation, Profile};

const EXPECTED_ORDER: &[&str] = &[
    "x-o-mart",
    "x-o-gql-query",
    "sec-ch-ua-platform",
    "x-o-segment",
    "device_profile_ref_id",
    "sec-ch-ua",
    "x-enable-server-timing",
    "sec-ch-ua-mobile",
    "baggage",
    "x-latency-trace",
    "traceparent",
    "wm_mp",
    "accept",
    "content-type",
    "x-apollo-operation-name",
    "tenant-id",
    "downlink",
    "wm_qos.correlation_id",
    "x-o-platform",
    "x-o-platform-version",
    "accept-language",
    "x-o-ccm",
    "x-o-bu",
    "dpr",
    "user-agent",
    "wm_page_url",
    "x-o-correlation-id",
    "origin",
    "sec-fetch-site",
    "sec-fetch-mode",
    "sec-fetch-dest",
    "referer",
    "accept-encoding",
    "cookie",
    "priority",
];

const STACK_INJECTED: &[&str] = &["host", "content-length"];

#[tokio::main]
async fn main() -> Result<()> {
    println!("─── 1) Local echo-server header-order check ───\n");
    run_local_echo_check().await?;

    if std::env::var("SKIP_PEET").is_err() {
        println!("\n─── 2) tls.peet.ws HTTP/2 fingerprint ───\n");
        if let Err(e) = run_peet_check().await {
            eprintln!("(skipped/failed): {e:#}");
        }
    } else {
        println!("\n(skipping tls.peet.ws check; SKIP_PEET set)");
    }

    Ok(())
}

async fn run_local_echo_check() -> Result<()> {
    use std::sync::Arc;
    let server = EchoServer::start("127.0.0.1:0").await?;
    let url = format!("{}/orchestra/idp/graphql", server.url());
    println!("echo server listening at {}", server.url());

    let jar = Arc::new(wreq::cookie::Jar::default());

    jar.add(
        "_px3=test_value; Path=/; Domain=127.0.0.1",
        "http://127.0.0.1/",
    );
    jar.add(
        "ak_bmsc=akamai_test; Path=/; Domain=127.0.0.1",
        "http://127.0.0.1/",
    );

    let client = wreq::Client::builder()
        .emulation(
            Emulation::builder()
                .profile(Profile::Chrome147)
                .headers(false)
                .build(),
        )
        .cookie_provider(jar.clone())
        .build()?;

    let dprid = device_profile_ref_id();
    let (_verifier, challenge) = pkce_pair();
    let page_url = format!(
        "https://identity.walmart.com/account/verifyyouraccount?scope=openid%20email%20offline_access&redirect_uri=https%3A%2F%2Fwww.walmart.com%2Faccount%2FverifyToken&client_id={}&tenant_id=elh9ie",
        queries::CLIENT_ID
    );

    use wreq::cookie::{CookieStore, Cookies};
    let cookie_uri: wreq::Uri = url.parse()?;
    let cookie_value = match jar.cookies(&cookie_uri, wreq::Version::HTTP_2) {
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
    };

    let headers = build_graphql_headers(
        "SignUp",
        "mutation SignUp",
        &dprid,
        &page_url,
        queries::PLATFORM_VERSION,
        cookie_value.as_deref(),
    );

    let body = serde_json::json!({
        "query": queries::SIGN_UP,
        "variables": { "test": true, "challenge": challenge },
    });

    let resp = client
        .post(&url)
        .headers(headers)
        .body(serde_json::to_string(&body)?)
        .send()
        .await?;
    let status = resp.status();
    let captured: CapturedRequest = resp.json().await?;
    println!(
        "client got back {status}, server captured {} headers\n",
        captured.headers.len()
    );

    println!("─ Captured headers (in wire order) ─");
    for [name, value] in &captured.headers {
        let v = if value.len() > 100 {
            format!("{}…", &value[..100])
        } else {
            value.clone()
        };
        println!("  {name:32} {v}");
    }

    println!();
    diff_against_expected(&captured.headers);

    Ok(())
}

fn diff_against_expected(captured: &[[String; 2]]) {
    let actual_filtered: Vec<&str> = captured
        .iter()
        .map(|h| h[0].to_ascii_lowercase())
        .collect::<Vec<_>>()
        .iter()
        .filter(|n| !STACK_INJECTED.contains(&n.as_str()))
        .map(|s| Box::leak(s.clone().into_boxed_str()) as &str)
        .collect();

    let expected: Vec<&str> = EXPECTED_ORDER.to_vec();

    let exp_set: std::collections::BTreeSet<&str> = expected.iter().copied().collect();
    let act_set: std::collections::BTreeSet<&str> = actual_filtered.iter().copied().collect();
    let missing: Vec<&str> = exp_set.difference(&act_set).copied().collect();
    let extra: Vec<&str> = act_set.difference(&exp_set).copied().collect();

    println!("─ Set diff vs expected (HAR entry 1001) ─");
    if missing.is_empty() && extra.is_empty() {
        println!("  ✓ same set of headers");
    } else {
        for m in &missing {
            println!("  - missing: {m}");
        }
        for e in &extra {
            println!("  + extra:   {e}");
        }
    }

    let common_actual: Vec<&str> = actual_filtered
        .iter()
        .copied()
        .filter(|n| exp_set.contains(n))
        .collect();
    let common_expected: Vec<&str> = expected
        .iter()
        .copied()
        .filter(|n| act_set.contains(n))
        .collect();

    println!("\n─ Order diff (over the intersection) ─");
    if common_actual == common_expected {
        println!("  ✓ order matches Chrome 147 / Walmart HAR exactly");
    } else {
        println!("  ✗ order differs; side-by-side:\n");
        let n = common_actual.len().max(common_expected.len());
        println!("  {:>3}  {:<32} | expected", "#", "actual");
        println!(
            "  {:>3}  {:<32}-+-{}",
            "---",
            "-".repeat(32),
            "-".repeat(32)
        );
        for i in 0..n {
            let a = common_actual.get(i).copied().unwrap_or("");
            let e = common_expected.get(i).copied().unwrap_or("");
            let mark = if a == e { " " } else { "≠" };
            println!("  {:>3} {mark}{:<32} | {}", i + 1, a, e);
        }
    }

    let injected_positions: Vec<(usize, &str)> = captured
        .iter()
        .enumerate()
        .filter_map(|(i, h)| {
            let name = h[0].to_ascii_lowercase();
            if STACK_INJECTED.contains(&name.as_str()) {
                Some((i, Box::leak(name.into_boxed_str()) as &str))
            } else {
                None
            }
        })
        .collect();
    if !injected_positions.is_empty() {
        println!("\n─ Stack-injected header positions (informational) ─");
        for (idx, name) in injected_positions {
            println!("  position {idx}: {name}");
        }
    }
}

async fn run_peet_check() -> Result<()> {
    let client = wreq::Client::builder()
        .emulation(
            Emulation::builder()
                .profile(Profile::Chrome147)
                .headers(false)
                .build(),
        )
        .build()?;
    let resp = client.get("https://tls.peet.ws/api/all").send().await?;
    let v: serde_json::Value = resp.json().await?;
    if let Some(http2) = v.get("http2") {
        println!("HTTP/2 section from tls.peet.ws:");
        println!("{}", serde_json::to_string_pretty(http2)?);
    } else {
        println!("(tls.peet.ws didn't return an `http2` block — likely fell back to H/1.1)");
    }
    if let Some(tls) = v.get("tls") {
        if let Some(ja4) = tls.get("ja4") {
            println!("\nTLS JA4: {ja4}");
        }
        if let Some(ja3_hash) = tls.get("ja3_hash") {
            println!("TLS JA3 hash: {ja3_hash}");
        }
        if let Some(peet) = tls.get("peetprint_hash") {
            println!("Peetprint hash: {peet}");
        }
    }
    if let Some(ua) = v.get("user_agent") {
        println!("Server saw User-Agent: {ua}");
    }
    Ok(())
}
