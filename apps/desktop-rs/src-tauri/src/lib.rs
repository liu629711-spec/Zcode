//! ZCode 桌面端主进程。
//! 按 zcode-protocol（packages/shared/src/zcode-protocol）通过 stdio 对接
//! Agent 运行时（apps/zcode-cli），协议只实现不改。

pub mod agent;
pub mod conversation_stream;
pub mod protocol;
pub mod taskdb;
pub mod taskgroup;
pub mod wire;

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};
use tauri::{Emitter, Manager, State};

use agent::manager::{self, AgentRuntime};

/// 全局 agent 实例状态。v1 单实例；多 workspace 管理后续对齐。
pub struct AgentState {
    runtime: tokio::sync::Mutex<Option<Arc<AgentRuntime>>>,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            runtime: tokio::sync::Mutex::new(None),
        }
    }
}

type AgentStateHandle<'a> = State<'a, AgentState>;

/// 启动 agent 并完成 capabilities 握手。
#[tauri::command]
async fn agent_start(
    state: AgentStateHandle<'_>,
    app: tauri::AppHandle,
    workspace: Option<String>,
) -> Result<Value, String> {
    let mut guard = state.runtime.lock().await;
    if let Some(rt) = guard.as_ref() {
        if rt.is_connected().await {
            return Ok(json!({
                "alreadyRunning": true,
                "pid": rt.pid,
                "workspace": rt.workspace.to_string_lossy(),
            }));
        }
    }

    // workspace 缺省用用户主目录；真实 workspace 由 UI 侧传入。
    let workspace = workspace.map(PathBuf::from).unwrap_or_else(|| dirs_home());
    let runtime = manager::start_agent(workspace.clone()).await?;
    let pid = runtime.pid;
    guard.replace(Arc::clone(&runtime));

    // 通知桥：把 agent 通知转发到前端事件总线。
    //
    // v4/conversation/frame **不直通**：先过 WireAssembler——
    // - complete 帧原样放行（顺手记墓碑，防迟到分片复活旧帧）；
    // - fragment 进 staging，收齐校验（CRC32/长度/UTF-8/JSON/信封一致）后
    //   才作为完整逻辑帧放行；前端永远见不到分片，也就不需要重组代码；
    // - fault（fail-closed）触发 same-sub resync，recovery 投递是唯一
    //   能解故障门的帧（真源恢复阶梯 wire-fault.ts）。
    // 其余通知（session/event 等）原样转发。
    if let Some(mut rx) = runtime.take_notifications().await {
        let handle = app.clone();
        let runtime_for_resync = Arc::clone(&runtime);
        tauri::async_runtime::spawn(async move {
            let mut assembler = wire::WireAssembler::new();
            while let Some(note) = rx.recv().await {
                let value = serde_json::to_value(&note).unwrap_or(Value::Null);
                if value["method"] == conversation_stream::NOTIFICATION_CONVERSATION_FRAME {
                    forward_conversation_frame(
                        &value,
                        &mut assembler,
                        &handle,
                        &runtime_for_resync,
                    )
                    .await;
                    continue;
                }
                let _ = handle.emit("agent://notification", value);
            }
        });
    }

    Ok(json!({
        "alreadyRunning": false,
        "pid": pid,
        "workspace": workspace.to_string_lossy(),
    }))
}

/// 处理一帧 `v4/conversation/frame` 通知：过重组器，只把完整逻辑帧发给前端。
///
/// 前端 `stream.rs` 已按「外层信封 + frame 字段」的 complete 形态解析，
/// 所以重组完成后**重塑回同一形态**再 emit——前端零改动就能吃到重组产物。
/// 分片 staging 期间不 emit 任何东西（真源 consumer 同样只见逻辑帧）。
async fn forward_conversation_frame(
    note: &Value,
    assembler: &mut wire::WireAssembler,
    handle: &tauri::AppHandle,
    runtime: &agent::manager::AgentRuntime,
) {
    let Some(params) = note.get("params") else {
        return;
    };
    let Ok(wire_frame) = serde_json::from_value::<conversation_stream::WireFrame>(params.clone())
    else {
        // 解析不了的帧对前端也是噪声，静默丢（宽容协议，不打断对话）。
        return;
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    for event in assembler.accept(&wire_frame, now) {
        match event {
            wire::AssemblyEvent::Complete(frame) => {
                // 重组/整帧完成：重塑成前端已解析的 complete 形态。
                // 注意逻辑帧序列化出来是 TopicFrame 的 camelCase JSON，
                // 与 agent 直发的 frame 字段同构（同一份 serde 结构）。
                let out = json!({
                    "method": conversation_stream::NOTIFICATION_CONVERSATION_FRAME,
                    "params": {
                        "wireVersion": wire_frame.wire_version,
                        "kind": "complete",
                        "logicalFrameId": wire_frame.logical_frame_id,
                        "logicalFrameOrdinal": wire_frame.logical_frame_ordinal,
                        "subscriptionId": wire_frame.subscription_id,
                        "topic": wire_frame.topic,
                        "deliveryKind": wire_frame.delivery_kind,
                        "frame": frame,
                    }
                });
                let _ = handle.emit("agent://notification", out);
            }
            wire::AssemblyEvent::Fault { topic, .. } => {
                // fail-closed：该路由后续帧会被 assembler 全部吞掉，
                // 必须 resync 拿 recovery 投递解门（真源恢复阶梯第一级）。
                // spawn 不阻塞后续帧转发；resync 失败则只能靠下一轮 recovery。
                if let Some(session_id) = topic.strip_prefix("conversation/") {
                    let client = Arc::clone(&runtime.client);
                    let conn = resolve_connection_id(None);
                    let topic_owned =
                        conversation_stream::SubscribeParams::conversation_topic(session_id);
                    tauri::async_runtime::spawn(async move {
                        let req = json!({
                            "connectionId": conn,
                            "topic": topic_owned,
                        });
                        let _ = client
                            .request(
                                conversation_stream::CONVERSATION_RESYNC,
                                Some(req),
                                agent::client::DEFAULT_REQUEST_TIMEOUT,
                            )
                            .await;
                    });
                }
            }
        }
    }
    // Fragment 分片刚到时 accept 不产事件——staging 中，什么都不发。
}

/// 查询 agent 状态。
#[tauri::command]
async fn agent_status(state: AgentStateHandle<'_>) -> Result<Value, String> {
    let guard = state.runtime.lock().await;
    match guard.as_ref() {
        Some(rt) => {
            let connected = rt.is_connected().await;
            let closed = rt.closed_reason.lock().await.clone();
            Ok(json!({
                "running": true,
                "connected": connected,
                "pid": rt.pid,
                "workspace": rt.workspace.to_string_lossy(),
                "closedReason": closed,
            }))
        }
        None => Ok(json!({ "running": false })),
    }
}

/// 通用协议请求转发（调试/联调用；业务面后续按方法族收口）。
#[tauri::command]
async fn agent_request(
    state: AgentStateHandle<'_>,
    method: String,
    params: Option<Value>,
) -> Result<Value, String> {
    let guard = state.runtime.lock().await;
    let rt = guard.as_ref().ok_or("agent 未启动")?;
    manager::forward_request(rt, &method, params, agent::client::DEFAULT_REQUEST_TIMEOUT).await
}

/// 用户主目录（会话创建的默认 workspace）。
#[tauri::command]
fn get_home() -> Value {
    json!({ "home": dirs_home().to_string_lossy() })
}

/// 列出目录内容（文件树数据源；直接读盘，不经 agent 协议）。
///
/// 对齐真源 `fileService.readdir({ includeHidden })`（useWorkspaceFileTreeData.ts:222-225）：
/// 返回**完整 path**，树形展开靠 path 前缀匹配定位父目录，因此不能只给文件名。
/// 噪声目录（node_modules/target 等）仍在此处过滤——真源靠 gitignore 索引过滤，
/// Rust 侧暂未接 gitService，用固定黑名单等价。
#[tauri::command]
fn fs_list(path: String, include_hidden: Option<bool>) -> Result<Value, String> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(format!("不是目录: {path}"));
    }
    let include_hidden = include_hidden.unwrap_or(true);
    let mut entries: Vec<Value> = Vec::new();
    let read = std::fs::read_dir(&dir).map_err(|e| format!("读取失败: {e}"))?;
    for entry in read.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        // 跳过隐藏项（点开头），除非显式要求包含。
        if !include_hidden && name.starts_with('.') {
            continue;
        }
        if file_type.is_dir()
            && matches!(
                name.as_str(),
                "node_modules" | "target" | "dist" | "__pycache__"
            )
        {
            continue;
        }
        entries.push(json!({
            "name": name,
            "path": entry.path().to_string_lossy(),
            "isDir": file_type.is_dir(),
            "isSymbolicLink": file_type.is_symlink(),
        }));
    }
    // 目录在前，同类型按名称排序。
    entries.sort_by(|a, b| {
        let dir_key = |v: &Value| !v["isDir"].as_bool().unwrap_or(false);
        dir_key(a).cmp(&dir_key(b)).then_with(|| {
            a["name"]
                .as_str()
                .unwrap_or("")
                .to_lowercase()
                .cmp(&b["name"].as_str().unwrap_or("").to_lowercase())
        })
    });
    Ok(json!({ "entries": entries }))
}

/// 读取文本文件内容（文件树点击预览；超长截断）。
#[tauri::command]
fn fs_read(path: String) -> Result<Value, String> {
    let file = PathBuf::from(&path);
    if !file.is_file() {
        return Err(format!("不是文件: {path}"));
    }
    // 二进制粗筛：读前 1KB 查 NUL。
    let bytes = std::fs::read(&file).map_err(|e| format!("读取失败: {e}"))?;
    if bytes.iter().take(1024).any(|&b| b == 0) {
        return Err("二进制文件暂不支持预览".into());
    }
    const MAX_LEN: usize = 512 * 1024;
    let truncated = bytes.len() > MAX_LEN;
    let content = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_LEN)]).into_owned();
    Ok(json!({
        "content": content,
        "truncated": truncated,
        "size": bytes.len(),
    }))
}

// ---------------------------------------------------------------------------
// git 状态（文件树 git 徽标数据源；shell 调 git CLI，对齐真源 IGitService.refresh 的载荷形态）
// ---------------------------------------------------------------------------

/// porcelain v1 字母 → GitChangeKind（闭集：modified/added/deleted/renamed）。
fn git_kind_of(status_char: char) -> &'static str {
    match status_char {
        'A' => "added",
        'D' => "deleted",
        'R' | 'C' => "renamed",
        // M / T（类型变更）/ 其他字母一律按 modified 兜底。
        _ => "modified",
    }
}

/// 相对路径（workspace 相对或仓库相对；前缀不匹配时原样返回）。
fn git_relative(base: &str, path: &str) -> String {
    let norm = |p: &str| p.replace('\\', "/").trim_end_matches('/').to_string();
    let base = norm(base);
    let path = norm(path);
    let prefix = format!("{base}/");
    if path.starts_with(&prefix) {
        path[prefix.len()..].to_string()
    } else {
        path
    }
}

/// `git_refresh(workspacePath)` → `GitRefreshResult` 形态
/// （真源 packages/shared/src/git.ts:250-256；文件树只读 summary + 两组 changes）。
///
/// 不引 git 库：三次 CLI 调用（--version / rev-parse / status --porcelain），
/// 与真源 isGitAvailable 的「git 可执行」语义一致；非 git 目录返回 isRepository:false。
#[tauri::command]
fn git_refresh(workspace_path: String) -> Result<Value, String> {
    let git_available = std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let unavailable = || {
        json!({
            "summary": {
                "workspacePath": workspace_path, "repoRoot": null, "workspaceInRepoPath": "",
                "autoRefreshWatchPaths": [], "branchName": null, "trackingBranchName": null,
                "headRefType": "branch", "ahead": 0, "behind": 0, "isDirty": false,
                "isGitAvailable": false, "isRepository": false,
            },
            "identity": null, "unstagedChanges": [], "stagedChanges": [], "branchComparison": null,
        })
    };
    if !git_available {
        return Ok(unavailable());
    }
    let ws = PathBuf::from(&workspace_path);
    let repo_root = std::process::Command::new("git")
        .arg("-C")
        .arg(&ws)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().trim_end_matches(['/', '\\']).to_string());
    let Some(repo_root) = repo_root else {
        return Ok(unavailable());
    };

    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(&ws)
        .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
        .output()
        .map_err(|e| format!("git status 执行失败: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git status 失败: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    // -z 格式：条目以 NUL 分隔；R/C 重命名的条目带第二个 NUL 段（原路径）。
    // 条目头两个字母 X=staged / Y=worktree，第三个字符是空格。
    let raw = String::from_utf8_lossy(&out.stdout);
    let mut segments = raw.split('\0').filter(|s| !s.is_empty()).peekable();
    let mut staged: Vec<Value> = Vec::new();
    let mut unstaged: Vec<Value> = Vec::new();
    while let Some(entry) = segments.next() {
        if entry.chars().count() < 4 {
            continue;
        }
        let mut chars = entry.chars();
        let x = chars.next().unwrap_or(' ');
        let y = chars.next().unwrap_or(' ');
        let path_part = &entry[3..];
        // R/C 条目的下一段是重命名原路径，消费掉。
        let is_rename = x == 'R' || x == 'C' || y == 'R' || y == 'C';
        if is_rename {
            segments.next();
        }
        let is_untracked = x == '?' && y == '?';
        let is_conflicted = x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D');
        let mut change = |kind: &str, section: &str, is_staged: bool| -> Value {
            json!({
                "path": path_part,
                "repoRelativePath": git_relative(&repo_root, path_part),
                "workspaceRelativePath": git_relative(&workspace_path, path_part),
                "x": x.to_string(), "y": y.to_string(),
                "kind": kind, "section": section,
                "added": 0, "removed": 0,
                "isStaged": is_staged,
                "isUntracked": is_untracked,
                "isConflicted": is_conflicted,
            })
        };
        // 未跟踪：一组 "?? path"，挂 unstaged（真源 buildWorkspaceFileGitStatusByPath
        // 只消费 isUntracked/section 的判定，载荷落哪组不影响状态词）。
        if is_untracked {
            unstaged.push(change("added", "untracked", false));
            continue;
        }
        if x != ' ' && x != '?' {
            staged.push(change(git_kind_of(x), "staged", true));
        }
        if y != ' ' && y != '?' {
            unstaged.push(change(git_kind_of(y), if is_conflicted { "conflicted" } else { "unstaged" }, false));
        }
    }

    let branch_name = std::process::Command::new("git")
        .arg("-C")
        .arg(&ws)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let is_dirty = !staged.is_empty() || !unstaged.is_empty();
    Ok(json!({
        "summary": {
            "workspacePath": workspace_path,
            "repoRoot": repo_root,
            "workspaceInRepoPath": git_relative(&repo_root, &workspace_path),
            "autoRefreshWatchPaths": [],
            "branchName": branch_name,
            "trackingBranchName": null,
            "headRefType": if branch_name.as_deref() == Some("HEAD") { "detached" } else { "branch" },
            "ahead": 0, "behind": 0,
            "isDirty": is_dirty,
            "isGitAvailable": true,
            "isRepository": true,
        },
        "identity": null,
        "unstagedChanges": unstaged,
        "stagedChanges": staged,
        "branchComparison": null,
    }))
}

// ---------------------------------------------------------------------------
// 文件监听（文件树 watcher 数据源；notify crate，非递归按目录）
// ---------------------------------------------------------------------------

/// 活跃 watcher 注册表：id → (watcher, 事件线程的停机信号)。
static FS_WATCHERS: std::sync::Mutex<Option<std::collections::HashMap<String, FsWatcherEntry>>> =
    std::sync::Mutex::new(None);

struct FsWatcherEntry {
    _watcher: notify::RecommendedWatcher,
    /// drop 即让事件转发线程退出（mpsc 断流）。
    _shutdown: std::sync::mpsc::Sender<()>,
}

/// `fs_watch(path)` → `{ id }`。目录非递归监听；变更去抖 250ms 后以
/// `fs-watcher-change` 事件广播 `{ id, dirPath }`（真源 IFileWatcherService.onDynamicChange）。
#[tauri::command]
fn fs_watch(path: String, window: tauri::Window) -> Result<Value, String> {
    use notify::Watcher as _;
    if !PathBuf::from(&path).is_dir() {
        return Err(format!("不是目录: {path}"));
    }
    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(tx)
        .map_err(|e| format!("watcher 初始化失败: {e}"))?;
    watcher
        .watch(PathBuf::from(&path).as_path(), notify::RecursiveMode::NonRecursive)
        .map_err(|e| format!("监听失败: {e}"))?;

    let id = uuid::Uuid::new_v4().to_string();
    let event_id = id.clone();
    let dir_path = path.clone();
    std::thread::spawn(move || {
        // 去抖：收到首个事件后静默 250ms，期间合并同类事件再广播一次。
        loop {
            match rx.recv() {
                Ok(_event) => {
                    // 吞掉窗口期内堆积的事件。
                    while rx.recv_timeout(std::time::Duration::from_millis(250)).is_ok() {}
                    let payload = json!({ "id": event_id, "dirPath": dir_path });
                    if window.emit("fs-watcher-change", payload).is_err() {
                        return;
                    }
                }
                Err(_) => return,
            }
            if shutdown_rx.try_recv().is_ok() {
                return;
            }
        }
    });

    FS_WATCHERS
        .lock()
        .map_err(|_| "watcher 注册表损坏")?
        .get_or_insert_with(std::collections::HashMap::new)
        .insert(id.clone(), FsWatcherEntry { _watcher: watcher, _shutdown: shutdown_tx });
    Ok(json!({ "id": id }))
}

/// `fs_unwatch(id)`：停止监听并释放注册。
#[tauri::command]
fn fs_unwatch(id: String) -> Result<Value, String> {
    if let Ok(mut guard) = FS_WATCHERS.lock() {
        if let Some(map) = guard.as_mut() {
            map.remove(&id);
        }
    }
    Ok(json!({ "ok": true }))
}

/// v4 createSession：新会话走 v4 主路径（外键约束要求会话在 v4 表登记）。
#[tauri::command]
async fn agent_create_session_v4(
    state: AgentStateHandle<'_>,
    workspace: Option<String>,
) -> Result<Value, String> {
    let guard = state.runtime.lock().await;
    let rt = guard.as_ref().ok_or("agent 未启动")?;
    let workspace = workspace.map(PathBuf::from).unwrap_or_else(dirs_home);
    let session_id = manager::create_session_v4(rt, &workspace.to_string_lossy()).await?;
    Ok(json!({ "sessionId": session_id }))
}

/// v4 sendText 主路径：构造 CommandEnvelope 经 `v4/command` 提交。
/// model_selection 可选（会话有默认模型时省略）。
#[tauri::command]
async fn agent_send_text(
    state: AgentStateHandle<'_>,
    session_id: String,
    text: String,
    model_selection: Option<Value>,
) -> Result<Value, String> {
    let guard = state.runtime.lock().await;
    let rt = guard.as_ref().ok_or("agent 未启动")?;
    let selection = model_selection.and_then(|m| {
        let provider = m["providerId"].as_str()?;
        let model = m["modelId"].as_str()?;
        Some((provider.to_string(), model.to_string()))
    });
    manager::send_text(
        rt,
        &session_id,
        &text,
        selection.as_ref().map(|(p, m)| (p.as_str(), m.as_str())),
    )
    .await
}

/// v4 会话行分页拉取（ConversationRowView 数据源；只读、超时重发安全）。
#[tauri::command]
async fn agent_rows_range(
    state: AgentStateHandle<'_>,
    session_id: String,
    limit: Option<i64>,
    before_row_id: Option<i64>,
) -> Result<Value, String> {
    let guard = state.runtime.lock().await;
    let rt = guard.as_ref().ok_or("agent 未启动")?;
    let mut params = serde_json::json!({
        "sessionId": session_id,
        "clientMode": "desktop-continuous",
        "limit": limit.unwrap_or(200),
    });
    if let Some(id) = before_row_id {
        params["beforeRowId"] = json!(id);
    }
    rt.client
        .request(
            agent::methods::V4_CONVERSATION_ROWS_RANGE,
            Some(params),
            agent::client::DEFAULT_REQUEST_TIMEOUT,
        )
        .await
}

// ---------------------------------------------------------------------------
// 任务索引库 commands（置顶/归档分区）
//
// 数据源是本地 SQLite（~/.zcode/v2/tasks-index.sqlite），不走 agent 协议——
// 真源的 pinned/archived 同样只存在 node 侧的库里，agent 无感知。
// ---------------------------------------------------------------------------

/// 按分区查询任务（kind = pinned | archived | active）。
#[tauri::command]
async fn task_list(
    workspace_path: String,
    workspace_identity: Option<String>,
    kind: String,
) -> Result<Value, String> {
    let kind = taskdb::TaskKind::parse(&kind).ok_or(format!("未知任务分区: {kind}"))?;
    let Some(conn) = taskdb::open_readonly()? else {
        // 库还没建（全新安装）：返回空列表，上层按"无置顶"处理。
        return Ok(json!({ "tasks": [] }));
    };
    let tasks = taskdb::list_tasks(&conn, &workspace_path, workspace_identity.as_deref(), kind)?;
    Ok(json!({ "tasks": tasks }))
}

/// 设置置顶状态。
///
/// 真源做了乐观更新 + 失败回滚（WorkspacePinnedTasksSection.tsx:271-337），
/// 回滚逻辑在渲染层；主进程只保证"要么写入成功、要么报错"。
#[tauri::command]
async fn task_set_pinned(
    workspace_path: String,
    workspace_identity: Option<String>,
    task_id: String,
    pinned: bool,
) -> Result<Value, String> {
    let conn = taskdb::open_readwrite()?;
    let changed = taskdb::set_pinned(
        &conn,
        &workspace_path,
        workspace_identity.as_deref(),
        &task_id,
        pinned,
    )?;
    if !changed {
        return Err(format!("任务不存在或不属于该工作区: {task_id}"));
    }
    Ok(json!({ "ok": true }))
}

/// 取消归档（真源 zcodeTaskService.unarchiveTask）。
///
/// 对应 `WorkspaceArchivedTasksFlatSection.tsx:231-238` 行内"取消归档"按钮。
#[tauri::command]
async fn task_unarchive(
    workspace_path: String,
    workspace_identity: Option<String>,
    task_id: String,
) -> Result<Value, String> {
    let conn = taskdb::open_readwrite()?;
    let changed = taskdb::set_archived(
        &conn,
        &workspace_path,
        workspace_identity.as_deref(),
        &task_id,
        false,
    )?;
    if !changed {
        return Err(format!("任务不存在或不属于该工作区: {task_id}"));
    }
    Ok(json!({ "ok": true }))
}

/// 读某工作区的分组视图（真源 `queryGroupedTaskViewStructure`）。
#[tauri::command]
async fn task_grouped_view(
    workspace_path: String,
    workspace_identity: Option<String>,
) -> Result<Value, String> {
    let Some(conn) = taskdb::open_readonly()? else {
        return Ok(json!({ "nodes": [] }));
    };
    let view = taskgroup::read_grouped_view(&conn, &workspace_path, workspace_identity.as_deref())?;
    Ok(json!(view))
}

/// 创建分组。
#[tauri::command]
async fn task_group_create(
    group_id: String,
    title: String,
    color: String,
) -> Result<Value, String> {
    let conn = taskdb::open_readwrite()?;
    let now = now_ms();
    let group = taskgroup::create_group(&conn, &group_id, &title, &color, now)?;
    Ok(json!(group))
}

/// 更新分组颜色。
#[tauri::command]
async fn task_group_color(group_id: String, color: String) -> Result<Value, String> {
    let conn = taskdb::open_readwrite()?;
    let changed = taskgroup::change_group_color(&conn, &group_id, &color, now_ms())?;
    if !changed {
        return Err(format!("分组不存在: {group_id}"));
    }
    Ok(json!({ "ok": true }))
}

/// 把任务移入分组。
#[tauri::command]
async fn task_group_add(
    workspace_path: String,
    workspace_identity: Option<String>,
    group_id: String,
    task_id: String,
) -> Result<Value, String> {
    let conn = taskdb::open_readwrite()?;
    taskgroup::add_task_to_group(
        &conn,
        &workspace_path,
        workspace_identity.as_deref(),
        &group_id,
        &task_id,
        now_ms(),
    )?;
    Ok(json!({ "ok": true }))
}

/// 移出分组。
#[tauri::command]
async fn task_group_remove(
    workspace_path: String,
    workspace_identity: Option<String>,
    task_id: String,
) -> Result<Value, String> {
    let conn = taskdb::open_readwrite()?;
    let changed = taskgroup::remove_task_from_group(
        &conn,
        &workspace_path,
        workspace_identity.as_deref(),
        &task_id,
    )?;
    if !changed {
        return Err(format!("任务不在任何分组中: {task_id}"));
    }
    Ok(json!({ "ok": true }))
}

/// 删除分组（连带清成员）。
#[tauri::command]
async fn task_group_delete(group_id: String) -> Result<Value, String> {
    let conn = taskdb::open_readwrite()?;
    let changed = taskgroup::delete_group(&conn, &group_id)?;
    if !changed {
        return Err(format!("分组不存在: {group_id}"));
    }
    Ok(json!({ "ok": true }))
}

/// 落库一次拖拽后的完整顺序（真源 `applyGroupedTaskViewOrder`）。
///
/// 真源 `viewToOrderInput`（useGroupedTaskView.ts:539-554）提交两份数据：
/// 顶层节点顺序 + 各组内部成员顺序。两者都写，且拖拽跨组时还要改成员关系。
///
/// 入参用前端 `groupedTasks::dnd` / `view` 的结构直接表达，不做隐式转换——
/// 转换规则留在前端一处，避免前后端各写一份导致口径漂移。
#[tauri::command]
async fn task_group_apply_order(
    workspace_path: String,
    workspace_identity: Option<String>,
    top_level: Vec<taskgroup::TopLevelNodeInput>,
    groups: Vec<taskgroup::GroupOrderInput>,
) -> Result<Value, String> {
    let mut conn = taskdb::open_readwrite()?;
    let nodes: Vec<taskgroup::GroupedNode> = top_level
        .into_iter()
        .map(taskgroup::TopLevelNodeInput::into_node)
        .collect();
    let group_orders: Vec<(String, Vec<String>)> = groups
        .into_iter()
        .map(|g| (g.group_id, g.task_ids))
        .collect();
    taskgroup::apply_grouped_order(
        &mut conn,
        &workspace_path,
        workspace_identity.as_deref(),
        &nodes,
        &group_orders,
        now_ms(),
    )?;
    Ok(json!({ "ok": true }))
}

/// 进程内稳定的 connectionId（stdio 单管道下由 host 分配，用于重订阅替换判定）。
///
/// 显式传入优先（前端可自管），否则取进程级单例。
fn resolve_connection_id(explicit: Option<String>) -> String {
    use std::sync::OnceLock;
    static CONNECTION_ID: OnceLock<String> = OnceLock::new();
    explicit.unwrap_or_else(|| {
        CONNECTION_ID
            .get_or_init(|| format!("desktop-rs-{}", uuid::Uuid::now_v7()))
            .clone()
    })
}

/// 订阅会话 topic 流（逐 token 流式的入口）。
///
/// 真源流程：`v4/conversation/subscribe` 建立订阅 → agent 推
/// `v4/conversation/frame` 通知 → 前端订阅通知后按 frame 增量更新。
#[tauri::command]
async fn conversation_subscribe(
    state: AgentStateHandle<'_>,
    session_id: String,
    workspace: Option<String>,
    connection_id: Option<String>,
) -> Result<Value, String> {
    let conn = resolve_connection_id(connection_id);
    let guard = state.runtime.lock().await;
    let rt = guard.as_ref().ok_or("agent 未启动")?;
    let params =
        conversation_stream::SubscribeParams::desktop_conversation(conn, &session_id, workspace);
    rt.client
        .request(
            conversation_stream::CONVERSATION_SUBSCRIBE,
            Some(serde_json::to_value(&params).map_err(|e| e.to_string())?),
            agent::client::DEFAULT_REQUEST_TIMEOUT,
        )
        .await
}

/// 取消订阅。
#[tauri::command]
async fn conversation_unsubscribe(
    state: AgentStateHandle<'_>,
    session_id: String,
    connection_id: Option<String>,
) -> Result<Value, String> {
    let conn = resolve_connection_id(connection_id);
    let guard = state.runtime.lock().await;
    let rt = guard.as_ref().ok_or("agent 未启动")?;
    let params = serde_json::json!({
        "connectionId": conn,
        "topic": conversation_stream::SubscribeParams::conversation_topic(&session_id),
    });
    rt.client
        .request(
            conversation_stream::CONVERSATION_UNSUBSCRIBE,
            Some(params),
            agent::client::DEFAULT_REQUEST_TIMEOUT,
        )
        .await
}

/// 请求 resync（流断线后重新对齐；真源 conversationResync）。
#[tauri::command]
async fn conversation_resync(
    state: AgentStateHandle<'_>,
    session_id: String,
    connection_id: Option<String>,
) -> Result<Value, String> {
    let conn = resolve_connection_id(connection_id);
    let guard = state.runtime.lock().await;
    let rt = guard.as_ref().ok_or("agent 未启动")?;
    let params = serde_json::json!({
        "connectionId": conn,
        "topic": conversation_stream::SubscribeParams::conversation_topic(&session_id),
    });
    rt.client
        .request(
            conversation_stream::CONVERSATION_RESYNC,
            Some(params),
            agent::client::DEFAULT_REQUEST_TIMEOUT,
        )
        .await
}

/// 当前 Unix 毫秒（真源多处用 Date.now() 落时间戳）。
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 软删除任务（真源 zcodeTaskService.deleteTask）。
///
/// 只写 `deleted = 1` 不物理删行——真源的 tombstone join 要读这类行。
#[tauri::command]
async fn task_delete(
    workspace_path: String,
    workspace_identity: Option<String>,
    task_id: String,
) -> Result<Value, String> {
    let conn = taskdb::open_readwrite()?;
    let changed = taskdb::delete_task(
        &conn,
        &workspace_path,
        workspace_identity.as_deref(),
        &task_id,
    )?;
    if !changed {
        return Err(format!("任务不存在或不属于该工作区: {task_id}"));
    }
    Ok(json!({ "ok": true }))
}

fn dirs_home() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(AgentState::default())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_home,
            fs_list,
            fs_read,
            git_refresh,
            fs_watch,
            fs_unwatch,
            agent_start,
            agent_status,
            agent_request,
            agent_send_text,
            agent_create_session_v4,
            agent_rows_range,
            task_list,
            task_set_pinned,
            task_unarchive,
            task_delete,
            task_grouped_view,
            task_group_create,
            task_group_add,
            task_group_remove,
            task_group_delete,
            task_group_color,
            task_group_apply_order,
            conversation_subscribe,
            conversation_unsubscribe,
            conversation_resync,
        ])
        .build(tauri::generate_context!())
        .expect("ZCode 桌面端启动失败");

    app.run(|app, event| {
        // 退出时清理 agent 进程树（Windows taskkill /T /F），防止 runtime/MCP 子进程残留。
        if let tauri::RunEvent::Exit = event {
            let state = app.state::<AgentState>();
            if let Ok(guard) = state.runtime.try_lock() {
                if let Some(rt) = guard.as_ref() {
                    agent::manager::shutdown_tree_sync(rt);
                }
            }
        }
    });
}

/// 前端自检用：返回应用基本信息，验证 IPC 链路。
#[tauri::command]
fn get_app_info() -> Value {
    json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    })
}
