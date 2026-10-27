# Base64 to File — Rust Native V2

One command, `index` / **Convert Base64 to File**, for `com.tezbar.native.base64-to-file`, replacing `raycast.base64-to-file`. Conversion runs offline in Rust. The original V1 implementation remains outside this directory. Original extension: **forrestzh**; own V1 fork: almatkai.

This is a compiled-in beta adapter, not a Rust plugin loader. Copying or installing the manifest cannot make its code run. Host vendoring, registration, a binary rebuild and real desktop QA remain separate work. Both manifest verification states intentionally remain `not-run`.

The form has Base64 Text, File Path (default `<host home>/Downloads/out.pdf`) and Convert. It preserves newer drafts across validation, write errors and delayed responses. A successful write is recorded separately from Show in Folder. A reveal failure offers **Retry Show in Folder** without rewriting; **Convert Another File** deliberately re-arms a new conversion. Neither automatic refresh nor duplicated/stale action IDs can repeat a write.

The decoder follows the executable own V1 `atob` oracle: standard alphabet, optional canonical padding, the five ASCII whitespaces, and ignored noncanonical trailing bits. Whitespace-only input writes an empty file; an empty field is required and cannot submit. Binary bytes remain exact. No data URL stripping or URL-safe alphabet is added. No clipboard initialization, external runtime, network access, shell interpolation, HUD, window hiding, or automatic dismissal is used.

Filesystem adaptations are deliberate:

- At most 24 MiB encoded text, 16 MiB decoded bytes, 4096 UTF-8 bytes per path, four in-flight writes and 128 sessions. Oversized input fails, never truncates. File I/O uses blocking workers; the OS can still stall a worker.
- Only explicit local absolute paths are accepted; control characters, dot/parent components and directory-only paths are rejected. Windows also rejects UNC, verbatim paths, device aliases and alternate streams. No tilde/environment expansion or parent-directory creation. The home-derived Downloads directory must already exist.
- Existing regular files are replaced, preserving their permissions. Unlike V1's direct truncation, a private staging file is flushed/synced and atomically renamed using an opened directory capability. Failed writes preserve the old destination and attempt to remove their private stage (cleanup can fail on OS errors or process death). New Unix files are mode 0600. Final links/special files and read-only destinations are rejected. A parent alias is canonicalized once; its canonical destination is validated before writing. Windows verbatim canonical drive prefixes are converted to ordinary drive paths for Explorer. Raced final links are never followed.
- Atomic replacement changes the inode: hard-link siblings keep their old content; ownership/ACLs, extended attributes and inode identity are not promised. Directory write permission is needed. Concurrent writers remain last-publication-wins; unrelated external changes after publication can affect what the canonical path reveals. The directory entry is not fsynced, so no crash/power-loss durability guarantee is made.

V1's LocalStorage hooks read and save React state, but displayed fields have no state binding/onChange and submit does not update that state. V2 intentionally does not persist drafts or read/migrate V1 storage. `payload.argumentValues` and `payload.preferences` are the beta contracts; V1 defines neither inputs nor preferences, so they are not repurposed as drafts. There are no field callbacks in this submit-only form. Beta callbacks would carry `formValues.value`; submission carries the complete field map.

The adapter uses the real pinned `core:platform:reveal-file` route with `[canonicalWrittenPath]`. It accepts only an object with `ok:true`, absent/null `error`, and absent/true `success`. Failure, null, booleans, contradictions, malformed replies, timeout and transport errors remain unconfirmed. On Windows the host acknowledgement means Explorer was spawned, not that its visible selection was verified. Conversion feedback uses supported Form.Description text and response messages (which replace the beta runtime’s persistent Working notice), not a genuine toast/HUD. Beta supports its built-in submit/action controls; arbitrary shortcuts and tooltips are not declared.

Action capabilities contain a process UUID nonce plus monotonic counter. Search/load-more return unchanged; refresh only renders committed IDs without modifying state. A dropped search/refresh clone cannot mint a reusable ID. Session closure invalidates capabilities. A worker checkpoints write intent before the await and owns the in-process receipt, retaining successful publication if IPC cancellation follows. Closing the form does not revoke a write already authorized on submit. Reveal intent is checkpointed before awaiting its route; cancellation/timeout requires deliberate retry. Receipts are not a cross-process journal: after a crash, inspect the destination before authorizing another conversion.

Read [INTEGRATION.md](INTEGRATION.md) for host registration and [VALIDATION.md](VALIDATION.md) for results, stubs and gaps. [PROVENANCE.json](PROVENANCE.json) inventories immutable fixtures and exact hashes; [NOTICES.md](NOTICES.md) identifies original licenses and evidence gaps.

Reproduce from this directory (Rust stable and 1.85.0, Node, Python, and the installed Windows GNU Rust target are prerequisites):

```sh
python3 scripts/prepare-offline.py
bash scripts/validate.sh
python3 scripts/package-repro.py
```

Cargo excludes nested Rust packages even when an include glob names them. The checksum-inventoried `fixtures/offline-support.tar.gz` therefore carries the unmodified vendored registry sources, pinned beta Rust fixtures and harness. `prepare-offline.py` verifies and expands these locally, refusing to overwrite changed sources. Registry licenses/checksums are retained inside that archive; no dependency cache or original host path is required. The Node oracle, renderer and full-schema validator are also shipped with genuine library source/license fixtures. No JavaScript is used by the production extension.
