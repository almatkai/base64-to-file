use super::common::{CoreContext, CoreResult};
use serde_json::Value;
use std::sync::{Arc, Mutex};
#[path = "../../../../../adapter.rs"]
pub mod adapter;
#[path = "../../../../../fixtures/beta/src-tauri/src/native_core/extensions/native.rs"]
#[rustfmt::skip]
mod native;
#[path = "../../../../../fixtures/beta/src-tauri/src/native_core/extensions/v2_sdk.rs"]
#[rustfmt::skip]
pub mod v2_sdk;
mod registry {
    use super::*;
    pub fn installed_command(_: &CoreContext, id: &str, command: &str) -> Option<Value> {
        (id == "com.tezbar.native.base64-to-file" && command == "index")
            .then(|| serde_json::json!({}))
    }
    pub fn preferences(_: &Value, _: Option<&str>) -> CoreResult {
        Ok(serde_json::json!({}))
    }
}
mod v2_runtime {
    use super::*;
    pub fn slug(id: &str) -> Option<&str> {
        (id == "com.tezbar.native.base64-to-file").then_some("base64-to-file")
    }
    pub fn supports(id: &str, command: &str) -> bool {
        slug(id).is_some() && adapter::commands().contains(&command)
    }
    pub async fn handle(
        ctx: &CoreContext,
        _: &str,
        command: &str,
        op: &str,
        p: &Value,
        s: &mut Value,
    ) -> CoreResult {
        adapter::handle(ctx, command, op, p, s).await
    }
}
#[derive(Default)]
pub struct Host {
    sessions: native::NativeSessions,
    files: Arc<Mutex<()>>,
}
impl Host {
    pub async fn call(&self, ctx: &CoreContext, channel: &str, body: Value) -> Value {
        self.sessions
            .handle(ctx, &self.files, channel, &[body])
            .await
            .expect("handled")
            .expect("result")
    }
    pub async fn clear(&self) {
        self.sessions.clear().await;
    }
}
