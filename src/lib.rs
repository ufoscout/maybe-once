#![doc = include_str!("../README.md")]
#![allow(clippy::test_attr_in_doctest)]

pub mod blocking;

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub mod tokio;
