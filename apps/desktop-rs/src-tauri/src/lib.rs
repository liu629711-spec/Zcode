//! ZCode 桌面端主进程。
//! 按 zcode-protocol（packages/shared/src/zcode-protocol）通过 stdio 对接
//! Agent 运行时（apps/zcode-cli），协议只实现不改。

pub mod agent;
pub mod protocol;

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{json, Value};
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
    let workspace = workspace
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs_home());
    let runtime = manager::start_agent(workspace.clone()).await?;
    let pid = runtime.pid;
    guard.replace(Arc::clone(&runtime));

    // 通知桥：把 agent 通知转发到前端事件总线。
    if let Some(mut rx) = runtime.take_notifications().await {
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(note) = rx.recv().await {
                let _ = handle.emit("agent://notification", serde_json::to_value(&note).ok());
            }
        });
    }

    Ok(json!({
        "alreadyRunning": false,
        "pid": pid,
        "workspace": workspace.to_string_lossy(),
    }))
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
#[tauri::command]
fn fs_list(path: String) -> Result<Value, String> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(format!("不是目录: {path}"));
    }
    let mut entries: Vec<Value> = Vec::new();
    let read = std::fs::read_dir(&dir).map_err(|e| format!("读取失败: {e}"))?;
    for entry in read.flatten() {
        let Ok(file_type) = entry.file_type() else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        // 跳过隐藏项与常见噪声目录，避免树被撑爆。
        if name.starts_with('.') {
            continue;
        }
        if file_type.is_dir()
            && matches!(name.as_str(), "node_modules" | "target" | "dist" | "__pycache__")
        {
            continue;
        }
        entries.push(json!({
            "name": name,
            "isDir": file_type.is_dir(),
        }));
    }
    // 目录在前，同类型按名称排序。
    entries.sort_by(|a, b| {
        let dir_key = |v: &Value| !v["isDir"].as_bool().unwrap_or(false);
        dir_key(a)
            .cmp(&dir_key(b))
            .then_with(|| {
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

/// v4 createSession：新会话走 v4 主路径（外键约束要求会话在 v4 表登记）。
#[tauri::command]
async fn agent_create_session_v4(
    state: AgentStateHandle<'_>,
    workspace: Option<String>,
) -> Result<Value, String> {
    let guard = state.runtime.lock().await;
    let rt = guard.as_ref().ok_or("agent 未启动")?;
    let workspace = workspace
        .map(PathBuf::from)
        .unwrap_or_else(dirs_home);
    let session_id =
        manager::create_session_v4(rt, &workspace.to_string_lossy()).await?;
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
            agent_start,
            agent_status,
            agent_request,
            agent_send_text,
            agent_create_session_v4,
            agent_rows_range,
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
