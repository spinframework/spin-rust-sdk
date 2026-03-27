//! Various WASI APIs

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[doc(hidden)]
/// Various WASI APIs
pub mod wit {
    #![allow(missing_docs)]

    wit_bindgen::generate!({
        world: "spin-sdk-wasi",
        path: "../../wit",
        generate_all,
    });
}
