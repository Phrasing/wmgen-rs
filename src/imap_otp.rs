use anyhow::{Context, Result};
use async_imap::Session;
use futures::StreamExt;
use mail_parser::MessageParser;
use regex::Regex;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

pub struct GmailImap {
    session: Session<TlsStream<TcpStream>>,
    code_re: Regex,
}

impl GmailImap {
    pub async fn connect(host: &str, port: u16, user: &str, pass: &str) -> Result<Self> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(config));

        let tcp = TcpStream::connect((host, port))
            .await
            .context("imap tcp connect")?;
        let server_name = ServerName::try_from(host.to_string()).context("imap servername")?;
        let tls = connector
            .connect(server_name, tcp)
            .await
            .context("imap tls")?;

        let client = async_imap::Client::new(tls);
        let mut session = client
            .login(user, pass)
            .await
            .map_err(|(e, _)| e)
            .context("imap login")?;
        session.select("INBOX").await.context("imap select INBOX")?;
        Ok(Self {
            session,
            code_re: Regex::new(r"\b(\d{6})\b").unwrap(),
        })
    }

    pub async fn fetch_walmart_otp(
        &mut self,
        target_email: &str,
        timeout: Duration,
        interval: Duration,
    ) -> Result<String> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if std::time::Instant::now() > deadline {
                anyhow::bail!("imap timeout waiting for Walmart OTP for {target_email}");
            }

            let _ = self.session.select("INBOX").await;

            let query = format!("UNSEEN HEADER Delivered-To {target_email} FROM walmart");
            let uids = self
                .session
                .uid_search(&query)
                .await
                .context("imap search")?;

            let uids = if uids.is_empty() {
                self.session
                    .uid_search(&format!("UNSEEN TO {target_email} FROM walmart"))
                    .await
                    .context("imap search (TO fallback)")?
            } else {
                uids
            };

            if let Some(&uid) = uids.iter().next() {
                let mut stream = self
                    .session
                    .uid_fetch(format!("{uid}"), "(BODY.PEEK[])")
                    .await
                    .context("imap fetch")?;

                let mut found_code: Option<String> = None;
                while let Some(msg) = stream.next().await {
                    let msg = msg.context("imap fetch frame")?;
                    if let Some(body) = msg.body() {
                        let parsed = MessageParser::default()
                            .parse(body)
                            .context("parse email body")?;

                        let mut hay = String::new();
                        if let Some(s) = parsed.subject() {
                            hay.push_str(s);
                            hay.push('\n');
                        }
                        for text in parsed.text_bodies() {
                            if let Some(t) = text.text_contents() {
                                hay.push_str(t);
                                hay.push('\n');
                            }
                        }
                        for html in parsed.html_bodies() {
                            if let Some(t) = html.text_contents() {
                                hay.push_str(t);
                                hay.push('\n');
                            }
                        }
                        if let Some(cap) = self.code_re.captures(&hay) {
                            found_code = Some(cap[1].to_string());
                            break;
                        }
                    }
                }
                drop(stream);

                if let Some(code) = found_code {
                    if let Ok(updates) = self
                        .session
                        .uid_store(format!("{uid}"), "+FLAGS (\\Seen)")
                        .await
                    {
                        let _: Vec<_> = updates.collect().await;
                    }
                    tracing::info!(target_email, "found Walmart OTP");
                    return Ok(code);
                }
            }
            tokio::time::sleep(interval).await;
        }
    }

    pub async fn logout(mut self) {
        let _ = self.session.logout().await;
    }
}
