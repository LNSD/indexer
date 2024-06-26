// Copyright 2023-, GraphOps and Semiotic Labs.
// SPDX-License-Identifier: Apache-2.0

pub mod address;
pub mod allocations;
pub mod attestations;
pub mod escrow_accounts;
pub mod graphql;
pub mod indexer_errors;
pub mod indexer_service;
pub mod metrics;
pub mod signature_verification;
pub mod subgraph_client;
pub mod tap;

#[cfg(test)]
mod test_vectors;

pub mod prelude {
    pub use super::{
        allocations::{
            monitor::indexer_allocations, Allocation, AllocationStatus, SubgraphDeployment,
        },
        attestations::{
            dispute_manager::dispute_manager, signer::AttestationSigner,
            signers::attestation_signers,
        },
        escrow_accounts::escrow_accounts,
        indexer_errors,
        subgraph_client::{DeploymentDetails, Query, QueryVariables, SubgraphClient},
        tap::IndexerTapContext,
    };
}
