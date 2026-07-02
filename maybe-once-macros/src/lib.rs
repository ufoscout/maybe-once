//! Procedural macros for the [`maybe-once`](https://docs.rs/maybe-once/) crate.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Turns an `async` test function into a synchronous `#[test]` that runs its
/// body on the shared tokio runtime provided by
/// [`maybe_once::tokio::run_test`](https://docs.rs/maybe-once/latest/maybe_once/tokio/fn.run_test.html).
///
/// All tests annotated with this attribute share the very same multi-thread
/// tokio runtime. This is what allows a [`maybe_once::tokio::MaybeOnceAsync`]
/// resource (e.g. a docker container started with `testcontainers`) to be
/// shared across every test and dropped only once the last one completes.
///
/// # Example
///
/// ```ignore
/// use maybe_once::tokio_shared;
///
/// #[tokio_shared::test]
/// async fn should_upgrade_structs_on_load() -> Result<(), C3p0Error> {
///     // test code
///     Ok(())
/// }
/// ```
///
/// expands to:
///
/// ```ignore
/// #[test]
/// fn should_upgrade_structs_on_load() -> Result<(), C3p0Error> {
///     ::maybe_once::tokio::run_test(async move {
///         // test code
///         Ok(())
///     })
/// }
/// ```
#[proc_macro_attribute]
pub fn test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn {
        attrs,
        vis,
        mut sig,
        block,
    } = parse_macro_input!(item as ItemFn);

    if sig.asyncness.take().is_none() {
        return syn::Error::new_spanned(
            sig.fn_token,
            "#[tokio_shared::test] can only be applied to `async fn` functions",
        )
        .to_compile_error()
        .into();
    }

    quote! {
        #[test]
        #(#attrs)*
        #vis #sig {
            ::maybe_once::tokio::run_test(async move #block)
        }
    }
    .into()
}
