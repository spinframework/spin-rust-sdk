use proc_macro::TokenStream;
use quote::quote;

const WIT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/wit");

/// The entrypoint to a Spin Redis component.
///
/// The component runs in response to messages on a Redis queue.
///
/// # Examples
///
/// A handler that logs the content of each message it receives.
///
/// ```ignore
/// # use anyhow::Result;
/// # use bytes::Bytes;
/// # use spin_sdk::redis_subscriber;
/// # use std::str::from_utf8;
/// #[redis_subscriber]
/// async fn on_message(message: Bytes) -> Result<()> {
///     println!("{}", from_utf8(&message)?);
///     Ok(())
/// }
/// ```
///
/// See <https://spinframework.dev/redis-trigger> for more information.
#[proc_macro_attribute]
pub fn redis_subscriber(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;

    if func.sig.asyncness.is_none() {
        return syn::Error::new_spanned(
            func.sig.fn_token,
            "the `#[redis_subscriber]` function must be `async`",
        )
        .to_compile_error()
        .into();
    }

    quote!(
        #func
        mod __spin_redis {
            mod preamble {
                #![allow(missing_docs)]
                ::spin_sdk::wit_bindgen::generate!({
                    world: "spin-sdk-macro-redis-trigger",
                    path: #WIT_PATH,
                    runtime_path: "::spin_sdk::wit_bindgen::rt",
                    generate_all,
                });
                pub struct Spin;
                export!(Spin);
            }
            impl self::preamble::exports::spin::redis::inbound_redis::Guest for preamble::Spin {
                async fn handle_message(msg: self::preamble::exports::spin::redis::inbound_redis::Payload) -> Result<(), self::preamble::spin::redis::redis::Error> {
                    match super::#func_name(msg.try_into().expect("cannot convert from Spin Redis payload")).await {
                        Ok(()) => Ok(()),
                        Err(e) => {
                            eprintln!("{}", e);
                            Err(self::preamble::spin::redis::redis::Error::Other(e.to_string()))
                        },
                    }
                }
            }
        }
    )
    .into()
}

/// Marks an `async fn` as an HTTP component entrypoint for Spin.
///
/// The `#[http_service]` attribute designates an asynchronous function as the
/// handler for incoming HTTP requests in a Spin component using the WASI Preview 3
/// (`wasip3`) HTTP ABI.  
///
/// When applied, this macro generates the necessary boilerplate to export the
/// function to the Spin runtime as a valid HTTP handler. The function must be
/// declared `async` and take a single argument implementing
/// [`FromRequest`], typically
/// [`Request`], and must return a type that
/// implements [`IntoResponse`].
///
/// # Requirements
///
/// - The annotated function **must** be `async`.
/// - The function’s parameter type must implement [`FromRequest`].
/// - The return type must implement [`IntoResponse`].
///
/// If the function is not asynchronous, the macro emits a compile-time error.
///
/// # Example
///
/// ```ignore
/// use spin_sdk::http::{,Request, IntoResponse};
/// use spin_sdk::http_service;
///
/// #[http_service]
/// async fn my_handler(request: Request) -> impl IntoResponse {
///   // Your logic goes here
/// }
/// ```
///
/// # Generated Code
///
/// The macro expands into a module containing a `Spin` struct that implements the
/// WASI `http.handler/Guest` interface, wiring the annotated function as the
/// handler’s entrypoint. This allows the function to be invoked automatically
/// by the Spin runtime when HTTP requests are received.
#[proc_macro_attribute]
pub fn http_service(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);

    if func.sig.asyncness.is_none() {
        return syn::Error::new_spanned(
            func.sig.fn_token,
            "the `#[http_service]` function must be `async`",
        )
        .to_compile_error()
        .into();
    }

    let func_name = &func.sig.ident;

    quote!(
        #func
        mod __spin_wasip3_http {
            use ::spin_sdk::http::IntoResponse;

            struct Spin;
            ::spin_sdk::wasip3::http::service::export!(Spin);

            impl ::spin_sdk::wasip3::exports::http::handler::Guest for self::Spin {
                async fn handle(request: ::spin_sdk::wasip3::http::types::Request) -> Result<::spin_sdk::wasip3::http::types::Response, ::spin_sdk::wasip3::http::types::ErrorCode> {
                    let request = <::spin_sdk::http::Request as ::spin_sdk::http::FromRequest>::from_request(request)?;
                    ::spin_sdk::http::IntoResponse::into_response(super::#func_name(request).await)
                }
            }
        }
    )
    .into()
}
