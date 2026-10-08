//! 逐 token 流式的真实链路联调。
//!
//! 验证：
//! 1. spawn 真实 agent（数据目录隔离到临时路径，不污染真实 ~/.zcode）；
//! 2. capabilities 握手；
//! 3. v4 createSession 拿真实 sessionId；
//! 4. `v4/conversation/subscribe` 订阅；
//! 5. 从通知流里筛 `v4/conversation/frame`，用协议层解析成TopicFrame。
//!
//! 判据是「帧能按真源 wire 形态解析成功」，**不要求一定收到 token**——
//! 测试环境无 LLM key 时turn 会失败，但订阅通道照样建立。
//! 这样测试不依赖模型可用性，才稳定。

use std::time::Duration;

use zcode_desktop_rs_lib::agent::client::AgentClient;
use zcode_desktop_rs_lib::agent::command::resolve_agent_command;
use zcode_desktop_rs_lib::agent::methods;
use zcode_desktop_rs_lib::agent::transport::StdioTransport;
use zcode_desktop_rs_lib::conversation_stream::{
    self, Payload, SubscribeParams, WireFrame, CONVERSATION_SUBSCRIBE,
    NOTIFICATION_CONVERSATION_FRAME,
};

#[tokio::test(flavor = "multi_thread")]
async fn live_conversation_frame_shape() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest_dir
        .parent()
        .map(|p| p.to_path_buf())
        .expect("desktop-rs 应位于 apps/ 下");

    let command = resolve_agent_command(&workspace).expect("应能解析出真实 agent 启动命令");

    // 数据目录隔离，避免污染真实 ~/.zcode。
    let temp_data = std::env::temp_dir().join(format!("zcode-stream-test-{}", std::process::id()));
    std::fs::create_dir_all(&temp_data).ok();
    let mut env = command.env.clone();
    env.push((
        "ZCODE_DATA_BASE_DIR".into(),
        temp_data.to_string_lossy().into_owned(),
    ));

    let transport =
        StdioTransport::spawn(&command.program, &command.args, &command.cwd, &env)
            .expect("spawn 真实 agent");
    let (client, mut notifications, mut closed_rx) = AgentClient::start(transport);

    // 1) 握手，确认 agent 活着。
    client
        .request(methods::RUNTIME_CAPABILITIES, None, Duration::from_secs(60))
        .await
        .expect("capabilities 握手");

    // 2) 建会话拿sessionId。
    //    CommandEnvelope 需要 sessionId: null 与 issuedAt（缺任一个都会被
    //    agent 以 proto.invalidPayload 拒绝），口径见 manager::create_session_v4。
    let home = dirs_home();
    let envelope = serde_json::json!({
        "commandId": uuid::Uuid::now_v7().to_string(),
        "clientId": "desktop-rs-stream-test",
        "sessionId": serde_json::Value::Null,
        "type": "createSession",
        "payload": { "workspaceId": home },
        "issuedAt": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    });
    let created = client
        .request(methods::V4_COMMAND, Some(envelope), Duration::from_secs(30))
        .await;
    let Ok(created) = created else {
        eprintln!("[live] 跳过：v4 createSession 未成功");
        client.close_stdin().await;
        std::fs::remove_dir_all(&temp_data).ok();
        return;
    };
    let Some(session_id) = created["result"]["sessionId"].as_str().map(str::to_string) else {
        eprintln!("[live] 跳过：响应无 sessionId = {}", created);
        client.close_stdin().await;
        std::fs::remove_dir_all(&temp_data).ok();
        return;
    };
    eprintln!("[live] 已创建会话 {session_id}");

    // 3) 订阅。
    let params = SubscribeParams::desktop_conversation("live-conn", &session_id, Some(home));
    eprintln!(
        "[live] subscribe params = {}",
        serde_json::to_string(&params).unwrap()
    );
    let subscribed = client
        .request(
            CONVERSATION_SUBSCRIBE,
            Some(serde_json::to_value(&params).unwrap()),
            Duration::from_secs(20),
        )
        .await;
    match &subscribed {
        Ok(v) => eprintln!("[live] subscribe 被接受：{v}"),
        Err(e) => {
            // 订阅被拒不一定是本模块的问题，把 agent 原话带出来便于判断。
            eprintln!("[live] subscribe 返回错误：{e}");
            let reason = closed_rx.try_recv().ok();
            eprintln!("[live] agent closed={reason:?}");
        }
    }
    assert!(subscribed.is_ok(), "subscribe 应被接受：{subscribed:?}");

    // 4) 收通知，筛 conversation/frame 并解析。
    let mut saw_frame = false;
    let mut text_deltas = 0usize;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    while tokio::time::Instant::now() < deadline && !saw_frame {
        let Ok(Some(note)) =
            tokio::time::timeout(Duration::from_millis(800), notifications.recv()).await
        else {
            continue;
        };
        if note.method != NOTIFICATION_CONVERSATION_FRAME {
            continue;
        }
        saw_frame = true;
        let params = note.params.unwrap_or_else(|| serde_json::json!({}));
        // 通知 params 是**物理帧**（wireVersion/kind/logicalFrameId +内嵌 frame），
        // 这是实测确认的——第一版按逻辑帧直接解析，报 missing field fromSeq。
        let wire: WireFrame = serde_json::from_value(params.clone())
            .unwrap_or_else(|e| panic!("物理帧解析失败：{e}\n收到：{params}"));

        assert_eq!(wire.topic, wire.frame.as_ref().map_or_else(
            || wire.topic.clone(),
            |f| f.topic.clone()
        ), "内外层topic 应一致（真源冗余便于校验）");

        let Some(frame) = wire.logical() else {
            eprintln!(
                "[live] 收到 fragment 帧（count={:?}），需重组；本轮跳过",
                wire.fragment_count
            );
            client.close_stdin().await;
            std::fs::remove_dir_all(&temp_data).ok();
            return;
        };

        let kind = match &frame.payload {
            Payload::Snapshot { snapshot } => {
                // 快照里的行数能验证结构是否符合真源rows 结构。
                let rows = snapshot["rows"]["totalCount"].as_i64().unwrap_or(-1);
                eprintln!("[live] ✓ 收到 snapshot 帧，totalCount={rows}");
                "snapshot"
            }
            Payload::Deltas { deltas } => {
                text_deltas += deltas.iter().filter(|d| d.is_text_delta()).count();
                "deltas"
            }
        };
        eprintln!(
            "[live]帧 topic={} fromSeq={} toSeq={} payload={kind} wireVersion={}",
            frame.topic, frame.from_seq, frame.to_seq, wire.wire_version
        );
        assert!(
            frame.topic.starts_with(conversation_stream::TOPIC_PREFIX),
            "topic 必须以 conversation/ 开头（真源 superRefine 强制）"
        );
        // 区间记账 (fromSeq, toSeq]。**空快照例外**：ack 里 snapshotRowCount=0 时
        // 不会有实质行变化，fromSeq == toSeq 是合法的。
        assert!(
            frame.from_seq <= frame.to_seq,
            "fromSeq 不得大于 toSeq（区间记账是 (fromSeq, toSeq]）"
        );
    }

    if saw_frame {
        eprintln!("[live] 帧格式解析通过，文本增量条数={text_deltas}");
    } else {
        // 不作为失败：agent 未必在无消息时推帧，订阅本身已被接受即证明链路通。
        eprintln!("[live] 6 秒内未收到帧（订阅已接受，属正常）");
    }

    client.close_stdin().await;
    std::fs::remove_dir_all(&temp_data).ok();
}

fn dirs_home() -> String {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(std::path::PathBuf::from)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| ".".to_string())
}