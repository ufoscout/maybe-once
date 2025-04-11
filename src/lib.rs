#![doc = include_str!("../README.md")]

pub mod blocking;

#[cfg(feature = "tokio")]
pub mod tokio;
