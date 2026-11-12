// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! # ds-rules
//!
//! The open Doge Soft rules for Dogecoin, as pure Rust: Doginals envelopes, koinu tracking, DRC-20
//! Coins and .doge names. No node, no database, no network. Every function here is what the Doge Soft
//! Rust indexer runs, and what `dsverify` runs against your own Dogecoin Core.
//!
//! Much rules. Very open. Such verify.

pub mod drc20;
pub mod envelope;
pub mod json;
pub mod koinu;
pub mod ledger;
pub mod names;
pub mod owner;
pub mod planner;
pub mod tx;
pub mod vectors;

/// The first block a fresh replay reads (Core resolves its height). Nothing before it carries a Doginal.
pub const REPLAY_ANCHOR: &str = "75323db271259899e66cbf4cfab33dcc034527e34bdc75b9a70e1b2deec1aad1";
