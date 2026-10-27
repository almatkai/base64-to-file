//! Explicit OS stub: no window, emission, profile or desktop operations.
#[derive(Clone)]
pub struct AppHandle;
pub trait Emitter {
    fn emit(&self, _: &str, _: serde_json::Value) -> Result<(), String> { Ok(()) }
}
impl Emitter for AppHandle {}
