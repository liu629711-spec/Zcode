//! 1:1 翻译 `packages/ui/src/lib/mapToolStatus.ts`（20 行）+
//! `packages/shared/src/tool-call-summary.ts` 的状态判定部分（:60-134）。
//!
//! ToolCallBlock 的状态词与运行态判定都从这里取，保证与真源同一套词表。


/// ai-elements Tool 组件期望的状态（真源 `ToolPart["state"]`，mapToolStatus.ts:8-15）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompactToolCallState {
    InputAvailable,
    InputStreaming,
    OutputAvailable,
    OutputDenied,
    OutputError,
}

impl CompactToolCallState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InputAvailable => "input-available",
            Self::InputStreaming => "input-streaming",
            Self::OutputAvailable => "output-available",
            Self::OutputDenied => "output-denied",
            Self::OutputError => "output-error",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "input-available" => Self::InputAvailable,
            "input-streaming" => Self::InputStreaming,
            "output-available" => Self::OutputAvailable,
            "output-denied" => Self::OutputDenied,
            "output-error" => Self::OutputError,
            _ => return None,
        })
    }

    /// 状态词的 i18n id（真源 TOOL_CALL_STATUS_MESSAGE_IDS，:38-44）。
    ///
    /// Rust 侧没有 i18n 框架，直接返回中文文案——与 zh-CN.ts 对齐：
    /// pending「等待中」/ running「执行中」/ completed「已完成」/
    /// failed「执行失败」/ denied「已拒绝」。
    pub fn status_label_zh(self) -> &'static str {
        match self {
            Self::InputStreaming => "等待中",
            Self::InputAvailable => "执行中",
            Self::OutputAvailable => "已完成",
            Self::OutputError => "执行失败",
            Self::OutputDenied => "已拒绝",
        }
    }
}

/// 旧 ChatToolCall.status → ai-elements 状态（mapToolStatus.ts:9-16）。
///
/// 注意 `denied`：真源注释「ZCode schema 定义里没这个字段」——防御性保留。
pub fn map_tool_status(status: &str) -> CompactToolCallState {
    match status {
        "pending" => CompactToolCallState::InputStreaming,
        "in_progress" => CompactToolCallState::InputAvailable,
        "completed" => CompactToolCallState::OutputAvailable,
        "failed" | "stopped" | "denied" => CompactToolCallState::OutputError,
        _ => CompactToolCallState::InputStreaming,
    }
}

/// 运行态集合（真源 TOOL_CALL_RUNNING_STATES，:30-33）。
pub fn is_compact_tool_call_running_state(state: &str) -> bool {
    state == "input-streaming" || state == "input-available"
}

/// 终态集合（真源 TOOL_CALL_FINISHED_STATES，:34-38）。
pub fn is_compact_tool_call_finished_state(state: &str) -> bool {
    matches!(state, "output-available" | "output-error" | "output-denied")
}

/// 状态词 id → 中文（真源 getCompactToolCallStatusMessageId，:115-125）。
///
/// rawStatus == "stopped" 优先（映射后是 output-error，但展示要用「已停止」）。
pub fn compact_tool_call_status_label(state: &str, raw_status: Option<&str>) -> &'static str {
    if raw_status == Some("stopped") {
        return "已停止";
    }
    CompactToolCallState::parse(state)
        .map(|s| s.status_label_zh())
        .unwrap_or("等待中")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_tool_status_matches_source_wordlist() {
        // 真源 statusMap（mapToolStatus.ts:9-15）。
        assert_eq!(
            map_tool_status("pending"),
            CompactToolCallState::InputStreaming
        );
        assert_eq!(
            map_tool_status("in_progress"),
            CompactToolCallState::InputAvailable
        );
        assert_eq!(
            map_tool_status("completed"),
            CompactToolCallState::OutputAvailable
        );
        assert_eq!(map_tool_status("failed"), CompactToolCallState::OutputError);
        // stopped 与 denied 同落 output-error（真源同表）。
        assert_eq!(
            map_tool_status("stopped"),
            CompactToolCallState::OutputError
        );
        assert_eq!(map_tool_status("denied"), CompactToolCallState::OutputError);
        // 未知值兜底 input-streaming（真源 ?? "input-streaming"）。
        assert_eq!(
            map_tool_status("bogus"),
            CompactToolCallState::InputStreaming
        );
    }

    #[test]
    fn running_and_finished_states_partition() {
        // 真源 :30-38：运行态与终态互斥并覆盖全部五态。
        for s in ["input-streaming", "input-available"] {
            assert!(is_compact_tool_call_running_state(s));
            assert!(!is_compact_tool_call_finished_state(s));
        }
        for s in ["output-available", "output-error", "output-denied"] {
            assert!(!is_compact_tool_call_running_state(s));
            assert!(is_compact_tool_call_finished_state(s));
        }
    }

    #[test]
    fn status_label_prefers_stopped() {
        // 真源 :116-119 —— rawStatus == "stopped" 抢占。
        assert_eq!(
            compact_tool_call_status_label("output-error", Some("stopped")),
            "已停止"
        );
        assert_eq!(
            compact_tool_call_status_label("output-error", None),
            "执行失败"
        );
        assert_eq!(
            compact_tool_call_status_label("output-available", None),
            "已完成"
        );
        assert_eq!(compact_tool_call_status_label("bogus", None), "等待中");
    }

    #[test]
    fn running_states_set_is_stable() {
        // 真源 TOOL_CALL_RUNNING_STATES（tool-call-summary.ts:30-33）只有这两个。
        assert!(is_compact_tool_call_running_state("input-streaming"));
        assert!(is_compact_tool_call_running_state("input-available"));
        // 其余状态都不算运行中。
        for other in ["output-available", "output-error", "unknown"] {
            assert!(
                !is_compact_tool_call_running_state(other),
                "{other} 不应被判为运行中"
            );
        }
    }
}
