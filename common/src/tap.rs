// Copyright 2023-, GraphOps and Semiotic Labs.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, fmt::Debug, sync::Arc, time::Duration};

use alloy_sol_types::Eip712Domain;
use eventuals::Eventual;
use sqlx::PgPool;
use tap_core::receipt::checks::ReceiptCheck;
use thegraph::types::Address;
use tracing::error;

use crate::{
    escrow_accounts::EscrowAccounts,
    prelude::Allocation,
    tap::checks::{
        allocation_eligible::AllocationEligible, deny_list_check::DenyListCheck,
        receipt_max_val_check::ReceiptMaxValueCheck, sender_balance_check::SenderBalanceCheck,
        timestamp_check::TimestampCheck,
    },
};

mod checks;
mod receipt_store;

#[derive(Clone)]
pub struct IndexerTapContext {
    pgpool: PgPool,
    domain_separator: Arc<Eip712Domain>,
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
}

impl IndexerTapContext {
    pub async fn get_checks(
        pgpool: PgPool,
        indexer_allocations: Eventual<HashMap<Address, Allocation>>,
        escrow_accounts: Eventual<EscrowAccounts>,
        domain_separator: Eip712Domain,
        timestamp_error_tolerance: Duration,
        receipt_max_value: u128,
    ) -> Vec<ReceiptCheck> {
        vec![
            Arc::new(AllocationEligible::new(indexer_allocations)),
            Arc::new(SenderBalanceCheck::new(
                escrow_accounts.clone(),
                domain_separator.clone(),
            )),
            Arc::new(TimestampCheck::new(timestamp_error_tolerance)),
            Arc::new(DenyListCheck::new(pgpool, escrow_accounts, domain_separator).await),
            Arc::new(ReceiptMaxValueCheck::new(receipt_max_value)),
        ]
    }

    pub async fn new(pgpool: PgPool, domain_separator: Eip712Domain) -> Self {
        Self {
            pgpool,
            domain_separator: Arc::new(domain_separator),
        }
    }
}
