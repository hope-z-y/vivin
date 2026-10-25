use crate::metadata::{Annotation, ProjectData, Term, TermException};
use base64::Engine;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use tauri::State;
use uuid::Uuid;

#[derive(Default)]
pub struct ProjectSession {
    root: Mutex<Option<PathBuf>>,
    mutation: Mutex<()>,
    dirty: Mutex<bool>,
    drafts: Mutex<HashMap<String, Draft>>,
    draft_sequences: Mutex<HashMap<String, u64>>,
}

#[derive(Clone)]
pub(crate) struct Draft {
    pub content: String,
    pub base_revision: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub(crate) name: String,
    pub(crate) path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeNode {
    name: String,
    path: String,
    kind: &'static str,
    children: Vec<TreeNode>,
    pinned: bool,
    lock_source: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub(crate) path: String,
    pub(crate) content: String,
    pub(crate) revision: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    path: String,
    offset: usize,
    excerpt: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceNode {
    name: String,
    path: String,
    local_path: String,
    kind: &'static str,
    file_type: String,
    children: Vec<ResourceNode>,
}

impl ProjectSession {
    pub(crate) fn root(&self) -> Result<PathBuf, String> {
        self.root
            .lock()
            .map_err(|_| "项目状态不可用".to_string())?
            .clone()
            .ok_or_else(|| "尚未打开项目".to_string())
    }

    pub(crate) fn mutation_guard(&self) -> Result<std::sync::MutexGuard<'_, ()>, String> {
        self.mutation
            .lock()
            .map_err(|_| "项目写入状态不可用".to_string())
    }

    pub(crate) fn set_root(&self, root: PathBuf) -> Result<(), String> {
        self.draft_sequences
            .lock()
            .map_err(|_| "草稿状态不可用".to_string())?
            .clear();
        self.drafts
            .lock()
            .map_err(|_| "草稿状态不可用".to_string())?
            .clear();
        *self
            .dirty
            .lock()
            .map_err(|_| "编辑状态不可用".to_string())? = false;
        *self.root.lock().map_err(|_| "项目状态不可用".to_string())? = Some(root);
        Ok(())
    }

    pub(crate) fn clear_root(&self) -> Result<(), String> {
        self.draft_sequences
            .lock()
            .map_err(|_| "草稿状态不可用".to_string())?
            .clear();
        self.drafts
            .lock()
            .map_err(|_| "草稿状态不可用".to_string())?
            .clear();
        *self
            .dirty
            .lock()
            .map_err(|_| "编辑状态不可用".to_string())? = false;
        *self.root.lock().map_err(|_| "项目状态不可用".to_string())? = None;
        Ok(())
    }

    pub(crate) fn has_dirty_draft(&self) -> Result<bool, String> {
        Ok(*self
            .dirty
            .lock()
            .map_err(|_| "编辑状态不可用".to_string())?
            || !self
                .drafts
                .lock()
                .map_err(|_| "草稿状态不可用".to_string())?
                .is_empty())
    }

    pub(crate) fn draft(&self, path: &str) -> Result<Option<Draft>, String> {
        Ok(self
            .drafts
            .lock()
            .map_err(|_| "草稿状态不可用".to_string())?
            .get(path)
            .cloned())
    }

    pub(crate) fn clear_draft_if(&self, path: &str, content: &str) -> Result<(), String> {
        let mut drafts = self
            .drafts
            .lock()
            .map_err(|_| "草稿状态不可用".to_string())?;
        if drafts
            .get(path)
            .is_some_and(|draft| draft.content == content)
        {
            drafts.remove(path);
        }
        Ok(())
    }

    pub(crate) fn remove_draft(&self, path: &str) -> Result<(), String> {
        self.drafts
            .lock()
            .map_err(|_| "草稿状态不可用".to_string())?
            .remove(path);
        Ok(())
    }

    pub(crate) fn has_draft_under(&self, path: &str) -> Result<bool, String> {
        Ok(self
            .drafts
            .lock()
            .map_err(|_| "草稿状态不可用".to_string())?
            .keys()
            .any(|candidate| candidate == path || candidate.starts_with(&format!("{path}/"))))
    }
}

#[tauri::command]
pub fn set_dirty_draft(dirty: bool, state: State<'_, ProjectSession>) -> Result<(), String> {
    *state
        .dirty
        .lock()
        .map_err(|_| "编辑状态不可用".to_string())? = dirty;
    Ok(())
}

#[tauri::command]
pub fn publish_draft(
    path: String,
    content: String,
    base_revision: String,
    sequence: u64,
    state: State<'_, ProjectSession>,
) -> Result<String, String> {
    let _guard = state.mutation_guard()?;
    let mut sequences = state
        .draft_sequences
        .lock()
        .map_err(|_| "草稿状态不可用".to_string())?;
    if sequences
        .get(&path)
        .is_some_and(|previous| *previous >= sequence)
    {
        return Ok(state
            .draft(&path)?
            .map_or_else(|| base_revision.clone(), |draft| revision(&draft.content)));
    }
    sequences.insert(path.clone(), sequence);
    let root = state.root()?;
    let file = checked_document(&root, &path)?;
    ProjectData::load(&root)?.require_unlocked(&path)?;
    let disk = fs::read_to_string(file).map_err(|error| error.to_string())?;
    if revision(&disk) != base_revision {
        if disk == content {
            state.remove_draft(&path)?;
            return Ok(revision(&content));
        }
        return Err("磁盘文档已改变，请先处理冲突".to_string());
    }
    let mut drafts = state
        .drafts
        .lock()
        .map_err(|_| "草稿状态不可用".to_string())?;
    if content == disk {
        drafts.remove(&path);
    } else {
        drafts.insert(
            path,
            Draft {
                content: content.clone(),
                base_revision,
            },
        );
    }
    Ok(revision(&content))
}

pub(crate) fn revision(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

pub(crate) fn utf16_to_byte(content: &str, offset: usize) -> Option<usize> {
    let mut utf16_offset = 0;
    for (byte_offset, character) in content.char_indices() {
        if utf16_offset == offset {
            return Some(byte_offset);
        }
        utf16_offset += character.len_utf16();
        if utf16_offset > offset {
            return None;
        }
    }
    (utf16_offset == offset).then_some(content.len())
}

pub(crate) fn checked_relative(path: &str) -> Result<&Path, String> {
    let relative = Path::new(path);
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("路径必须位于当前项目内".to_string());
    }
    if relative
        .components()
        .any(|component| component.as_os_str().to_string_lossy().starts_with('.'))
    {
        return Err("不能操作隐藏项目数据".to_string());
    }
    Ok(relative)
}

pub(crate) fn safe_project_directory(
    root: &Path,
    relative: &str,
    create: bool,
) -> Result<PathBuf, String> {
    let relative = Path::new(relative);
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("项目目录路径无效".to_string());
    }
    let mut path = root.to_path_buf();
    for component in relative.components() {
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err("项目目录不能是符号链接或文件".to_string());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && create => {
                fs::create_dir(&path).map_err(|error| error.to_string())?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(root.join(relative));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    let canonical = fs::canonicalize(&path).map_err(|error| error.to_string())?;
    if !canonical.starts_with(root) {
        return Err("项目目录不在当前项目内".to_string());
    }
    Ok(canonical)
}

pub(crate) fn checked_member(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = checked_relative(relative)?;
    if relative
        .components()
        .next()
        .is_some_and(|part| part.as_os_str() == "resources")
    {
        return Err("资源库不能作为文档目录操作".to_string());
    }
    let absolute = fs::canonicalize(root.join(relative)).map_err(|error| error.to_string())?;
    if !absolute.starts_with(root) {
        return Err("路径不在当前项目内".to_string());
    }
    Ok(absolute)
}

fn relative_key(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path.strip_prefix(root).map_err(|error| error.to_string())?;
    Ok(relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/"))
}

pub(crate) fn checked_document(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = checked_relative(relative)?;
    if !relative
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        return Err("只能编辑 Markdown 文档".to_string());
    }
    let absolute = checked_member(root, &relative.to_string_lossy())?;
    if !absolute.is_file() {
        return Err("文档不在当前项目内".to_string());
    }
    Ok(absolute)
}

pub(crate) fn checked_rules(root: &Path) -> Result<PathBuf, String> {
    if fs::symlink_metadata(root.join("AGENTS.md"))
        .map_err(|error| error.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("项目规则文件不能是符号链接".to_string());
    }
    let path = fs::canonicalize(root.join("AGENTS.md")).map_err(|error| error.to_string())?;
    if !path.starts_with(root) || !path.is_file() {
        return Err("项目规则文件不在当前项目内".to_string());
    }
    Ok(path)
}

pub(crate) fn collect_tree(
    root: &Path,
    directory: &Path,
    depth: usize,
    data: &ProjectData,
) -> Result<Vec<TreeNode>, String> {
    if depth > 64 {
        return Err("目录层级超过支持范围".to_string());
    }

    let mut nodes = Vec::new();
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.')
            || (directory == root
                && (name == "resources" || name == "attachments" || name == "AGENTS.md"))
        {
            continue;
        }

        let path = entry.path();
        let relative = relative_key(root, &path)?;
        if file_type.is_dir() {
            nodes.push(TreeNode {
                name,
                path: relative.clone(),
                kind: "directory",
                children: collect_tree(root, &path, depth + 1, data)?,
                pinned: false,
                lock_source: data.lock_source(&relative),
            });
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            nodes.push(TreeNode {
                name,
                path: relative.clone(),
                kind: "document",
                children: Vec::new(),
                pinned: data.pinned.contains(&relative),
                lock_source: data.lock_source(&relative),
            });
        }
    }

    nodes.sort_by(|left, right| {
        (
            left.kind != "directory",
            !left.pinned,
            left.name.to_lowercase(),
        )
            .cmp(&(
                right.kind != "directory",
                !right.pinned,
                right.name.to_lowercase(),
            ))
    });
    Ok(nodes)
}

#[tauri::command]
pub fn open_project(path: String, state: State<'_, ProjectSession>) -> Result<ProjectInfo, String> {
    let _guard = state.mutation_guard()?;
    let root = fs::canonicalize(path).map_err(|error| error.to_string())?;
    if !root.is_dir() {
        return Err("请选择项目文件夹".to_string());
    }

    let name = root
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.to_string_lossy().into_owned());
    let info = ProjectInfo {
        name,
        path: root.to_string_lossy().into_owned(),
    };
    ProjectData::load(&root)?;
    let rules = root.join("AGENTS.md");
    if !rules.exists() {
        fs::write(&rules, "# 项目写作规则\n\n").map_err(|error| error.to_string())?;
    }
    if !fs::canonicalize(&rules)
        .map_err(|error| error.to_string())?
        .starts_with(&root)
    {
        return Err("项目规则文件不能指向项目外部".to_string());
    }
    state.set_root(root)?;
    Ok(info)
}

#[tauri::command]
pub fn create_project(
    parent: String,
    name: String,
    state: State<'_, ProjectSession>,
) -> Result<ProjectInfo, String> {
    let _guard = state.mutation_guard()?;
    let parent = fs::canonicalize(parent).map_err(|error| error.to_string())?;
    if !parent.is_dir() {
        return Err("请选择用于存放项目的文件夹".to_string());
    }
    let name_path = checked_relative(&name)?;
    if name_path.components().count() != 1 || name.starts_with('.') {
        return Err("项目名称不能包含路径或以点开头".to_string());
    }

    let root = parent.join(&name);
    fs::create_dir(&root).map_err(|error| error.to_string())?;
    let result = (|| {
        fs::create_dir(root.join("resources")).map_err(|error| error.to_string())?;
        fs::write(root.join("AGENTS.md"), "# 项目写作规则\n\n")
            .map_err(|error| error.to_string())?;
        Ok::<(), String>(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&root);
        return Err(error);
    }

    let info = ProjectInfo {
        name,
        path: root.to_string_lossy().into_owned(),
    };
    state.set_root(root)?;
    Ok(info)
}

#[tauri::command]
pub fn list_project_tree(state: State<'_, ProjectSession>) -> Result<Vec<TreeNode>, String> {
    let root = state.root()?;
    let data = ProjectData::load(&root)?;
    collect_tree(&root, &root, 0, &data)
}

#[tauri::command]
pub fn read_document(path: String, state: State<'_, ProjectSession>) -> Result<Document, String> {
    let root = state.root()?;
    let absolute = checked_document(&root, &path)?;
    let content = fs::read_to_string(absolute).map_err(|error| error.to_string())?;
    Ok(Document {
        path,
        revision: revision(&content),
        content,
    })
}

#[tauri::command]
pub fn save_document(
    path: String,
    content: String,
    expected_revision: String,
    state: State<'_, ProjectSession>,
) -> Result<Document, String> {
    let _guard = state.mutation_guard()?;
    let root = state.root()?;
    let saved = save_document_unlocked(&root, path.clone(), content.clone(), expected_revision)?;
    state.clear_draft_if(&path, &content)?;
    Ok(saved)
}

pub(crate) fn save_document_unlocked(
    root: &Path,
    path: String,
    content: String,
    expected_revision: String,
) -> Result<Document, String> {
    let absolute = checked_document(&root, &path)?;
    ProjectData::load(&root)?.require_unlocked(&path)?;
    let current = fs::read_to_string(&absolute).map_err(|error| error.to_string())?;
    if revision(&current) != expected_revision {
        return Err("文档已被其他操作修改，请重新加载或另存为新文档".to_string());
    }

    let parent = absolute
        .parent()
        .ok_or_else(|| "文档路径无效".to_string())?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    temporary
        .write_all(content.as_bytes())
        .map_err(|error| error.to_string())?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    let latest = fs::read_to_string(&absolute).map_err(|error| error.to_string())?;
    if revision(&latest) != expected_revision {
        return Err("文档已被其他操作修改，请重新加载或另存为新文档".to_string());
    }
    temporary
        .persist(&absolute)
        .map_err(|error| error.to_string())?;

    Ok(Document {
        path,
        revision: revision(&content),
        content,
    })
}

#[tauri::command]
pub fn create_document(
    directory: String,
    name: String,
    state: State<'_, ProjectSession>,
) -> Result<Document, String> {
    let root = state.root()?;
    let _guard = state
        .mutation
        .lock()
        .map_err(|_| "项目写入状态不可用".to_string())?;
    let parent = if directory.is_empty() {
        root.clone()
    } else {
        let resolved = checked_member(&root, &directory)?;
        if !resolved.is_dir() {
            return Err("目录不在当前项目内".to_string());
        }
        resolved
    };

    let file_name = checked_relative(&name)?;
    if file_name.components().count() != 1
        || !file_name
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        return Err("文档名称必须以 .md 结尾且不能包含路径".to_string());
    }
    let absolute = parent.join(file_name);
    let path = relative_key(&root, &absolute)?;
    ProjectData::load(&root)?.require_unlocked(&path)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&absolute)
        .map_err(|error| error.to_string())?;
    Ok(Document {
        path,
        content: String::new(),
        revision: revision(""),
    })
}

#[tauri::command]
pub fn create_directory(
    directory: String,
    name: String,
    state: State<'_, ProjectSession>,
) -> Result<String, String> {
    let root = state.root()?;
    let _guard = state
        .mutation
        .lock()
        .map_err(|_| "项目写入状态不可用".to_string())?;
    let parent = if directory.is_empty() {
        root.clone()
    } else {
        let parent = checked_member(&root, &directory)?;
        if !parent.is_dir() {
            return Err("请选择目录".to_string());
        }
        parent
    };
    let name_path = checked_relative(&name)?;
    if name_path.components().count() != 1 {
        return Err("目录名称不能包含路径".to_string());
    }
    let path = relative_key(&root, &parent.join(name_path))?;
    ProjectData::load(&root)?.require_unlocked(&path)?;
    fs::create_dir(parent.join(name_path)).map_err(|error| error.to_string())?;
    Ok(path)
}

#[tauri::command]
pub fn move_entry(
    path: String,
    destination_directory: String,
    name: String,
    state: State<'_, ProjectSession>,
) -> Result<String, String> {
    let root = state.root()?;
    let _guard = state
        .mutation
        .lock()
        .map_err(|_| "项目写入状态不可用".to_string())?;
    let source = checked_member(&root, &path)?;
    let destination = if destination_directory.is_empty() {
        root.clone()
    } else {
        checked_member(&root, &destination_directory)?
    };
    if !destination.is_dir() || destination.starts_with(&source) {
        return Err("目标目录无效或位于被移动目录之内".to_string());
    }
    let name_path = checked_relative(&name)?;
    if name_path.components().count() != 1 || (source.is_file() && !name.ends_with(".md")) {
        return Err("名称无效，文档必须以 .md 结尾".to_string());
    }
    let target = destination.join(name_path);
    if target.exists() {
        return Err("目标路径已存在".to_string());
    }
    let new_path = relative_key(&root, &target)?;
    let mut data = ProjectData::load(&root)?;
    data.require_unlocked(&path)?;
    data.require_unlocked(&new_path)?;
    if data
        .locked
        .iter()
        .any(|locked| locked.starts_with(&format!("{path}/")))
    {
        return Err("目录内有独立锁定的内容，请先解锁".to_string());
    }
    data.remap_paths(&path, &new_path);
    fs::rename(&source, &target).map_err(|error| error.to_string())?;
    if let Err(error) = data.save(&root) {
        let _ = fs::rename(&target, &source);
        return Err(error);
    }
    Ok(new_path)
}

#[tauri::command]
pub fn delete_entry(path: String, state: State<'_, ProjectSession>) -> Result<(), String> {
    let root = state.root()?;
    let _guard = state
        .mutation
        .lock()
        .map_err(|_| "项目写入状态不可用".to_string())?;
    let source = checked_member(&root, &path)?;
    let mut data = ProjectData::load(&root)?;
    data.require_unlocked(&path)?;
    if data
        .locked
        .iter()
        .any(|locked| locked.starts_with(&format!("{path}/")))
    {
        return Err("目录内有独立锁定的内容，请先解锁".to_string());
    }
    let trash = safe_project_directory(&root, ".vivin/trash", true)?;
    let staged = trash.join(Uuid::new_v4().to_string());
    fs::rename(&source, &staged).map_err(|error| error.to_string())?;
    data.remove_paths(&path);
    if let Err(error) = data.save(&root) {
        let _ = fs::rename(&staged, &source);
        return Err(error);
    }
    if staged.is_dir() {
        fs::remove_dir_all(staged).map_err(|error| error.to_string())?;
    } else {
        fs::remove_file(staged).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_pinned(
    path: String,
    pinned: bool,
    state: State<'_, ProjectSession>,
) -> Result<(), String> {
    let root = state.root()?;
    let _guard = state
        .mutation
        .lock()
        .map_err(|_| "项目写入状态不可用".to_string())?;
    checked_document(&root, &path)?;
    let mut data = ProjectData::load(&root)?;
    data.require_unlocked(&path)?;
    if pinned {
        data.pinned.insert(path);
    } else {
        data.pinned.remove(&path);
    }
    data.save(&root)
}

#[tauri::command]
pub fn set_locked(
    path: String,
    locked: bool,
    state: State<'_, ProjectSession>,
) -> Result<(), String> {
    let root = state.root()?;
    let _guard = state
        .mutation
        .lock()
        .map_err(|_| "项目写入状态不可用".to_string())?;
    checked_member(&root, &path)?;
    let mut data = ProjectData::load(&root)?;
    if !locked && data.lock_source(&path).is_some_and(|source| source != path) {
        return Err("该项目由上级目录锁定，请先解锁上级目录".to_string());
    }
    if locked {
        data.locked.insert(path);
    } else {
        data.locked.remove(&path);
    }
    data.save(&root)
}

#[tauri::command]
pub fn get_project_data(state: State<'_, ProjectSession>) -> Result<ProjectData, String> {
    ProjectData::load(&state.root()?)
}

#[tauri::command]
pub fn upsert_term(
    id: Option<String>,
    name: String,
    definition: String,
    state: State<'_, ProjectSession>,
) -> Result<Term, String> {
    let root = state.root()?;
    let _guard = state
        .mutation
        .lock()
        .map_err(|_| "项目写入状态不可用".to_string())?;
    let name = name.trim().to_string();
    if name.is_empty() || name.len() > 200 {
        return Err("术语名称必须为 1–200 个字符".to_string());
    }
    let mut data = ProjectData::load(&root)?;
    if data
        .terms
        .iter()
        .any(|term| term.name == name && id.as_deref() != Some(&term.id))
    {
        return Err("项目中已有同名术语".to_string());
    }
    let term = Term {
        id: id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        name,
        definition,
    };
    if let Some(existing) = data.terms.iter_mut().find(|item| item.id == term.id) {
        *existing = term.clone();
    } else {
        data.terms.push(term.clone());
    }
    data.save(&root)?;
    Ok(term)
}

#[tauri::command]
pub fn delete_term(id: String, state: State<'_, ProjectSession>) -> Result<(), String> {
    let root = state.root()?;
    let _guard = state
        .mutation
        .lock()
        .map_err(|_| "项目写入状态不可用".to_string())?;
    let mut data = ProjectData::load(&root)?;
    data.terms.retain(|term| term.id != id);
    data.term_exceptions
        .retain(|exception| exception.term_id != id);
    data.save(&root)
}

#[tauri::command]
pub fn set_term_exception(
    document: String,
    term_id: String,
    offset: usize,
    disabled: bool,
    exception_id: Option<String>,
    state: State<'_, ProjectSession>,
) -> Result<Option<TermException>, String> {
    let _guard = state.mutation_guard()?;
    let root = state.root()?;
    let file = checked_document(&root, &document)?;
    let mut data = ProjectData::load(&root)?;
    data.require_unlocked(&document)?;
    let term = data
        .terms
        .iter()
        .find(|term| term.id == term_id)
        .ok_or("术语不存在")?;
    if !disabled {
        data.term_exceptions.retain(|item| {
            if let Some(id) = &exception_id {
                &item.id != id
            } else {
                !(item.document == document && item.term_id == term_id && item.offset == offset)
            }
        });
        data.save(&root)?;
        return Ok(None);
    }
    let disk = fs::read_to_string(file).map_err(|error| error.to_string())?;
    let content = if let Some(draft) = state.draft(&document)? {
        if revision(&disk) != draft.base_revision {
            return Err("文档发生外部修改，请先处理冲突".to_string());
        }
        draft.content
    } else {
        disk
    };
    let start = utf16_to_byte(&content, offset).ok_or("术语位置无效")?;
    let end_offset = offset
        .checked_add(term.name.encode_utf16().count())
        .ok_or("术语位置无效")?;
    let end = utf16_to_byte(&content, end_offset).ok_or("术语位置无效")?;
    if content.get(start..end) != Some(term.name.as_str()) {
        return Err("术语位置与当前原文不匹配".to_string());
    }
    if let Some(existing) = data
        .term_exceptions
        .iter()
        .find(|item| item.document == document && item.term_id == term_id && item.offset == offset)
    {
        return Ok(Some(existing.clone()));
    }
    let exception = TermException {
        id: Uuid::new_v4().to_string(),
        document,
        term_id,
        offset,
        context_before: content[..start]
            .chars()
            .rev()
            .take(32)
            .collect::<String>()
            .chars()
            .rev()
            .collect(),
        context_after: content[end..].chars().take(32).collect(),
    };
    data.term_exceptions.push(exception.clone());
    data.save(&root)?;
    Ok(Some(exception))
}

#[tauri::command]
pub fn upsert_annotation(
    annotation: Annotation,
    state: State<'_, ProjectSession>,
) -> Result<Annotation, String> {
    let root = state.root()?;
    let _guard = state
        .mutation
        .lock()
        .map_err(|_| "项目写入状态不可用".to_string())?;
    checked_document(&root, &annotation.document)?;
    let mut data = ProjectData::load(&root)?;
    data.require_unlocked(&annotation.document)?;
    if annotation.quote.is_empty() {
        return Err("请先选择要标注的原文".to_string());
    }
    let mut annotation = annotation;
    if annotation.id.is_empty() {
        let file = checked_document(&root, &annotation.document)?;
        let disk = fs::read_to_string(file).map_err(|error| error.to_string())?;
        let content = if let Some(draft) = state.draft(&annotation.document)? {
            if revision(&disk) != draft.base_revision {
                return Err("文档发生外部修改，请先处理冲突".to_string());
            }
            draft.content
        } else {
            disk
        };
        let start = utf16_to_byte(&content, annotation.offset).ok_or("标注位置无效")?;
        let end_offset = annotation
            .offset
            .checked_add(annotation.quote.encode_utf16().count())
            .ok_or("标注位置无效")?;
        let end = utf16_to_byte(&content, end_offset).ok_or("标注位置无效")?;
        if content.get(start..end) != Some(annotation.quote.as_str()) {
            return Err("标注位置与当前原文不匹配".to_string());
        }
        annotation.id = Uuid::new_v4().to_string();
    }
    if let Some(existing) = data
        .annotations
        .iter_mut()
        .find(|item| item.id == annotation.id)
    {
        if existing.document != annotation.document {
            return Err("不能更改标注所属文档".to_string());
        }
        *existing = annotation.clone();
    } else {
        data.annotations.push(annotation.clone());
    }
    data.save(&root)?;
    Ok(annotation)
}

#[tauri::command]
pub fn delete_annotation(id: String, state: State<'_, ProjectSession>) -> Result<(), String> {
    let root = state.root()?;
    let _guard = state
        .mutation
        .lock()
        .map_err(|_| "项目写入状态不可用".to_string())?;
    let mut data = ProjectData::load(&root)?;
    let annotation = data
        .annotations
        .iter()
        .find(|item| item.id == id)
        .ok_or("标注不存在")?;
    data.require_unlocked(&annotation.document)?;
    data.annotations.retain(|item| item.id != id);
    data.save(&root)
}

#[tauri::command]
pub fn search_documents(
    query: String,
    state: State<'_, ProjectSession>,
) -> Result<Vec<SearchHit>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    if query.chars().count() > 200 {
        return Err("搜索词过长".to_string());
    }
    let root = state.root()?;
    let data = ProjectData::load(&root)?;
    fn walk(
        root: &Path,
        nodes: Vec<TreeNode>,
        query: &str,
        hits: &mut Vec<SearchHit>,
    ) -> Result<(), String> {
        for node in nodes {
            if node.kind == "directory" {
                walk(root, node.children, query, hits)?;
            } else {
                let text =
                    fs::read_to_string(root.join(&node.path)).map_err(|error| error.to_string())?;
                for (offset, _) in text.match_indices(query).take(100) {
                    let start = text[..offset]
                        .char_indices()
                        .rev()
                        .nth(40)
                        .map_or(0, |(index, _)| index);
                    let end_offset = offset + query.len();
                    let end = text[end_offset..]
                        .char_indices()
                        .nth(40)
                        .map_or(text.len(), |(index, _)| end_offset + index);
                    hits.push(SearchHit {
                        path: node.path.clone(),
                        offset: text[..offset].encode_utf16().count(),
                        excerpt: text[start..end].replace('\n', " "),
                    });
                }
            }
        }
        Ok(())
    }
    let mut hits = Vec::new();
    walk(
        &root,
        collect_tree(&root, &root, 0, &data)?,
        query,
        &mut hits,
    )?;
    Ok(hits)
}

pub(crate) fn collect_resources(
    root: &Path,
    directory: &Path,
    depth: usize,
) -> Result<Vec<ResourceNode>, String> {
    if depth > 64 {
        return Err("资源目录层级超过支持范围".to_string());
    }
    let mut nodes = Vec::new();
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = relative_key(root, &path)?;
        if file_type.is_dir() {
            nodes.push(ResourceNode {
                name,
                path: relative,
                local_path: path.to_string_lossy().into_owned(),
                kind: "directory",
                file_type: String::new(),
                children: collect_resources(root, &path, depth + 1)?,
            });
        } else if file_type.is_file() {
            let extension = path.extension().map_or(String::new(), |value| {
                value.to_string_lossy().to_uppercase()
            });
            nodes.push(ResourceNode {
                name,
                path: relative,
                local_path: path.to_string_lossy().into_owned(),
                kind: "file",
                file_type: extension,
                children: Vec::new(),
            });
        }
    }
    nodes.sort_by(|a, b| {
        (a.kind != "directory", a.name.to_lowercase())
            .cmp(&(b.kind != "directory", b.name.to_lowercase()))
    });
    Ok(nodes)
}

#[tauri::command]
pub fn list_resources(state: State<'_, ProjectSession>) -> Result<Vec<ResourceNode>, String> {
    let root = state.root()?;
    let directory = safe_project_directory(&root, "resources", true)?;
    collect_resources(&root, &directory, 0)
}

#[tauri::command]
pub fn import_resource(
    source: String,
    directory: String,
    state: State<'_, ProjectSession>,
) -> Result<String, String> {
    let root = state.root()?;
    let _guard = state
        .mutation
        .lock()
        .map_err(|_| "项目写入状态不可用".to_string())?;
    let source = fs::canonicalize(source).map_err(|error| error.to_string())?;
    if !source.is_file() {
        return Err("请选择资源文件".to_string());
    }
    let resources = safe_project_directory(&root, "resources", true)?;
    let target_directory = if directory.is_empty() {
        resources.clone()
    } else {
        let path = checked_relative(&directory)?;
        let resolved = fs::canonicalize(root.join(path)).map_err(|error| error.to_string())?;
        if !resolved.starts_with(&resources) || !resolved.is_dir() {
            return Err("资源目标目录无效".to_string());
        }
        resolved
    };
    fs::create_dir_all(&target_directory).map_err(|error| error.to_string())?;
    let name = source.file_name().ok_or("资源文件名无效")?;
    let target = target_directory.join(name);
    if target.exists() {
        return Err("资源库中已有同名文件".to_string());
    }
    fs::copy(source, &target).map_err(|error| error.to_string())?;
    relative_key(&root, &target)
}

fn image_type(extension: &str) -> Option<&'static str> {
    match extension.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

#[tauri::command]
pub fn import_image(
    document: String,
    name: String,
    bytes_base64: String,
    state: State<'_, ProjectSession>,
) -> Result<String, String> {
    let _guard = state.mutation_guard()?;
    let root = state.root()?;
    checked_document(&root, &document)?;
    ProjectData::load(&root)?.require_unlocked(&document)?;
    let extension = Path::new(&name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    image_type(&extension).ok_or("仅支持 PNG、JPEG、GIF 和 WebP 图片")?;
    if bytes_base64.len() > 28_000_000 {
        return Err("图片不能超过 20 MB".to_string());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(bytes_base64)
        .map_err(|_| "图片数据无效".to_string())?;
    if bytes.len() > 20_000_000 {
        return Err("图片不能超过 20 MB".to_string());
    }
    let folder = safe_project_directory(&root, "attachments", true)?;
    let filename = format!("{}.{}", Uuid::new_v4(), extension);
    let target = folder.join(&filename);
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|error| error.to_string())?;
    output
        .write_all(&bytes)
        .map_err(|error| error.to_string())?;
    let depth = Path::new(&document)
        .parent()
        .map_or(0, |parent| parent.components().count());
    Ok(format!("{}attachments/{filename}", "../".repeat(depth)))
}

#[tauri::command]
pub fn read_project_image(
    document: String,
    source: String,
    state: State<'_, ProjectSession>,
) -> Result<String, String> {
    let root = state.root()?;
    let document_file = checked_document(&root, &document)?;
    let source_path = Path::new(&source);
    if source_path.is_absolute()
        || source_path
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::ParentDir))
    {
        return Err("图片路径无效".to_string());
    }
    let parent = document_file.parent().ok_or("文档路径无效")?;
    let absolute = fs::canonicalize(parent.join(source_path)).map_err(|error| error.to_string())?;
    if !absolute.starts_with(&root) || !absolute.is_file() {
        return Err("图片不在当前项目中".to_string());
    }
    let extension = absolute
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let mime = image_type(extension).ok_or("图片格式不受支持")?;
    let bytes = fs::read(absolute).map_err(|error| error.to_string())?;
    if bytes.len() > 20_000_000 {
        return Err("图片不能超过 20 MB".to_string());
    }
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[tauri::command]
pub fn read_project_rules(state: State<'_, ProjectSession>) -> Result<Document, String> {
    let root = state.root()?;
    let content = fs::read_to_string(checked_rules(&root)?).map_err(|error| error.to_string())?;
    Ok(Document {
        path: "AGENTS.md".to_string(),
        revision: revision(&content),
        content,
    })
}

#[tauri::command]
pub fn save_project_rules(
    content: String,
    expected_revision: String,
    state: State<'_, ProjectSession>,
) -> Result<Document, String> {
    let root = state.root()?;
    let _guard = state
        .mutation
        .lock()
        .map_err(|_| "项目写入状态不可用".to_string())?;
    let path = checked_rules(&root)?;
    let current = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    if revision(&current) != expected_revision {
        return Err("项目规则已被其他程序修改，请先重新加载".to_string());
    }
    let mut temporary =
        tempfile::NamedTempFile::new_in(&root).map_err(|error| error.to_string())?;
    temporary
        .write_all(content.as_bytes())
        .map_err(|error| error.to_string())?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    temporary.persist(path).map_err(|error| error.to_string())?;
    Ok(Document {
        path: "AGENTS.md".to_string(),
        revision: revision(&content),
        content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_paths_outside_the_project() {
        assert!(checked_relative("../outside.md").is_err());
        assert!(checked_relative("/absolute.md").is_err());
        assert!(checked_relative("./document.md").is_err());
        assert!(checked_relative("notes/document.md").is_ok());
    }

    #[test]
    fn tree_excludes_resources_and_uses_portable_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir(root.join("notes")).unwrap();
        fs::create_dir(root.join("resources")).unwrap();
        fs::write(root.join("notes/hello.md"), "hello").unwrap();
        fs::write(root.join("resources/source.pdf"), "source").unwrap();

        let tree = collect_tree(root, root, 0, &ProjectData::default()).unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].path, "notes");
        assert_eq!(tree[0].children[0].path, "notes/hello.md");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_to_document_outside_project() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_file = outside.path().join("outside.md");
        fs::write(&outside_file, "private").unwrap();
        symlink(&outside_file, project.path().join("link.md")).unwrap();

        let canonical_project = fs::canonicalize(project.path()).unwrap();
        assert!(checked_document(&canonical_project, "link.md").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_managed_directories() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(project.path()).unwrap();
        symlink(outside.path(), root.join("attachments")).unwrap();
        symlink(outside.path(), root.join(".vivin")).unwrap();

        assert!(safe_project_directory(&root, "attachments", true).is_err());
        assert!(safe_project_directory(&root, ".vivin/ai-trash", true).is_err());
        assert!(safe_project_directory(&root, ".vivin", false).is_err());
    }
}
