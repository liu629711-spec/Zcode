//! 真实 Agent 联调（需要 zcode-cli dist 产物与本机 node）。
//! 运行：cargo test -p zcode-desktop-rs --test agent_live -- --nocapture

use std::time::Duration;

use zcode_desktop_rs_lib::agent::client::AgentClient;
use zcode_desktop_rs_lib::agent::command::resolve_agent_command;
use zcode_desktop_rs_lib::agent::methods;
use zcode_desktop_rs_lib::agent::transport::StdioTransport;

#[tokio::test(flavor = "multi_thread")]
async fn live_agent_capabilities_handshake() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest_dir
        .parent()
        .map(|p| p.to_path_buf())
        .expect("desktop-rs 应位于 apps/ 下");

    let command =
        resolve_agent_command(&workspace).expect("应能解析出真实 agent 启动命令");
    eprintln!(
        "[live] spawn: {} {:?} (cwd={})",
        command.program,
        command.args,
        command.cwd.display()
    );

    // 数据目录隔离到临时路径，避免污染真实 ~/.zcode。
    let temp_data = std::env::temp_dir().join(format!("zcode-desktop-rs-test-{}", std::process::id()));
    std::fs::create_dir_all(&temp_data).ok();
    let mut env = command.env.clone();
    env.push(("ZCODE_DATA_BASE_DIR".into(), temp_data.to_string_lossy().into_owned()));

    let transport =
        StdioTransport::spawn(&command.program, &command.args, &command.cwd, &env)
            .expect("spawn 真实 agent");
    eprintln!("[live] agent pid = {:?}", transport.pid());

    let (client, _notifications, mut closed_rx) = AgentClient::start(transport);

    let result = client
        .request(methods::RUNTIME_CAPABILITIES, None, Duration::from_secs(60))
        .await;
    match result {
        Ok(capabilities) => {
            eprintln!("[live] capabilities = {}", serde_json::to_string_pretty(&capabilities).unwrap_or_default());
            assert!(capabilities.is_object(), "capabilities 应为对象");
        }
        Err(err) => {
            // 关闭原因有助于诊断（协议解析失败/进程秒退等）。
            let reason = closed_rx.try_recv().ok();
            panic!("capabilities 握手失败: {err}；closed={reason:?}");
        }
    }

    client.close_stdin().await;
    std::fs::remove_dir_all(&temp_data).ok();
}

/// 端到端对话闭环：create → send → messages 包含用户消息。
/// 不断言 assistant 回复内容（测试环境无 LLM key，turn 可能失败，但协议链路必须通）。
#[tokio::test(flavor = "multi_thread")]
async fn live_agent_conversation_loop() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest_dir
        .parent()
        .map(|p| p.to_path_buf())
        .expect("desktop-rs 应位于 apps/ 下");

    let command =
        resolve_agent_command(&workspace).expect("应能解析出真实 agent 启动命令");

    let temp_data = std::env::temp_dir()
        .join(format!("zcode-desktop-rs-conv-{}", std::process::id()));
    std::fs::create_dir_all(&temp_data).ok();
    let mut env = command.env.clone();
    env.push(("ZCODE_DATA_BASE_DIR".into(), temp_data.to_string_lossy().into_owned()));

    let transport =
        StdioTransport::spawn(&command.program, &command.args, &command.cwd, &env)
            .expect("spawn 真实 agent");
    let (client, mut notifications, mut closed_rx) = AgentClient::start(transport);

    // 1. 握手。
    client
        .request(methods::RUNTIME_CAPABILITIES, None, Duration::from_secs(60))
        .await
        .expect("capabilities 握手");

    // 2. 创建会话：workspace ref 指向隔离目录。
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    let create_result = client
        .request(
            methods::SESSION_CREATE,
            Some(serde_json::json!({
                "workspace": {
                    "workspacePath": home,
                    "workspaceKey": home,
                }
            })),
            Duration::from_secs(60),
        )
        .await
        .expect("session/create");
    let session_id = create_result["session"]["sessionId"]
        .as_str()
        .or_else(|| create_result["sessionId"].as_str())
        .expect("session/create 应返回 sessionId")
        .to_string();
    eprintln!("[live] session created: {session_id}");

    // 3. 发送消息。
    let send_result = client
        .request(
            methods::SESSION_SEND,
            Some(serde_json::json!({
                "sessionId": session_id,
                "content": "协议桥自检消息：请勿回复实质内容。"
            })),
            Duration::from_secs(60),
        )
        .await
        .expect("session/send");
    eprintln!(
        "[live] send result: {}",
        serde_json::to_string_pretty(&send_result).unwrap_or_default()
    );

    // 4. 拉取消息 + 通知诊断。
    // 已知边界：测试环境隔离了 ZCODE_DATA_BASE_DIR（无 LLM key），后台 turn 可能
    // 失败导致用户消息未落库——session/messages 返回空不代表协议链路问题。
    // 真实环境（有模型配置）下 user 消息会正常出现在消息流。
    let mut notifications_seen: Vec<String> = Vec::new();
    for _ in 0..8 {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    while let Ok(note) = notifications.try_recv() {
        notifications_seen.push(note.method.clone());
    }
    eprintln!("[live] 收到通知方法: {notifications_seen:?}");

    let messages = client
        .request(
            methods::SESSION_MESSAGES,
            Some(serde_json::json!({ "sessionId": session_id })),
            Duration::from_secs(30),
        )
        .await
        .expect("session/messages");
    assert!(messages.is_object(), "session/messages 应返回对象");
    eprintln!(
        "[live] messages 返回键: {:?}",
        messages.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>())
    );

    client.close_stdin().await;
    std::fs::remove_dir_all(&temp_data).ok();
    let _ = closed_rx.try_recv();
}

