use wasm_bindgen::prelude::*;

use crate::evaluate;

#[wasm_bindgen]
extern "C" {
    pub fn evaluate3(xml: &str, xpath: &str, default_element_namespace: Option<&str>);
}

#[wasm_bindgen]
pub fn evaluate2(xml: &str, xpath: &str) {
    evaluate(xml, xpath, None);
}
