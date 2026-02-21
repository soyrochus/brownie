use crate::session::{SessionMeta, SCHEMA_VERSION};
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
    let session: SessionMeta = serde_json::from_slice(&data)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
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
