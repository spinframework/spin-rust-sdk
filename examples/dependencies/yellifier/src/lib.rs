spin_sdk::wit_bindgen::generate!({
    runtime_path: "::spin_sdk::wit_bindgen::rt",
});

struct Yellifier;

impl exports::loudness_services::yelling::yelling::Guest for Yellifier {
    fn yell(text: String) -> String {
        text.to_uppercase()
    }
}

export!(Yellifier);
