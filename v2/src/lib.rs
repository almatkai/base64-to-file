//! Bounded WHATWG forgiving-base64 decoder and directory-capability atomic writer.
//! No clipboard, subprocesses, network, profile access, or JavaScript execution.
use cap_std::{ambient_authority, fs::Dir};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

pub const MAX_INPUT: usize = 24 * 1024 * 1024;
pub const MAX_OUTPUT: usize = 16 * 1024 * 1024;
pub const MAX_PATH: usize = 4096;

/// Browser atob semantics: five ASCII whitespaces; optional padding; trailing bits
/// are ignored. Deliberately no data: or URL-safe decoding, and no UTF-8 conversion.
pub fn decode(input: &str) -> Result<Vec<u8>, String> {
    if input.len() > MAX_INPUT {
        return Err("Base64 input exceeds 24 MiB".into());
    }
    let mut text: Vec<u8> = input
        .bytes()
        .filter(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 12))
        .collect();
    if text.len() % 4 == 0 && text.last() == Some(&b'=') {
        text.pop();
        if text.last() == Some(&b'=') {
            text.pop();
        }
    }
    if text.len() % 4 == 1 {
        return Err("Invalid Base64 text".into());
    }
    let size = text.len() / 4 * 3 + (text.len() % 4 * 6 / 8);
    if size > MAX_OUTPUT {
        return Err("Decoded file exceeds 16 MiB".into());
    }
    let mut bytes = Vec::with_capacity(size);
    let (mut bits, mut count) = (0u32, 0u32);
    for b in text {
        let n = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err("Invalid Base64 text".into()),
        };
        bits = ((bits << 6) | u32::from(n)) & 0xffff;
        count += 6;
        if count >= 8 {
            count -= 8;
            bytes.push((bits >> count) as u8);
        }
    }
    Ok(bytes)
}

/// Require explicit local absolute paths. No expansion, URLs, relative paths,
/// parent traversal, Windows alternate streams, devices, UNC or verbatim paths.
/// The home-derived default uses the same validation on submit.
pub fn validate_path(text: &str) -> Result<PathBuf, String> {
    if text.is_empty() || text.len() > MAX_PATH || text.chars().any(char::is_control) {
        return Err("Enter a local absolute file path of at most 4096 bytes".into());
    }
    #[cfg(windows)]
    let separators = &['/', '\\'][..];
    #[cfg(not(windows))]
    let separators = &['/'][..];
    // Path::components normalizes interior/trailing '.', so inspect the literal
    // spelling before PathBuf can reinterpret a directory-only '/file/.' target.
    if text
        .split(separators)
        .any(|part| matches!(part, "." | ".."))
    {
        return Err("Enter a file path without dot or parent components".into());
    }
    let path = PathBuf::from(text);
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("Enter a local absolute file path without parent traversal".into());
    }
    if text.ends_with(['/', '\\']) || path.file_name().is_none() {
        return Err("Enter a file name, not a directory".into());
    }
    #[cfg(windows)]
    {
        use std::path::Prefix;
        if !matches!(path.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_)))
        {
            return Err("Only local drive paths are supported".into());
        }
        for c in path.components() {
            if let Component::Normal(name) = c {
                let name = name.to_str().ok_or("Invalid file path")?;
                let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
                if name.ends_with(['.', ' '])
                    || name.contains([':', '*', '?', '"', '<', '>', '|'])
                    || matches!(
                        stem.as_str(),
                        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
                    )
                    || ["COM¹", "COM²", "COM³", "LPT¹", "LPT²", "LPT³"].contains(&stem.as_str())
                    || (stem.len() == 4
                        && (stem.starts_with("COM") || stem.starts_with("LPT"))
                        && matches!(stem.as_bytes()[3], b'1'..=b'9'))
                {
                    return Err("Invalid or reserved Windows file path".into());
                }
            }
        }
    }
    Ok(path)
}

#[derive(Clone, Debug)]
pub struct Written {
    pub path: PathBuf,
    pub bytes: usize,
}

struct Stage<'a> {
    dir: &'a Dir,
    name: String,
    file: Option<cap_std::fs::File>,
    cleanup: bool,
}
impl Drop for Stage<'_> {
    fn drop(&mut self) {
        // Close before unlinking, including failure paths on Windows.
        drop(self.file.take());
        if self.cleanup {
            let _ = self.dir.remove_file(&self.name);
        }
    }
}

impl Stage<'_> {
    fn publish(&mut self, destination: &std::ffi::OsStr) -> Result<(), String> {
        self.dir
            .rename(&self.name, self.dir, destination)
            .map_err(|_| "Cannot publish file; destination was not replaced")?;
        // The old entry no longer belongs to us. Never unlink a new file that
        // another process creates at the same staging name after publication.
        self.cleanup = false;
        Ok(())
    }
}

fn create_stage(dir: &Dir, name: String) -> Result<Stage<'_>, String> {
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    let file = dir
        .open_with(&name, &options)
        .map_err(|_| "Cannot create staging file")?;
    // Own cleanup only after successful exclusive creation. A name collision
    // belongs to somebody else and must never be removed by our guard.
    Ok(Stage {
        dir,
        name,
        file: Some(file),
        cleanup: true,
    })
}

/// Atomic publication, including replacement of an existing regular file.
/// Parent canonicalization follows parent symlinks once; an opened directory
/// capability anchors every subsequent operation even if ancestors are renamed.
/// A final symlink/reparse point is rejected; rename never follows a raced-in link.
/// Concurrent regular-file writers are last-publication-wins, as V1 overwrote too.
pub fn write_file(path: &Path, bytes: &[u8]) -> Result<Written, String> {
    publish(path, bytes, |writer, data| writer.write_all(data))
}
fn publish<F>(path: &Path, bytes: &[u8], write: F) -> Result<Written, String>
where
    F: FnOnce(&mut cap_std::fs::File, &[u8]) -> std::io::Result<()>,
{
    if bytes.len() > MAX_OUTPUT {
        return Err("Decoded file exceeds 16 MiB".into());
    }
    let text = path.to_str().ok_or("Invalid file path")?;
    validate_path(text)?;
    let parent = path.parent().ok_or("File path has no parent")?;
    let parent =
        std::fs::canonicalize(parent).map_err(|_| "Destination directory is unavailable")?;
    let name = path.file_name().ok_or("File path has no name")?;
    let canonical_destination = parent.join(name);
    let canonical_text = canonical_destination
        .to_str()
        .ok_or("Canonical destination is not representable")?;
    // Windows canonicalize returns a verbatim path; Explorer's /select route
    // expects an ordinary local drive path. Normalize and validate before any
    // write, rejecting aliases that resolve to UNC/device paths.
    #[cfg(windows)]
    let canonical_text = canonical_text
        .strip_prefix(r"\\?\")
        .unwrap_or(canonical_text);
    let receipt_path = validate_path(canonical_text)?;
    let dir = Dir::open_ambient_dir(&parent, ambient_authority())
        .map_err(|_| "Cannot open destination directory")?;
    let permissions = match dir.symlink_metadata(name) {
        Ok(m) => {
            if !m.is_file() || m.file_type().is_symlink() {
                return Err("Destination must be a regular file, not a link or directory".into());
            }
            if m.permissions().readonly() {
                return Err("Destination is read-only".into());
            }
            Some(m.permissions())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err("Cannot inspect destination".into()),
    };
    let name_of_stage = format!(".tezbar-base64-{}.tmp", uuid::Uuid::new_v4());
    let mut stage = create_stage(&dir, name_of_stage)?;
    let file = stage.file.as_mut().ok_or("Staging file is unavailable")?;
    // New files are private; replacement preserves regular file permissions.
    #[cfg(unix)]
    {
        use cap_std::fs::PermissionsExt;
        file.set_permissions(cap_std::fs::Permissions::from_mode(0o600))
            .map_err(|_| "Cannot protect staging file")?;
    }
    write(file, bytes).map_err(|_| "Cannot write file; destination was not replaced")?;
    if let Some(permissions) = permissions {
        file.set_permissions(permissions)
            .map_err(|_| "Cannot preserve destination permissions")?;
    }
    file.flush()
        .map_err(|_| "Cannot flush file; destination was not replaced")?;
    file.sync_all()
        .map_err(|_| "Cannot sync file; destination was not replaced")?;
    drop(stage.file.take());
    // Recheck the final entry, but never claim this eliminates concurrent changes.
    if let Ok(m) = dir.symlink_metadata(name) {
        if !m.is_file() || m.file_type().is_symlink() {
            return Err("Destination changed to a link or directory".into());
        }
    }
    stage.publish(name)?;
    Ok(Written {
        path: receipt_path,
        bytes: bytes.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stage_collision_never_removes_or_modifies_an_existing_entry() {
        let d = tempfile::tempdir().unwrap();
        let dir = Dir::open_ambient_dir(d.path(), ambient_authority()).unwrap();
        std::fs::write(d.path().join("collision"), b"belongs to somebody else").unwrap();
        assert!(create_stage(&dir, "collision".into()).is_err());
        assert_eq!(
            std::fs::read(d.path().join("collision")).unwrap(),
            b"belongs to somebody else"
        );
    }
    #[test]
    fn successful_publication_never_cleans_a_reused_staging_name() {
        let d = tempfile::tempdir().unwrap();
        let dir = Dir::open_ambient_dir(d.path(), ambient_authority()).unwrap();
        let mut stage = create_stage(&dir, "stage".into()).unwrap();
        stage
            .file
            .as_mut()
            .unwrap()
            .write_all(b"published")
            .unwrap();
        drop(stage.file.take());
        stage.publish(std::ffi::OsStr::new("out")).unwrap();
        std::fs::write(d.path().join("stage"), b"new owner").unwrap();
        drop(stage);
        assert_eq!(std::fs::read(d.path().join("out")).unwrap(), b"published");
        assert_eq!(std::fs::read(d.path().join("stage")).unwrap(), b"new owner");
    }
    #[test]
    fn partial_failure_leaves_original_and_no_stage() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("out");
        std::fs::write(&p, b"before").unwrap();
        assert!(publish(&p, b"after", |f, b| {
            f.write_all(&b[..2])?;
            Err(std::io::ErrorKind::Other.into())
        })
        .is_err());
        assert_eq!(std::fs::read(&p).unwrap(), b"before");
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1);
    }
}
