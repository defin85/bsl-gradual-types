use std::collections::HashMap;
use std::sync::Arc;

use salsa::Setter;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

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
    pub text: String,
    pub version: i32,
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
}

impl Default for AnalysisHostV2 {
    fn default() -> Self {
        Self {
            db: AnalysisDatabase::default(),
            files: HashMap::new(),
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
        }
    }

    pub fn set_file(&mut self, file_id: FileId, text: Arc<str>, version: i32) {
        match self.files.get(&file_id).copied() {
            Some(file) => {
                file.set_text(&mut self.db).to(text.to_string());
                file.set_version(&mut self.db).to(version);
            }
            None => {
                let file = SourceFile::new(&self.db, file_id.0, text.to_string(), version);
                self.files.insert(file_id, file);
            }
        }
    }

    pub fn has_file(&self, file_id: FileId) -> bool {
        self.files.contains_key(&file_id)
    }

    pub fn analysis(&self) -> AnalysisV2 {
        AnalysisV2 {
            db: self.db.clone(),
            files: self.files.clone(),
        }
    }
}

pub struct AnalysisV2 {
    db: AnalysisDatabase,
    files: HashMap<FileId, SourceFile>,
}

impl AnalysisV2 {
    pub fn file_text_len(&self, file_id: FileId) -> Cancellable<Option<usize>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| file_text_len(&self.db, file)).map(Some)
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
    fn file_text_len_updates_after_set_file() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("abc"),
            version: 1,
        });

        {
            let analysis = host.analysis();
            assert_eq!(analysis.file_text_len(file_id).unwrap(), Some(3));
        }

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("abcd"),
            version: 2,
        });

        {
            let analysis = host.analysis();
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
        assert_eq!(analysis.file_text_len(file_id).unwrap(), None);
    }
}
