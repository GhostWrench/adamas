//! Adamas
//! 
//! Rust library for compressing small amounts of structured data

type Block = u64;
type DoubleBlock = u128;
type SignedBlock = i64;
type SignedDoubleBlock = i128;

mod accum;
pub mod data;
