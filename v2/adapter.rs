//! Production compiled-in adapter for beta fd300e276356587726566026f072563eb9c34cb1.
//! Vendored unchanged beside src/. The host includes this file under extensions/.
#[path = "src/lib.rs"]
#[allow(dead_code)]
mod core;
use super::v2_sdk;
use crate::native_core::common::{CoreContext, CoreResult};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::Duration;

static WRITES: AtomicU64 = AtomicU64::new(0);
static COUNTER: AtomicU64 = AtomicU64::new(1);
static NONCE: OnceLock<String> = OnceLock::new();
static SESSIONS: OnceLock<Mutex<HashMap<String, Arc<Mutex<Session>>>>> = OnceLock::new();
fn token() -> String {
    format!(
        "b64:{}:{}",
        NONCE.get_or_init(|| uuid::Uuid::new_v4().to_string()),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}
fn sessions() -> &'static Mutex<HashMap<String, Arc<Mutex<Session>>>> {
    SESSIONS.get_or_init(Default::default)
}
#[derive(Clone)]
enum Reveal {
    NotAttempted,
    InProgress(String),
    Uncertain,
    Confirmed,
}
#[derive(Clone)]
enum Phase {
    Form,
    Writing,
    Written(core::Written, Reveal),
}
struct Session {
    submit: String,
    reveal: String,
    another: String,
    default_path: String,
    message: String,
    phase: Phase,
}
fn locked<T>(m: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, String> {
    m.lock()
        .map_err(|_| "Conversion state is unavailable; reopen the command".into())
}
fn get(state: &Value) -> Result<Arc<Mutex<Session>>, String> {
    let id = state["capability"]
        .as_str()
        .ok_or("Conversion session is unavailable")?;
    locked(sessions())?
        .get(id)
        .cloned()
        .ok_or("Conversion session is closed".into())
}
pub fn commands() -> &'static [&'static str] {
    &["index"]
}
pub async fn handle(
    ctx: &CoreContext,
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> CoreResult {
    if command != "index" {
        return Err("Unknown Base64 to File command".into());
    }
    match operation {
        "extension:run-command" => {
            if state.get("capability").is_some() {
                return Err("Conversion session already launched".into());
            }
            // Source has no arguments/preferences/clipboard initialization. Do not
            // repurpose payload.argumentValues or payload.preferences as form drafts.
            let default_path = ctx
                .paths
                .home_dir
                .join("Downloads")
                .join("out.pdf")
                .to_str()
                .ok_or("Home directory is not representable")?
                .to_owned();
            let session = Arc::new(Mutex::new(Session {
                submit: token(),
                reveal: token(),
                another: token(),
                default_path,
                message: "Convert base64 to a file. Existing regular files are replaced.".into(),
                phase: Phase::Form,
            }));
            let mut all = locked(sessions())?;
            if all.len() >= 128 {
                return Err("Too many conversion sessions; close a command first".into());
            }
            let capability = token();
            all.insert(capability.clone(), session.clone());
            *state = json!({"capability":capability});
            drop(all);
            let result = render(&*locked(&session)?);
            Ok(result)
        }
        "extension:dispose-session" => {
            if let Some(id) = state["capability"].as_str() {
                locked(sessions())?.remove(id);
            }
            *state = json!({});
            Ok(v2_sdk::unchanged())
        }
        "extension:search-text-changed" | "extension:load-more" => {
            get(state)?;
            Ok(v2_sdk::unchanged())
        }
        "extension:refresh-session" => {
            let session = get(state)?;
            let result = render(&*locked(&session)?);
            Ok(result)
        }
        "extension:invoke-action" => {
            let session = get(state)?;
            let (path, bytes, permit) = match prepare(&session, payload)? {
                Prepared::View(v) => return Ok(v),
                Prepared::Reveal => return reveal(ctx, session).await,
                Prepared::Write(p, b, permit) => (p, b, permit),
            };
            let receipt = session.clone();
            // The worker owns its receipt. It records publication before completion
            // is observable; cancelling the IPC future cannot lose an acknowledged
            // write or turn a later refresh into an implicit write retry.
            let worker = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                let result = core::write_file(&path, &bytes);
                if let Ok(mut s) = receipt.lock() {
                    match result {
                        Ok(written) => {
                            s.phase = Phase::Written(written, Reveal::NotAttempted);
                            s.message = "File saved. Show in Folder has not been confirmed.".into();
                        }
                        Err(message) => {
                            s.phase = Phase::Form;
                            s.message = format!("Error: {message}");
                        }
                    }
                }
            });
            if worker.await.is_err() {
                locked(&session)?.message = "Write outcome is uncertain; inspect the destination before reopening to authorize a new conversion".into();
                // Keep Writing: publication is uncertain. Only reopening and a
                // deliberate new conversion can authorize another write.
                return Err(
                    "Write outcome is uncertain; inspect the destination before a new conversion"
                        .into(),
                );
            }
            let saved = matches!(locked(&session)?.phase, Phase::Written(_, _));
            if saved {
                reveal(ctx, session).await
            } else {
                let value = render(&*locked(&session)?);
                Ok(value)
            }
        }
        _ => Err("Unsupported Base64 to File operation".into()),
    }
}
struct WritePermit;
impl WritePermit {
    fn acquire() -> Result<Self, String> {
        WRITES
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 4).then_some(n + 1)
            })
            .map(|_| Self)
            .map_err(|_| {
                "Four conversions are already writing; try again after one finishes".into()
            })
    }
}
impl Drop for WritePermit {
    fn drop(&mut self) {
        WRITES.fetch_sub(1, Ordering::AcqRel);
    }
}
enum Prepared {
    View(Value),
    Reveal,
    Write(std::path::PathBuf, Vec<u8>, WritePermit),
}
fn prepare(session: &Mutex<Session>, payload: &Value) -> Result<Prepared, String> {
    let action = payload["actionId"]
        .as_str()
        .ok_or("Missing action capability")?;
    let mut s = locked(session)?;
    if action == s.another
        && matches!(s.phase, Phase::Written(_, _))
        && !matches!(s.phase, Phase::Written(_, Reveal::InProgress(_)))
    {
        s.phase = Phase::Form;
        s.submit = token();
        s.another = token();
        s.message =
            "Ready for another conversion. Submitting replaces the selected regular file.".into();
        return Ok(Prepared::View(render(&s)));
    }
    if action == s.reveal
        && matches!(
            s.phase,
            Phase::Written(
                _,
                Reveal::NotAttempted | Reveal::Uncertain | Reveal::Confirmed
            )
        )
    {
        return Ok(Prepared::Reveal);
    }
    if action != s.submit || !matches!(s.phase, Phase::Form) {
        return Err("Stale or invalid conversion action; refresh the command".into());
    }
    // Consume before validation or any await; a duplicated submit can never
    // reuse this capability, including after a dropped future.
    s.submit = token();
    let values = payload["formValues"].as_object();
    let prepared = (|| {
        let values = values.ok_or("Submit both Base64 Text and File Path")?;
        if values.len() != 2 {
            return Err("Submit exactly Base64 Text and File Path".into());
        }
        let input = values
            .get("base64")
            .and_then(Value::as_str)
            .ok_or("Base64 Text is required")?;
        let path = values
            .get("filePath")
            .and_then(Value::as_str)
            .ok_or("File Path is required")?;
        if input.is_empty() || path.is_empty() {
            return Err("Base64 Text and File Path are required".into());
        }
        let permit = WritePermit::acquire()?;
        let bytes = core::decode(input)?;
        let path = core::validate_path(path)?;
        Ok::<_, String>((path, bytes, permit))
    })();
    let (path, bytes, permit) = match prepared {
        Ok(v) => v,
        Err(e) => {
            s.message = format!("Error: {e}");
            return Ok(Prepared::View(render(&s)));
        }
    };
    // A newly authorized destination also starts a new reveal/result capability
    // generation. Old result actions must not reveal a later file if this IPC is
    // dropped after writing but before automatic reveal.
    s.reveal = token();
    s.another = token();
    s.phase = Phase::Writing;
    s.message = "Writing file. The submitted destination is fixed for this conversion.".into();

    Ok(Prepared::Write(path, bytes, permit))
}
/// Strict route acknowledgements. Null, boolean, success-only objects and
/// contradictory responses are not confirmation, even if an OS effect happened.
fn acknowledged(value: &Value) -> bool {
    value.is_object()
        && value.get("ok") == Some(&json!(true))
        && value.get("error").is_none_or(Value::is_null)
        && value.get("success").is_none_or(|v| v == &json!(true))
}
struct RevealCheckpoint {
    session: Arc<Mutex<Session>>,
    attempt: String,
}
impl Drop for RevealCheckpoint {
    fn drop(&mut self) {
        if let Ok(mut s) = self.session.lock() {
            if let Phase::Written(_, Reveal::InProgress(attempt)) = &s.phase {
                if attempt == &self.attempt {
                    if let Phase::Written(w, _) = s.phase.clone() {
                        s.phase = Phase::Written(w, Reveal::Uncertain);
                    }
                    s.message="File saved. Show in Folder failed or was not confirmed; its outcome is uncertain. Retry only when wanted; it will not rewrite the file.".into();
                }
            }
        }
    }
}
async fn reveal(ctx: &CoreContext, session: Arc<Mutex<Session>>) -> CoreResult {
    let (path, attempt) = {
        let mut s = locked(&session)?;
        let Phase::Written(w, status) = s.phase.clone() else {
            return Err("No saved file to reveal".into());
        };
        if matches!(status, Reveal::InProgress(_)) {
            return Err("Show in Folder is already in progress".into());
        }
        let path = w
            .path
            .to_str()
            .ok_or("Saved path is not representable")?
            .to_owned();
        let attempt = token();
        s.reveal = token();
        s.phase = Phase::Written(w, Reveal::InProgress(attempt.clone()));
        s.message = "File saved. Showing it in its folder…".into();
        (path, attempt)
    };
    let checkpoint = RevealCheckpoint {
        session: session.clone(),
        attempt,
    };
    let response = tokio::time::timeout(
        Duration::from_secs(15),
        ctx.request("core:platform:reveal-file", vec![json!(path)]),
    )
    .await;
    if matches!(response, Ok(Ok(ref value)) if acknowledged(value)) {
        let mut s = locked(&session)?;
        if let Phase::Written(w, _) = s.phase.clone() {
            s.phase = Phase::Written(w, Reveal::Confirmed);
        }
        s.message = "File saved. The host acknowledged Show in Folder.".into();
    }
    drop(checkpoint);
    let value = render(&*locked(&session)?);
    Ok(value)
}
fn render(s: &Session) -> Value {
    let action = |id: &str, title: &str, kind: &str| json!({"id":id,"title":title,"kind":kind});
    let mut actions = Vec::new();
    match &s.phase {
        Phase::Form => actions.push(action(&s.submit, "Convert", "submit-form")),
        Phase::Written(_, Reveal::InProgress(_)) | Phase::Writing => {}
        Phase::Written(_, status) => {
            actions.push(action(
                &s.reveal,
                if matches!(status, Reveal::Uncertain) {
                    "Retry Show in Folder"
                } else {
                    "Show in Folder"
                },
                "action",
            ));
            actions.push(action(&s.another, "Convert Another File", "action"));
        }
    }
    let mut message = s.message.clone();
    if let Phase::Written(w, _) = &s.phase {
        message.push_str(&format!(
            " Saved {} bytes to {}.",
            w.bytes,
            w.path.display()
        ));
    }
    // Only the stable default is supplied. No on-change actionIds and no echoed
    // submitted values: reconciliation retains even a draft typed during an await.
    let mut result = v2_sdk::view(
        "Convert Base64 to File",
        json!({"type":"Form","props":{},"children":[
            {"type":"Form.Description","props":{"text":message}},
            {"type":"Form.TextArea","props":{"id":"base64","title":"Base64 Text","placeholder":"Please enter the string encoded in base64"}},
            {"type":"Form.TextField","props":{"id":"filePath","title":"File Path","value":s.default_path,"placeholder":"Please enter the file path to convert to"}}
        ]}),
        actions,
    );
    // The pinned outer runtime keeps its Working notice unless a response message
    // replaces it. This is supported response feedback, not a toast/HUD effect.
    result["message"] = json!(message);
    result
}
