use crate::session::{SessionMeta, SCHEMA_VERSION};
use crate::ui::workspace::CanvasWorkspaceState;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn sessions_dir() -> PathBuf {
    home_dir().join(".brownie").join("sessions")
}

fn session_path(session_id: &str) -> PathBuf {
    sessions_dir().join(format!("{session_id}.json"))
}

fn read_session_file(path: &Path) -> Result<SessionMeta, String> {
    let data = fs::read(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let mut session: SessionMeta = serde_json::from_slice(&data)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    if session.schema_version == 1 {
        session.canvas_workspace = CanvasWorkspaceState::default();
        return Ok(session);
    }

    if session.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unknown schema_version in {}: {}",
            path.display(),
            session.schema_version
        ));
    }
    Ok(session)
}

pub fn ensure_sessions_dir() -> io::Result<PathBuf> {
    let dir = sessions_dir();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn save(meta: &SessionMeta) -> io::Result<()> {
    let dir = ensure_sessions_dir()?;
    let final_path = session_path(&meta.session_id);
    let tmp_path = dir.join(format!("{}.json.tmp", meta.session_id));
    let bytes = serde_json::to_vec_pretty(meta)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;

    fs::write(&tmp_path, bytes)?;
    match fs::rename(&tmp_path, &final_path) {
        Ok(()) => Ok(()),
        Err(rename_err) => {
            if final_path.exists() {
                fs::remove_file(&final_path)?;
                fs::rename(&tmp_path, &final_path)?;
                Ok(())
            } else {
                Err(rename_err)
            }
        }
    }
}

pub fn load_all() -> (Vec<SessionMeta>, Vec<String>) {
    let mut sessions = Vec::new();
    let mut warnings = Vec::new();

    let dir = match ensure_sessions_dir() {
        Ok(dir) => dir,
        Err(err) => {
            warnings.push(format!("failed to initialize sessions directory: {err}"));
            return (sessions, warnings);
        }
    };

    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            warnings.push(format!("failed to read sessions directory: {err}"));
            return (sessions, warnings);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension() != Some(OsStr::new("json")) {
            continue;
        }

        match read_session_file(&path) {
            Ok(session) => sessions.push(session),
            Err(err) => warnings.push(err),
        }
    }

    sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    (sessions, warnings)
}

pub fn load_one(session_id: &str) -> (Option<SessionMeta>, Option<String>) {
    let dir = match ensure_sessions_dir() {
        Ok(dir) => dir,
        Err(err) => {
            return (
                None,
                Some(format!("failed to initialize sessions directory: {err}")),
            );
        }
    };

    let path = dir.join(format!("{session_id}.json"));
    if !path.exists() {
        return (
            None,
            Some(format!(
                "session file missing for id {session_id}: {}",
                path.display()
            )),
        );
    }

    match read_session_file(&path) {
        Ok(session) => (Some(session), None),
        Err(err) => (None, Some(err)),
    }
}

#[cfg(test)]
mod tests {
    use super::read_session_file;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "brownie_session_store_{prefix}_{}_{}.json",
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn read_session_file_supports_legacy_schema_without_workspace() {
        let path = temp_file("legacy");
        let data = r#"{
  "schema_version": 1,
  "session_id": "legacy-session",
  "workspace": "/tmp/demo",
  "title": "Legacy",
  "created_at": "1",
  "messages": []
}"#;
        fs::write(&path, data).expect("legacy session fixture should write");

        let session = read_session_file(&path).expect("legacy schema should load");
        assert_eq!(session.schema_version, 1);
        assert!(session.canvas_workspace.blocks.is_empty());
        assert!(session.canvas_workspace.active_block_id.is_none());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_session_file_loads_workspace_aware_schema() {
        let path = temp_file("workspace_aware");
        let data = r#"{
  "schema_version": 2,
  "session_id": "workspace-session",
  "workspace": "/tmp/demo",
  "title": "Workspace",
  "created_at": "1",
  "messages": [],
  "canvas_workspace": {
    "active_block_id": "block-1",
    "blocks": [
      {
        "block_id": "block-1",
        "template_id": "builtin.file_listing.default",
        "title": "Workspace Explorer",
        "provider_id": "builtin-default",
        "provider_kind": "builtin",
        "schema": {
          "schema_version": 1,
          "outputs": [],
          "components": []
        },
        "intent": {
          "primary": "file_listing",
          "operations": ["list"],
          "tags": ["workspace"]
        },
        "minimized": false,
        "form_state": {}
      }
    ]
  }
}"#;
        fs::write(&path, data).expect("workspace-aware session fixture should write");

        let session = read_session_file(&path).expect("workspace-aware schema should load");
        assert_eq!(session.schema_version, 2);
        assert_eq!(session.canvas_workspace.blocks.len(), 1);
        assert_eq!(
            session.canvas_workspace.active_block_id.as_deref(),
            Some("block-1")
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_session_file_rejects_unknown_schema() {
        let path = temp_file("unknown");
        let data = r#"{
  "schema_version": 99,
  "session_id": "unknown-session",
  "workspace": "/tmp/demo",
  "title": "Unknown",
  "created_at": "1",
  "messages": []
}"#;
        fs::write(&path, data).expect("unknown schema fixture should write");

        let error = read_session_file(&path).expect_err("unknown schema should fail");
        assert!(error.contains("unknown schema_version"));

        let _ = fs::remove_file(path);
    }
}
