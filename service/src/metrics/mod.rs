// Copyright 2023-, GraphOps and Semiotic Labs.
// SPDX-License-Identifier: Apache-2.0

use autometrics::{encode_global_metrics, global_metrics_exporter};
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use indexer_common::metrics::{register_metrics, REGISTRY};
use once_cell::sync::Lazy;

use prometheus::{linear_buckets, HistogramOpts, HistogramVec, IntCounterVec, Opts};
use std::{net::SocketAddr, str::FromStr};
use tracing::info;

pub static QUERIES: Lazy<IntCounterVec> = Lazy::new(|| {
    let m = IntCounterVec::new(
        Opts::new("queries", "Incoming queries")
            .namespace("indexer")
            .subsystem("service"),
        &["deployment"],
    )
    .expect("Failed to create queries counters");
    prometheus::register(Box::new(m.clone())).expect("Failed to register queries counter");
    m
});

pub static SUCCESSFUL_QUERIES: Lazy<IntCounterVec> = Lazy::new(|| {
    let m = IntCounterVec::new(
        Opts::new("successfulQueries", "Successfully executed queries")
            .namespace("indexer")
            .subsystem("service"),
        &["deployment"],
    )
    .expect("Failed to create successfulQueries counters");
    prometheus::register(Box::new(m.clone()))
        .expect("Failed to register successfulQueries counter");
    m
});

pub static FAILED_QUERIES: Lazy<IntCounterVec> = Lazy::new(|| {
    let m = IntCounterVec::new(
        Opts::new("failedQueries", "Queries that failed to execute")
            .namespace("indexer")
            .subsystem("service"),
        &["deployment"],
    )
    .expect("Failed to create failedQueries counters");
    prometheus::register(Box::new(m.clone())).expect("Failed to register failedQueries counter");
    m
});

pub static QUERIES_WITH_INVALID_RECEIPT_HEADER: Lazy<IntCounterVec> = Lazy::new(|| {
    let m = IntCounterVec::new(
        Opts::new(
            "queriesWithInvalidReceiptHeader",
            "Queries that failed executing because they came with an invalid receipt header",
        )
        .namespace("indexer")
        .subsystem("service"),
        &["deployment"],
    )
    .expect("Failed to create queriesWithInvalidReceiptHeader counters");
    prometheus::register(Box::new(m.clone()))
        .expect("Failed to register queriesWithInvalidReceiptHeader counter");
    m
});

pub static QUERIES_WITHOUT_RECEIPT: Lazy<IntCounterVec> = Lazy::new(|| {
    let m = IntCounterVec::new(
        Opts::new(
            "queriesWithoutReceipt",
            "Queries that failed executing because they came without a receipt",
        )
        .namespace("indexer")
        .subsystem("service"),
        &["deployment"],
    )
    .expect("Failed to create queriesWithoutReceipt counters");
    prometheus::register(Box::new(m.clone()))
        .expect("Failed to register queriesWithoutReceipt counter");
    m
});

pub static QUERY_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    let m = HistogramVec::new(
        HistogramOpts::new(
            "query_duration",
            "Duration of processing a query from start to end",
        )
        .namespace("indexer")
        .subsystem("service")
        .buckets(linear_buckets(0.0, 1.0, 20).unwrap()),
        &["deployment"],
    )
    .expect("Failed to create query_duration histograms");
    prometheus::register(Box::new(m.clone())).expect("Failed to register query_duration counter");
    m
});

/// Start the basic metrics for indexer services
pub fn register_query_metrics() {
    register_metrics(
        &REGISTRY,
        vec![
            Box::new(QUERIES.clone()),
            Box::new(SUCCESSFUL_QUERIES.clone()),
            Box::new(FAILED_QUERIES.clone()),
            Box::new(QUERIES_WITH_INVALID_RECEIPT_HEADER.clone()),
            Box::new(QUERIES_WITHOUT_RECEIPT.clone()),
            Box::new(QUERY_DURATION.clone()),
        ],
    );
}

/// This handler serializes the metrics into a string for Prometheus to scrape
#[allow(dead_code)]
pub async fn get_metrics() -> (StatusCode, String) {
    match encode_global_metrics() {
        Ok(metrics) => (StatusCode::OK, metrics),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{err:?}")),
    }
}

#[allow(dead_code)]
pub async fn handle_serve_metrics(host: String, port: u16) {
    // Set up the exporter to collect metrics
    let _exporter = global_metrics_exporter();

    let app = Router::new().route("/metrics", get(get_metrics));
    let addr =
        SocketAddr::from_str(&format!("{}:{}", host, port)).expect("Start Prometheus metrics");
    let server = axum::Server::bind(&addr);
    info!(
        address = addr.to_string(),
        "Prometheus Metrics port exposed"
    );

    server
        .serve(app.into_make_service())
        .await
        .expect("Error starting example API server");
}
