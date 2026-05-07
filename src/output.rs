use crate::types::CreatedAccount;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

pub async fn append(path: &Path, account: &CreatedAccount) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
    }

    let needs_header = !path.exists();

    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
        .with_context(|| format!("open {}", path.display()))?;

    if needs_header {
        f.write_all(b"email,password,phone,date\n").await?;
    }

    let date = account
        .created_at
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();
    let line = format!(
        "{},{},{},{}\n",
        csv_field(&account.email),
        csv_field(&account.password),
        csv_field(&account.phone),
        csv_field(&date),
    );
    f.write_all(line.as_bytes()).await?;
    f.flush().await?;
    Ok(())
}

fn csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
