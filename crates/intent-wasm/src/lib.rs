//! WebAssembly bridge: lets a web page drive the Intent engine.
//!
//! The page talks to [`IntentSession`]. Every method returns JSON text, which
//! keeps the boundary simple and the same on every browser. All behaviour
//! lives in [`view::Demo`], which is plain Rust and tested natively; this
//! file only converts to and from what JavaScript can hold.

pub mod view;

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::view::{Demo, TurnInput};

fn json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("view types always serialise")
}

fn js_error(message: impl AsRef<str>) -> JsError {
    JsError::new(message.as_ref())
}

/// A chord input session for one page.
#[wasm_bindgen]
pub struct IntentSession {
    demo: Demo,
}

#[wasm_bindgen]
impl IntentSession {
    /// Compiles a layout file and starts a session. Throws if the layout has errors.
    #[wasm_bindgen(constructor)]
    pub fn new(layout_toml: &str) -> Result<IntentSession, JsError> {
        Demo::new(layout_toml)
            .map(|demo| IntentSession { demo })
            .map_err(js_error)
    }

    /// Everything static about the layout, as JSON.
    pub fn layout(&self) -> String {
        json(&self.demo.layout())
    }

    /// A key went down. `key` is 0 (left pinky) to 9 (right pinky).
    pub fn press(&mut self, key: u8, at_ms: f64) -> Result<String, JsError> {
        self.demo
            .press(key, at_ms)
            .map(|update| json(&update))
            .map_err(js_error)
    }

    /// A key came up.
    pub fn release(&mut self, key: u8, at_ms: f64) -> Result<String, JsError> {
        self.demo
            .release(key, at_ms)
            .map(|update| json(&update))
            .map_err(js_error)
    }

    /// Applies a chord written in notation, such as `I-M`.
    pub fn strike(&mut self, chord: &str, at_ms: f64) -> Result<String, JsError> {
        self.demo
            .strike(chord, at_ms)
            .map(|update| json(&update))
            .map_err(js_error)
    }

    /// Drops any stroke in progress. Call when the page loses focus.
    #[wasm_bindgen(js_name = releaseAll)]
    pub fn release_all(&mut self) -> String {
        json(&self.demo.release_all())
    }

    /// The current state, without changing anything.
    pub fn state(&self) -> String {
        json(&self.demo.state())
    }

    /// Chat messages asking a model to decode the last sent utterance.
    ///
    /// `history_json` is a JSON array of `{ "utterance", "text" }` objects.
    #[wasm_bindgen(js_name = decodeMessages)]
    pub fn decode_messages(&self, history_json: &str, language: &str) -> Result<String, JsError> {
        let history: Vec<TurnInput> = serde_json::from_str(history_json)
            .map_err(|error| js_error(format!("history is not valid: {error}")))?;
        self.demo
            .decode_messages(&history, language)
            .map(|messages| json(&messages))
            .map_err(js_error)
    }
}

/// Tidies a model's reply: trims it and removes one pair of wrapping quotes.
#[wasm_bindgen(js_name = cleanReply)]
pub fn clean_reply(reply: &str) -> String {
    intent_core::prompt::clean(reply)
}
