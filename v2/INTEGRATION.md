# Separate host integration

Reference: `fd300e276356587726566026f072563eb9c34cb1`. The unrelated host checkout was read through Git objects only. No host/profile files were changed. Host `origin/main` is not the implementation baseline.

1. Choose and review the intended host task branch separately. Preserve earlier sequential Native V2 ports. Vendor this directory to **`src-tauri/native-extensions/base64-to-file`**, preserving at least `adapter.rs`, `src/lib.rs`, manifest, assets, notices/licenses and provenance. Copy the rest if in-host reproducibility is wanted; avoid build outputs. The adapter includes its own core by relative path, so it never depends on this temporary clone.
2. Generate a patch against the *current chosen host tree*: `python3 integration/registration.py /path/to/chosen/host > /tmp/base64-registration.patch`. The generator reads only and emits a diff; it never applies changes. Review and apply in the separately authorized host task. It refuses duplicate registration or an unexpected catalog count/anchor. `integration/pinned-beta-registration.patch` is the reviewable pinned example, not a replacement for later sequential changes.
3. The patch adds the stable module below, runtime slug recognition, `commands()` / `handle()` dispatch, the complete schema2 catalog entry, the existing count + 1, and `cap-std = "=3.4.5"`. At the pristine pin, 11 entries become 12. At a tree with earlier ports, their entries, dispatch branches, modules and count are retained. The existing beta catalog test checks canonical IDs, replacement aliases, declared command count and compiled coverage. `v2_catalog.rs` derives coverage from dispatch and platform; the JSON alone is insufficient.

```rust
#[path = "../../../native-extensions/base64-to-file/adapter.rs"]
mod v2_base64_to_file;
```

The module lives in `src-tauri/src/native_core/extensions/mod.rs`. It imports the sibling `super::v2_sdk` and the real `crate::native_core::common::CoreContext`. Its dependencies are the host's existing `serde_json`, `tokio` (rt/sync/time), `uuid` (v4), plus `cap-std`. Keep host dependency updates/Cargo.lock regeneration separate and inspect their diff. The host's own lock is not supplied or modified here.

4. Copy **original** `assets/command-icon.png` to `src/renderer/public/extensions/icons/base64-to-file.png`. Its public URL is `/extensions/icons/base64-to-file.png`; size 13,421; SHA256 `e1a8868a61378242995331e9d156f8c53f70adb62fe04cc7cf831bcec757eb27`. Do not optimize/recolor/redesign it. The patch is textual and intentionally does not embed the binary copy.
5. Rebuild the actual host, run its catalog/compiled-coverage tests and native CLI listing, then install the canonical package through its supported install path. Preserve the old own V1 `com.tezbar.base64-to-file` and Raycast package. Do not edit the profile manually.

Alias policy: the manifest replaces `raycast.base64-to-file`. The beta runtime recognizes canonical, Raycast and raw slug forms. The real registry chooses compiled canonical dispatch for Raycast requests only when the native package/command is installed and covered; otherwise V1 can coexist/fall back. The own V1 `com.tezbar.base64-to-file` ID is not silently aliased or removed. Native installation uses the normal marker + manifest/database policy; the test harness stubs that policy and is not evidence of installation QA. This extension has no preferences or useful persistent draft state to migrate.

The production route is already present in the pinned platform core:

```rust
ctx.request("core:platform:reveal-file", vec![json!(canonical_written_path)]).await
```

It calls `/usr/bin/open -R <path>` on macOS and spawns `explorer.exe` with one `/select,<path>` argument on Windows, then returns `{ "ok": true }`. The adapter does not invoke either executable itself. No additional reveal host patch is required at this pin. No `window:hide`, clipboard, focus, paste or shell-open route is needed. Registration/rebuild, real macOS/Windows filesystem/UI/reveal/focus/profile QA and Windows execution remain **NOT-RUN**. Both verification states must stay `not-run` until those separate checks have evidence.
