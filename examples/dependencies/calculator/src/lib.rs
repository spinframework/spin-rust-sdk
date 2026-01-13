spin_sdk::wit_bindgen::generate!({
    runtime_path: "::spin_sdk::wit_bindgen::rt",
});

struct Calculator;

impl exports::calculator::calc::addition::Guest for Calculator {
    fn add(a: i32, b:i32) -> i32 {
        a + b
    }
}

export!(Calculator);
