//! `prajna-core` — the Prajna cognitive engine.
//!
//! This crate defines the cognitive primitives that sit above the storage
//! engine. The storage engine (`prajna-storage`) knows how to read and write
//! bytes efficiently; this crate knows what those bytes *mean*.
//!
//! # Modules
//!
//! | Module            | Responsibility                                     |
//! |-------------------|----------------------------------------------------|
//! | [`memory`]        | `MemoryObject` — the fundamental cognitive unit    |
//! | [`memory::salience`] | Salience scoring, decay, goal-conditioned ranking |
//!
//! # Planned modules (future phases)
//!
//! - `context` — the Context Compiler (goal-conditioned state reconstruction)
//! - `identity` — the Identity Kernel (persistent agent self-model)
//! - `graph` — causal edge traversal and contradiction detection
//! - `compression` — hierarchical memory compression pipeline
//! - `query` — the Prajna Query Language parser and planner

pub mod memory;
