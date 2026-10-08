//! Agent stdio 传输层。
//!
//! 对应 `packages/services/src/zcode-agent/zcodeStdioTransport.ts`。
//! 帧格式：NDJSON，帧边界只认 LF（TS 侧注释明确：Node readline 会把
//! U+2028/U+2029 当换行切断合法 JSON；Rust 的 read_until(b'\n') 天然正确）。
//! 行尾 `\r` 去掉、空行跳过；解析失败视为协议错误并关闭连接。

use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use crate::protocol::ProtocolMessage;

/// 传输层关闭原因（对应 TS 的 ZCodeProtocolTransportClosedEvent）。
#[derive(Debug, Clone)]
pub enum TransportClosed {
    Exited { code: Option<i32> },
    Reason { reason: String },
}

pub struct StdioTransport {
    child: Child,
    /// 主进程出站消息通道（request/notification 共用）。
    outbound_tx: mpsc::UnboundedSender<ProtocolMessage>,
    /// 从 agent 收到的入站消息。
    inbound_rx: mpsc::UnboundedReceiver<ProtocolMessage>,
    closed_rx: mpsc::Receiver<TransportClosed>,
    /// 通知写循环关闭 stdin（agent 收到 EOF 后自行收尾）。
    close_stdin_tx: mpsc::Sender<()>,
}

impl StdioTransport {
    /// 拉起 agent 子进程并启动读写循环。
    pub fn spawn(
        program: &str,
        args: &[String],
        cwd: &Path,
        env: &[(String, String)],
    ) -> std::io::Result<Self> {
        let mut cmd = Command::new(program);
        cmd.args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        // CREATE_NO_WINDOW：桌面 GUI 进程 spawn 控制台子进程时避免闪烁 console 窗口。
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000);
        for (k, v) in env {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().expect("stdin piped");
        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");
        let pid = child.id();

        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (closed_tx, closed_rx) = mpsc::channel(1);
        let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<ProtocolMessage>();
        let (close_stdin_tx, mut close_stdin_rx) = mpsc::channel::<()>(1);

        // 读循环：LF 定界（tokio lines() 按 \n 切）→ 去 CR → 跳空行 → 解析 → 入站通道。
        // 解析失败即关闭（照抄 TS 行为：protocol_parse_error 触发 fireClose）。
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let line = line.strip_suffix('\r').unwrap_or(&line);
                        if line.trim().is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<ProtocolMessage>(line) {
                            Ok(msg) => {
                                if inbound_tx.send(msg).is_err() {
                                    break;
                                }
                            }
                            Err(err) => {
                                let _ = closed_tx
                                    .send(TransportClosed::Reason {
                                        reason: format!("protocol_parse_error: {err}"),
                                    })
                                    .await;
                                break;
                            }
                        }
                    }
                    Ok(None) => {
                        let _ = closed_tx.send(TransportClosed::Reason {
                            reason: "stdout_closed".into(),
                        });
                        break;
                    }
                    Err(err) => {
                        let _ = closed_tx.send(TransportClosed::Reason {
                            reason: format!("stdout_error: {err}"),
                        });
                        break;
                    }
                }
            }
        });

        // stderr 收集线程：第一版仅转发到日志，避免管道写满阻塞 agent。
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let line = line.strip_suffix('\r').unwrap_or(&line);
                if !line.trim().is_empty() {
                    tracing_if_debug(line, pid);
                }
            }
        });

        // 写循环：出站通道 → stdin（JSON + \n）；收到 close 信号后 shutdown stdin（EOF）。
        // stdin 由本任务独占持有，StdioTransport 经 close_stdin_tx 触发关闭。
        tokio::spawn(async move {
            let mut stdin = stdin;
            loop {
                tokio::select! {
                    msg = outbound_rx.recv() => {
                        match msg {
                            Some(msg) => {
                                let mut frame =
                                    serde_json::to_string(&msg).expect("serialize protocol message");
                                frame.push('\n');
                                if stdin.write_all(frame.as_bytes()).await.is_err() {
                                    break;
                                }
                                if stdin.flush().await.is_err() {
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                    _ = close_stdin_rx.recv() => {
                        let _ = stdin.shutdown().await;
                        break;
                    }
                }
            }
        });

        Ok(Self {
            child,
            outbound_tx,
            inbound_rx,
            closed_rx,
            close_stdin_tx,
        })
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    pub fn send(&self, message: ProtocolMessage) -> std::io::Result<()> {
        self.outbound_tx.send(message).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "agent stdio transport closed",
            )
        })
    }

    pub fn take_inbound_rx(&mut self) -> mpsc::UnboundedReceiver<ProtocolMessage> {
        std::mem::replace(&mut self.inbound_rx, mpsc::unbounded_channel().1)
    }

    pub fn take_closed_rx(&mut self) -> mpsc::Receiver<TransportClosed> {
        std::mem::replace(&mut self.closed_rx, mpsc::channel(1).1)
    }

    /// 请求自然退出：关闭 stdin（EOF），agent 收到 EOF 后自行收尾。
    /// 完整的进程树清理（Windows taskkill 兜底）后续在会话管理器补齐。
    pub async fn close_stdin(&mut self) {
        let _ = self.close_stdin_tx.send(()).await;
    }

    pub async fn kill(&mut self) {
        let _ = self.child.kill().await;
    }
}

fn tracing_if_debug(line: &str, pid: Option<u32>) {
    // stderr 只在调试场景看；避免默认日志噪声。
    if std::env::var("ZCODE_DESKTOP_RS_DEBUG_STDERR").as_deref() == Ok("1") {
        eprintln!("[agent:{pid:?}] stderr: {line}");
    }
}

use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ProtocolMessage, ProtocolRequest, RequestId};
    use std::time::Duration;

    /// 用 node 子进程模拟 agent：收到请求后原样回一个 response 帧。
    /// 验证 spawn → NDJSON 写读 → 解析 → 配对的完整链路。
    #[tokio::test]
    async fn roundtrip_with_mock_agent() {
        let script = r#"
const rl = require("readline").createInterface({ input: process.stdin });
rl.on("line", (line) => {
  if (!line.trim()) return;
  const msg = JSON.parse(line);
  process.stdout.write(JSON.stringify({ id: msg.id, result: { echo: msg.method } }) + "\n");
});
"#;
        let dir = std::env::temp_dir();
        let mut transport =
            StdioTransport::spawn("node", &["-e".to_string(), script.to_string()], &dir, &[])
                .expect("spawn mock agent");

        transport
            .send(ProtocolMessage::Request(ProtocolRequest {
                id: RequestId::Str("t1".into()),
                method: "test.ping".into(),
                params: None,
                trace: None,
            }))
            .expect("send request");

        let mut inbound = transport.take_inbound_rx();
        let msg = tokio::time::timeout(Duration::from_secs(10), inbound.recv())
            .await
            .expect("timeout waiting response")
            .expect("channel closed");
        match msg {
            ProtocolMessage::Response(res) => {
                assert_eq!(res.id, RequestId::Str("t1".into()));
                assert_eq!(res.result["echo"], "test.ping");
            }
            other => panic!("expected response, got {other:?}"),
        }

        transport.close_stdin().await;
    }
}
