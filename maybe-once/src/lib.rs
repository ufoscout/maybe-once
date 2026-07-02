#![doc = include_str!("../../README.md")]
#![allow(clippy::test_attr_in_doctest)]

pub mod blocking;

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub mod tokio;

/// Attribute macros for writing tokio tests that share a single runtime.
///
/// Use [`macro@tokio_shared::test`] in place of `#[tokio::test]` to run every
/// annotated test on the shared runtime returned by [`tokio::run_test`].
#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub mod tokio_shared {
    /// See the [module-level documentation](self) for details.
    pub use maybe_once_macros::test;
}
