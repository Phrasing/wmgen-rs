# wmgen-rs

Rust CLI for a configured account workflow with SMS provider support, proxy handling, structured logging, and CSV output.

Use this project only where you have explicit permission.

## Requirements

- Rust stable
- A configured SMS provider account
- `emails.txt`
- `proxies.txt`, unless `SKIP_PROXY=true`

## Setup

Create a local environment file:

```bash
cp .env.example .env
```

Fill in the provider values in `.env`.

Create runtime input files:

```text
emails.txt
proxies.txt
```

Proxy lines may be either:

```text
host:port:user:pass
user:pass@host:port
```

## Run

```bash
cargo run --release -- --emails emails.txt --proxies proxies.txt --output output/accounts.csv
```

Optional limit:

```bash
cargo run --release -- --max 5
```

## Checks

```bash
cargo fmt
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Files

- `.env.example`: safe example configuration
- `emails.txt`: input emails, ignored by Git
- `proxies.txt`: input proxies, ignored by Git
- `output/`: generated CSV output, ignored by Git
- `logs/`: runtime logs, ignored by Git
