//! 逐 token 流式前端接线（配合主进程 `conversation_stream.rs`）。
//!
//! 数据流：
//! ```text
//! agent 推 v4/conversation/frame（物理帧）
//!   → 主进程转发 `agent://notification`
//!   → 本模块 feed() 解析 wire 帧 → 累积 row.delta 的 append
//!   → ChatView 渲染时按 rowId 取流式文本覆盖 rowsRange 的静态文本
//! ```
//!
//! 关键设计（对齐真源 client 行为）：
//! - `row.delta` 只累积**不整页重拉**——这是逐 token 的关键；
//! - 收到 `row.upserted` 时清累积（整行是权威覆盖，否则重复拼接）；
//! - 收到 snapshot 帧时清全部累积（客户端整体替换）；
//! - 终止态（`state:"complete"`）也清累积，让 rowsRange 的权威文本接管。
//!
//! 真源对照：wire.ts（物理帧）、transport.ts:160-178（逻辑帧信封）、
//! delta.ts:94-146（七个 op）、wire-reassembly.ts（fragment 重组，本轮未做）。

use leptos::prelude::*;
use serde_json::Value;

/// 一个逻辑帧的解析结果。
#[derive(Debug, Clone, PartialEq)]
pub enum FrameOutcome {
    /// 整帧全量：客户端应整体替换（清空所有流式累积）。
    Snapshot,
    /// 带文本增量的行数 > 0（前端需要重绘这些行）。
    TextDeltas { rows: Vec<i64> },
    /// 只有非文本结构变化（upsert/removed/state），需要重拉 rowsRange。
    Structural,
    /// fragment 分片：本轮未实现重组，调用方应触发 resync。
    NeedsResync,
    /// 无法解析（宽容忽略，不打断对话）。
    Ignored,
}

/// 逐 token 流式累积器（前端侧）。
///
/// 与主进程 `StreamAccumulator` 同语义，但放在 wasm 侧——
/// 主进程只做转发与协议解析，累积要贴着渲染状态才划算。
#[derive(Clone, Copy)]
pub struct StreamStore {
    /// sessionId → (rowId → (path, 已累积文本))
    by_session: RwSignal<std::collections::HashMap<String, std::collections::HashMap<i64, (String, String)>>>,
    /// 需要触发重绘的会话 → 待重绘的行。
    dirty: RwSignal<std::collections::HashMap<String, Vec<i64>>>,
    /// 最近一次判定（用于 UI 显示流式状态）。
    last_outcome: RwSignal<String>,
}

impl StreamStore {
    pub fn new() -> Self {
        Self {
            by_session: RwSignal::new(std::collections::HashMap::new()),
            dirty: RwSignal::new(std::collections::HashMap::new()),
            last_outcome: RwSignal::new(String::new()),
        }
    }

    /// 施加一个通知的 params，返回判定结果。
    ///
    /// `session_id` 用来定位累积容器——一个进程可能同时订阅多个会话。
    pub fn feed(&self, session_id: &str, params: &Value) -> FrameOutcome {
        // 第一层：物理帧（有 wireVersion/kind）。
        let outcome = apply_wire_frame(session_id, params, &self.by_session, &self.dirty);
        self.last_outcome.set(describe(&outcome));
        outcome
    }

    /// 某行当前的流式文本（优先于 rowsRange 的静态文本）。
    pub fn text_of(&self, session_id: &str, row_id: i64) -> Option<String> {
        self.by_session
            .get_untracked()
            .get(session_id)
            .and_then(|rows| rows.get(&row_id))
            .map(|(_, text)| text.clone())
    }

    /// 某会话是否有正在流式的行。
    pub fn has_streaming(&self, session_id: &str) -> bool {
        self.by_session
            .get_untracked()
            .get(session_id)
            .is_some_and(|rows| !rows.is_empty())
    }

    /// 清掉某会话的全部累积（收到 upsert/snapshot 或终态时调用）。
    pub fn reset(&self, session_id: &str) {
        self.by_session.update(|m| {
            m.remove(session_id);
        });
    }

    /// 清掉某行的累积（rowsRange 的权威文本已接管该行）。
    pub fn reset_row(&self, session_id: &str, row_id: i64) {
        self.by_session.update(|m| {
            if let Some(rows) = m.get_mut(session_id) {
                rows.remove(&row_id);
            }
        });
    }

    /// 取走某会话待重绘的行（消费语义，避免重复绘制）。
    pub fn take_dirty(&self, session_id: &str) -> Vec<i64> {
        self.dirty
            .get_untracked()
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn last_outcome(&self) -> String {
        self.last_outcome.get_untracked()
    }
}

fn describe(outcome: &FrameOutcome) -> String {
    match outcome {
        FrameOutcome::Snapshot => "snapshot".into(),
        FrameOutcome::TextDeltas { rows } => format!("text-deltas({} 行)", rows.len()),
        FrameOutcome::Structural => "structural".into(),
        FrameOutcome::NeedsResync => "needs-resync".into(),
        FrameOutcome::Ignored => "ignored".into(),
    }
}

/// 解析物理帧 → 逻辑帧 → 施加 delta。
fn apply_wire_frame(
    session_id: &str,
    params: &Value,
    by_session: &RwSignal<std::collections::HashMap<String, std::collections::HashMap<i64, (String, String)>>>,
    dirty: &RwSignal<std::collections::HashMap<String, Vec<i64>>>,
) -> FrameOutcome {
    // 外层信封：有 kind + wireVersion 才是物理帧；否则容忍直接给逻辑帧的情况。
    let logical = match params.get("kind").and_then(|k| k.as_str()) {
        Some("complete") => match params.get("frame") {
            Some(f) => f,
            None => return FrameOutcome::Ignored,
        },
        Some("fragment") => return FrameOutcome::NeedsResync,
        _ => params,
    };

    // 逻辑帧必须带 topic 才是会话帧。
    let Some(topic) = logical.get("topic").and_then(|t| t.as_str()) else {
        return FrameOutcome::Ignored;
    };
    // topic 不属于当前会话 → 不是这个会话的流（agent 可能复用连接）。
    if !topic.ends_with(session_id) {
        return FrameOutcome::Ignored;
    }

    let Some(payload) = logical.get("payload") else {
        return FrameOutcome::Ignored;
    };
    match payload.get("kind").and_then(|k| k.as_str()) {
        Some("snapshot") => {
            // 全量替换：清掉该会话所有累积。
            by_session.update(|m| {
                m.remove(session_id);
            });
            FrameOutcome::Snapshot
        }
        Some("deltas") => {
            let Some(deltas) = payload.get("deltas").and_then(|d| d.as_array()) else {
                return FrameOutcome::Ignored;
            };
            let mut text_rows: Vec<i64> = Vec::new();
            let mut structural = false;
            for delta in deltas {
                match delta.get("op").and_then(|o| o.as_str()) {
                    Some("row.delta") => {
                        let Some(row_id) = delta.get("rowId").and_then(|r| r.as_i64()) else {
                            continue;
                        };
                        let path = delta
                            .get("path")
                            .and_then(|p| p.as_str())
                            .unwrap_or("text")
                            .to_string();
                        let append = delta.get("append").and_then(|a| a.as_str()).unwrap_or("");
                        by_session.update(|m| {
                            let rows = m.entry(session_id.to_string()).or_default();
                            let entry = rows
                                .entry(row_id)
                                .or_insert_with(|| (path.clone(), String::new()));
                            if entry.0 != path {
                                entry.0.clone_from(&path);
                                entry.1.clear();
                            }
                            entry.1.push_str(append);
                        });
                        if !text_rows.contains(&row_id) {
                            text_rows.push(row_id);
                        }
                    }
                    Some("row.upserted") => {
                        // 整行覆盖是权威：清该行累积，让 rowsRange 的文本接管。
                        if let Some(row_id) = delta.get("row").and_then(|r| r.get("rowId")).and_then(|r| r.as_i64()) {
                            by_session.update(|m| {
                                if let Some(rows) = m.get_mut(session_id) {
                                    rows.remove(&row_id);
                                }
                            });
                        }
                        structural = true;
                    }
                    Some("row.appended") => structural = true,
                    Some("row.removed") => {
                        // 该行及之后都失效：清该会话全部累积最简单可靠
                        // （编辑/重试分支频率低，不值得精确定位）。
                        by_session.update(|m| {
                            m.remove(session_id);
                        });
                        structural = true;
                    }
                    Some("state.updated") => structural = true,
                    _ => {}
                }
            }
            if !text_rows.is_empty() {
                dirty.update(|m| {
                    m.insert(session_id.to_string(), text_rows.clone());
                });
                FrameOutcome::TextDeltas { rows: text_rows }
            } else if structural {
                FrameOutcome::Structural
            } else {
                FrameOutcome::Ignored
            }
        }
        _ => FrameOutcome::Ignored,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> StreamStore {
        StreamStore::new()
    }

    /// 真实 agent 推送的物理帧（complete，snapshot）。
    fn wire_snapshot() -> Value {
        serde_json::json!({
            "deliveryKind": "initial",
            "kind": "complete",
            "logicalFrameId": "sub-1-lf-1",
            "logicalFrameOrdinal": 1,
            "subscriptionId": "sub-1",
            "topic": "conversation/s1",
            "wireVersion": 3,
            "frame": {
                "topic": "conversation/s1",
                "subscriptionId": "sub-1",
                "fromSeq": 0,
                "toSeq": 0,
                "payload": {
                    "kind": "snapshot",
                    "snapshot": { "rows": { "totalCount": 0, "window": [] } }
                }
            }
        })
    }

    fn wire_deltas(session: &str, deltas: Value) -> Value {
        serde_json::json!({
            "kind": "complete",
            "logicalFrameId": "sub-1-lf-2",
            "logicalFrameOrdinal": 2,
            "subscriptionId": "sub-1",
            "topic": format!("conversation/{session}"),
            "wireVersion": 3,
            "frame": {
                "topic": format!("conversation/{session}"),
                "subscriptionId": "sub-1",
                "fromSeq": 1,
                "toSeq": 5,
                "payload": { "kind": "deltas", "deltas": deltas }
            }
        })
    }

    #[test]
    fn snapshot_frame_clears_accumulation() {
        let s = store();
        // 先喂一点流式文本。
        s.feed(
            "s1",
            &wire_deltas(
                "s1",
                serde_json::json!([{ "op": "row.delta", "rowId": 7, "path": "text", "append": "旧" }]),
            ),
        );
        assert_eq!(s.text_of("s1", 7).as_deref(), Some("旧"));

        let outcome = s.feed("s1", &wire_snapshot());
        assert_eq!(outcome, FrameOutcome::Snapshot);
        assert!(!s.has_streaming("s1"), "snapshot 应清空累积");
    }

    #[test]
    fn text_deltas_concatenate_in_order() {
        let s = store();
        for piece in ["你", "好", "，", "世界"] {
            s.feed(
                "s1",
                &wire_deltas(
                    "s1",
                    serde_json::json!([{ "op": "row.delta", "rowId": 42, "path": "text", "append": piece }]),
                ),
            );
        }
        assert_eq!(s.text_of("s1", 42).as_deref(), Some("你好，世界"));
        assert!(s.has_streaming("s1"));
    }

    #[test]
    fn text_deltas_outcome_lists_dirty_rows() {
        let s = store();
        let outcome = s.feed(
            "s1",
            &wire_deltas(
                "s1",
                serde_json::json!([
                    { "op": "row.delta", "rowId": 1, "path": "text", "append": "a" },
                    { "op": "row.delta", "rowId": 2, "path": "text", "append": "b" }
                ]),
            ),
        );
        let mut rows = match outcome {
            FrameOutcome::TextDeltas { rows } => rows,
            other => panic!("期望 TextDeltas，实际 {other:?}"),
        };
        rows.sort_unstable();
        assert_eq!(rows, vec![1, 2]);
        assert_eq!(s.take_dirty("s1"), vec![1, 2]);
    }

    #[test]
    fn upsert_clears_only_that_row() {
        let s = store();
        s.feed(
            "s1",
            &wire_deltas(
                "s1",
                serde_json::json!([
                    { "op": "row.delta", "rowId": 1, "path": "text", "append": "A" },
                    { "op": "row.delta", "rowId": 2, "path": "text", "append": "B" }
                ]),
            ),
        );
        let outcome = s.feed(
            "s1",
            &wire_deltas(
                "s1",
                serde_json::json!([{ "op": "row.upserted", "row": { "rowId": 1, "text": "权威" } }]),
            ),
        );
        assert_eq!(outcome, FrameOutcome::Structural);
        assert_eq!(s.text_of("s1", 1), None, "upsert 后该行不再用流式文本");
        assert_eq!(s.text_of("s1", 2).as_deref(), Some("B"), "其他行不受影响");
    }

    #[test]
    fn removed_clears_all_for_session() {
        let s = store();
        s.feed(
            "s1",
            &wire_deltas(
                "s1",
                serde_json::json!([{ "op": "row.delta", "rowId": 1, "path": "text", "append": "A" }]),
            ),
        );
        s.feed(
            "s1",
            &wire_deltas("s1", serde_json::json!([{ "op": "row.removed", "fromRowId": 1 }])),
        );
        assert!(!s.has_streaming("s1"), "removed 应清空该会话累积");
    }

    #[test]
    fn path_switch_resets_text() {
        let s = store();
        s.feed(
            "s1",
            &wire_deltas(
                "s1",
                serde_json::json!([{ "op": "row.delta", "rowId": 5, "path": "text", "append": "AAA" }]),
            ),
        );
        s.feed(
            "s1",
            &wire_deltas(
                "s1",
                serde_json::json!([{ "op": "row.delta", "rowId": 5, "path": "reasoning", "append": "B" }]),
            ),
        );
        assert_eq!(s.text_of("s1", 5).as_deref(), Some("B"));
    }

    #[test]
    fn frames_of_other_sessions_are_ignored() {
        let s = store();
        // 同一个连接上可能同时订阅多个会话，topic 不匹配就不该累积。
        let outcome = s.feed(
            "s1",
            &wire_deltas(
                "s-other",
                serde_json::json!([{ "op": "row.delta", "rowId": 1, "path": "text", "append": "X" }]),
            ),
        );
        assert_eq!(outcome, FrameOutcome::Ignored);
        assert!(!s.has_streaming("s1"));
    }

    #[test]
    fn fragment_requires_resync() {
        let s = store();
        let fragment = serde_json::json!({
            "kind": "fragment",
            "topic": "conversation/s1",
            "subscriptionId": "sub-1",
            "logicalFrameId": "sub-1-lf-9",
            "logicalFrameOrdinal": 9,
            "wireVersion": 3,
            "fragmentIndex": 0,
            "fragmentCount": 4
        });
        assert_eq!(s.feed("s1", &fragment), FrameOutcome::NeedsResync);
    }

    #[test]
    fn sessions_are_isolated() {
        let s = store();
        s.feed(
            "s1",
            &wire_deltas(
                "s1",
                serde_json::json!([{ "op": "row.delta", "rowId": 1, "path": "text", "append": "A" }]),
            ),
        );
        s.feed(
            "s2",
            &wire_deltas(
                "s2",
                serde_json::json!([{ "op": "row.delta", "rowId": 1, "path": "text", "append": "B" }]),
            ),
        );
        assert_eq!(s.text_of("s1", 1).as_deref(), Some("A"));
        assert_eq!(s.text_of("s2", 1).as_deref(), Some("B"));
        s.reset("s1");
        assert!(!s.has_streaming("s1"));
        assert!(s.has_streaming("s2"), "reset 只影响指定会话");
    }

    #[test]
    fn malformed_frame_is_ignored_without_panic() {
        let s = store();
        for bad in [
            serde_json::json!({}),
            serde_json::json!({ "kind": "complete" }),
            serde_json::json!({ "kind": "complete", "frame": {} }),
            serde_json::json!({ "kind": "complete", "frame": { "topic": "conversation/s1" } }),
        ] {
            let outcome = s.feed("s1", &bad);
            assert!(
                matches!(outcome, FrameOutcome::Ignored),
                "坏帧应被忽略而非崩溃：{bad}"
            );
        }
    }

    #[test]
    fn reset_row_clears_single_row() {
        let s = store();
        s.feed(
            "s1",
            &wire_deltas(
                "s1",
                serde_json::json!([
                    { "op": "row.delta", "rowId": 1, "path": "text", "append": "A" },
                    { "op": "row.delta", "rowId": 2, "path": "text", "append": "B" }
                ]),
            ),
        );
        s.reset_row("s1", 1);
        assert_eq!(s.text_of("s1", 1), None);
        assert_eq!(s.text_of("s1", 2).as_deref(), Some("B"));
    }
}