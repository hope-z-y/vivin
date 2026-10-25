use crate::metadata::ProjectData;
use crate::project::{checked_document, checked_relative, revision, safe_project_directory};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub time: u64,
    pub operation: String,
    pub target: String,
    pub before_content: Option<String>,
    pub after_content: Option<String>,
    pub before_data: Option<ProjectData>,
    pub after_data: Option<ProjectData>,
    #[serde(default)]
    pub before_directory: Option<String>,
    #[serde(default)]
    pub after_directory: Option<String>,
    #[serde(default)]
    pub before_path: Option<String>,
    #[serde(default)]
    pub after_path: Option<String>,
    #[serde(default)]
    pub backup_id: Option<String>,
    #[serde(default)]
    pub after_digest: Option<String>,
    #[serde(default)]
    pub created_project: bool,
    #[serde(default)]
    pub archive_path: Option<String>,
}

pub fn load(root: &Path) -> Result<Vec<HistoryEntry>, String> {
    let path = safe_project_directory(root, ".vivin", false)?.join("history.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

fn save(root: &Path, entries: &[HistoryEntry]) -> Result<(), String> {
    let directory = safe_project_directory(root, ".vivin", true)?;
    let mut temp =
        tempfile::NamedTempFile::new_in(&directory).map_err(|error| error.to_string())?;
    serde_json::to_writer_pretty(&mut temp, entries).map_err(|error| error.to_string())?;
    temp.write_all(b"\n").map_err(|error| error.to_string())?;
    temp.as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    temp.persist(directory.join("history.json"))
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn append(root: &Path, mut entry: HistoryEntry) -> Result<HistoryEntry, String> {
    entry.id = Uuid::new_v4().to_string();
    entry.time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    let mut entries = load(root)?;
    entries.push(entry.clone());
    save(root, &entries)?;
    Ok(entry)
}

pub fn document_entry(
    operation: &str,
    path: &str,
    before: Option<String>,
    after: Option<String>,
) -> HistoryEntry {
    HistoryEntry {
        id: String::new(),
        time: 0,
        operation: operation.to_string(),
        target: path.to_string(),
        before_content: before,
        after_content: after,
        before_data: None,
        after_data: None,
        before_directory: None,
        after_directory: None,
        before_path: None,
        after_path: None,
        backup_id: None,
        after_digest: None,
        created_project: false,
        archive_path: None,
    }
}

pub fn metadata_entry(
    operation: &str,
    target: &str,
    before: ProjectData,
    after: ProjectData,
) -> HistoryEntry {
    HistoryEntry {
        id: String::new(),
        time: 0,
        operation: operation.to_string(),
        target: target.to_string(),
        before_content: None,
        after_content: None,
        before_data: Some(before),
        after_data: Some(after),
        before_directory: None,
        after_directory: None,
        before_path: None,
        after_path: None,
        backup_id: None,
        after_digest: None,
        created_project: false,
        archive_path: None,
    }
}

pub fn directory_entry(
    operation: &str,
    path: &str,
    before: Option<String>,
    after: Option<String>,
) -> HistoryEntry {
    HistoryEntry {
        id: String::new(),
        time: 0,
        operation: operation.to_string(),
        target: path.to_string(),
        before_content: None,
        after_content: None,
        before_data: None,
        after_data: None,
        before_directory: before,
        after_directory: after,
        before_path: None,
        after_path: None,
        backup_id: None,
        after_digest: None,
        created_project: false,
        archive_path: None,
    }
}

pub fn moved_entry(
    operation: &str,
    before_path: &str,
    after_path: &str,
    before_data: ProjectData,
    after_data: ProjectData,
    after_digest: String,
) -> HistoryEntry {
    let mut entry = metadata_entry(operation, before_path, before_data, after_data);
    entry.before_path = Some(before_path.to_string());
    entry.after_path = Some(after_path.to_string());
    entry.after_digest = Some(after_digest);
    entry
}

pub fn deleted_entry(
    path: &str,
    backup_id: &str,
    before_data: ProjectData,
    after_data: ProjectData,
    after_digest: String,
) -> HistoryEntry {
    let mut entry = metadata_entry("delete_entry", path, before_data, after_data);
    entry.before_path = Some(path.to_string());
    entry.backup_id = Some(backup_id.to_string());
    entry.after_digest = Some(after_digest);
    entry
}

pub fn project_created(rules: String, data: ProjectData) -> HistoryEntry {
    let mut entry = document_entry("create_project", "project", None, Some(rules));
    entry.after_data = Some(data);
    entry.created_project = true;
    entry
}

pub fn tree_digest(path: &Path) -> Result<String, String> {
    fn visit(path: &Path, digest: &mut Sha256, depth: usize) -> Result<(), String> {
        if depth > 64 {
            return Err("目录层级超过支持范围".to_string());
        }
        let meta = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
        if meta.file_type().is_symlink() {
            return Err("不能操作包含符号链接的目录".to_string());
        }
        if meta.is_file() {
            digest.update(b"file");
            digest.update(fs::read(path).map_err(|error| error.to_string())?);
        } else if meta.is_dir() {
            digest.update(b"directory");
            let mut children = fs::read_dir(path)
                .map_err(|error| error.to_string())?
                .map(|entry| entry.map_err(|error| error.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            children.sort_by_key(|entry| entry.file_name());
            for child in children {
                digest.update(child.file_name().to_string_lossy().as_bytes());
                visit(&child.path(), digest, depth + 1)?;
            }
        }
        Ok(())
    }
    let mut digest = Sha256::new();
    visit(path, &mut digest, 0)?;
    Ok(format!("{:x}", digest.finalize()))
}

pub fn undo(root: &Path, id: &str) -> Result<HistoryEntry, String> {
    let original = load(root)?
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or("修改记录不存在")?;
    if original.operation == "undo_delete" {
        return Err("删除恢复记录不能再次撤销".to_string());
    }
    if original.created_project {
        let expected_rules = original
            .after_content
            .as_ref()
            .ok_or("项目创建记录不完整")?;
        let rules = fs::read_to_string(crate::project::checked_rules(root)?)
            .map_err(|error| error.to_string())?;
        if &rules != expected_rules
            || original.after_data.as_ref() != Some(&ProjectData::load(root)?)
        {
            return Err("项目内容或状态已有后续修改，无法撤销创建".to_string());
        }
        for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
            let name = entry.map_err(|error| error.to_string())?.file_name();
            if name != "AGENTS.md" && name != "resources" && name != ".vivin" {
                return Err("项目已有其他内容，无法撤销创建".to_string());
            }
        }
        let resources = safe_project_directory(root, "resources", false)?;
        if resources.exists()
            && fs::read_dir(resources)
                .map_err(|error| error.to_string())?
                .next()
                .is_some()
        {
            return Err("资源库已有文件，无法撤销创建".to_string());
        }
        let parent = root.parent().ok_or("项目路径无效")?;
        let archive_directory = safe_project_directory(parent, ".vivin-removed", true)?;
        let archive = archive_directory.join(Uuid::new_v4().to_string());
        fs::rename(root, &archive).map_err(|error| error.to_string())?;
        let mut record = document_entry("undo_create_project", "project", None, None);
        record.archive_path = Some(archive.to_string_lossy().into_owned());
        return match append(&archive, record) {
            Ok(recorded) => Ok(recorded),
            Err(error) => {
                let _ = fs::rename(&archive, root);
                Err(error)
            }
        };
    }
    if let (
        Some(before_path),
        Some(after_path),
        Some(before_data),
        Some(after_data),
        Some(expected_digest),
    ) = (
        &original.before_path,
        &original.after_path,
        &original.before_data,
        &original.after_data,
        &original.after_digest,
    ) {
        let current = ProjectData::load(root)?;
        if current != *after_data {
            return Err("项目状态已有后续修改，无法撤销".to_string());
        }
        current.require_unlocked(after_path)?;
        current.require_unlocked(before_path)?;
        let source = crate::project::checked_member(root, after_path)?;
        if tree_digest(&source)? != *expected_digest {
            return Err(format!("{after_path} 已有后续修改，无法撤销"));
        }
        let destination = root.join(checked_relative(before_path)?);
        if destination.exists() {
            return Err(format!("{before_path} 已存在，无法撤销"));
        }
        let parent = fs::canonicalize(destination.parent().ok_or("目标路径无效")?)
            .map_err(|error| error.to_string())?;
        if !parent.starts_with(root) {
            return Err("目标路径不在项目内".to_string());
        }
        fs::rename(&source, &destination).map_err(|error| error.to_string())?;
        if let Err(error) = before_data.save(root) {
            let _ = fs::rename(&destination, &source);
            return Err(error);
        }
        let undo = moved_entry(
            "undo",
            after_path,
            before_path,
            after_data.clone(),
            before_data.clone(),
            expected_digest.clone(),
        );
        return match append(root, undo) {
            Ok(recorded) => Ok(recorded),
            Err(error) => {
                let _ = fs::rename(&destination, &source);
                let _ = after_data.save(root);
                Err(error)
            }
        };
    }
    if let (
        Some(before_path),
        Some(backup_id),
        Some(before_data),
        Some(after_data),
        Some(expected_digest),
    ) = (
        &original.before_path,
        &original.backup_id,
        &original.before_data,
        &original.after_data,
        &original.after_digest,
    ) {
        Uuid::parse_str(backup_id).map_err(|_| "记录备份标识无效".to_string())?;
        let current = ProjectData::load(root)?;
        if current != *after_data {
            return Err("项目状态已有后续修改，无法撤销".to_string());
        }
        current.require_unlocked(before_path)?;
        let backup = safe_project_directory(root, ".vivin/ai-trash", false)?.join(backup_id);
        if tree_digest(&backup)? != *expected_digest {
            return Err("删除备份已有后续修改，无法撤销".to_string());
        }
        let destination = root.join(checked_relative(before_path)?);
        if destination.exists() {
            return Err(format!("{before_path} 已存在，无法撤销"));
        }
        let parent = fs::canonicalize(destination.parent().ok_or("目标路径无效")?)
            .map_err(|error| error.to_string())?;
        if !parent.starts_with(root) {
            return Err("目标路径不在项目内".to_string());
        }
        fs::rename(&backup, &destination).map_err(|error| error.to_string())?;
        if let Err(error) = before_data.save(root) {
            let _ = fs::rename(&destination, &backup);
            return Err(error);
        }
        let undo = metadata_entry(
            "undo_delete",
            before_path,
            after_data.clone(),
            before_data.clone(),
        );
        return match append(root, undo) {
            Ok(recorded) => Ok(recorded),
            Err(error) => {
                let _ = fs::rename(&destination, &backup);
                let _ = after_data.save(root);
                Err(error)
            }
        };
    }
    if let (Some(before), Some(after)) = (&original.before_data, &original.after_data) {
        let current = ProjectData::load(root)?;
        if current != *after {
            return Err(format!(
                "{} 的项目状态已有后续修改，无法撤销",
                original.target
            ));
        }
        if !original.target.is_empty() {
            current.require_unlocked(&original.target)?;
        }
        before.save(root)?;
        let undo = metadata_entry("undo", &original.target, after.clone(), before.clone());
        return match append(root, undo) {
            Ok(recorded) => Ok(recorded),
            Err(error) => {
                let _ = after.save(root);
                Err(error)
            }
        };
    }

    if let (None, Some(after)) = (&original.before_directory, &original.after_directory) {
        let absolute = crate::project::checked_member(root, after)?;
        ProjectData::load(root)?.require_unlocked(after)?;
        if !absolute.is_dir()
            || fs::read_dir(&absolute)
                .map_err(|error| error.to_string())?
                .next()
                .is_some()
        {
            return Err(format!("{after} 已有内容或已改变，无法撤销"));
        }
        fs::remove_dir(&absolute).map_err(|error| error.to_string())?;
        return match append(
            root,
            directory_entry("undo", after, Some(after.clone()), None),
        ) {
            Ok(recorded) => Ok(recorded),
            Err(error) => {
                let _ = fs::create_dir(&absolute);
                Err(error)
            }
        };
    }
    if let (Some(before), None) = (&original.before_directory, &original.after_directory) {
        let relative = checked_relative(before)?;
        let absolute = root.join(relative);
        if absolute.exists() {
            return Err(format!("{before} 已存在，无法撤销"));
        }
        let parent = fs::canonicalize(absolute.parent().ok_or("目录路径无效")?)
            .map_err(|error| error.to_string())?;
        if !parent.starts_with(root) {
            return Err("目录不在项目内".to_string());
        }
        fs::create_dir(&absolute).map_err(|error| error.to_string())?;
        return match append(
            root,
            directory_entry("undo", before, None, Some(before.clone())),
        ) {
            Ok(recorded) => Ok(recorded),
            Err(error) => {
                let _ = fs::remove_dir(&absolute);
                Err(error)
            }
        };
    }

    let path = checked_relative(&original.target)?;
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        return Err("记录目标不是 Markdown 文档".to_string());
    }
    ProjectData::load(root)?.require_unlocked(&original.target)?;
    let candidate = root.join(path);
    let parent = candidate.parent().ok_or("记录路径无效")?;
    let canonical_parent =
        fs::canonicalize(parent).map_err(|_| "目标目录已改变，无法撤销".to_string())?;
    if !canonical_parent.starts_with(root) {
        return Err("目标目录已移出项目，无法撤销".to_string());
    }
    let absolute = canonical_parent.join(candidate.file_name().ok_or("记录路径无效")?);
    if absolute.exists() {
        checked_document(root, &original.target)?;
    }
    let current = match fs::read_to_string(&absolute) {
        Ok(content) => Some(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.to_string()),
    };
    if current.as_ref().map(|content| revision(content))
        != original
            .after_content
            .as_ref()
            .map(|content| revision(content))
    {
        return Err(format!("{} 已有后续修改，无法撤销", original.target));
    }
    match &original.before_content {
        Some(content) => {
            fs::write(&absolute, content).map_err(|error| error.to_string())?;
        }
        None => {
            if absolute.exists() {
                fs::remove_file(&absolute).map_err(|error| error.to_string())?;
            }
        }
    }
    let undo = document_entry(
        "undo",
        &original.target,
        original.after_content,
        original.before_content,
    );
    match append(root, undo) {
        Ok(recorded) => Ok(recorded),
        Err(error) => {
            match current {
                Some(content) => {
                    let _ = fs::write(&absolute, content);
                }
                None => {
                    let _ = fs::remove_file(&absolute);
                }
            }
            Err(error)
        }
    }
}
