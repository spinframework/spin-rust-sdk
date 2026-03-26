//! Various WASI APIs

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[doc(hidden)]
/// Module containing wit bindgen generated code.
///
/// This is only meant for internal consumption.
pub mod wit {
    #![allow(missing_docs)]

    wit_bindgen::generate!({
        world: "imports",
        path: "./wit",
        generate_all,
    });
}
