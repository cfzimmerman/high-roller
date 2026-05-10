//! # High Roller
//!
//! This `no_std` library includes tools for tracking statistics
//! in dynamic latency-sensitive systems. The motivating case was
//! reporting downsampled performance telemetry in embedded applications,
//! but it's hopefully useful in other domains as well.
//!
//! This crate contains three members:
//! - Rolling Max: tracks the greatest value in a fixed-size window.
//! - Rolling Sum: tracks the sum of entries in a fixed-size window.
//! - Decimal32: a 32-bit decimal type that can be easily used with Max and Sum.
//!
//! This crate has the following design motivations:
//! - Algorithmic optimality: Max and overflow-resistent Sum expose asymptotically
//!   optimal operations.
//! - Performance orientation: no_std, no heap allocs, and a performance-aware
//!   approach to implementation. Demonstrated performance improvement contributions
//!   are also very welcome.
//! - Code simplicity: this crate has more lines of docs than actual code. When feature
//!   availability and simplicity collide, the latter is chosen.
//!
//! # Example
//!
//! The example below accumulates request latency and Root Mean Squared Error with
//! 1/10 downsampling.
//!
//! ```
//! use high_roller::decimal::Decimal32;
//! use high_roller::rolling_max::RollingMax;
//! use high_roller::rolling_sum::RollingSum;
//!
//! // TODO: finish this example
//!
//! ```
//
// # Profiling
//
// TO DO
#![no_std]

pub mod decimal;
pub mod rolling_max;
pub mod rolling_sum;
