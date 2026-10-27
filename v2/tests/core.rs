use std::fmt::Write as _;
use tezbar_base64_to_file::{decode, validate_path, write_file, MAX_INPUT, MAX_OUTPUT};
#[test]
fn executable_v1_oracle_bytes_and_rejections() {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/v1-oracle.json")).unwrap();
    for case in oracle["outputs"].as_array().unwrap() {
        let Some(input) = case["input"].as_str() else {
            continue;
        };
        if input.is_empty() {
            assert!(decode(input).unwrap().is_empty());
            continue;
        }
        let write = case["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e[0] == "write");
        match write {
            Some(event) => {
                let actual = decode(input).unwrap();
                let mut hex = String::with_capacity(actual.len() * 2);
                for byte in actual {
                    write!(hex, "{byte:02x}").unwrap();
                }
                assert_eq!(hex, event[1], "{input:?}");
            }
            None => assert!(decode(input).is_err(), "{input:?}"),
        }
    }
}
#[test]
fn bounded_work_never_truncates() {
    assert!(decode(&"A".repeat(MAX_INPUT + 1)).is_err());
    assert!(decode(&"A".repeat(MAX_INPUT)).is_err());
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("out");
    assert!(write_file(&p, &vec![0; MAX_OUTPUT + 1]).is_err());
    assert!(!p.exists());
    for p in [
        "",
        "relative",
        "~/Downloads/out.pdf",
        "/tmp/../out",
        "/tmp/out/.",
        "/tmp/./out",
        "/tmp/out\0",
        "/tmp/out\n",
        "/tmp/",
    ] {
        assert!(validate_path(p).is_err(), "{p:?}");
    }
}
#[test]
fn real_create_binary_overwrite_zero_and_failures() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("日本 ' ; $ out.bin");
    let w = write_file(&p, &[0, 255, 128]).unwrap();
    assert_eq!(w.bytes, 3);
    assert_eq!(std::fs::read(&p).unwrap(), [0, 255, 128]);
    write_file(&p, b"longer before").unwrap();
    write_file(&p, b"x").unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), b"x");
    write_file(&p, b"").unwrap();
    assert!(std::fs::read(&p).unwrap().is_empty());
    assert!(write_file(d.path(), b"x").is_err());
    assert!(write_file(&d.path().join("missing/out"), b"x").is_err());
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1);
}
#[cfg(unix)]
#[test]
fn link_policy_permissions_and_parent_alias() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let d = tempfile::tempdir().unwrap();
    let target = d.path().join("original");
    std::fs::write(&target, b"keep").unwrap();
    let link = d.path().join("link");
    symlink(&target, &link).unwrap();
    assert!(write_file(&link, b"new").is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"keep");
    let dangling = d.path().join("dangling");
    symlink(d.path().join("absent"), &dangling).unwrap();
    assert!(write_file(&dangling, b"x").is_err());
    let fifo = d.path().join("directory");
    std::fs::create_dir(&fifo).unwrap();
    assert!(write_file(&fifo, b"x").is_err());
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).unwrap();
    write_file(&target, b"new").unwrap();
    assert_eq!(
        std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o640
    );
    let hard = d.path().join("hard");
    std::fs::hard_link(&target, &hard).unwrap();
    write_file(&target, b"separate inode").unwrap();
    assert_eq!(std::fs::read(&hard).unwrap(), b"new");
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o400)).unwrap();
    assert!(write_file(&target, b"no").is_err());
    let alias = d.path().join("parent-alias");
    symlink(d.path(), &alias).unwrap();
    let w = write_file(&alias.join("output"), b"ok").unwrap();
    assert_eq!(w.path, d.path().canonicalize().unwrap().join("output"));
}
#[test]
fn concurrent_publication_has_only_complete_outputs_and_no_stages() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("same");
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let p = p.clone();
            std::thread::spawn(move || write_file(&p, &vec![i; 8192]).unwrap())
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    let bytes = std::fs::read(&p).unwrap();
    assert_eq!(bytes.len(), 8192);
    assert!(bytes.iter().all(|b| *b == bytes[0]));
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn raced_final_symlink_never_writes_its_target() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let victim = dir.path().join("victim");
    let path = dir.path().join("out");
    std::fs::write(&victim, b"must remain exact").unwrap();
    let p = path.clone();
    let v = victim.clone();
    let racer = std::thread::spawn(move || {
        for _ in 0..200 {
            let _ = std::fs::remove_file(&p);
            let _ = symlink(&v, &p);
            std::thread::yield_now();
        }
    });
    for _ in 0..100 {
        let _ = write_file(&path, b"converted");
    }
    racer.join().unwrap();
    assert_eq!(std::fs::read(&victim).unwrap(), b"must remain exact");
    assert!(std::fs::read_dir(dir.path()).unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".tezbar-")));
}
#[cfg(windows)]
#[test]
fn windows_local_path_policy() {
    for p in [r"C:\out.bin", r"D:\Downloads\out.pdf"] {
        assert!(validate_path(p).is_ok(), "{p}");
    }
    for p in [
        r"C:out.bin",
        r"\\server\share\out",
        r"\\?\C:\out",
        r"C:\NUL.txt",
        r"C:\CONIN$",
        r"C:\COM¹",
        r"C:\out:stream",
        r"C:\out.",
        r"C:\out ",
        r"C:\bad?file",
    ] {
        assert!(validate_path(p).is_err(), "{p}");
    }
}
