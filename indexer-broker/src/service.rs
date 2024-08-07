// Copyright 2023-, Edge & Node, GraphOps, and Semiotic Labs.
// SPDX-License-Identifier: Apache-2.0

use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use axum::{async_trait, routing::post, Json, Router};
use clap::Parser;
use indexer_common::indexer_service::{
    IndexerService, IndexerServiceImpl, IndexerServiceOptions, IndexerServiceRelease,
    IndexerServiceResponse,
};
use indexer_config::Config as MainConfig;
use reqwest::Url;
use serde_json::json;
use sqlx::PgPool;
use thegraph_core::types::{Attestation, DeploymentId};

use super::{config::Config, error::SubgraphServiceError, routes};
use crate::{cli::Cli, database};

#[derive(Debug)]
struct SubgraphServiceResponse {
    inner: String,
    attestable: bool,
}

impl SubgraphServiceResponse {
    pub fn new(inner: String, attestable: bool) -> Self {
        Self { inner, attestable }
    }
}

impl IndexerServiceResponse for SubgraphServiceResponse {
    type Data = Json<serde_json::Value>;
    type Error = SubgraphServiceError; // not used

    fn is_attestable(&self) -> bool {
        self.attestable
    }

    fn as_str(&self) -> Result<&str, Self::Error> {
        Ok(self.inner.as_str())
    }

    fn finalize(self, attestation: Option<Attestation>) -> Self::Data {
        Json(json!({
            "graphQLResponse": self.inner,
            "attestation": attestation
        }))
    }
}

pub struct SubgraphServiceState {
    pub config: Config,
    pub database: PgPool,
    pub cost_schema: routes::cost::CostSchema,
    pub graph_node_client: reqwest::Client,
    pub graph_node_status_url: String,
    pub graph_node_query_base_url: String,
}

struct SubgraphService {
    state: Arc<SubgraphServiceState>,
}

impl SubgraphService {
    fn new(state: Arc<SubgraphServiceState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl IndexerServiceImpl for SubgraphService {
    type Error = SubgraphServiceError;
    type Request = serde_json::Value;
    type Response = SubgraphServiceResponse;
    type State = SubgraphServiceState;

    async fn process_request(
        &self,
        deployment: DeploymentId,
        request: Self::Request,
    ) -> Result<(Self::Request, Self::Response), Self::Error> {
        let deployment_url = Url::parse(&format!(
            "{}/subgraphs/id/{}",
            &self.state.graph_node_query_base_url, deployment
        ))
        .map_err(|_| SubgraphServiceError::InvalidDeployment(deployment))?;

        let response = self
            .state
            .graph_node_client
            .post(deployment_url)
            .json(&request)
            .send()
            .await
            .map_err(SubgraphServiceError::QueryForwardingError)?;

        let attestable = response
            .headers()
            .get("graph-attestable")
            .map_or(false, |value| {
                value.to_str().map(|value| value == "true").unwrap_or(false)
            });

        let body = response
            .text()
            .await
            .map_err(SubgraphServiceError::QueryForwardingError)?;

        Ok((request, SubgraphServiceResponse::new(body, attestable)))
    }
}

/// Run the subgraph indexer service
pub async fn run() -> anyhow::Result<()> {
    // Parse command line and environment arguments
    let cli = Cli::parse();

    // Load the json-rpc service configuration, which is a combination of the
    // general configuration options for any indexer service and specific
    // options added for JSON-RPC
    let config =
        MainConfig::parse(indexer_config::ConfigPrefix::Service, &cli.config).map_err(|e| {
            tracing::error!(
                "Invalid configuration file `{}`: {}",
                cli.config.display(),
                e
            );
            anyhow!(e)
        })?;

    let config: Config = config.into();

    // Parse basic configurations
    build_info::build_info!(fn build_info);
    let release = IndexerServiceRelease::from(build_info());

    // Some of the subgraph service configuration goes into the so-called
    // "state", which will be passed to any request handler, middleware etc.
    // that is involved in serving requests
    let state = Arc::new(SubgraphServiceState {
        config: config.clone(),
        database: database::connect(&config.database.postgres_url).await,
        cost_schema: routes::cost::build_schema().await,
        graph_node_client: reqwest::ClientBuilder::new()
            .tcp_nodelay(true)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to init HTTP client for Graph Node"),
        graph_node_status_url: config
            .graph_node
            .as_ref()
            .expect("Config must have `common.graph_node.status_url` set")
            .status_url
            .clone(),
        graph_node_query_base_url: config
            .graph_node
            .as_ref()
            .expect("config must have `common.graph_node.query_url` set")
            .query_base_url
            .clone(),
    });

    IndexerService::run(IndexerServiceOptions {
        release,
        config: config.into_inner(),
        url_namespace: "subgraphs",
        metrics_prefix: "subgraph",
        service_impl: SubgraphService::new(state.clone()),
        extra_routes: Router::new()
            .route("/cost", post(routes::cost::handle))
            .route("/status", post(routes::status::handle))
            .with_state(state),
    })
    .await
}
