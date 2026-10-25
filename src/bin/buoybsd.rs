// SPDX-License-Identifier: Apache-2.0

use buoybsd::{cli, diagnostics::Diagnostics};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let diagnostics = Diagnostics::new(Box::new(std::io::stderr()));
    let cancel = CancellationToken::new();
    let signal_cancel = cancel.clone();
    let signal = tokio::spawn(async move {
        #[cfg(unix)]
        {
            let mut terminate =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
            tokio::select! {
                result = tokio::signal::ctrl_c() => result?,
                _ = terminate.recv() => {}
            }
        }
        #[cfg(not(unix))]
        tokio::signal::ctrl_c().await?;
        signal_cancel.cancel();
        Ok::<(), std::io::Error>(())
    });
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = cli::run_sensor(&cancel, &args, Box::new(std::io::stdout()), &diagnostics).await;
    signal.abort();
    let _ = signal.await;
    if let Err(error) = result {
        let error = anyhow::Error::new(error);
        diagnostics.failed("run", "sensor failed", &error);
        std::process::exit(1);
    }
}
