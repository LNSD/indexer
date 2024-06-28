// Copyright 2023-, Edge & Node, GraphOps, and Semiotic Labs.
// SPDX-License-Identifier: Apache-2.0

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt::init();
    if let Err(e) = indexer_broker::service::run().await {
        tracing::error!("Indexer service error: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
