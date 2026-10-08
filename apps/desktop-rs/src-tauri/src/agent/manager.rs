//! Agent 会话管理器：进程生命周期 + 客户端持有。
//!
//! v1 范围：单 agent 实例（与 TS 侧每 workspace 一个 connection scope 的模型
//! 不同，先做全局单实例打通链路，多 workspace 管理在后续阶段对齐）。

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::{mpsc, Mutex};

use crate::agent::client::AgentClient;
use crate::agent::command::resolve_agent_command;
use crate::agent::methods;
use crate::agent::transport::StdioTransport;
use crate::protocol::ProtocolNotification;

/// Agent 运行实例。
pub struct AgentRuntime {
    pub client: Arc<AgentClient>,
    pub pid: Option<u32>,
    pub workspace: PathBuf,
    /// 通知接收端；由前端桥或后续服务消费（take 后不归还）。
    notifications: Mutex<Option<mpsc::UnboundedReceiver<ProtocolNotification>>>,
    /// 连接关闭原因；None 表示仍连接。
    pub closed_reason: Arc<Mutex<Option<String>>>,
}

impl AgentRuntime {
    pub async fn take_notifications(
        &self,
    ) -> Option<mpsc::UnboundedReceiver<ProtocolNotification>> {
        self.notifications.lock().await.take()
    }

    pub async fn is_connected(&self) -> bool {
        self.closed_reason.lock().await.is_none()
    }
}

/// 启动 agent：解析命令 → spawn → 建客户端。
/// 启动后立即发 `runtime/capabilities` 做握手验证，失败则视为启动失败并回收进程。
pub async fn start_agent(workspace: PathBuf) -> Result<Arc<AgentRuntime>, String> {
    let command = resolve_agent_command(&workspace)
        .ok_or_else(|| "未找到 ZCode agent 启动命令（缺 zcode-cli dist 或 tsx 入口）".to_string())?;

    let transport = StdioTransport::spawn(
        &command.program,
        &command.args,
        &command.cwd,
        &command.env,
    )
    .map_err(|e| format!("spawn agent 失败: {e}"))?;

    let pid = transport.pid();
    // AgentClient::start 已返回 Arc 包装，直接使用。
    let (client, notifications_rx, mut closed_rx) = AgentClient::start(transport);

    // 连接关闭监听：记录原因（首个）。
    let closed_reason: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let closed_reason_loop = Arc::clone(&closed_reason);
    tokio::spawn(async move {
        if let Some(reason) = closed_rx.recv().await {
            *closed_reason_loop.lock().await = Some(reason);
        }
    });

    // 握手验证：runtime/capabilities。
    let capabilities = client
        .request(methods::RUNTIME_CAPABILITIES, None, std::time::Duration::from_secs(30))
        .await
        .map_err(|e| {
            format!("capabilities 握手失败（agent pid={pid:?}）: {e}")
        })?;

    let connected = capabilities.get("protocol").is_some() || capabilities.is_object();
    if !connected {
        return Err("capabilities 返回为空".into());
    }

    Ok(Arc::new(AgentRuntime {
        client,
        pid,
        workspace,
        notifications: Mutex::new(Some(notifications_rx)),
        closed_reason,
    }))
}

/// 请求转发：直接调 client.request。
pub async fn forward_request(
    runtime: &AgentRuntime,
    method: &str,
    params: Option<Value>,
    timeout: std::time::Duration,
) -> Result<Value, String> {
    if !runtime.is_connected().await {
        return Err("agent 未连接".into());
    }
    runtime.client.request(method, params, timeout).await
}

/// 进程内稳定 clientId（对齐 zcodeV4HostCommand.ts 的 hostV4ClientId：
/// host 重启 = 新提交端，幂等表以 commandId 为键不受影响）。
fn client_id() -> &'static str {
    static CLIENT_ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CLIENT_ID.get_or_init(|| format!("desktop-rs-{}", uuid::Uuid::new_v4()))
}

/// v4 sendText 主路径（对齐 zcodeAgentService.ts 5173 的提交方式）：
/// JSON-RPC request `v4/command`，params = CommandEnvelope。
/// payload 按 zcode-protocol-v4/command.ts 96 行 sendText schema 构造。
pub async fn send_text(
    runtime: &AgentRuntime,
    session_id: &str,
    text: &str,
) -> Result<Value, String> {
    if !runtime.is_connected().await {
        return Err("agent 未连接".into());
    }
    let envelope = serde_json::json!({
        // uuid v7（RFC 9562）：时间有序，与 renderer/host 工厂同构。
        "commandId": uuid::Uuid::now_v7().to_string(),
        "clientId": client_id(),
        "sessionId": session_id,
        "type": "sendText",
        "payload": {
            "text": text,
            "requestedDelivery": "startNow",
        },
        "issuedAt": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    });
    let ack = runtime
        .client
        .request(methods::V4_COMMAND, Some(envelope), std::time::Duration::from_secs(60))
        .await?;
    // CommandAck（command.ts 524-538）：accepted/duplicate 视为提交成功，
    // rejected/stale/failed 带原因上抛。
    let status = ack["status"].as_str().unwrap_or("unknown").to_string();
    match status.as_str() {
        "accepted" | "duplicate" => Ok(ack),
        other => Err(format!(
            "v4 命令被拒绝（{}）: {}",
            other,
            ack["message"].as_str().unwrap_or("")
        )),
    }
}

/// 应用退出时的进程树清理（同步版）。///
/// 对应 zcodeStdioTransport.disposeAndWait 的最终兜底层：Windows 下 agent wrapper
/// 会拉起 runtime/MCP 子进程，只 kill 父进程会残留后代，必须 taskkill 整棵树。
/// 优雅路径（stdin EOF + 宽限等待）在 GUI 退出钩子里时间不足，直接强杀树——
/// 与 TS 版超时兜底后的行为一致（terminateProcessTreeAndWait 的 force 分支）。
pub fn shutdown_tree_sync(runtime: &AgentRuntime) {
    let Some(pid) = runtime.pid else { return };
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000)
            .status();
    }
    #[cfg(not(windows))]
    {
        // POSIX：kill 进程组（agent spawn 时未 detach，树随组回收）。
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status();
    }
}
