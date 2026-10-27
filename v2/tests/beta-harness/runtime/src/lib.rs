//! Explicit harness: pinned common/SDK/NativeSessions unchanged; only Tauri and installation policy stubbed.
#![allow(dead_code)]
pub mod native_core {
    pub mod common {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/beta/src-tauri/src/native_core/common.rs"
        ));
    }
    pub mod extensions;
}
pub use native_core::{
    common::{CoreContext, CorePaths},
    extensions::{adapter, Host},
};
