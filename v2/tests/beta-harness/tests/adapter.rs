use beta_harness::{adapter, CoreContext, CorePaths, Host};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
static TEST_LANE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
struct Fixture {
    dir: tempfile::TempDir,
    ctx: CoreContext,
    host: Arc<Host>,
    replies: Arc<Mutex<VecDeque<Result<Value, String>>>>,
    reveals: Arc<Mutex<Vec<Vec<Value>>>>,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let ctx = CoreContext::new(
            CorePaths {
                data_dir: p.join("data"),
                config_dir: p.join("config"),
                resource_dir: p.join("resources"),
                temp_dir: p.join("tmp"),
                home_dir: p.into(),
            },
            None,
        );
        let replies = Arc::new(Mutex::new(VecDeque::new()));
        let reveals = Arc::new(Mutex::new(Vec::new()));
        let r = replies.clone();
        let calls = reveals.clone();
        ctx.set_router(Arc::new(move |channel, args| {
            let r = r.clone();
            let calls = calls.clone();
            Box::pin(async move {
                assert_eq!(channel, "core:platform:reveal-file");
                assert_eq!(args.len(), 1);
                assert!(std::path::Path::new(args[0].as_str().unwrap()).is_file());
                calls.lock().unwrap().push(args);
                r.lock()
                    .unwrap()
                    .pop_front()
                    .unwrap_or(Ok(json!({"ok":true})))
            })
        }))
        .unwrap();
        Self {
            dir,
            ctx,
            host: Arc::new(Host::default()),
            replies,
            reveals,
        }
    }
    async fn call(&self, op: &str, body: Value) -> Value {
        self.host
            .call(&self.ctx, &format!("extension:{op}"), body)
            .await
    }
    async fn launch(&self) -> Value {
        self.call("run-command",json!({"extensionId":"com.tezbar.native.base64-to-file","commandName":"index","argumentValues":{"base64":"ignored"},"preferences":{"base64":"ignored"}})).await
    }
    async fn submit(&self, view: &Value, input: Value, path: Value) -> Value {
        self.call("invoke-action",json!({"sessionId":view["sessionId"],"actionId":view["actions"][0]["id"],"formValues":{"base64":input,"filePath":path}})).await
    }
    async fn refresh(&self, view: &Value) -> Value {
        self.call("refresh-session", json!({"sessionId":view["sessionId"]}))
            .await
    }
    async fn action(&self, view: &Value, index: usize) -> Value {
        self.call(
            "invoke-action",
            json!({"sessionId":view["sessionId"],"actionId":view["actions"][index]["id"]}),
        )
        .await
    }
}
fn message(v: &Value) -> &str {
    v["root"]["children"][0]["props"]["text"].as_str().unwrap()
}
#[tokio::test]
async fn actual_native_sessions_binary_write_reveal_once_and_stale_cross_session() {
    let _lane = TEST_LANE.lock().await;
    let f = Fixture::new();
    let view = f.launch().await;
    assert_eq!(view["root"]["type"], "Form");
    assert_eq!(view["actions"][0]["kind"], "submit-form");
    assert!(view["root"]["children"][1]["props"].get("value").is_none());
    assert!(view["root"]["children"][1]["props"]
        .get("actionId")
        .is_none());
    assert_eq!(
        view["root"]["children"][2]["props"]["value"],
        f.dir.path().join("Downloads/out.pdf").to_str().unwrap()
    );
    let refresh = f.refresh(&view).await;
    assert_eq!(refresh["actions"], view["actions"]);
    let other = f.launch().await;
    let forged=f.call("invoke-action",json!({"sessionId":other["sessionId"],"actionId":view["actions"][0]["id"],"formValues":{"base64":"AA==","filePath":f.dir.path().join("forged")}})).await;
    assert_eq!(forged["ok"], false);
    let p = f.dir.path().join("out.bin");
    std::fs::write(&p, b"long before").unwrap();
    let saved = f.submit(&view, json!("AP+A"), json!(p)).await;
    assert!(message(&saved).contains("host acknowledged"));
    assert_eq!(std::fs::read(&p).unwrap(), [0, 255, 128]);
    assert_eq!(f.reveals.lock().unwrap().len(), 1);
    std::fs::write(&p, b"edited externally").unwrap();
    assert_eq!(f.submit(&view, json!("Zg=="), json!(p)).await["ok"], false);
    let again = f.action(&saved, 0).await;
    assert!(message(&again).contains("host acknowledged"));
    assert_eq!(std::fs::read(&p).unwrap(), b"edited externally");
    let new = f.action(&again, 1).await;
    let overwritten = f.submit(&new, json!("Zg=="), json!(p)).await;
    assert!(message(&overwritten).contains("host acknowledged"));
    assert_eq!(std::fs::read(&p).unwrap(), b"f");
    assert_eq!(
        f.call("dispose-session", json!({"sessionId":view["sessionId"]}))
            .await,
        true
    );
    assert_eq!(f.refresh(&view).await["ok"], false);
    f.host.clear().await;
}
#[tokio::test]
async fn malformed_inputs_validation_io_errors_preserve_drafts_and_never_reveal() {
    let _lane = TEST_LANE.lock().await;
    let f = Fixture::new();
    let mut v = f.launch().await;
    for (input, path) in [
        (json!(""), json!("/x")),
        (json!("AA=="), json!("")),
        (Value::Null, json!("/x")),
        (json!("-_8="), json!("/x")),
        (json!("AA=="), json!("relative")),
        (json!("AA=="), json!(f.dir.path().join("absent/out"))),
    ] {
        let next = f.submit(&v, input, path).await;
        assert_eq!(next["ok"], true);
        assert_eq!(next["actions"][0]["title"], "Convert");
        assert!(next["root"]["children"][1]["props"].get("value").is_none());
        assert_ne!(next["actions"][0]["id"], v["actions"][0]["id"]);
        v = next;
    }
    assert!(f.reveals.lock().unwrap().is_empty());
    assert!(!f.dir.path().join("data").exists());
    let saved = f
        .submit(&v, json!(" \t\n\r\u{c}"), json!(f.dir.path().join("empty")))
        .await;
    assert_eq!(saved["ok"], true);
    assert_eq!(
        std::fs::metadata(f.dir.path().join("empty")).unwrap().len(),
        0
    );
    f.host.clear().await;
}
#[tokio::test]
async fn strict_acknowledgements_reveal_retry_never_rewrites() {
    let _lane = TEST_LANE.lock().await;
    let f = Fixture::new();
    for reply in [
        Ok(Value::Null),
        Ok(json!(true)),
        Ok(json!(false)),
        Ok(json!({"success":true})),
        Ok(json!({"ok":true,"error":"failure"})),
        Ok(json!({"ok":true,"success":false})),
        Ok(json!({"ok":true,"success":null})),
        Ok(json!({"ok":"true"})),
        Err("transport failure and private details".into()),
    ] {
        let v = f.launch().await;
        let p = f.dir.path().join("out");
        f.replies.lock().unwrap().push_back(reply);
        let saved = f.submit(&v, json!("Zg=="), json!(p)).await;
        assert!(message(&saved).contains("uncertain"));
        assert!(!message(&saved).contains("private details"));
        assert_eq!(saved["actions"][0]["title"], "Retry Show in Folder");
        std::fs::write(&p, b"external").unwrap();
        f.replies
            .lock()
            .unwrap()
            .push_back(Ok(json!({"ok":true,"error":null,"success":true})));
        let success = f.action(&saved, 0).await;
        assert!(message(&success).contains("host acknowledged"));
        assert_eq!(std::fs::read(&p).unwrap(), b"external");
        assert_eq!(f.action(&saved, 0).await["ok"], false);
        f.call("dispose-session", json!({"sessionId":v["sessionId"]}))
            .await;
    }
    f.host.clear().await;
}
#[tokio::test]
async fn cloned_search_refresh_state_and_token_lifecycle() {
    let _lane = TEST_LANE.lock().await;
    let f = Fixture::new();
    let mut state = json!({});
    let first = adapter::handle(
        &f.ctx,
        "index",
        "extension:run-command",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    let original = state.clone();
    for op in [
        "extension:refresh-session",
        "extension:search-text-changed",
        "extension:load-more",
    ] {
        let mut staged = state.clone();
        let result = adapter::handle(&f.ctx, "index", op, &json!({}), &mut staged)
            .await
            .unwrap();
        assert_eq!(staged, original);
        if op.ends_with("refresh-session") {
            assert_eq!(result["actions"], first["actions"]);
        }
    }
    assert!(adapter::handle(
        &f.ctx,
        "index",
        "extension:invoke-action",
        &json!({"actionId":"b64:forged:1","formValues":{"value":"x"}}),
        &mut state
    )
    .await
    .is_err());
    adapter::handle(
        &f.ctx,
        "index",
        "extension:dispose-session",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    let second = adapter::handle(
        &f.ctx,
        "index",
        "extension:run-command",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert_ne!(first["actions"], second["actions"]);
    adapter::handle(
        &f.ctx,
        "index",
        "extension:dispose-session",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn cancelled_reveal_retains_saved_receipt_and_requires_deliberate_retry() {
    let _lane = TEST_LANE.lock().await;
    let d = tempfile::tempdir().unwrap();
    let p = d.path();
    let ctx = CoreContext::new(
        CorePaths {
            data_dir: p.join("data"),
            config_dir: p.join("config"),
            resource_dir: p.into(),
            temp_dir: p.into(),
            home_dir: p.into(),
        },
        None,
    );
    let entered = Arc::new(tokio::sync::Notify::new());
    let e = entered.clone();
    ctx.set_router(Arc::new(move |channel, _| {
        assert_eq!(channel, "core:platform:reveal-file");
        let e = e.clone();
        Box::pin(async move {
            e.notify_one();
            std::future::pending().await
        })
    }))
    .unwrap();
    let host = Arc::new(Host::default());
    let v = host
        .call(
            &ctx,
            "extension:run-command",
            json!({"extensionId":"com.tezbar.native.base64-to-file","commandName":"index"}),
        )
        .await;
    let body = json!({"sessionId":v["sessionId"],"actionId":v["actions"][0]["id"],"formValues":{"base64":"AP+A","filePath":p.join("out")}});
    let h = host.clone();
    let c = ctx.clone();
    let t = tokio::spawn(async move { h.call(&c, "extension:invoke-action", body).await });
    tokio::time::timeout(Duration::from_secs(3), entered.notified())
        .await
        .unwrap();
    t.abort();
    assert!(t.await.unwrap_err().is_cancelled());
    let result = host
        .call(
            &ctx,
            "extension:refresh-session",
            json!({"sessionId":v["sessionId"]}),
        )
        .await;
    assert!(message(&result).contains("uncertain"));
    assert_eq!(std::fs::read(p.join("out")).unwrap(), [0, 255, 128]);
    host.clear().await;
}
#[test]
fn cancelled_write_future_worker_still_records_completion() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let _lane = TEST_LANE.lock().await;
        let f = Fixture::new();
        let mut state = json!({});
        let launch = adapter::handle(
            &f.ctx,
            "index",
            "extension:run-command",
            &json!({}),
            &mut state,
        )
        .await
        .unwrap();
        let prior = adapter::handle(
            &f.ctx,
            "index",
            "extension:invoke-action",
            &json!({
                "actionId":launch["actions"][0]["id"],
                "formValues":{"base64":"YQ==","filePath":f.dir.path().join("first")}
            }),
            &mut state,
        )
        .await
        .unwrap();
        let stale_reveal = prior["actions"][0]["id"].clone();
        let view = adapter::handle(
            &f.ctx,
            "index",
            "extension:invoke-action",
            &json!({
                "actionId":prior["actions"][1]["id"]
            }),
            &mut state,
        )
        .await
        .unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let blocker = tokio::task::spawn_blocking(move || {
            ready_tx.send(()).unwrap();
            rx.recv().unwrap();
        });
        ready_rx.await.unwrap();
        let c = f.ctx.clone();
        let mut staged = state.clone();
        let body = json!({"actionId":view["actions"][0]["id"],"formValues":{
            "base64":"Zg==","filePath":f.dir.path().join("out")
        }});
        let task = tokio::spawn(async move {
            adapter::handle(&c, "index", "extension:invoke-action", &body, &mut staged).await
        });
        let mut observed = false;
        for _ in 0..100 {
            let v = adapter::handle(
                &f.ctx,
                "index",
                "extension:refresh-session",
                &json!({}),
                &mut state,
            )
            .await
            .unwrap();
            if message(&v).contains("Writing file") {
                observed = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        assert!(observed);
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        tx.send(()).unwrap();
        blocker.await.unwrap();
        let mut saved = None;
        for _ in 0..100 {
            let v = adapter::handle(
                &f.ctx,
                "index",
                "extension:refresh-session",
                &json!({}),
                &mut state,
            )
            .await
            .unwrap();
            if message(&v).contains("File saved") {
                saved = Some(v);
                break;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        let saved = saved.unwrap();
        assert!(message(&saved).contains("not been confirmed"));
        assert_eq!(f.reveals.lock().unwrap().len(), 1);
        // A reveal capability issued for the previous destination must never
        // revive for the later file after a cancelled write IPC.
        assert!(adapter::handle(
            &f.ctx,
            "index",
            "extension:invoke-action",
            &json!({
                "actionId":stale_reveal
            }),
            &mut state
        )
        .await
        .is_err());
        let p = f.dir.path().join("out");
        std::fs::write(&p, b"external").unwrap();
        adapter::handle(
            &f.ctx,
            "index",
            "extension:invoke-action",
            &json!({
                "actionId":saved["actions"][0]["id"]
            }),
            &mut state,
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(p).unwrap(), b"external");
        adapter::handle(
            &f.ctx,
            "index",
            "extension:dispose-session",
            &json!({}),
            &mut state,
        )
        .await
        .unwrap();
    });
}

#[tokio::test]
async fn export_renderer_samples_when_requested() {
    let _lane = TEST_LANE.lock().await;
    let Ok(output) = std::env::var("TEZBAR_ADAPTER_VIEWS") else {
        return;
    };
    let f = Fixture::new();
    let initial = f.launch().await;
    let validation = f.submit(&initial, json!(""), json!("/x")).await;
    let saved = f
        .submit(&validation, json!("AP+A"), json!(f.dir.path().join("out")))
        .await;
    f.replies.lock().unwrap().push_back(Ok(Value::Null));
    let reveal_failure = f.action(&saved, 0).await;
    let refreshed = f.refresh(&reveal_failure).await;
    std::fs::write(output,serde_json::to_vec_pretty(&json!({"initial":initial,"validation":validation,"saved":saved,"revealFailure":reveal_failure,"refreshed":refreshed})).unwrap()).unwrap();
    f.host.clear().await;
}

#[test]
fn four_write_limit_rejects_without_io_and_recovers_after_workers_finish() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async{
        let _lane=TEST_LANE.lock().await;let f=Fixture::new();let mut states=Vec::new();let mut views=Vec::new();
        for _ in 0..5 {let mut state=json!({});views.push(adapter::handle(&f.ctx,"index","extension:run-command",&json!({}),&mut state).await.unwrap());states.push(state);}
        let (tx,rx)=std::sync::mpsc::channel();let (ready_tx,ready_rx)=tokio::sync::oneshot::channel();let blocker=tokio::task::spawn_blocking(move||{ready_tx.send(()).unwrap();rx.recv().unwrap();});ready_rx.await.unwrap();
        let mut tasks=Vec::new();
        for i in 0..4 {
            let c=f.ctx.clone();let mut state=states[i].clone();let body=json!({"actionId":views[i]["actions"][0]["id"],"formValues":{"base64":"Zg==","filePath":f.dir.path().join(format!("out-{i}"))}});
            tasks.push(tokio::spawn(async move{adapter::handle(&c,"index","extension:invoke-action",&body,&mut state).await}));
            let mut writing=false;for _ in 0..100 {let v=adapter::handle(&f.ctx,"index","extension:refresh-session",&json!({}),&mut states[i]).await.unwrap();if message(&v).contains("Writing file"){writing=true;break}tokio::time::sleep(Duration::from_millis(2)).await;}assert!(writing);
        }
        let fifth=f.dir.path().join("fifth");let body=json!({"actionId":views[4]["actions"][0]["id"],"formValues":{"base64":"Zg==","filePath":fifth}});
        let rejected=adapter::handle(&f.ctx,"index","extension:invoke-action",&body,&mut states[4]).await.unwrap();assert!(message(&rejected).contains("Four conversions"));assert!(!fifth.exists());
        tx.send(()).unwrap();blocker.await.unwrap();for t in tasks{t.await.unwrap().unwrap();}
        let body=json!({"actionId":rejected["actions"][0]["id"],"formValues":{"base64":"Zg==","filePath":fifth}});
        let result=adapter::handle(&f.ctx,"index","extension:invoke-action",&body,&mut states[4]).await.unwrap();assert!(message(&result).contains("host acknowledged"));assert_eq!(std::fs::read(&fifth).unwrap(),b"f");
        for state in &mut states{adapter::handle(&f.ctx,"index","extension:dispose-session",&json!({}),state).await.unwrap();}
    });
}
