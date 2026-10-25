// SPDX-License-Identifier: Apache-2.0

use buoybsd::{cli, diagnostics::Diagnostics};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(error) = cli::run_control(&args, &mut std::io::stdout()) {
        let error = anyhow::Error::new(error);
        Diagnostics::new(Box::new(std::io::stderr())).failed(
            "control",
            "control request failed",
            &error,
        );
        std::process::exit(1);
    }
}
