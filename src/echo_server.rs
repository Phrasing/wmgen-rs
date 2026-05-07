use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CapturedRequest {
    pub method: String,
    pub path: String,
    pub http_version: String,

    pub headers: Vec<[String; 2]>,
    pub body: String,
}

pub struct EchoServer {
    pub port: u16,
    pub captured: Arc<Mutex<Vec<CapturedRequest>>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl EchoServer {
    pub async fn start(addr: &str) -> Result<Self> {
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("bind {addr}"))?;
        let port = listener.local_addr()?.port();
        let captured: Arc<Mutex<Vec<CapturedRequest>>> = Arc::new(Mutex::new(Vec::new()));
        let captured_for_loop = captured.clone();
        let (tx, mut rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut rx => break,
                    accept = listener.accept() => {
                        let (mut stream, _peer) = match accept {
                            Ok(t) => t,
                            Err(_) => continue,
                        };
                        let captured = captured_for_loop.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_one(&mut stream, &captured).await {
                                eprintln!("echo_server: handler error: {e:#}");
                            }
                            let _ = stream.shutdown().await;
                        });
                    }
                }
            }
        });

        Ok(Self {
            port,
            captured,
            shutdown_tx: Some(tx),
        })
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn last(&self) -> Option<CapturedRequest> {
        self.captured.lock().unwrap().last().cloned()
    }
}

impl Drop for EchoServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

async fn handle_one(
    stream: &mut TcpStream,
    captured: &Arc<Mutex<Vec<CapturedRequest>>>,
) -> Result<()> {
    let raw = read_request(stream).await?;
    let cap = parse_request(&raw).context("parse request")?;
    captured.lock().unwrap().push(cap.clone());
    let body = serde_json::to_string(&cap).unwrap_or_default();
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(resp.as_bytes()).await?;
    Ok(())
}

async fn read_request(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(8192);
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(headers_end) = find_double_crlf(&buf) {
            let mut headers = [httparse::EMPTY_HEADER; 96];
            let mut req = httparse::Request::new(&mut headers);
            if req.parse(&buf).is_ok() {
                let cl = headers
                    .iter()
                    .find(|h| h.name.eq_ignore_ascii_case("content-length"))
                    .and_then(|h| std::str::from_utf8(h.value).ok())
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);
                if buf.len() >= headers_end + cl {
                    break;
                }
            }
        }
        if buf.len() > 1_000_000 {
            break;
        }
    }
    Ok(buf)
}

fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

fn parse_request(buf: &[u8]) -> Option<CapturedRequest> {
    let mut headers = [httparse::EMPTY_HEADER; 96];
    let mut req = httparse::Request::new(&mut headers);
    let body_offset = match req.parse(buf).ok()? {
        httparse::Status::Complete(n) => n,
        httparse::Status::Partial => return None,
    };
    Some(CapturedRequest {
        method: req.method.unwrap_or("?").to_string(),
        path: req.path.unwrap_or("?").to_string(),
        http_version: format!("HTTP/1.{}", req.version.unwrap_or(0)),
        headers: headers
            .iter()
            .filter(|h| !h.name.is_empty())
            .map(|h| {
                [
                    h.name.to_string(),
                    String::from_utf8_lossy(h.value).into_owned(),
                ]
            })
            .collect(),
        body: String::from_utf8_lossy(&buf[body_offset..]).into_owned(),
    })
}
