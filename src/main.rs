use anyhow::{Context, Result};
use clap::Parser;
use rand::seq::SliceRandom;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::io::AsyncBufReadExt;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::prelude::*;
use wmgen_rs::config::Config;
use wmgen_rs::getatext::Getatext;
use wmgen_rs::output;
use wmgen_rs::pvacodes::Pvacodes;
use wmgen_rs::sms::SmsProvider;
use wmgen_rs::types::{CreateError, CreatedAccount, Proxy};
use wmgen_rs::walmart::flow::{create_account, AccountInputs};

#[derive(Parser, Debug)]
#[command(name = "wmgen-rs")]
struct Args {
    #[arg(long, default_value = "emails.txt")]
    emails: PathBuf,

    #[arg(long, default_value = "proxies.txt")]
    proxies: PathBuf,

    #[arg(long, default_value = "output/accounts.csv")]
    output: PathBuf,

    #[arg(long)]
    max: Option<usize>,
}

struct RunInputs {
    emails: Vec<String>,
    proxies: Vec<Proxy>,
    max_accounts: usize,
}

#[derive(Default)]
struct RunStats {
    successes: usize,
    failures: usize,
}

struct LoggingGuards {
    _px: tracing_appender::non_blocking::WorkerGuard,
    _accounts: tracing_appender::non_blocking::WorkerGuard,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let args = Args::parse();
    let _logging = init_logging();

    let cfg = Config::from_env_and_args(
        args.emails.clone(),
        args.proxies.clone(),
        args.output.clone(),
    )?;
    let inputs = load_inputs(&args, &cfg).await?;
    verify_network(&cfg, &inputs.proxies).await?;

    let sms_provider = build_sms_provider(&cfg);
    tracing::info!(
        emails = inputs.emails.len(),
        proxies = inputs.proxies.len(),
        sms = %sms_provider_label(&cfg),
        "ready"
    );

    let stats = run_accounts(inputs, &sms_provider, &cfg).await;
    tracing::info!(
        "done — {} created, {} failed",
        stats.successes,
        stats.failures
    );
    Ok(())
}

fn init_logging() -> LoggingGuards {
    let _ = std::fs::create_dir_all("logs");

    let (px_writer, px_guard) =
        tracing_appender::non_blocking(tracing_appender::rolling::never("logs", "px.log"));
    let (accounts_writer, accounts_guard) =
        tracing_appender::non_blocking(tracing_appender::rolling::never("logs", "accounts.log"));
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,wmgen_rs=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(px_writer)
                .with_ansi(false)
                .with_filter(filter_fn(|meta| meta.target().starts_with("wmgen_rs::px"))),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(accounts_writer)
                .with_ansi(false)
                .with_filter(filter_fn(|meta| !meta.target().starts_with("wmgen_rs::px"))),
        )
        .init();

    LoggingGuards {
        _px: px_guard,
        _accounts: accounts_guard,
    }
}

async fn load_inputs(args: &Args, cfg: &Config) -> Result<RunInputs> {
    let emails = load_emails(cfg).await?;
    let proxies = load_proxies(&cfg.proxies_path).await?;

    if emails.is_empty() {
        anyhow::bail!("emails.txt is empty");
    }
    if proxies.is_empty() {
        anyhow::bail!("proxies.txt has no valid entries");
    }

    let max_accounts = args
        .max
        .or_else(|| std::env::var("MAX_ACCOUNTS").ok()?.parse().ok())
        .unwrap_or(emails.len());

    Ok(RunInputs {
        emails,
        proxies,
        max_accounts,
    })
}

async fn load_emails(cfg: &Config) -> Result<Vec<String>> {
    let all_emails = read_lines(&cfg.emails_path)
        .await
        .context("read emails.txt")?;
    let already_created = read_csv_emails(&cfg.output_path).await;
    let removed = all_emails
        .iter()
        .filter(|email| already_created.contains(email.as_str()))
        .count();
    let emails: Vec<String> = all_emails
        .into_iter()
        .filter(|email| !already_created.contains(email.as_str()))
        .collect();

    if removed > 0 {
        rewrite_lines(&cfg.emails_path, &emails)
            .await
            .context("rewrite emails.txt")?;
        tracing::info!("removed {removed} already-created email(s) from emails.txt");
    }

    Ok(emails)
}

async fn load_proxies(path: &Path) -> Result<Vec<Proxy>> {
    let lines = read_lines(path).await.context("read proxies.txt")?;
    Ok(lines
        .iter()
        .enumerate()
        .filter_map(|(line_number, line)| match Proxy::parse_line(line) {
            Ok(proxy) => Some(proxy),
            Err(err) => {
                tracing::warn!("skipping bad proxy at line {}: {err}", line_number + 1);
                None
            }
        })
        .collect())
}

async fn verify_network(cfg: &Config, proxies: &[Proxy]) -> Result<()> {
    if cfg.no_proxy {
        tracing::warn!("no proxy: all requests use the real IP");
        let real_ip = fetch_ip(None).await.unwrap_or_else(|_| "unknown".into());
        tracing::info!(ip = %real_ip, "outbound IP");
        return Ok(());
    }

    verify_proxy_ip(&proxies[0]).await
}

fn build_sms_provider(cfg: &Config) -> SmsProvider {
    match cfg.sms_provider.as_str() {
        "pvacodes" => SmsProvider::Pvacodes(Pvacodes::new(
            cfg.pvacodes_api_key
                .clone()
                .expect("pvacodes_api_key checked at startup"),
            cfg.pvacodes_app.clone(),
        )),
        _ => SmsProvider::Getatext(Getatext::new(
            cfg.getatext_api_key.clone(),
            cfg.getatext_service.clone(),
        )),
    }
}

fn sms_provider_label(cfg: &Config) -> String {
    match cfg.sms_provider.as_str() {
        "pvacodes" => format!("pvacodes({})", cfg.pvacodes_app),
        _ => format!("getatext({})", cfg.getatext_service),
    }
}

async fn run_accounts(inputs: RunInputs, sms_provider: &SmsProvider, cfg: &Config) -> RunStats {
    let mut stats = RunStats::default();
    let mut burned = HashSet::new();

    for (index, email) in inputs.emails.into_iter().enumerate() {
        if stats.successes >= inputs.max_accounts {
            break;
        }

        let Some(proxy_index) = select_proxy_index(inputs.proxies.len(), &burned) else {
            tracing::error!("all proxies exhausted (burned) — stopping");
            break;
        };

        tracing::info!("[{}] {}", index + 1, email);
        let account_inputs = AccountInputs {
            email: email.clone(),
            proxy: inputs.proxies[proxy_index].clone(),
        };

        match create_account(account_inputs, sms_provider, cfg).await {
            Ok(account) => {
                persist_success(cfg, &account.email, &email, &account).await;
                stats.successes += 1;
                tracing::info!("SUCCESS {}", account.email);
            }
            Err(CreateError::PhoneOutOfStock) => {
                tracing::error!("phone number service is out of stock — stopping");
                break;
            }
            Err(CreateError::InsufficientFunds) => {
                tracing::error!("SMS provider has insufficient funds — stopping");
                break;
            }
            Err(ref e) if is_inkiru_failure(e) => {
                burned.insert(proxy_index);
                stats.failures += 1;
                tracing::warn!(
                    burned = burned.len(),
                    remaining = inputs.proxies.len() - burned.len(),
                    "proxy burned (Inkiru)"
                );
                tracing::error!("FAILED [{}] Inkiru: {}", email, e);
            }
            Err(CreateError::AccountExists(_)) => {
                if let Err(e) = remove_email_from_file(&cfg.emails_path, &email).await {
                    tracing::warn!("failed to remove {email} from emails file: {e:#}");
                }
                tracing::warn!("SKIP {} — account already exists", email);
            }
            Err(e) => {
                stats.failures += 1;
                tracing::error!("FAILED [{}] {}", email, e);
            }
        }
    }

    stats
}

fn select_proxy_index(proxy_count: usize, burned: &HashSet<usize>) -> Option<usize> {
    let available: Vec<usize> = (0..proxy_count)
        .filter(|index| !burned.contains(index))
        .collect();
    available.choose(&mut rand::thread_rng()).copied()
}

async fn persist_success(
    cfg: &Config,
    account_email: &str,
    input_email: &str,
    account: &CreatedAccount,
) {
    if let Err(err) = output::append(&cfg.output_path, account).await {
        tracing::error!("failed to write output for {account_email}: {err:#}");
    }
    if let Err(err) = remove_email_from_file(&cfg.emails_path, input_email).await {
        tracing::warn!("failed to remove {input_email} from emails file: {err:#}");
    }
}

async fn fetch_ip(proxy: Option<&Proxy>) -> Result<String> {
    let mut builder = wreq::Client::builder();
    if let Some(p) = proxy {
        builder = builder.proxy(wreq::Proxy::all(p.http_url()).context("build proxy")?);
    }
    let client = builder.build().context("build ip-check client")?;
    let body = client
        .get("https://checkip.amazonaws.com")
        .send()
        .await
        .context("ip-check request")?
        .text()
        .await
        .context("ip-check body")?;
    let ip = body.trim().to_string();

    if ip.contains('<') || ip.is_empty() {
        anyhow::bail!(
            "ip-check returned unexpected body: {}",
            &ip[..ip.len().min(120)]
        );
    }
    Ok(ip)
}

async fn verify_proxy_ip(proxy: &Proxy) -> Result<()> {
    let (real, proxied) = tokio::try_join!(fetch_ip(None), fetch_ip(Some(proxy)))?;
    if real == proxied {
        anyhow::bail!(
            "proxy check failed: proxied IP ({proxied}) matches real IP ({real}) — \
             the proxy is not routing traffic. Set SKIP_PROXY=1 to bypass this check."
        );
    }
    tracing::info!("proxy OK  via={proxied}  ({})", proxy.host);
    Ok(())
}

fn is_inkiru_failure(e: &CreateError) -> bool {
    matches!(e, CreateError::WalmartErrors(s) if s.contains("INKIRU_FAILED"))
}

async fn read_csv_emails(path: &Path) -> HashSet<String> {
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => return HashSet::new(),
    };
    content
        .lines()
        .skip(1)
        .filter_map(|line| {
            let field = if line.starts_with('"') {
                let inner = line.trim_start_matches('"');
                let end = inner.find('"').unwrap_or(inner.len());
                inner[..end].replace("\"\"", "\"")
            } else {
                line.split(',').next().unwrap_or("").to_string()
            };
            let field = field.trim().to_string();
            if field.is_empty() {
                None
            } else {
                Some(field)
            }
        })
        .collect()
}

async fn remove_email_from_file(path: &Path, email: &str) -> Result<()> {
    let content = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("read {}", path.display()))?;
    let filtered: String = content
        .lines()
        .filter(|l| l.trim() != email)
        .flat_map(|l| [l, "\n"])
        .collect();
    write_atomic(path, &filtered).await
}

async fn rewrite_lines(path: &Path, lines: &[String]) -> Result<()> {
    let content: String = lines
        .iter()
        .flat_map(|line| [line.as_str(), "\n"])
        .collect();
    write_atomic(path, &content).await
}

async fn write_atomic(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, content)
        .await
        .with_context(|| format!("write {}", tmp.display()))?;
    tokio::fs::rename(&tmp, path)
        .await
        .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))
}

async fn read_lines(path: &Path) -> Result<Vec<String>> {
    let f = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("open {}", path.display()))?;
    let reader = tokio::io::BufReader::new(f);
    let mut out = Vec::new();
    let mut lines = reader.lines();
    while let Some(l) = lines.next_line().await? {
        let l = l.trim();
        if !l.is_empty() && !l.starts_with('#') {
            out.push(l.to_string());
        }
    }
    Ok(out)
}
