#[derive(Debug, Clone)]
struct WorkspaceFile {
    root_id: String,
    root_path: PathBuf,
    rel_path: String,
    abs_path: PathBuf,
}

#[derive(Debug)]
struct DocumentSnapshot {
    file: DocumentRefDto,
    abs_path: PathBuf,
    text: String,
    version: i32,
}

fn collect_scope_files(
    roots: &[RootEntry],
    hot_set: &HashSet<DocumentKey>,
    scope: WorkspaceScopeTagged,
) -> Result<Vec<WorkspaceFile>, rmcp::ErrorData> {
    match scope {
        WorkspaceScopeTagged::Project => collect_project_files(roots),
        WorkspaceScopeTagged::Hot => collect_hot_files(roots, hot_set),
        WorkspaceScopeTagged::File { document } => {
            let key = document_key_from_ref(roots, &document)?;
            let (root_path, abs_path, file_ref) = resolve_doc_path(roots, &key)?;
            Ok(vec![WorkspaceFile {
                root_id: file_ref.root_id,
                root_path,
                rel_path: file_ref.path,
                abs_path,
            }])
        }
    }
}

fn collect_hot_files(
    roots: &[RootEntry],
    hot_set: &HashSet<DocumentKey>,
) -> Result<Vec<WorkspaceFile>, rmcp::ErrorData> {
    let mut files = Vec::new();
    for key in hot_set {
        let (root_path, abs_path, file_ref) = resolve_doc_path(roots, key)?;
        files.push(WorkspaceFile {
            root_id: file_ref.root_id,
            root_path,
            rel_path: file_ref.path,
            abs_path,
        });
    }
    files.sort_by(|a, b| {
        (a.root_id.as_str(), a.rel_path.as_str()).cmp(&(b.root_id.as_str(), b.rel_path.as_str()))
    });
    files.dedup_by(|a, b| a.root_id == b.root_id && a.rel_path == b.rel_path);
    Ok(files)
}

fn collect_project_files(roots: &[RootEntry]) -> Result<Vec<WorkspaceFile>, rmcp::ErrorData> {
    let mut files = Vec::new();
    for root in roots {
        for entry in WalkDir::new(&root.path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                if !entry.file_type().is_dir() {
                    return true;
                }
                let name = entry.file_name().to_string_lossy();
                name != ".git" && name != "target"
            })
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if entry.file_type().is_symlink() {
                continue;
            }
            if !path.is_file() {
                continue;
            }
            if !bsl_runtime::system::fs_utils::is_bsl_file(path) {
                continue;
            }
            let rel_path = match path.strip_prefix(&root.path) {
                Ok(rel) => rel,
                Err(_) => continue,
            };
            let rel_path = normalize_path_components(rel_path);
            files.push(WorkspaceFile {
                root_id: root.root_id.clone(),
                root_path: root.path.clone(),
                rel_path,
                abs_path: path.to_path_buf(),
            });
        }
    }
    files.sort_by(|a, b| {
        (a.root_id.as_str(), a.rel_path.as_str()).cmp(&(b.root_id.as_str(), b.rel_path.as_str()))
    });
    files.dedup_by(|a, b| a.root_id == b.root_id && a.rel_path == b.rel_path);
    Ok(files)
}

fn normalize_path_components(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        if let Component::Normal(value) = component {
            parts.push(value.to_string_lossy().to_string());
        }
    }
    parts.join("/")
}

fn load_document_snapshot(
    file: &WorkspaceFile,
    overlays: &HashMap<DocumentKey, DocumentOverlay>,
) -> Result<Option<DocumentSnapshot>, rmcp::ErrorData> {
    let key = DocumentKey {
        root_id: file.root_id.clone(),
        path: file.rel_path.clone(),
    };

    if let Some(overlay) = overlays.get(&key) {
        return Ok(Some(DocumentSnapshot {
            file: DocumentRefDto {
                root_id: key.root_id,
                path: key.path,
            },
            abs_path: file.abs_path.clone(),
            text: overlay.text.clone(),
            version: overlay_version_i32(overlay.version),
        }));
    }

    let text = load_disk_text_with_limits(&file.root_path, &file.abs_path)?;
    Ok(text.map(|text| DocumentSnapshot {
        file: DocumentRefDto {
            root_id: key.root_id,
            path: key.path,
        },
        abs_path: file.abs_path.clone(),
        text,
        version: 0,
    }))
}

fn load_disk_text_with_limits(
    root_path: &Path,
    path: &Path,
) -> Result<Option<String>, rmcp::ErrorData> {
    let canonical = match std::fs::canonicalize(path) {
        Ok(path) => path,
        Err(_) => return Ok(None),
    };
    if !canonical.starts_with(root_path) {
        return Err(rmcp::ErrorData::invalid_params("path escapes roots", None));
    }

    let metadata = match std::fs::metadata(&canonical) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(None),
    };
    if metadata.len() > MAX_DISK_FILE_BYTES {
        return Err(rmcp::ErrorData::invalid_params(
            format!(
                "file too large: {} ({} bytes)",
                canonical.display(),
                metadata.len()
            ),
            None,
        ));
    }
    bsl_runtime::system::fs_utils::read_bsl_file(&canonical)
        .map(Some)
        .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))
}

fn overlay_version_i32(version: u64) -> i32 {
    if version > i32::MAX as u64 {
        i32::MAX
    } else {
        version as i32
    }
}

fn document_key_from_ref(
    roots: &[RootEntry],
    doc: &DocumentRef,
) -> Result<DocumentKey, rmcp::ErrorData> {
    fn relative_path_to_slash(rel: &Path) -> Result<String, rmcp::ErrorData> {
        let mut components = Vec::new();
        for component in rel.components() {
            match component {
                Component::Normal(value) => components.push(value.to_string_lossy().to_string()),
                Component::CurDir => {}
                Component::ParentDir => {
                    return Err(rmcp::ErrorData::invalid_params(
                        "path must not contain '..'",
                        None,
                    ))
                }
                Component::Prefix(_) | Component::RootDir => {
                    return Err(rmcp::ErrorData::invalid_params(
                        "path must be relative",
                        None,
                    ))
                }
            }
        }
        if components.is_empty() {
            return Err(rmcp::ErrorData::invalid_params(
                "path must be non-empty",
                None,
            ));
        }
        Ok(components.join("/"))
    }

    fn normalize_absolute_path_best_effort(path: &str) -> Result<PathBuf, rmcp::ErrorData> {
        if path.trim().is_empty() {
            return Err(rmcp::ErrorData::invalid_params(
                "path must be non-empty",
                None,
            ));
        }
        let input = PathBuf::from(path);
        if !input.is_absolute() {
            return Err(rmcp::ErrorData::invalid_params(
                "path must be absolute",
                None,
            ));
        }

        let mut normalized = PathBuf::new();
        for component in input.components() {
            match component {
                Component::Normal(value) => normalized.push(value),
                Component::CurDir => {}
                Component::ParentDir => {
                    return Err(rmcp::ErrorData::invalid_params(
                        "path must not contain '..'",
                        None,
                    ))
                }
                Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
                Component::RootDir => normalized.push(Path::new("/")),
            }
        }
        if !normalized.is_absolute() {
            return Err(rmcp::ErrorData::invalid_params(
                "path must be absolute",
                None,
            ));
        }
        Ok(normalized)
    }

    match doc {
        DocumentRef::Canonical(doc) => {
            let root_id = doc.root_id.as_str();
            if !roots.iter().any(|root| root.root_id == root_id) {
                return Err(rmcp::ErrorData::invalid_params("unknown root_id", None));
            }
            Ok(DocumentKey {
                root_id: doc.root_id.clone(),
                path: normalize_relative_path(&doc.path)?,
            })
        }
        DocumentRef::PathObject(doc) => {
            document_key_from_ref(roots, &DocumentRef::Path(doc.path.clone()))
        }
        DocumentRef::Path(path) => {
            let raw = path.as_str();
            let candidate = PathBuf::from(raw);
            if candidate.is_absolute() {
                let abs = normalize_absolute_path_best_effort(raw)?;
                let mut best: Option<(&RootEntry, usize)> = None;
                for root in roots {
                    if abs.starts_with(&root.path) {
                        let depth = root.path.components().count();
                        if best
                            .map(|(_, best_depth)| depth > best_depth)
                            .unwrap_or(true)
                        {
                            best = Some((root, depth));
                        }
                    }
                }
                let (root, _) = best.ok_or_else(|| {
                    rmcp::ErrorData::invalid_params("path is outside roots", None)
                })?;
                let rel = abs
                    .strip_prefix(&root.path)
                    .map_err(|_| rmcp::ErrorData::invalid_params("path is outside roots", None))?;
                Ok(DocumentKey {
                    root_id: root.root_id.clone(),
                    path: relative_path_to_slash(rel)?,
                })
            } else if roots.len() == 1 {
                Ok(DocumentKey {
                    root_id: roots[0].root_id.clone(),
                    path: normalize_relative_path(raw)?,
                })
            } else {
                Err(rmcp::ErrorData::invalid_params(
                    "root_id is required for relative paths in multi-root; provide an absolute path instead",
                    None,
                ))
            }
        }
    }
}

fn resolve_doc_path(
    roots: &[RootEntry],
    key: &DocumentKey,
) -> Result<(PathBuf, PathBuf, DocumentRefDto), rmcp::ErrorData> {
    let root = roots
        .iter()
        .find(|root| root.root_id == key.root_id)
        .ok_or_else(|| rmcp::ErrorData::invalid_params("unknown root_id", None))?;
    let abs = root.path.join(PathBuf::from(&key.path));
    Ok((
        root.path.clone(),
        abs,
        DocumentRefDto {
            root_id: key.root_id.clone(),
            path: key.path.clone(),
        },
    ))
}

fn select_effective_text(
    file: &FileRef,
    key: &DocumentKey,
    overlays: &HashMap<DocumentKey, DocumentOverlay>,
    root_path: &Path,
    abs_path: &Path,
) -> Result<String, rmcp::ErrorData> {
    if let Some(text) = &file.text {
        if file.version.is_none() {
            return Err(rmcp::ErrorData::invalid_params(
                "version is required when text is provided",
                None,
            ));
        }
        if text.len() > MAX_OVERLAY_BYTES {
            return Err(rmcp::ErrorData::invalid_params(
                format!("overlay text exceeds MAX_OVERLAY_BYTES={MAX_OVERLAY_BYTES}"),
                None,
            ));
        }
        return Ok(text.clone());
    }

    if let Some(overlay) = overlays.get(key) {
        return Ok(overlay.text.clone());
    }

    load_disk_text_with_limits(root_path, abs_path)?
        .ok_or_else(|| rmcp::ErrorData::invalid_params("file not found", None))
}

fn select_effective_version(
    file: &FileRef,
    key: &DocumentKey,
    overlays: &HashMap<DocumentKey, DocumentOverlay>,
) -> i32 {
    if let Some(version) = file.version {
        return overlay_version_i32(version);
    }
    overlays
        .get(key)
        .map(|overlay| overlay_version_i32(overlay.version))
        .unwrap_or(0)
}
