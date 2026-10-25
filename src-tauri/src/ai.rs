use crate::history::{self, HistoryEntry};
use crate::metadata::{Annotation, AnnotationKind, ProjectData, Term};
use crate::project::{self, ProjectSession};
use axum::{
    Router,
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use rmcp::{
    ErrorData as McpError, ServerHandler, handler::server::wrapper::Parameters, model::*,
    schemars::JsonSchema, tool, tool_handler, tool_router,
};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

const PORT: u16 = 38419;

#[derive(Clone)]
pub struct VivinMcp {
    app: AppHandle,
}

#[derive(Deserialize, JsonSchema)]
struct PathArg {
    path: String,
}

#[derive(Deserialize, JsonSchema)]
struct OpenProjectArg {
    path: String,
}
#[derive(Deserialize, JsonSchema)]
struct CreateProjectArg {
    parent: String,
    name: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct MoveArg {
    path: String,
    destination_directory: String,
    name: String,
    rules_revision: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DeleteEntryArg {
    path: String,
    rules_revision: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct WriteArg {
    path: String,
    content: String,
    expected_revision: String,
    rules_revision: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CreateArg {
    directory: String,
    name: String,
    rules_revision: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct AnnotationArg {
    document: String,
    kind: String,
    quote: String,
    offset: usize,
    note: String,
    rules_revision: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct TermArg {
    id: Option<String>,
    name: String,
    definition: String,
    rules_revision: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct IdArg {
    id: String,
    rules_revision: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct FlagArg {
    path: String,
    value: bool,
    rules_revision: String,
}
#[derive(Deserialize, JsonSchema)]
struct UndoArg {
    id: String,
}

fn result<T: serde::Serialize>(value: Result<T, String>) -> Result<CallToolResult, McpError> {
    Ok(match value {
        Ok(value) => CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()),
        )]),
        Err(error) => CallToolResult::error(vec![ContentBlock::text(error)]),
    })
}

fn rules(root: &Path, expected: &str) -> Result<String, String> {
    let content =
        fs::read_to_string(project::checked_rules(root)?).map_err(|error| error.to_string())?;
    if project::revision(&content) != expected {
        return Err("项目规则已改变，请先重新读取项目规则再操作".to_string());
    }
    Ok(content)
}

fn root(app: &AppHandle) -> Result<PathBuf, String> {
    app.state::<ProjectSession>().root()
}

#[tool_router]
impl VivinMcp {
    #[tool(
        description = "Create and open a new vivin project in a local parent directory. The new project includes resources and AGENTS.md. Project creation is recorded for undo."
    )]
    fn create_project(
        &self,
        Parameters(arg): Parameters<CreateProjectArg>,
    ) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            if state.has_dirty_draft()? {
                return Err("当前项目有未保存的编辑内容，请先保存或处理后再切换项目".to_string());
            }
            let parent = fs::canonicalize(&arg.parent).map_err(|error| error.to_string())?;
            if !parent.is_dir() {
                return Err("项目父目录不存在".to_string());
            }
            let name_path = project::checked_relative(&arg.name)?;
            if name_path.components().count() != 1 {
                return Err("项目名称不能包含路径".to_string());
            }
            let root = parent.join(name_path);
            fs::create_dir(&root).map_err(|error| error.to_string())?;
            let rules = "# 项目写作规则\n\n".to_string();
            let initial = (|| {
                fs::create_dir(root.join("resources")).map_err(|error| error.to_string())?;
                fs::write(root.join("AGENTS.md"), &rules).map_err(|error| error.to_string())?;
                let data = ProjectData::load(&root)?;
                data.save(&root)?;
                history::append(&root, history::project_created(rules.clone(), data))?;
                Ok::<(), String>(())
            })();
            if let Err(error) = initial {
                let _ = fs::remove_dir_all(&root);
                return Err(error);
            }
            state.set_root(root.clone())?;
            let _ = self.app.emit(
                "vivin://project-opened",
                json!({ "name": arg.name, "path": root }),
            );
            Ok(
                json!({ "name": arg.name, "path": root, "rules": rules, "rulesRevision": project::revision(&rules) }),
            )
        })())
    }

    fn new(app: AppHandle) -> Self {
        Self { app }
    }

    #[tool(
        description = "Open an existing local project folder in vivin and make it active. Returns the project path and AGENTS.md rules revision."
    )]
    fn open_project(
        &self,
        Parameters(arg): Parameters<OpenProjectArg>,
    ) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            if state.has_dirty_draft()? {
                return Err(
                    "当前项目有未保存的编辑内容，请先在 vivin 中保存或处理后再切换项目".to_string(),
                );
            }
            let root = fs::canonicalize(&arg.path).map_err(|error| error.to_string())?;
            if !root.is_dir() {
                return Err("请选择项目目录".to_string());
            }
            ProjectData::load(&root)?;
            let rules_path = root.join("AGENTS.md");
            if !rules_path.exists() {
                fs::write(&rules_path, "# 项目写作规则\n\n").map_err(|error| error.to_string())?;
            }
            let rules = fs::read_to_string(project::checked_rules(&root)?)
                .map_err(|error| error.to_string())?;
            let name = root.file_name().map_or_else(
                || root.to_string_lossy().into_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
            state.set_root(root.clone())?;
            let _ = self.app.emit(
                "vivin://project-opened",
                json!({ "name": name, "path": root }),
            );
            Ok(
                json!({ "name": name, "path": root, "rules": rules, "rulesRevision": project::revision(&rules) }),
            )
        })())
    }

    #[tool(
        description = "Get the active vivin project, its AGENTS.md writing rules and rule revision. Read this before every write; resolve conflicting instructions with the user."
    )]
    fn project_context(&self) -> Result<CallToolResult, McpError> {
        result((|| {
            let root = root(&self.app)?;
            let rules = fs::read_to_string(project::checked_rules(&root)?)
                .map_err(|error| error.to_string())?;
            let revision = project::revision(&rules);
            Ok(json!({ "path": root, "rules": rules, "rulesRevision": revision }))
        })())
    }

    #[tool(
        description = "List the active project's Markdown document tree, with pinned and lock information."
    )]
    fn list_project_tree(&self) -> Result<CallToolResult, McpError> {
        result((|| {
            let root = root(&self.app)?;
            project::collect_tree(&root, &root, 0, &ProjectData::load(&root)?)
        })())
    }

    #[tool(
        description = "Read a Markdown document from the active project. Returns content and revision. Paths are project-relative."
    )]
    fn read_document(
        &self,
        Parameters(arg): Parameters<PathArg>,
    ) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let root = state.root()?;
            let file = project::checked_document(&root, &arg.path)?;
            let disk = fs::read_to_string(file).map_err(|error| error.to_string())?;
            let content = if let Some(draft) = state.draft(&arg.path)? {
                if project::revision(&disk) != draft.base_revision {
                    return Err(
                        "文档在草稿编辑期间发生外部修改，请先在 vivin 中处理冲突".to_string()
                    );
                }
                draft.content
            } else {
                disk
            };
            Ok(
                json!({ "path": arg.path, "revision": project::revision(&content), "content": content, "lockSource": ProjectData::load(&root)?.lock_source(&arg.path) }),
            )
        })())
    }

    #[tool(
        description = "Replace a Markdown document using its exact expected revision. Requires the current AGENTS.md rules revision; writes are logged and locked files are rejected."
    )]
    fn write_document(
        &self,
        Parameters(arg): Parameters<WriteArg>,
    ) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            let root = state.root()?;
            rules(&root, &arg.rules_revision)?;
            let file = project::checked_document(&root, &arg.path)?;
            let before_disk = fs::read_to_string(&file).map_err(|error| error.to_string())?;
            let draft = state.draft(&arg.path)?;
            let before = if let Some(draft) = &draft {
                if project::revision(&before_disk) != draft.base_revision {
                    return Err("磁盘文档已改变，请先处理外部修改冲突".to_string());
                }
                if project::revision(&draft.content) != arg.expected_revision {
                    return Err("版本冲突：用户草稿已在读取后继续修改".to_string());
                }
                draft.content.clone()
            } else {
                if project::revision(&before_disk) != arg.expected_revision {
                    return Err("版本冲突：文档已在读取后修改".to_string());
                }
                before_disk.clone()
            };
            let expected_disk_revision =
                draft.map_or(arg.expected_revision, |draft| draft.base_revision);
            let saved = project::save_document_unlocked(
                &root,
                arg.path.clone(),
                arg.content.clone(),
                expected_disk_revision,
            )?;
            if let Err(error) = history::append(
                &root,
                history::document_entry(
                    "write_document",
                    &arg.path,
                    Some(before.clone()),
                    Some(arg.content.clone()),
                ),
            ) {
                let _ = fs::write(&file, before_disk);
                return Err(error);
            }
            state.remove_draft(&arg.path)?;
            let _ = self.app.emit(
                "vivin://document-written",
                json!({ "document": &saved, "beforeContent": before }),
            );
            Ok(saved)
        })())
    }

    #[tool(
        description = "Create a Markdown document in the active project. The directory is project-relative or empty for the root. Requires the current rules revision."
    )]
    fn create_document(
        &self,
        Parameters(arg): Parameters<CreateArg>,
    ) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            let root = state.root()?;
            rules(&root, &arg.rules_revision)?;
            let parent = if arg.directory.is_empty() {
                root.clone()
            } else {
                project::checked_member(&root, &arg.directory)?
            };
            if !parent.is_dir() {
                return Err("目标目录不存在".to_string());
            }
            let name = project::checked_relative(&arg.name)?;
            if name.components().count() != 1
                || !name
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                return Err("文档名称必须以 .md 结尾且不能包含路径".to_string());
            }
            let path = if arg.directory.is_empty() {
                arg.name.clone()
            } else {
                format!("{}/{}", arg.directory, arg.name)
            };
            ProjectData::load(&root)?.require_unlocked(&path)?;
            let file = parent.join(name);
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&file)
                .map_err(|error| error.to_string())?;
            if let Err(error) = history::append(
                &root,
                history::document_entry("create_document", &path, None, Some(String::new())),
            ) {
                let _ = fs::remove_file(&file);
                return Err(error);
            }
            let document = project::Document {
                path,
                content: String::new(),
                revision: project::revision(""),
            };
            let _ = self.app.emit("vivin://project-changed", ());
            Ok(document)
        })())
    }

    #[tool(
        description = "Create a directory in the active project. Requires current AGENTS.md rules revision."
    )]
    fn create_directory(
        &self,
        Parameters(arg): Parameters<CreateArg>,
    ) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            let root = state.root()?;
            rules(&root, &arg.rules_revision)?;
            let parent = if arg.directory.is_empty() {
                root.clone()
            } else {
                project::checked_member(&root, &arg.directory)?
            };
            if !parent.is_dir() {
                return Err("目标目录不存在".to_string());
            }
            let name = project::checked_relative(&arg.name)?;
            if name.components().count() != 1 {
                return Err("目录名称不能包含路径".to_string());
            }
            let path = if arg.directory.is_empty() {
                arg.name.clone()
            } else {
                format!("{}/{}", arg.directory, arg.name)
            };
            ProjectData::load(&root)?.require_unlocked(&path)?;
            let absolute = parent.join(name);
            fs::create_dir(&absolute).map_err(|error| error.to_string())?;
            if let Err(error) = history::append(
                &root,
                history::directory_entry("create_directory", &path, None, Some(path.clone())),
            ) {
                let _ = fs::remove_dir(&absolute);
                return Err(error);
            }
            let _ = self.app.emit("vivin://project-changed", ());
            Ok(json!({ "path": path }))
        })())
    }

    #[tool(
        description = "Move or rename a Markdown document or directory within the active project. Requires current project rules revision. The operation is recorded for undo."
    )]
    fn move_entry(&self, Parameters(arg): Parameters<MoveArg>) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            let root = state.root()?;
            rules(&root, &arg.rules_revision)?;
            let source = project::checked_member(&root, &arg.path)?;
            if state.has_draft_under(&arg.path)? {
                return Err("目标包含未保存草稿，请先保存".to_string());
            }
            let destination_dir = if arg.destination_directory.is_empty() {
                root.clone()
            } else {
                project::checked_member(&root, &arg.destination_directory)?
            };
            if !destination_dir.is_dir() || destination_dir.starts_with(&source) {
                return Err("目标目录无效或位于源目录内".to_string());
            }
            let name = project::checked_relative(&arg.name)?;
            if name.components().count() != 1 || (source.is_file() && !arg.name.ends_with(".md")) {
                return Err("目标名称无效".to_string());
            }
            let target = destination_dir.join(name);
            if target.exists() {
                return Err("目标路径已存在".to_string());
            }
            let new_path = if arg.destination_directory.is_empty() {
                arg.name.clone()
            } else {
                format!("{}/{}", arg.destination_directory, arg.name)
            };
            let mut data = ProjectData::load(&root)?;
            data.require_unlocked(&arg.path)?;
            data.require_unlocked(&new_path)?;
            if data
                .locked
                .iter()
                .any(|locked| locked.starts_with(&format!("{}/", arg.path)))
            {
                return Err("目录内有独立锁定内容，请先解锁".to_string());
            }
            let digest = history::tree_digest(&source)?;
            let before = data.clone();
            data.remap_paths(&arg.path, &new_path);
            fs::rename(&source, &target).map_err(|error| error.to_string())?;
            if let Err(error) = data.save(&root) {
                let _ = fs::rename(&target, &source);
                return Err(error);
            }
            if let Err(error) = history::append(
                &root,
                history::moved_entry(
                    "move_entry",
                    &arg.path,
                    &new_path,
                    before.clone(),
                    data,
                    digest,
                ),
            ) {
                let _ = fs::rename(&target, &source);
                let _ = before.save(&root);
                return Err(error);
            }
            let _ = self.app.emit(
                "vivin://entry-moved",
                json!({ "from": arg.path, "to": new_path }),
            );
            let _ = self.app.emit("vivin://project-changed", ());
            Ok(json!({ "path": new_path }))
        })())
    }

    #[tool(
        description = "Delete a Markdown document or directory and preserve a project-local backup for undo. Requires current project rules revision."
    )]
    fn delete_entry(
        &self,
        Parameters(arg): Parameters<DeleteEntryArg>,
    ) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            let root = state.root()?;
            rules(&root, &arg.rules_revision)?;
            let source = project::checked_member(&root, &arg.path)?;
            if state.has_draft_under(&arg.path)? {
                return Err("目标包含未保存草稿，请先保存".to_string());
            }
            let mut data = ProjectData::load(&root)?;
            data.require_unlocked(&arg.path)?;
            if data
                .locked
                .iter()
                .any(|locked| locked.starts_with(&format!("{}/", arg.path)))
            {
                return Err("目录内有独立锁定内容，请先解锁".to_string());
            }
            let digest = history::tree_digest(&source)?;
            let before = data.clone();
            data.remove_paths(&arg.path);
            let backup_dir = project::safe_project_directory(&root, ".vivin/ai-trash", true)?;
            let backup_id = Uuid::new_v4().to_string();
            let backup = backup_dir.join(&backup_id);
            fs::rename(&source, &backup).map_err(|error| error.to_string())?;
            if let Err(error) = data.save(&root) {
                let _ = fs::rename(&backup, &source);
                return Err(error);
            }
            if let Err(error) = history::append(
                &root,
                history::deleted_entry(&arg.path, &backup_id, before.clone(), data, digest),
            ) {
                let _ = fs::rename(&backup, &source);
                let _ = before.save(&root);
                return Err(error);
            }
            let _ = self
                .app
                .emit("vivin://entry-deleted", json!({ "path": arg.path }));
            let _ = self.app.emit("vivin://project-changed", ());
            Ok(json!({ "deleted": arg.path }))
        })())
    }

    #[tool(
        description = "List resource files in the active project's resources directory, returning their project-relative and local paths. Resource contents are not parsed."
    )]
    fn list_resources(&self) -> Result<CallToolResult, McpError> {
        result((|| {
            let root = root(&self.app)?;
            let dir = project::safe_project_directory(&root, "resources", false)?;
            if !dir.exists() {
                return Ok(json!([]));
            }
            let nodes = project::collect_resources(&root, &dir, 0)?;
            Ok(json!({ "projectRoot": root, "resources": nodes }))
        })())
    }

    #[tool(
        description = "List current project data, including terms, annotations, pins and locks."
    )]
    fn project_data(&self) -> Result<CallToolResult, McpError> {
        result((|| ProjectData::load(&root(&self.app)?))())
    }

    #[tool(
        description = "Add a todo, comment, question or error annotation to selected Markdown source text. The note may be empty. Requires current project rules revision."
    )]
    fn add_annotation(
        &self,
        Parameters(arg): Parameters<AnnotationArg>,
    ) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            let root = state.root()?;
            rules(&root, &arg.rules_revision)?;
            let file = project::checked_document(&root, &arg.document)?;
            let disk = fs::read_to_string(file).map_err(|error| error.to_string())?;
            let content = if let Some(draft) = state.draft(&arg.document)? {
                if project::revision(&disk) != draft.base_revision {
                    return Err("文档发生外部修改，请先处理冲突".to_string());
                }
                draft.content
            } else {
                disk
            };
            let end_offset = arg
                .offset
                .checked_add(arg.quote.encode_utf16().count())
                .ok_or("标注位置无效")?;
            let start = project::utf16_to_byte(&content, arg.offset).ok_or("标注位置无效")?;
            let end = project::utf16_to_byte(&content, end_offset).ok_or("标注位置无效")?;
            if arg.quote.is_empty() || content.get(start..end) != Some(arg.quote.as_str()) {
                return Err("标注位置与当前原文不匹配".to_string());
            }
            let kind = match arg.kind.as_str() {
                "todo" => AnnotationKind::Todo,
                "comment" => AnnotationKind::Comment,
                "question" => AnnotationKind::Question,
                "error" => AnnotationKind::Error,
                _ => return Err("标注类型无效".to_string()),
            };
            let mut data = ProjectData::load(&root)?;
            data.require_unlocked(&arg.document)?;
            let before = data.clone();
            let annotation = Annotation {
                id: Uuid::new_v4().to_string(),
                document: arg.document.clone(),
                kind,
                quote: arg.quote,
                offset: arg.offset,
                note: arg.note,
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
            data.annotations.push(annotation.clone());
            data.save(&root)?;
            if let Err(error) = history::append(
                &root,
                history::metadata_entry("add_annotation", &arg.document, before.clone(), data),
            ) {
                let _ = before.save(&root);
                return Err(error);
            }
            let _ = self.app.emit("vivin://project-changed", ());
            Ok(annotation)
        })())
    }

    #[tool(
        description = "Delete an annotation by ID. Requires the current project rules revision."
    )]
    fn delete_annotation(
        &self,
        Parameters(arg): Parameters<IdArg>,
    ) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            let root = state.root()?;
            rules(&root, &arg.rules_revision)?;
            let mut data = ProjectData::load(&root)?;
            let before = data.clone();
            let annotation = data
                .annotations
                .iter()
                .find(|item| item.id == arg.id)
                .ok_or("标注不存在")?;
            data.require_unlocked(&annotation.document)?;
            let target = annotation.document.clone();
            data.annotations.retain(|item| item.id != arg.id);
            data.save(&root)?;
            if let Err(error) = history::append(
                &root,
                history::metadata_entry("delete_annotation", &target, before.clone(), data),
            ) {
                let _ = before.save(&root);
                return Err(error);
            }
            let _ = self.app.emit("vivin://project-changed", ());
            Ok(json!({ "deleted": arg.id }))
        })())
    }

    #[tool(
        description = "Create or update a shared project term. Requires current project rules revision."
    )]
    fn upsert_term(
        &self,
        Parameters(arg): Parameters<TermArg>,
    ) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            let root = state.root()?;
            rules(&root, &arg.rules_revision)?;
            let mut data = ProjectData::load(&root)?;
            let before = data.clone();
            let name = arg.name.trim();
            if name.is_empty() {
                return Err("术语名称不能为空".to_string());
            }
            if data
                .terms
                .iter()
                .any(|item| item.name == name && Some(&item.id) != arg.id.as_ref())
            {
                return Err("术语名称已存在".to_string());
            }
            let term = Term {
                id: arg.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
                name: name.to_string(),
                definition: arg.definition,
            };
            if let Some(existing) = data.terms.iter_mut().find(|item| item.id == term.id) {
                *existing = term.clone();
            } else {
                data.terms.push(term.clone());
            }
            data.save(&root)?;
            if let Err(error) = history::append(
                &root,
                history::metadata_entry(
                    "upsert_term",
                    &format!("term:{}", term.id),
                    before.clone(),
                    data,
                ),
            ) {
                let _ = before.save(&root);
                return Err(error);
            }
            let _ = self.app.emit("vivin://project-changed", ());
            Ok(term)
        })())
    }

    #[tool(
        description = "Delete a shared project term by ID. Requires current project rules revision."
    )]
    fn delete_term(&self, Parameters(arg): Parameters<IdArg>) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            let root = state.root()?;
            rules(&root, &arg.rules_revision)?;
            let mut data = ProjectData::load(&root)?;
            let before = data.clone();
            if !data.terms.iter().any(|term| term.id == arg.id) {
                return Err("术语不存在".to_string());
            }
            data.terms.retain(|term| term.id != arg.id);
            data.term_exceptions
                .retain(|exception| exception.term_id != arg.id);
            data.save(&root)?;
            if let Err(error) = history::append(
                &root,
                history::metadata_entry(
                    "delete_term",
                    &format!("term:{}", arg.id),
                    before.clone(),
                    data,
                ),
            ) {
                let _ = before.save(&root);
                return Err(error);
            }
            let _ = self.app.emit("vivin://project-changed", ());
            Ok(json!({ "deleted": arg.id }))
        })())
    }

    #[tool(
        description = "Pin or unpin a Markdown document. Requires current project rules revision."
    )]
    fn set_pinned(&self, Parameters(arg): Parameters<FlagArg>) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            let root = state.root()?;
            rules(&root, &arg.rules_revision)?;
            project::checked_document(&root, &arg.path)?;
            let mut data = ProjectData::load(&root)?;
            data.require_unlocked(&arg.path)?;
            let before = data.clone();
            if arg.value {
                data.pinned.insert(arg.path.clone());
            } else {
                data.pinned.remove(&arg.path);
            }
            data.save(&root)?;
            if let Err(error) = history::append(
                &root,
                history::metadata_entry("set_pinned", &arg.path, before.clone(), data),
            ) {
                let _ = before.save(&root);
                return Err(error);
            }
            let _ = self.app.emit("vivin://project-changed", ());
            Ok(json!({ "path": arg.path, "pinned": arg.value }))
        })())
    }

    #[tool(
        description = "Lock or unlock a document or directory. An inherited lock cannot be removed on a child. Requires current project rules revision."
    )]
    fn set_locked(&self, Parameters(arg): Parameters<FlagArg>) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            let root = state.root()?;
            rules(&root, &arg.rules_revision)?;
            project::checked_member(&root, &arg.path)?;
            let mut data = ProjectData::load(&root)?;
            let before = data.clone();
            if !arg.value
                && data
                    .lock_source(&arg.path)
                    .is_some_and(|source| source != arg.path)
            {
                return Err("请先解锁上级目录".to_string());
            }
            if arg.value {
                data.locked.insert(arg.path.clone());
            } else {
                data.locked.remove(&arg.path);
            }
            data.save(&root)?;
            if let Err(error) = history::append(
                &root,
                history::metadata_entry("set_locked", &arg.path, before.clone(), data),
            ) {
                let _ = before.save(&root);
                return Err(error);
            }
            let _ = self.app.emit("vivin://project-changed", ());
            Ok(json!({ "path": arg.path, "locked": arg.value }))
        })())
    }

    #[tool(description = "List AI changes to the current project, newest first.")]
    fn list_history(&self) -> Result<CallToolResult, McpError> {
        result((|| {
            let mut entries = history::load(&root(&self.app)?)?;
            entries.reverse();
            Ok(entries)
        })())
    }

    #[tool(
        description = "Undo one AI change by its history ID. Refuses to overwrite later changes or bypass locks."
    )]
    fn undo_change(
        &self,
        Parameters(arg): Parameters<UndoArg>,
    ) -> Result<CallToolResult, McpError> {
        result((|| {
            let state = self.app.state::<ProjectSession>();
            let _guard = state.mutation_guard()?;
            let root = state.root()?;
            let original = history::load(&root)?
                .into_iter()
                .find(|entry| entry.id == arg.id)
                .ok_or("修改记录不存在")?;
            if original.created_project && state.has_dirty_draft()? {
                return Err("项目有未保存草稿，无法撤销创建".to_string());
            }
            if state.has_draft_under(&original.target)?
                || original
                    .after_path
                    .as_deref()
                    .is_some_and(|path| state.has_draft_under(path).unwrap_or(true))
            {
                return Err("目标包含未保存草稿，请先保存或处理".to_string());
            }
            let entry = history::undo(&root, &arg.id)?;
            if entry.operation == "undo_create_project" {
                state.clear_root()?;
                let _ = self.app.emit("vivin://project-closed", ());
                return Ok(entry);
            }
            let _ = self.app.emit("vivin://project-changed", ());
            if let (Some(from), Some(to)) = (&entry.before_path, &entry.after_path) {
                let _ = self
                    .app
                    .emit("vivin://entry-moved", json!({ "from": from, "to": to }));
            }
            if entry.before_content.is_some() || entry.after_content.is_some() {
                let _ = self.app.emit("vivin://document-changed", &entry.target);
            }
            Ok(entry)
        })())
    }
}

#[tool_handler]
impl ServerHandler for VivinMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions("The active vivin project is selected in the desktop app. Call project_context before writing; follow its AGENTS.md rules and resolve contradictions with the user. Every write requires rulesRevision and follows project lock and version checks.".to_string())
    }
}

async fn auth(
    State(token): State<Arc<String>>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let valid = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        == Some(format!("Bearer {}", token).as_str());
    let host_ok = headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|host| {
            host == format!("127.0.0.1:{PORT}") || host == format!("localhost:{PORT}")
        });
    let origin_ok = headers
        .get("origin")
        .and_then(|value| value.to_str().ok())
        .is_none_or(|origin| {
            origin.starts_with("http://127.0.0.1:") || origin.starts_with("http://localhost:")
        });
    if !valid {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if !host_ok || !origin_ok {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(request).await)
}

pub fn start(app: AppHandle) -> Result<(), String> {
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    let path = directory.join("bridge.json");
    let token = if path.exists() {
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(&path).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?
        .get("token")
        .and_then(|value| value.as_str())
        .ok_or("桥接配置缺少令牌")?
        .to_string()
    } else {
        let token = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());
        fs::write(
            &path,
            serde_json::to_vec_pretty(
                &json!({ "url": format!("http://127.0.0.1:{PORT}/mcp"), "token": token }),
            )
            .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
                .map_err(|error| error.to_string())?;
        }
        token
    };
    tauri::async_runtime::spawn(async move {
        let shared_app = app.clone();
        let service = StreamableHttpService::new(
            move || Ok(VivinMcp::new(shared_app.clone())),
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig::default()
                .with_legacy_session_mode(false)
                .with_json_response(true),
        );
        let router = Router::new()
            .nest_service("/mcp", service)
            .layer(middleware::from_fn_with_state(Arc::new(token), auth));
        match tokio::net::TcpListener::bind(("127.0.0.1", PORT)).await {
            Ok(listener) => {
                if let Err(error) = axum::serve(listener, router).await {
                    eprintln!("vivin MCP server: {error}");
                }
            }
            Err(error) => eprintln!("vivin MCP server cannot bind: {error}"),
        }
    });
    Ok(())
}

#[tauri::command]
pub fn bridge_info(app: AppHandle) -> Result<serde_json::Value, String> {
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    Ok(
        json!({ "url": format!("http://127.0.0.1:{PORT}/mcp"), "configPath": directory.join("bridge.json") }),
    )
}

#[tauri::command]
pub fn list_history(state: tauri::State<'_, ProjectSession>) -> Result<Vec<HistoryEntry>, String> {
    let mut entries = history::load(&state.root()?)?;
    entries.reverse();
    Ok(entries)
}

#[tauri::command]
pub fn undo_change(
    id: String,
    app: AppHandle,
    state: tauri::State<'_, ProjectSession>,
) -> Result<HistoryEntry, String> {
    let _guard = state.mutation_guard()?;
    let root = state.root()?;
    let original = history::load(&root)?
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or("修改记录不存在")?;
    if original.created_project && state.has_dirty_draft()? {
        return Err("项目有未保存草稿，无法撤销创建".to_string());
    }
    if state.has_draft_under(&original.target)?
        || original
            .after_path
            .as_deref()
            .is_some_and(|path| state.has_draft_under(path).unwrap_or(true))
    {
        return Err("目标包含未保存草稿，请先保存或处理".to_string());
    }
    let entry = history::undo(&root, &id)?;
    if entry.operation == "undo_create_project" {
        state.clear_root()?;
        let _ = app.emit("vivin://project-closed", ());
        return Ok(entry);
    }
    let _ = app.emit("vivin://project-changed", ());
    if let (Some(from), Some(to)) = (&entry.before_path, &entry.after_path) {
        let _ = app.emit("vivin://entry-moved", json!({ "from": from, "to": to }));
    }
    if entry.before_content.is_some() || entry.after_content.is_some() {
        let _ = app.emit("vivin://document-changed", &entry.target);
    }
    Ok(entry)
}
