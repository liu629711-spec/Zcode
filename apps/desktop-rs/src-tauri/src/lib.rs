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
            agent_start,
            agent_status,
            agent_request,
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
