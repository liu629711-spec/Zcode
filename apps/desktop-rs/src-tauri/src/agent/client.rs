//! Agent 客户端：请求/响应配对 + 通知分发。
//!
//! 对应 `packages/services/src/zcode-agent/zcodeProtocolClient.ts` 的
//! request 配对模型；通知通过通道供上层（后续的会话管理器）消费。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot, Mutex};

use crate::protocol::{ProtocolMessage, ProtocolRequest, RequestId};

/// 默认请求超时。TS 侧由调用方自行控制；Rust 端先统一兜底。
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

pub struct AgentClient {
    transport: Arc<Mutex<super::transport::StdioTransport>>,
    pending: Arc<Mutex<HashMap<RequestId, oneshot::Sender<serde_json::Value>>>>,
    next_id: AtomicU64,
}

impl AgentClient {
    /// 启动客户端：spawn agent 并接管消息分发循环。
    pub fn start(
        transport: super::transport::StdioTransport,
    ) -> (Arc<Self>, mpsc::UnboundedReceiver<crate::protocol::ProtocolNotification>, mpsc::UnboundedReceiver<String>) {
        let mut transport = transport;
        let inbound = transport.take_inbound_rx();
        let closed_rx = transport.take_closed_rx();
        let transport = Arc::new(Mutex::new(transport));

        let (notifications_tx, notifications_rx) = mpsc::unbounded_channel();
        let (closed_tx, closed_rx_out) = mpsc::unbounded_channel();
        let pending: Arc<Mutex<HashMap<RequestId, oneshot::Sender<serde_json::Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        // 闭包与 Self 各持一份 sender。
        let notifications_tx_loop = notifications_tx.clone();

        // 分发循环：response → pending 配对；notification → 通知通道；
        // request（agent 反向调用）→ 内置 handler 响应。
        // 真机联调发现：agent 在 session/create 时会反向请求
        // session/requestRuntimePreferences（15s 超时），不响应会导致创建会话失败。
        let pending_loop = Arc::clone(&pending);
        let closed_loop = closed_tx.clone();
        let transport_loop = Arc::clone(&transport);
        tokio::spawn(async move {
            let mut closed_rx = closed_rx;
            let mut inbound = inbound;
            loop {
                tokio::select! {
                    msg = inbound.recv() => {
                        match msg {
                            Some(ProtocolMessage::Response(res)) => {
                                if let Some(waiter) = pending_loop.lock().await.remove(&res.id)
                                {
                                    let _ = waiter.send(res.result);
                                }
                            }
                            Some(ProtocolMessage::Error(err)) => {
                                if let Some(waiter) = pending_loop.lock().await.remove(&err.id)
                                {
                                    let _ = waiter.send(serde_json::json!({
                                        "__protocol_error": err.error,
                                    }));
                                }
                            }
                            Some(ProtocolMessage::Notification(note)) => {
                                let _ = notifications_tx_loop.send(note);
                            }
                            Some(ProtocolMessage::Request(req)) => {
                                let response = handle_agent_request(&req);
                                let _ = transport_loop
                                    .lock()
                                    .await
                                    .send(response);
                            }
                            None => break,
                        }
                    }
                    closed = closed_rx.recv() => {
                        if let Some(super::transport::TransportClosed::Reason { reason }) = closed {
                            let _ = closed_loop.send(reason);
                        } else {
                            let _ = closed_loop.send("agent exited".into());
                        }
                        break;
                    }
                }
            }
            // 连接关闭：唤醒所有 pending（返回 protocol_error 占位）。
            let mut map = pending_loop.lock().await;
            for (_, waiter) in map.drain() {
                let _ = waiter.send(serde_json::json!({
                    "__protocol_error": { "code": -32000, "message": "connection closed" },
                }));
            }
        });

        let client = Arc::new(Self {
            transport,
            pending,
            next_id: AtomicU64::new(1),
        });
        (client, notifications_rx, closed_rx_out)
    }

    pub fn pid(&self) -> Option<u32> {
        // pid 仅用于展示；拿不到锁就放弃，避免阻塞。
        match self.transport.try_lock() {
            Ok(t) => t.pid(),
            Err(_) => None,
        }
    }

    /// 发送请求并等待响应。返回 `__protocol_error` 包装表示协议错误/连接关闭。
    pub async fn request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        timeout: Duration,
    ) -> Result<serde_json::Value, String> {
        let id = RequestId::Int(self.next_id.fetch_add(1, Ordering::Relaxed) as i64);
        // 超时清理要用 id，而 id 会被 move 进请求结构，提前克隆一份。
        let id_for_cleanup = id.clone();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id.clone(), tx);

        self.transport
            .lock()
            .await
            .send(ProtocolMessage::Request(ProtocolRequest {
                id,
                method: method.to_string(),
                params,
                trace: None,
            }))
            .map_err(|e| format!("transport send failed: {e}"))?;

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(result)) => {
                if let Some(err) = result.get("__protocol_error") {
                    Err(format!("protocol error: {err}"))
                } else {
                    Ok(result)
                }
            }
            Ok(Err(_)) => Err("request waiter dropped".into()),
            Err(_) => {
                // 超时后必须清理本次请求的 pending 项，防止表泄漏。
                self.pending.lock().await.remove(&id_for_cleanup);
                Err(format!("request timeout after {timeout:?}"))
            }
        }
    }

    pub async fn notify(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<(), String> {
        self.transport
            .lock()
            .await
            .send(ProtocolMessage::Notification(
                crate::protocol::ProtocolNotification {
                    method: method.to_string(),
                    params,
                    trace: None,
                },
            ))
            .map_err(|e| format!("transport send failed: {e}"))
    }

    pub async fn close_stdin(&self) {
        self.transport.lock().await.close_stdin().await;
    }
}

/// agent 反向请求的内置处理（zcodeProtocolMethods 中 CLI→Host 单向问答族）。
/// 返回应发给 agent 的 ProtocolMessage（response 或 error）。
fn handle_agent_request(
    req: &crate::protocol::ProtocolRequest,
) -> crate::protocol::ProtocolMessage {
    use crate::protocol::{ProtocolError, ProtocolErrorBody, ProtocolResponse};

    match req.method.as_str() {
        // session/requestRuntimePreferences（index.ts 1719-1749）：
        // host 按 workspace/服务形态给默认偏好；桌面 v1 全部走保守默认。
        "session/requestRuntimePreferences" => {
            crate::protocol::ProtocolMessage::Response(ProtocolResponse {
                id: req.id.clone(),
                result: serde_json::json!({
                    "nativeSearchEnhancementsEnabled": false,
                    "memoryEnabled": false,
                    "askUserQuestionAutoResolutionEnabled": true,
                    "modelContextBudgetStrategy": "preflight-v1",
                }),
            })
        }
        // automation/checkTaskBinding、agentDispatch/checkTaskRetired 等其余
        // 反向问答当前桌面端无业务面，按 JSON-RPC 约定回 method not found。
        other => crate::protocol::ProtocolMessage::Error(ProtocolError {
            id: req.id.clone(),
            error: ProtocolErrorBody {
                code: -32601,
                message: format!("desktop-rs 未实现 agent 反向方法: {other}"),
                data: None,
            },
        }),
    }
}
