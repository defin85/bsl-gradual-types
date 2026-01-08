use std::collections::HashMap;
use std::sync::Arc;

use salsa::Setter;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

pub const DEPS_SCHEMA_VERSION: &str = "deps-snapshot-v1";
pub const SETTINGS_SCHEMA_VERSION: &str = "settings-v1";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DepsSnapshotId(String);

impl DepsSnapshotId {
    pub fn from_hash(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DepsSnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SettingsId(String);

impl SettingsId {
    pub fn from_hash(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SettingsId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone)]
pub enum Change {
    SetFile {
        file_id: FileId,
        text: Arc<str>,
        version: i32,
    },
    RemoveFile {
        file_id: FileId,
    },
    SetDepsId {
        deps_id: DepsSnapshotId,
    },
    SetSettingsId {
        settings_id: SettingsId,
    },
}

pub type Cancellable<T> = Result<T, salsa::Cancelled>;

fn cancellable<T>(op: impl FnOnce() -> T) -> Cancellable<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(op)) {
        Ok(value) => Ok(value),
        Err(payload) => match payload.downcast::<salsa::Cancelled>() {
            Ok(cancelled) => Err(*cancelled),
            Err(payload) => std::panic::resume_unwind(payload),
        },
    }
}

#[salsa::input]
pub struct SourceFile {
    pub id: u32,
    #[returns(ref)]
    pub text: Arc<str>,
    pub version: i32,
}

#[salsa::input]
pub struct DepsSnapshot {
    #[returns(ref)]
    pub id: DepsSnapshotId,
}

#[salsa::input]
pub struct SettingsSnapshot {
    #[returns(ref)]
    pub id: SettingsId,
}

#[salsa::tracked]
pub fn file_text_len(db: &dyn salsa::Database, file: SourceFile) -> usize {
    file.text(db).len()
}

#[salsa::db]
#[derive(Clone, Default)]
pub struct AnalysisDatabase {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for AnalysisDatabase {}

pub struct AnalysisHostV2 {
    db: AnalysisDatabase,
    files: HashMap<FileId, SourceFile>,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
}

impl Default for AnalysisHostV2 {
    fn default() -> Self {
        let db = AnalysisDatabase::default();
        let deps = DepsSnapshot::new(&db, DepsSnapshotId::from_hash(""));
        let settings = SettingsSnapshot::new(&db, SettingsId::from_hash(""));
        Self {
            db,
            files: HashMap::new(),
            deps,
            settings,
        }
    }
}

impl AnalysisHostV2 {
    pub fn apply_change(&mut self, change: Change) {
        match change {
            Change::SetFile {
                file_id,
                text,
                version,
            } => self.set_file(file_id, text, version),
            Change::RemoveFile { file_id } => {
                self.files.remove(&file_id);
            }
            Change::SetDepsId { deps_id } => {
                self.deps.set_id(&mut self.db).to(deps_id);
            }
            Change::SetSettingsId { settings_id } => {
                self.settings.set_id(&mut self.db).to(settings_id);
            }
        }
    }

    pub fn set_file(&mut self, file_id: FileId, text: Arc<str>, version: i32) {
        match self.files.get(&file_id).copied() {
            Some(file) => {
                file.set_text(&mut self.db).to(text);
                file.set_version(&mut self.db).to(version);
            }
            None => {
                let file = SourceFile::new(&self.db, file_id.0, text, version);
                self.files.insert(file_id, file);
            }
        }
    }

    pub fn has_file(&self, file_id: FileId) -> bool {
        self.files.contains_key(&file_id)
    }

    pub fn deps_id(&self) -> DepsSnapshotId {
        self.deps.id(&self.db).clone()
    }

    pub fn settings_id(&self) -> SettingsId {
        self.settings.id(&self.db).clone()
    }

    pub fn snapshot(&self) -> AnalysisV2 {
        AnalysisV2 {
            db: self.db.clone(),
            files: self.files.clone(),
            deps: self.deps,
            settings: self.settings,
        }
    }

    pub fn analysis(&self) -> AnalysisV2 {
        self.snapshot()
    }
}

pub struct AnalysisV2 {
    db: AnalysisDatabase,
    files: HashMap<FileId, SourceFile>,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
}

impl AnalysisV2 {
    pub fn file_text(&self, file_id: FileId) -> Cancellable<Option<Arc<str>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| file.text(&self.db).clone()).map(Some)
    }

    pub fn file_version(&self, file_id: FileId) -> Cancellable<Option<i32>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| file.version(&self.db)).map(Some)
    }

    pub fn file_text_len(&self, file_id: FileId) -> Cancellable<Option<usize>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| file_text_len(&self.db, file)).map(Some)
    }

    pub fn deps_id(&self) -> Cancellable<DepsSnapshotId> {
        cancellable(|| self.deps.id(&self.db).clone())
    }

    pub fn settings_id(&self) -> Cancellable<SettingsId> {
        cancellable(|| self.settings.id(&self.db).clone())
    }

    pub fn completion(&self, _file_id: FileId, _line: u32, _character: u32) -> Cancellable<()> {
        Ok(())
    }

    pub fn hover(&self, _file_id: FileId, _line: u32, _character: u32) -> Cancellable<()> {
        Ok(())
    }

    pub fn signature_help(
        &self,
        _file_id: FileId,
        _line: u32,
        _character: u32,
    ) -> Cancellable<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_text_and_version_update_after_set_file() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("abc"),
            version: 1,
        });

        {
            let analysis = host.analysis();
            assert_eq!(analysis.file_text(file_id).unwrap().as_deref(), Some("abc"));
            assert_eq!(analysis.file_version(file_id).unwrap(), Some(1));
            assert_eq!(analysis.file_text_len(file_id).unwrap(), Some(3));
        }

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("abcd"),
            version: 2,
        });

        {
            let analysis = host.analysis();
            assert_eq!(analysis.file_text(file_id).unwrap().as_deref(), Some("abcd"));
            assert_eq!(analysis.file_version(file_id).unwrap(), Some(2));
            assert_eq!(analysis.file_text_len(file_id).unwrap(), Some(4));
        }
    }

    #[test]
    fn remove_file_makes_queries_return_none() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("abc"),
            version: 1,
        });
        host.apply_change(Change::RemoveFile { file_id });

        let analysis = host.analysis();
        assert_eq!(analysis.file_text(file_id).unwrap(), None);
        assert_eq!(analysis.file_version(file_id).unwrap(), None);
        assert_eq!(analysis.file_text_len(file_id).unwrap(), None);
    }

    #[test]
    fn deps_and_settings_ids_are_read_from_snapshot() {
        use std::sync::{Arc, Mutex};
        use std::time::Duration;

        let host = Arc::new(Mutex::new(AnalysisHostV2::default()));
        host.lock().unwrap().apply_change(Change::SetDepsId {
            deps_id: DepsSnapshotId::from_hash("deps-a"),
        });
        host.lock().unwrap().apply_change(Change::SetSettingsId {
            settings_id: SettingsId::from_hash("settings-a"),
        });

        let analysis_a = host.lock().unwrap().snapshot();
        assert_eq!(analysis_a.deps_id().unwrap().as_str(), "deps-a");
        assert_eq!(analysis_a.settings_id().unwrap().as_str(), "settings-a");

        let (locked_tx, locked_rx) = std::sync::mpsc::channel::<()>();
        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        let host_for_update = host.clone();
        let update_thread = std::thread::spawn(move || {
            let mut host = host_for_update.lock().unwrap();
            locked_tx.send(()).unwrap();
            host.apply_change(Change::SetDepsId {
                deps_id: DepsSnapshotId::from_hash("deps-b"),
            });
            host.apply_change(Change::SetSettingsId {
                settings_id: SettingsId::from_hash("settings-b"),
            });
            done_tx.send(()).unwrap();
        });

        locked_rx.recv_timeout(Duration::from_secs(1)).unwrap();

        assert!(done_rx.recv_timeout(Duration::from_millis(200)).is_err());
        assert_eq!(analysis_a.deps_id().unwrap().as_str(), "deps-a");
        assert_eq!(analysis_a.settings_id().unwrap().as_str(), "settings-a");

        drop(analysis_a);
        done_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        update_thread.join().unwrap();

        let analysis_b = host.lock().unwrap().snapshot();
        assert_eq!(analysis_b.deps_id().unwrap().as_str(), "deps-b");
        assert_eq!(analysis_b.settings_id().unwrap().as_str(), "settings-b");
    }
}
