use proc_macro::TokenStream;
use quote::quote;

/// Marks an `async fn` as an HTTP component entrypoint for Spin.
///
/// The `#[http_service]` attribute designates an asynchronous function as the
/// handler for incoming HTTP requests in a Spin component using the WASI Preview 3
/// (`wasip3`) HTTP ABI.  
///
/// When applied, this macro generates the necessary boilerplate to export the
/// function to the Spin runtime as a valid HTTP handler. The function must be
/// declared `async` and take a single argument implementing
/// [`FromRequest`](::spin_sdk::http_wasip3::FromRequest), typically
/// [`Request`](::spin_sdk::http_wasip3::Request), and must return a type that
/// implements [`IntoResponse`](::spin_sdk::http_wasip3::IntoResponse).
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
/// use spin_sdk::http_wasip3::{http_service, Request, IntoResponse};
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
            "the `#[http_component]` function must be `async`",
        )
        .to_compile_error()
        .into();
    }

    let func_name = &func.sig.ident;

    quote!(
        #func
        mod __spin_wasip3_http {
            use ::spin_sdk::http_wasip3::IntoResponse;

            struct Spin;
            ::spin_sdk::http_wasip3::wasip3::http::proxy::export!(Spin);

            impl ::spin_sdk::http_wasip3::wasip3::exports::http::handler::Guest for self::Spin {
                async fn handle(request: ::spin_sdk::http_wasip3::wasip3::http::types::Request) -> Result<::spin_sdk::http_wasip3::wasip3::http::types::Response, ::spin_sdk::http_wasip3::wasip3::http::types::ErrorCode> {
                    let request = <::spin_sdk::http_wasip3::Request as ::spin_sdk::http_wasip3::FromRequest>::from_request(request)?;
                    ::spin_sdk::http_wasip3::IntoResponse::into_response(super::#func_name(request).await)
                }
            }
        }
    )
    .into()
}
