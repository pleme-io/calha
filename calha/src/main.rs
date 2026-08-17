use clap::Parser;

/// `calha` — drives the restart-required half of a config hot-swap change
/// through a normal rolling update. See `theory/CALHA.md`.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Fleet-wide freeze switch (mirrors breathe's `!writeEnabled`). When
    /// set, every `CalhaPolicy` reconcile shadows unconditionally.
    #[arg(long, env = "CALHA_FROZEN", default_value_t = false)]
    frozen: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let client = kube::Client::try_default().await?;

    calha::controller::run(client, args.frozen).await;

    Ok(())
}
