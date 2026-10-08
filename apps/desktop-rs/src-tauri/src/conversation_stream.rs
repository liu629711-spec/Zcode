//! v4 会话 topic 订阅与下行帧处理（对齐真源
//! `packages/shared/src/zcode-protocol-v4/transport.ts` + `delta.ts` + `wire.ts`）。
//!
//! 上行：`v4/conversation/subscribe`（V4_METHODS.conversationSubscribe，transport.ts:337）。
//! 下行：通知 `v4/conversation/frame`（V4_NOTIFICATIONS.conversationFrame，:408），
//!       params = ConversationTopicFrame。
//!
//! 帧信封（transport.ts:160-178 createTopicFrameSchema）：
//! ```text
//! { topic, subscriptionId, fromSeq, toSeq, sentAt,
//!   payload: { kind:"snapshot", snapshot } | { kind:"deltas", deltas:[...] } }
//! ```
//! `fromSeq/toSeq` 是区间记账 **(fromSeq, toSeq]**；snapshot 帧 fromSeq 固定 0。
//!
//! 物理层还有 TopicWireFrame（wire.ts:19-45）的 complete/fragment 分片，
//! 大帧才走那条路；重组逻辑在 wire-reassembly.ts。本模块按 `complete` 路径处理，
//! fragment 交给前端（见下方 TODO 说明）。

use serde::{Deserialize, Serialize};

/// V4 方法名（真源 transport.ts:332-385 V4_METHODS）。
pub const CONVERSATION_SUBSCRIBE: &str = "v4/conversation/subscribe";
pub const CONVERSATION_UNSUBSCRIBE: &str = "v4/conversation/unsubscribe";
pub const CONVERSATION_RESYNC: &str = "v4/conversation/resync";

/// 下行通知（真源 transport.ts:407-415 V4_NOTIFICATIONS）。
pub const NOTIFICATION_CONVERSATION_FRAME: &str = "v4/conversation/frame";

/// 客户端模式（真源 v4ConversationSubscribeParamsSchema.clientMode）。
/// 桌面端连续流用 `desktop-continuous`（rowsRange 也用同一个值，口径一致）。
pub const CLIENT_MODE_DESKTOP_CONTINUOUS: &str = "desktop-continuous";

/// topic 前缀（真源 conversationTopicFrameSchema.superRefine 要求）。
pub const TOPIC_PREFIX: &str = "conversation/";

/// 构造订阅参数（对齐 v4ConversationSubscribeParamsSchema）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeParams {
    pub connection_id: String,
    pub topic: String,
    pub client_mode: &'static str,
    /// 冷恢复用的 workspace（live 不消费，但协议要求可选带上）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_task_ids: Option<Vec<String>>,
}

/// workspace 引用（`zcode-protocol-legacy-types.ts:45-51` zcodeWorkspaceRefSchema）。
///
/// **`.strict()` + `workspaceKey` 必填**（nonEmptyString）——这是真实联调才发现的：
/// 只传 workspacePath 会被 agent 以 ZodError 拒绝（path: ["workspace","workspaceKey"]）。
/// workspaceKey 口径与 tasks 表一致：identity优先、回退 path（`paths.ts:188` getWorkspaceKey）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRef {
    pub workspace_key: String,
    pub workspace_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_identity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_session_id: Option<String>,
}

impl WorkspaceRef {
    /// 本地 workspace：key 与 path 相同（无 identity）。
    pub fn local(workspace_path: impl Into<String>) -> Self {
        let path = workspace_path.into();
        Self {
            workspace_key: path.clone(),
            workspace_path: path,
            workspace_identity: None,
            remote_session_id: None,
        }
    }

    /// 远端 workspace：key 取 identity（口径同 tasks 表 workspace_key）。
    pub fn remote(workspace_path: impl Into<String>, identity: impl Into<String>) -> Self {
        let identity = identity.into();
        Self {
            workspace_key: identity.clone(),
            workspace_path: workspace_path.into(),
            workspace_identity: Some(identity),
            remote_session_id: None,
        }
    }
}

impl SubscribeParams {
    /// 会话 topic（真源 parseConversationTopic，transport.ts:1148）。
    pub fn conversation_topic(session_id: &str) -> String {
        format!("{TOPIC_PREFIX}{session_id}")
    }

    /// 桌面端连续流订阅（live 场景：不需要 legacyTaskIds）。
    pub fn desktop_conversation(
        connection_id: impl Into<String>,
        session_id: &str,
        workspace_path: Option<String>,
    ) -> Self {
        Self {
            connection_id: connection_id.into(),
            topic: Self::conversation_topic(session_id),
            client_mode: CLIENT_MODE_DESKTOP_CONTINUOUS,
            workspace: workspace_path.map(WorkspaceRef::local),
            legacy_task_ids: None,
        }
    }
}

// ---------------------------------------------------------------------------
// 下行帧
// ---------------------------------------------------------------------------

/// 物理帧（真源 TopicWireFrame，wire.ts:19-45）。
///
/// **实测形态**：通知 params 就是这个结构 —— 外层带 `wireVersion/kind/logicalFrameId/
/// logicalFrameOrdinal/topic/subscriptionId`，逻辑帧在 `frame` 字段里（complete 时）。
/// 逻辑帧的 topic/subscriptionId 在内外两层都有（真源刻意冗余，便于重组时校验）。
///
/// `kind:"fragment"` 时逻辑帧被切碎，载荷是 `dataBase64` + checksum + 碎片序号，
/// 需要 wire-reassembly.ts 那套重组。本模块暂不处理 fragment（小帧走 complete）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WireFrame {
    pub wire_version: u32,
    pub kind: WireFrameKind,
    pub topic: String,
    pub subscription_id: String,
    pub logical_frame_id: String,
    pub logical_frame_ordinal: u32,
    /// kind=complete 时承载逻辑帧。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<TopicFrame>,
    /// kind=fragment 时承载碎片载荷（本模块不重组，故不建模）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment_index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment_count: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WireFrameKind {
    /// 整帧到达，逻辑帧在 `frame` 里。
    Complete,
    /// 大帧分片，需重组（本模块暂不支持）。
    Fragment,
}

impl WireFrame {
    /// 取逻辑帧；fragment 返回 None（调用方应触发 resync 而不是硬拼）。
    pub fn logical(&self) -> Option<&TopicFrame> {
        self.frame.as_ref()
    }

    /// 是否需要重组才能拿到逻辑帧。
    pub fn needs_reassembly(&self) -> bool {
        self.kind == WireFrameKind::Fragment
    }
}

/// 会话 topic 帧（真源 ConversationTopicFrame）。
///
/// 宽容解析：只取本客户端用得到的字段，未知键忽略
/// （真源 zod 是 .strict()，但 Rust 端不用 deny_unknown_fields——
/// agent 侧新增可选键不应导致整帧解析失败）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TopicFrame {
    pub topic: String,
    pub subscription_id: String,
    /// 区间记账 (fromSeq, toSeq]。
    pub from_seq: f64,
    pub to_seq: f64,
    #[serde(default)]
    pub sent_at: Option<f64>,
    pub payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Payload {
    /// 首帧全量快照。
    Snapshot { snapshot: serde_json::Value },
    /// 后续增量（真源 payload.kind === "deltas"，注意是复数）。
    Deltas { deltas: Vec<Delta> },
}

/// 会话增量（真源 ConversationDelta，delta.ts:94-146）。
///
/// 七个 op 的封闭集合（delta.ts 头注释：没有 row.inserted / row.moved /
/// 字段级 JSON patch——模型表达不了的结构变化一律发 snapshot resync）。
///
/// **注意：op 值含点号（`row.delta`），所以每个变体必须显式 `rename`**——
/// `rename_all = "camelCase"` 会把 `RowDelta` 变成 `rowDelta`，协议对不上。
///（字段名仍用 rename_all 转成 fromRowId 这类驼峰，与真源一致。）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Delta {
    /// 追加到尾部（99% 的情况）。
    #[serde(rename = "row.appended")]
    RowAppended { row: serde_json::Value },
    /// 按 rowId 整行替换（状态机迁移，如 streaming → complete）。
    #[serde(rename = "row.upserted")]
    RowUpserted { row: serde_json::Value },
    /// 删除该行及之后所有（edit/retry 分支）。
    #[serde(rename = "row.removed")]
    RowRemoved {
        #[serde(rename = "fromRowId")]
        from_row_id: f64,
    },
    /// **流式文本追加——逐 token 流式的核心**。
    #[serde(rename = "row.delta")]
    RowDelta {
        #[serde(rename = "rowId")]
        row_id: f64,
        path: String,
        append: String,
    },
    /// 键级整体替换的状态补丁（delta.ts:33-60 statePatchSchema）。
    #[serde(rename = "state.updated")]
    StateUpdated { patch: serde_json::Value },
    /// workflowRun 的键级增量（键内整替换，只下探两级）。
    #[serde(rename = "workflowRun.updated")]
    WorkflowRunUpdated {
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "revision")]
        revision: Option<f64>,
        #[serde(flatten)]
        extra: serde_json::Map<String, serde_json::Value>,
    },
    /// 某 run 被淘汰。
    #[serde(rename = "workflowRun.removed")]
    WorkflowRunRemoved {
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "revision")]
        revision: Option<f64>,
    },
}

impl Delta {
    /// 是否是流式文本增量（前端只需对这类做局部追加，不必整页重拉）。
    pub fn is_text_delta(&self) -> bool {
        matches!(self, Delta::RowDelta { .. })
    }
}

// ---------------------------------------------------------------------------
// 流式文本累积
// ---------------------------------------------------------------------------

/// 按 rowId 累积流式文本。
///
/// 真源客户端行为：收到 `row.delta` 就把 `append` 拼到该 row 的对应 path 上，
/// **不等整行 upsert**——这样才是逐 token 出字。
/// `row.upserted` 到达时整行覆盖（服务端给权威文本），所以这里记录已累积的长度，
/// 覆盖后要清零，避免把已覆盖的文本再拼一遍。
#[derive(Debug, Default)]
pub struct StreamAccumulator {
    /// rowId → (path, 已累积文本)
    entries: std::collections::HashMap<i64, (String, String)>,
}

impl StreamAccumulator {
    /// 施加一条 delta；返回该 row 是否产生了文本变化。
    ///
    /// `row.removed` 会连带清掉该 rowId 及之后的所有累积（真源删除语义）。
    pub fn apply(&mut self, delta: &Delta) -> bool {
        match delta {
            Delta::RowDelta {
                row_id, path, append,
            } => {
                let entry = self
                    .entries
                    .entry(*row_id as i64)
                    .or_insert_with(|| (path.clone(), String::new()));
                // 同 rowId 换了 path（服务端切字段）→ 重开累积。
                if entry.0.as_str() != path {
                    entry.0.clone_from(path);
                    entry.1.clear();
                }
                entry.1.push_str(append);
                true
            }
            // 整行替换：清掉该行的累积，下次 delta 从零开始。
            Delta::RowUpserted { row } => {
                if let Some(id) = row_id_of(row) {
                    self.entries.remove(&id);
                }
                false
            }
            Delta::RowRemoved { from_row_id } => {
                self.entries.retain(|id, _| *id < *from_row_id as i64);
                false
            }
            _ => false,
        }
    }

    /// 某行当前累积出的文本（未收到 upsert 前的实时内容）。
    pub fn text_of(&self, row_id: i64) -> Option<&str> {
        self.entries.get(&row_id).map(|(_, text)| text.as_str())
    }

    /// 丢掉某行的累积（前端已用 upsert 的权威文本刷新过）。
    pub fn clear(&mut self, row_id: i64) {
        self.entries.remove(&row_id);
    }

    /// 当前累积中的行数（诊断用）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 从 row 里取 rowId（真源 rows.ts rowBaseFields.rowId 是 number）。
fn row_id_of(row: &serde_json::Value) -> Option<i64> {
    row.get("rowId")?.as_i64()
}

/// 施加整帧，返回其中文本增量的条数（前端据此决定是否局部刷新）。
pub fn apply_frame(acc: &mut StreamAccumulator, frame: &TopicFrame) -> usize {
    let Payload::Deltas { deltas } = &frame.payload else {
        // snapshot 全量到达：客户端应整体替换，清空所有累积。
        acc.entries.clear();
        return 0;
    };
    let mut text_deltas = 0;
    for delta in deltas {
        if delta.is_text_delta() {
            text_deltas += 1;
        }
        acc.apply(delta);
    }
    text_deltas
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(deltas: Vec<Delta>) -> TopicFrame {
        TopicFrame {
            topic: "conversation/s1".into(),
            subscription_id: "sub-1".into(),
            from_seq: 0.0,
            to_seq: 1.0,
            sent_at: Some(1.0),
            payload: Payload::Deltas { deltas },
        }
    }

    #[test]
    fn conversation_topic_format() {
        // 真源 parseConversationTopic 要求 conversation/ 前缀且非空。
        assert_eq!(SubscribeParams::conversation_topic("s1"), "conversation/s1");
        assert!(SubscribeParams::conversation_topic("s1").starts_with(TOPIC_PREFIX));
    }

    #[test]
    fn subscribe_params_shape_is_camel_case() {
        let p = SubscribeParams::desktop_conversation("conn-1", "s1", Some("C:/ws".into()));
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["connectionId"], "conn-1");
        assert_eq!(json["topic"], "conversation/s1");
        assert_eq!(json["clientMode"], CLIENT_MODE_DESKTOP_CONTINUOUS);
        assert_eq!(
            json["workspace"]["workspaceKey"], "C:/ws",
            "workspaceKey 必填（ZodError 实测），本地 workspace 下与 path 相同"
        );
        assert_eq!(json["workspace"]["workspacePath"], "C:/ws");
        // live 场景不带 legacyTaskIds。
        assert!(json.get("legacyTaskIds").is_none(), "None 字段应被跳过");
    }

    #[test]
    fn subscribe_params_without_workspace_omits_it() {
        let p = SubscribeParams::desktop_conversation("c", "s1", None);
        let json = serde_json::to_value(&p).unwrap();
        assert!(json.get("workspace").is_none());
    }

    #[test]
    fn parses_deltas_frame_with_camel_case_ops() {
        let raw = serde_json::json!({
            "topic": "conversation/s1",
            "subscriptionId": "sub-1",
            "fromSeq": 3.0,
            "toSeq": 5.0,
            "sentAt": 1234.5,
            "payload": {
                "kind": "deltas",
                "deltas": [
                    { "op": "row.delta", "rowId": 7, "path": "text", "append": "你" },
                    { "op": "row.removed", "fromRowId": 9 }
                ],
            }
        });
        let parsed: TopicFrame = serde_json::from_value(raw).unwrap();
        assert_eq!(parsed.from_seq, 3.0);
        assert_eq!(parsed.to_seq, 5.0);
        let Payload::Deltas { deltas } = &parsed.payload else {
            panic!("应为 deltas 载荷");
        };
        assert_eq!(deltas.len(), 2);
        assert!(deltas[0].is_text_delta());
        match &deltas[0] {
            Delta::RowDelta { row_id, append, .. } => {
                assert_eq!(*row_id, 7.0);
                assert_eq!(append, "你");
            }
            other => panic!("期望 RowDelta，实际 {other:?}"),
        }
    }

    #[test]
    fn parses_snapshot_frame() {
        let raw = serde_json::json!({
            "topic": "conversation/s1",
            "subscriptionId": "sub-1",
            "fromSeq": 0.0,
            "toSeq": 2.0,
            "payload": { "kind": "snapshot", "snapshot": { "rows": [] } }
        });
        let parsed: TopicFrame = serde_json::from_value(raw).unwrap();
        assert!(matches!(parsed.payload, Payload::Snapshot { .. }));
        // snapshot 帧允许没有 sentAt。
        assert!(parsed.sent_at.is_none());
    }

    #[test]
    fn unknown_delta_op_is_rejected_not_silently_dropped() {
        // 封闭 op 集合：遇到不认识的 op 必须报错，不能默默吞掉——
        // 默默吞掉会让客户端状态与服务端悄悄分叉（delta.ts 头注释强调压缩错误面）。
        let raw = serde_json::json!({
            "op": "row.moved", "rowId": 1
        });
        assert!(serde_json::from_value::<Delta>(raw).is_err());
    }

    #[test]
    fn unknown_top_level_keys_are_tolerated() {
        // agent 侧新增可选键（如 ttft）不应导致整帧解析失败。
        let raw = serde_json::json!({
            "topic": "conversation/s1",
            "subscriptionId": "sub-1",
            "fromSeq": 0.0,
            "toSeq": 1.0,
            "payload": { "kind": "deltas", "deltas": [] },
            "ttft": { "firstTokenMs": 120 }
        });
        assert!(serde_json::from_value::<TopicFrame>(raw).is_ok());
    }

    #[test]
    fn accumulator_concatenates_text_in_order() {
        let mut acc = StreamAccumulator::default();
        assert!(acc.is_empty());
        // 逐 token：三次 append 应拼成完整句。
        for piece in ["Hel", "lo", "!"] {
            acc.apply(&Delta::RowDelta {
                row_id: 7.0,
                path: "text".into(),
                append: piece.into(),
            });
        }
        assert_eq!(acc.text_of(7), Some("Hello!"));
        assert_eq!(acc.len(), 1);
    }

    #[test]
    fn accumulator_separates_rows() {
        let mut acc = StreamAccumulator::default();
        acc.apply(&Delta::RowDelta {
            row_id: 1.0,
            path: "text".into(),
            append: "A".into(),
        });
        acc.apply(&Delta::RowDelta {
            row_id: 2.0,
            path: "text".into(),
            append: "B".into(),
        });
        assert_eq!(acc.text_of(1), Some("A"));
        assert_eq!(acc.text_of(2), Some("B"));
    }

    #[test]
    fn upsert_clears_accumulated_text_for_that_row() {
        let mut acc = StreamAccumulator::default();
        acc.apply(&Delta::RowDelta {
            row_id: 7.0,
            path: "text".into(),
            append: "半截".into(),
        });
        assert!(acc.text_of(7).is_some());

        // upsert 是权威整行，必须清累积，否则之后再来 delta 会重复拼接。
        acc.apply(&Delta::RowUpserted {
            row: serde_json::json!({ "rowId": 7, "text": "权威全文" }),
        });
        assert!(acc.text_of(7).is_none(), "upsert 后应清掉该行累积");
    }

    #[test]
    fn path_switch_resets_accumulation() {
        let mut acc = StreamAccumulator::default();
        acc.apply(&Delta::RowDelta {
            row_id: 7.0,
            path: "text".into(),
            append: "AAA".into(),
        });
        // 服务端切到另一个可流式字段 → 重新累积。
        acc.apply(&Delta::RowDelta {
            row_id: 7.0,
            path: "reasoning".into(),
            append: "B".into(),
        });
        assert_eq!(acc.text_of(7), Some("B"), "换 path 应重开累积");
    }

    #[test]
    fn removed_clears_that_row_and_after() {
        let mut acc = StreamAccumulator::default();
        for id in [1i64, 5, 9] {
            acc.apply(&Delta::RowDelta {
                row_id: id as f64,
                path: "text".into(),
                append: "x".into(),
            });
        }
        acc.apply(&Delta::RowRemoved { from_row_id: 5.0 });
        assert!(acc.text_of(1).is_some(), "小于 fromRowId 的保留");
        assert!(acc.text_of(5).is_none(), "等于 fromRowId 的清掉");
        assert!(acc.text_of(9).is_none(), "大于 fromRowId 的清掉");
    }

    #[test]
    fn apply_frame_counts_text_deltas() {
        let mut acc = StreamAccumulator::default();
        let f = frame(vec![
            Delta::RowDelta {
                row_id: 1.0,
                path: "text".into(),
                append: "a".into(),
            },
            Delta::StateUpdated {
                patch: serde_json::json!({ "revision": 2 }),
            },
            Delta::RowDelta {
                row_id: 1.0,
                path: "text".into(),
                append: "b".into(),
            },
        ]);
        assert_eq!(apply_frame(&mut acc, &f), 2, "只统计文本增量条数");
        assert_eq!(acc.text_of(1), Some("ab"));
    }

    #[test]
    fn snapshot_frame_clears_accumulator() {
        let mut acc = StreamAccumulator::default();
        acc.apply(&Delta::RowDelta {
            row_id: 1.0,
            path: "text".into(),
            append: "旧".into(),
        });
        let snapshot = TopicFrame {
            topic: "conversation/s1".into(),
            subscription_id: "sub".into(),
            from_seq: 0.0,
            to_seq: 1.0,
            sent_at: None,
            payload: Payload::Snapshot {
                snapshot: serde_json::json!({ "rows": [] }),
            },
        };
        assert_eq!(apply_frame(&mut acc, &snapshot), 0);
        assert!(acc.is_empty(), "snapshot 应清空累积（客户端整体替换）");
    }
}
#[cfg(test)]
mod wire_contract_tests {
    use super::Delta;

    /// 锁住 wire 形态：op 值含点号，必须逐个rename；
    /// 字段名走 camelCase。两边都写死，改错立刻炸。
    #[test]
    fn delta_op_values_keep_dots() {
        let cases: [(&str, &str); 6] = [
            (r#"{"op":"row.appended","row":{}}"#, "row.appended"),
            (r#"{"op":"row.upserted","row":{}}"#, "row.upserted"),
            (r#"{"op":"row.removed","fromRowId":1}"#, "row.removed"),
            (r#"{"op":"row.delta","rowId":1,"path":"text","append":"x"}"#, "row.delta"),
            (r#"{"op":"state.updated","patch":{}}"#, "state.updated"),
            (r#"{"op":"workflowRun.removed","runId":"r1"}"#, "workflowRun.removed"),
        ];
        for (json, expected_op) in cases {
            let parsed: Delta = serde_json::from_str(json).unwrap_or_else(|e| {
                panic!("{json} 解析失败: {e}（op 名或字段名与协议不符）");
            });
            assert!(
                serde_json::to_string(&parsed).unwrap().contains(expected_op),
                "序列化后必须保�� {expected_op}"
            );
        }
    }

    /// `rename_all` 在指定了变体 rename 后**不会**再转字段名，
    /// 所以字段必须逐个 rename（如 from_row_id → fromRowId）。
    #[test]
    fn delta_fields_are_camel_case() {
        let parsed: Delta =
            serde_json::from_str(r#"{"op":"row.removed","fromRowId":42}"#).unwrap();
        match parsed {
            Delta::RowRemoved { from_row_id } => assert_eq!(from_row_id, 42.0),
            other => panic!("期望 RowRemoved，实际 {other:?}"),
        }

        let parsed: Delta =
            serde_json::from_str(r#"{"op":"workflowRun.updated","runId":"r1","revision":3}"#)
                .unwrap();
        match parsed {
            Delta::WorkflowRunUpdated { run_id, revision, .. } => {
                assert_eq!(run_id, "r1");
                assert_eq!(revision, Some(3.0));
            }
            other => panic!("期望 WorkflowRunUpdated，实际 {other:?}"),
        }
    }
}

#[cfg(test)]
mod wire_frame_tests {
    use super::{TopicFrame, WireFrame, WireFrameKind};

    /// 实测形态（真实 agent 推送的 notification params 原文结构）：
    /// 外层 wireVersion/kind/logicalFrameId/logicalFrameOrdinal/topic/subscriptionId，
    /// 逻辑帧在 `frame` 里。这是**第二层**信封，别按逻辑帧直接解析。
    #[test]
    fn parses_real_complete_wire_frame() {
        let raw = serde_json::json!({
            "deliveryKind": "initial",
            "frame": {
                "fromSeq": 0,
                "payload": {
                    "kind": "snapshot",
                    "snapshot": {
                        "rows": { "firstRowId": null, "totalCount": 0, "window": [] },
                        "revision": 0,
                        "seq": 0
                    }
                },
                "sentAt": 1791453274205i64,
                "subscriptionId": "sub-1",
                "toSeq": 0,
                "topic": "conversation/s1"
            },
            "kind": "complete",
            "logicalFrameId": "sub-1-lf-1",
            "logicalFrameOrdinal": 1,
            "subscriptionId": "sub-1",
            "topic": "conversation/s1",
            "wireVersion": 3
        });
        let wire: WireFrame = serde_json::from_value(raw).unwrap();
        assert_eq!(wire.wire_version, 3, "实测协议版本是 3");
        assert_eq!(wire.kind, WireFrameKind::Complete);
        assert_eq!(wire.logical_frame_id, "sub-1-lf-1");
        assert_eq!(wire.logical_frame_ordinal, 1);
        assert!(!wire.needs_reassembly());

        let logical = wire.logical().expect("complete 帧必带逻辑帧");
        assert_eq!(logical.topic, "conversation/s1");
        assert_eq!(logical.from_seq, 0.0);
        assert_eq!(logical.to_seq, 0.0);
        // 外层冗余的 topic 必须与逻辑帧一致（真源设计如此，便于重组时校验）。
        assert_eq!(wire.topic, logical.topic);
    }

    #[test]
    fn fragment_frame_has_no_logical_payload() {
        // 大帧分片：逻辑帧不在 frame 里，需重组（本模块暂不支持）。
        let raw = serde_json::json!({
            "kind": "fragment",
            "topic": "conversation/s1",
            "subscriptionId": "sub-1",
            "logicalFrameId": "sub-1-lf-2",
            "logicalFrameOrdinal": 2,
            "wireVersion": 3,
            "fragmentIndex": 0,
            "fragmentCount": 3,
            "logicalBytes": 90000,
            "checksum": { "algorithm": "crc32", "value": "1a2b3c4d" },
            "dataBase64": "AAAA"
        });
        let wire: WireFrame = serde_json::from_value(raw).unwrap();
        assert_eq!(wire.kind, WireFrameKind::Fragment);
        assert!(wire.needs_reassembly(), "fragment 必须走重组路径");
        assert!(wire.logical().is_none(), "fragment 不带完整逻辑帧");
        assert_eq!(wire.fragment_count, Some(3));
        // 未知字段（dataBase64/checksum/deliveryKind）被宽容忽略，不影响解析。
    }

    #[test]
    fn delta_carrying_wire_frame_parses() {
        // 带文本增量的帧：这是逐 token 的实际载体。
        let raw = serde_json::json!({
            "kind": "complete",
            "topic": "conversation/s1",
            "subscriptionId": "sub-1",
            "logicalFrameId": "sub-1-lf-3",
            "logicalFrameOrdinal": 3,
            "wireVersion": 3,
            "frame": {
                "topic": "conversation/s1",
                "subscriptionId": "sub-1",
                "fromSeq": 5.0,
                "toSeq": 8.0,
                "payload": {
                    "kind": "deltas",
                    "deltas": [
                        { "op": "row.delta", "rowId": 42, "path": "text", "append": "你" },
                        { "op": "row.delta", "rowId": 42, "path": "text", "append": "好" }
                    ]
                }
            }
        });
        let wire: WireFrame = serde_json::from_value(raw).unwrap();
        let logical: &TopicFrame = wire.logical().unwrap();
        let mut acc = super::StreamAccumulator::default();
        let text_deltas = super::apply_frame(&mut acc, logical);
        assert_eq!(text_deltas, 2);
        assert_eq!(acc.text_of(42), Some("你好"), "逐 token 应按序拼接");
    }
}
