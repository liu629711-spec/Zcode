//! 工具调用状态映射链（1:1 翻译三层）。
//!
//! 三级串联（真源）：
//! ```text
//! v4 row.status (6 值)
//!   → legacy status (5 值)      toolCallRowAdapter.ts:12-19
//!   → ai-elements state (5 值)  mapToolStatus.ts:9-16
//!   → 中文状态词                 tool-call-summary.ts:197-205 + zh-CN.ts:4819-4824
//! ```
//!
//! **关键：状态不靠颜色区分，靠文案 + kindLabel 扫光**
//! （`ToolLayout.tsx:138-147`）。运行态 kindLabel 加 `animated-gradient-text`
//! 4s 循环扫光，**图标不旋转**（真源注释：流式期间旋转图标会长期占用渲染资源）。
//!
//! **interrupted 与 failed 视觉完全相同**——`mapToolStatus` 把 `stopped`
//! 也映射成 `output-error`，只有状态词文案不同（已停止 vs 执行失败）。
//! 所以 Rust 侧不需要单独的 interrupted 视觉分支。

use serde::{Deserialize, Serialize};

/// v4 行的工具状态（真源 `toolCallRowSchema.status`，rows.ts:214）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RowStatus {
    InputStreaming,
    PendingApproval,
    Running,
    Success,
    Error,
    Cancelled,
    /// 真源 zod 枚举外的值（宽容兜底，Rust 侧不 deny_unknown_fields 的结果）。
    #[serde(other)]
    Unknown,
}

impl RowStatus {
    pub fn parse(s: &str) -> Self {
        match s {
            "inputStreaming" => Self::InputStreaming,
            "pendingApproval" => Self::PendingApproval,
            "running" => Self::Running,
            "success" => Self::Success,
            "error" => Self::Error,
            "cancelled" => Self::Cancelled,
            _ => Self::Unknown,
        }
    }
}

/// 中间态（真源 legacy `ChatToolCall.status`，5 值词表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Stopped,
}

impl LegacyStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "stopped" => Self::Stopped,
            // 真源 statusMap 无匹配时回 input-streaming（mapToolStatus.ts:18）。
            _ => Self::Pending,
        }
    }
}

/// 展示态（真源 ai-elements `ToolPart["state"]`，5 值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayState {
    InputStreaming,
    InputAvailable,
    OutputAvailable,
    OutputError,
    OutputDenied,
}

impl DisplayState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InputStreaming => "input-streaming",
            Self::InputAvailable => "input-available",
            Self::OutputAvailable => "output-available",
            Self::OutputError => "output-error",
            Self::OutputDenied => "output-denied",
        }
    }

    /// 运行中（真源 `TOOL_CALL_RUNNING_STATES`，tool-call-summary.ts:27-30）。
    pub fn is_running(self) -> bool {
        matches!(self, Self::InputStreaming | Self::InputAvailable)
    }

    /// 已终结（真源 `TOOL_CALL_FINISHED_STATES`，:32-36）。
    pub fn is_finished(self) -> bool {
        matches!(
            self,
            Self::OutputAvailable | Self::OutputError | Self::OutputDenied
        )
    }

    /// 是否为失败态（含被中断——真源把 stopped 也映射到 output-error）。
    pub fn is_failure(self) -> bool {
        matches!(self, Self::OutputError)
    }
}

/// 第一级：v4 row.status → legacy（真源 toolCallRowAdapter.ts:12-19）。
///
/// `pendingApproval` 视为 pending——审批中输入已定，展示为待执行。
pub fn row_to_legacy(status: RowStatus) -> LegacyStatus {
    match status {
        RowStatus::InputStreaming | RowStatus::PendingApproval => LegacyStatus::Pending,
        RowStatus::Running => LegacyStatus::InProgress,
        RowStatus::Success => LegacyStatus::Completed,
        RowStatus::Error => LegacyStatus::Failed,
        RowStatus::Cancelled => LegacyStatus::Stopped,
        // 真源 statusMap 无此 key → mapToolStatus 回 input-streaming → pending。
        RowStatus::Unknown => LegacyStatus::Pending,
    }
}

/// 第二级：legacy → 展示态（真源 mapToolStatus.ts:9-16）。
///
/// 注意 `stopped` 与 `failed` **都映射到 output-error**——这就是
/// interrupted 与 failed 视觉相同的原因（只有文案不同）。
pub fn legacy_to_display(status: LegacyStatus) -> DisplayState {
    match status {
        LegacyStatus::Pending => DisplayState::InputStreaming,
        LegacyStatus::InProgress => DisplayState::InputAvailable,
        LegacyStatus::Completed => DisplayState::OutputAvailable,
        LegacyStatus::Failed | LegacyStatus::Stopped => DisplayState::OutputError,
    }
}

/// 第三级：状态词文案（真源 `getCompactToolCallStatusMessageId`，:197-205）。
///
/// 特例：`rawStatus == "stopped"` → "已停止"，**先于** state 查表判定。
pub fn status_label(state: DisplayState, raw_status: LegacyStatus) -> &'static str {
    if raw_status == LegacyStatus::Stopped {
        return "已停止";
    }
    match state {
        DisplayState::InputStreaming => "等待中",
        DisplayState::InputAvailable => "执行中",
        DisplayState::OutputAvailable => "已执行",
        DisplayState::OutputError => "执行失败",
        DisplayState::OutputDenied => "已拒绝",
    }
}

/// 一步到位：v4 row.status → 展示态 + 状态词。
pub fn resolve_status(row_status: RowStatus) -> (DisplayState, &'static str) {
    let legacy = row_to_legacy(row_status);
    let display = legacy_to_display(legacy);
    (display, status_label(display, legacy))
}

// ---------------------------------------------------------------------------
// kindLabel：运行态文案 + 扫光
// ---------------------------------------------------------------------------

/// kindLabel 的类名（真源 ToolLayout.tsx:144-147）。
///
/// 运行态加 `animated-gradient-text`（4s 扫光，styles.css:1068-1087）表达"仍在进行"；
/// 非运行态用最浅文本色 `text-foreground-subtlest`，避免摘要喧宾夺主。
/// **图标保持静态**——真源刻意不用旋转 loading。
pub fn kind_label_class(is_running: bool) -> &'static str {
    if is_running {
        "font-medium whitespace-nowrap shrink-0 animated-gradient-text"
    } else {
        "font-medium whitespace-nowrap shrink-0 text-foreground-subtlest"
    }
}

/// 工具 kind 的中文标签（真源 zh-CN.ts:4838-4843 + 各卡片的 running 文案）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKindLabel {
    Read,
    Write,
    Edit,
    Terminal,
    Search,
    /// 无专用 kind 标签（fallback / mcp 等）。
    None,
}

/// kindLabel 的文案（真源各 renderer 的 `kindLabel` prop）。
///
/// 运行中用进行时文案，完成态用名词（真源：read 卡 `read.reading` vs `kind.read`，
/// execute 卡 `execute.running` vs `kind.terminal`）。
pub fn kind_label_text(kind: ToolKindLabel, is_running: bool) -> &'static str {
    match (kind, is_running) {
        (ToolKindLabel::Read, true) => "正在读取",
        (ToolKindLabel::Read, false) => "读取",
        (ToolKindLabel::Write, true) => "正在写入",
        (ToolKindLabel::Write, false) => "写入",
        (ToolKindLabel::Edit, true) => "正在编辑",
        (ToolKindLabel::Edit, false) => "编辑",
        (ToolKindLabel::Terminal, true) => "正在执行",
        (ToolKindLabel::Terminal, false) => "终端",
        (ToolKindLabel::Search, true) => "正在搜索",
        (ToolKindLabel::Search, false) => "搜索",
        (ToolKindLabel::None, _) => "",
    }
}

/// 是否应强制显示状态词（真源 ToolLayout.tsx:119）。
///
/// `showFailureStatus` 为真时失败态**强制显示**状态词——失败信息不该被折叠隐藏。
/// 成功态默认不显示（`showStatusLabel` 由各卡片按需开启）。
pub fn should_show_status_label(
    state: DisplayState,
    show_status_label: bool,
    show_failure_status: bool,
    has_status_label: bool,
) -> bool {
    if !has_status_label {
        return false;
    }
    // 失败态强制显示（真源：(showStatusLabel || showFailureStatus) && statusLabel != null）。
    if state.is_failure() && show_failure_status {
        return true;
    }
    show_status_label
}

/// 失败态状态词的类名（真源 ToolLayout.tsx:248-289）。
///
/// 失败时加虚线下划线 + `cursor-help`，hover 出 tooltip 装错误详情（可复制）。
pub fn status_label_class(state: DisplayState, has_tooltip: bool) -> &'static str {
    if state.is_failure() && has_tooltip {
        "whitespace-nowrap underline decoration-dotted underline-offset-2 cursor-help"
    } else {
        "whitespace-nowrap"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_status_to_legacy_matches_source_map() {
        // 真源 toolCallRowAdapter.ts:12-19 逐条对齐。
        let cases = [
            (RowStatus::InputStreaming, LegacyStatus::Pending),
            (RowStatus::PendingApproval, LegacyStatus::Pending),
            (RowStatus::Running, LegacyStatus::InProgress),
            (RowStatus::Success, LegacyStatus::Completed),
            (RowStatus::Error, LegacyStatus::Failed),
            (RowStatus::Cancelled, LegacyStatus::Stopped),
        ];
        for (row, legacy) in cases {
            assert_eq!(row_to_legacy(row), legacy, "{row:?} 的 legacy 应为 {legacy:?}");
        }
    }

    #[test]
    fn pending_approval_is_treated_as_pending() {
        // 真源注释：pendingApproval 视为 pending——审批中输入已定。
        assert_eq!(
            row_to_legacy(RowStatus::PendingApproval),
            row_to_legacy(RowStatus::InputStreaming)
        );
    }

    #[test]
    fn unknown_row_status_falls_back_to_pending() {
        // 真源 statusMap 无匹配 key 时回 input-streaming（= pending）。
        assert_eq!(row_to_legacy(RowStatus::Unknown), LegacyStatus::Pending);
        let (_, label) = resolve_status(RowStatus::Unknown);
        assert_eq!(label, "等待中");
    }

    #[test]
    fn stopped_and_failed_share_the_same_visual_state() {
        // ★mapToolStatus 把 stopped 也映射成 output-error——这是
        // interrupted 与 failed 视觉完全相同的原因，只有文案不同。
        let stopped = legacy_to_display(LegacyStatus::Stopped);
        let failed = legacy_to_display(LegacyStatus::Failed);
        assert_eq!(stopped, failed);
        assert_eq!(stopped, DisplayState::OutputError);
        assert!(stopped.is_failure(), "两者都算失败态");
        // 文案不同。
        assert_eq!(status_label(stopped, LegacyStatus::Stopped), "已停止");
        assert_eq!(status_label(failed, LegacyStatus::Failed), "执行失败");
    }

    #[test]
    fn display_state_labels_match_zh_cn() {
        // zh-CN.ts:4819-4824
        assert_eq!(
            status_label(DisplayState::InputStreaming, LegacyStatus::Pending),
            "等待中"
        );
        assert_eq!(
            status_label(DisplayState::InputAvailable, LegacyStatus::InProgress),
            "执行中"
        );
        assert_eq!(
            status_label(DisplayState::OutputAvailable, LegacyStatus::Completed),
            "已执行"
        );
        assert_eq!(
            status_label(DisplayState::OutputError, LegacyStatus::Failed),
            "执行失败"
        );
        assert_eq!(
            status_label(DisplayState::OutputDenied, LegacyStatus::Pending),
            "已拒绝"
        );
    }

    #[test]
    fn running_states_match_source_set() {
        // 真源 TOOL_CALL_RUNNING_STATES = { input-streaming, input-available }
        assert!(DisplayState::InputStreaming.is_running());
        assert!(DisplayState::InputAvailable.is_running());
        assert!(!DisplayState::OutputAvailable.is_running());
        assert!(!DisplayState::OutputError.is_running());
        assert!(!DisplayState::OutputDenied.is_running());
    }

    #[test]
    fn finished_states_match_source_set() {
        // 真源 TOOL_CALL_FINISHED_STATES = { output-available, output-error, output-denied }
        assert!(!DisplayState::InputStreaming.is_finished());
        assert!(DisplayState::OutputAvailable.is_finished());
        assert!(DisplayState::OutputError.is_finished());
        assert!(DisplayState::OutputDenied.is_finished());
    }

    #[test]
    fn full_chain_end_to_end() {
        // v4 原始字符串 → 中文状态词，验证三级串联。
        assert_eq!(
            resolve_status(RowStatus::parse("running")),
            (DisplayState::InputAvailable, "执行中")
        );
        assert_eq!(
            resolve_status(RowStatus::parse("success")),
            (DisplayState::OutputAvailable, "已执行")
        );
        assert_eq!(
            resolve_status(RowStatus::parse("error")),
            (DisplayState::OutputError, "执行失败")
        );
        assert_eq!(
            resolve_status(RowStatus::parse("cancelled")),
            (DisplayState::OutputError, "已停止")
        );
        assert_eq!(
            resolve_status(RowStatus::parse("inputStreaming")),
            (DisplayState::InputStreaming, "等待中")
        );
        assert_eq!(
            resolve_status(RowStatus::parse("pendingApproval")),
            (DisplayState::InputStreaming, "等待中")
        );
    }

    #[test]
    fn kind_label_switches_between_gradient_and_muted() {
        // 运行态扫光、非运行态最浅色（真源 ToolLayout.tsx:144-147）。
        assert!(kind_label_class(true).contains("animated-gradient-text"));
        assert!(!kind_label_class(false).contains("animated-gradient-text"));
        assert!(kind_label_class(false).contains("text-foreground-subtlest"));
        // 两者共有的基础类名。
        for c in [kind_label_class(true), kind_label_class(false)] {
            assert!(c.contains("font-medium"));
            assert!(c.contains("whitespace-nowrap"));
            assert!(c.contains("shrink-0"));
        }
    }

    #[test]
    fn kind_label_text_uses_progressive_when_running() {
        // read 卡：running="正在读取"，完成="读取"（zh-CN.ts:5620 / 4838）。
        assert_eq!(kind_label_text(ToolKindLabel::Read, true), "正在读取");
        assert_eq!(kind_label_text(ToolKindLabel::Read, false), "读取");
        assert_eq!(kind_label_text(ToolKindLabel::Terminal, true), "正在执行");
        assert_eq!(kind_label_text(ToolKindLabel::Terminal, false), "终端");
        assert_eq!(kind_label_text(ToolKindLabel::Edit, true), "正在编辑");
        assert_eq!(kind_label_text(ToolKindLabel::Edit, false), "编辑");
        assert_eq!(kind_label_text(ToolKindLabel::None, false), "");
    }

    #[test]
    fn failure_forces_status_label_visible() {
        // 真源 ToolLayout.tsx:119 —— 失败态强制显示状态词，不该被折叠隐藏。
        assert!(should_show_status_label(
            DisplayState::OutputError,
            false,
            true,
            true
        ));
        // 无状态词时不显示。
        assert!(!should_show_status_label(
            DisplayState::OutputError,
            true,
            true,
            false
        ));
        // 成功态默认不显示（各卡片按需开）。
        assert!(!should_show_status_label(
            DisplayState::OutputAvailable,
            false,
            false,
            true
        ));
        assert!(should_show_status_label(
            DisplayState::OutputAvailable,
            true,
            false,
            true
        ));
    }

    #[test]
    fn failure_status_gets_dotted_underline_tooltip_style() {
        // 真源 ToolLayout.tsx:248-289 —— 失败 + 有 tooltip 才加虚线下划线。
        let with_tooltip = status_label_class(DisplayState::OutputError, true);
        assert!(with_tooltip.contains("decoration-dotted"));
        assert!(with_tooltip.contains("cursor-help"));
        // 无 tooltip 时只有基础类名。
        assert_eq!(
            status_label_class(DisplayState::OutputError, false),
            "whitespace-nowrap"
        );
        // 成功态不加装饰。
        assert_eq!(
            status_label_class(DisplayState::OutputAvailable, true),
            "whitespace-nowrap"
        );
    }

    #[test]
    fn legacy_status_parse_defaults_to_pending() {
        // 真源 mapToolStatus 对未知 status 回 input-streaming（= pending）。
        assert_eq!(LegacyStatus::parse("unknown"), LegacyStatus::Pending);
        assert_eq!(LegacyStatus::parse(""), LegacyStatus::Pending);
        assert_eq!(LegacyStatus::parse("in_progress"), LegacyStatus::InProgress);
    }
}