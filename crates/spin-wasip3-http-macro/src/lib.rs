use proc_macro::TokenStream;
use quote::quote;

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
