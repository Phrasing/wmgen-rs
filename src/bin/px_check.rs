use anyhow::Result;
use wmgen_rs::px::PxSession;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("debug,wmgen_rs=debug")
        .init();

    let client = wreq::Client::builder().build()?;

    println!("─── 1) PX init.js tag extraction ───");
    let px = PxSession::new(client).await?;
    println!("PxSession ready\n");

    println!("─── 2) Two-step collector flow ───");
    let cookies = px.solve("https://www.walmart.com/").await?;

    println!("\n─── Results ───");
    println!(
        "_px3  ({} chars): {}…",
        cookies.px3.len(),
        &cookies.px3[..cookies.px3.len().min(60)]
    );
    println!("_pxvid ({} chars): {}", cookies.pxvid.len(), cookies.pxvid);
    println!("pxcts  ({} chars): {}", cookies.pxcts.len(), cookies.pxcts);

    if cookies.px3.is_empty() {
        eprintln!("\nFAIL: _px3 is empty");
        std::process::exit(1);
    } else {
        println!("\nPASS: got non-empty _px3");
    }
    Ok(())
}
