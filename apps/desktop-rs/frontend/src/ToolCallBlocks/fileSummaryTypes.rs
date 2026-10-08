//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/fileSummaryTypes.ts`（321 行）。
//!
//! 本文件是**纯类型定义**（真源是 TS interface/type，无运行时逻辑），
//! 加上少量配套的解析函数（:34-133 的 diff/patch 读取helpers）。
//!
//! 行号注释均指真源文件。

use serde_json::Value;

// ---------------------------------------------------------------------------
// 编辑操作类型
// ---------------------------------------------------------------------------

/// `EditOperationKind`（真源 :313）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditOperationKind {
    Write,
    Edit,
    Update,
    Delete,
}

impl EditOperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::Edit => "edit",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "write" => Some(Self::Write),
            "edit" => Some(Self::Edit),
            "update" => Some(Self::Update),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }
}

/// `EditKindLabelId`（真源 :314-321）——i18n key，不是文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKindLabelId {
    /// chat.toolCall.edit.writing
    Writing,
    /// chat.toolCall.edit.updating
    Updating,
    /// chat.toolCall.edit.deleting
    Deleting,
    /// chat.toolCall.edit.editing
    Editing,
    /// chat.toolCall.kind.write
    KindWrite,
    /// chat.toolCall.kind.delete
    KindDelete,
    /// chat.toolCall.kind.edit
    KindEdit,
}

impl EditKindLabelId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Writing => "chat.toolCall.edit.writing",
            Self::Updating => "chat.toolCall.edit.updating",
            Self::Deleting => "chat.toolCall.edit.deleting",
            Self::Editing => "chat.toolCall.edit.editing",
            Self::KindWrite => "chat.toolCall.kind.write",
            Self::KindDelete => "chat.toolCall.kind.delete",
            Self::KindEdit => "chat.toolCall.kind.edit",
        }
    }

    /// 运行态的 id（真源 renderers.tsx:136-148 按isRunning 三元选择）。
    pub fn is_running_variant(self) -> bool {
        matches!(
            self,
            Self::Writing | Self::Updating | Self::Deleting | Self::Editing
        )
    }
}

/// 文件动作标签（真源 :137 `actionLabel: "Created" | "Edited" | "Deleted"`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileActionLabel {
    Created,
    Edited,
    Deleted,
}

impl FileActionLabel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::Edited => "Edited",
            Self::Deleted => "Deleted",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Created" => Some(Self::Created),
            "Edited" => Some(Self::Edited),
            "Deleted" => Some(Self::Deleted),
            _ => None,
        }
    }

    /// 动作 → 操作类型（`inferEditOperation` 的基础映射）。
    pub fn to_operation(self) -> EditOperationKind {
        match self {
            Self::Created => EditOperationKind::Write,
            Self::Edited => EditOperationKind::Edit,
            Self::Deleted => EditOperationKind::Delete,
        }
    }
}

/// 变更统计（真源 :140 `changeStat?: { added, removed }`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChangeStat {
    pub added: u32,
    pub removed: u32,
}

// ---------------------------------------------------------------------------
// RawToolCallFileSummary（真源 :135-144）
// ---------------------------------------------------------------------------

/// 原始工具调用的文件摘要（真源 `RawToolCallFileSummary`，:135-144）。
///
/// 抽取优先级见 `fileSummaries.ts:103-231`（四级），本struct 只承载结果。
#[derive(Debug, Clone, PartialEq)]
pub struct RawToolCallFileSummary {
    /// 绝对路径（:136）。
    pub path: String,
    /// 动作标签（:137）。
    pub action_label: String,
    /// 操作类型（:138）。
    pub operation_kind: String,
    /// 文件名（:139）。
    pub file_name: String,
    /// 展示用目录路径（:140，fileDisplay 生成；无则为 null）。
    pub file_path: Option<String>,
    /// 图标 src（:141）。
    pub file_icon_src: String,
    /// 增删统计（:142，可选）。
    pub change_stat: Option<ChangeStat>,
    /// unified diff 原文（:143，可选）。
    pub patch: Option<String>,
}

impl RawToolCallFileSummary {
    /// 操作类型的强类型视图（从字符串解析；未知回 `Edit`）。
    pub fn operation(&self) -> EditOperationKind {
        EditOperationKind::parse(&self.operation_kind).unwrap_or(EditOperationKind::Edit)
    }

    /// 动作标签的强类型视图（未知回 `Edited`）。
    pub fn action(&self) -> FileActionLabel {
        FileActionLabel::parse(&self.action_label).unwrap_or(FileActionLabel::Edited)
    }
}

// ---------------------------------------------------------------------------
// EditKindSource（真源 :304-311）
// ---------------------------------------------------------------------------

/// 编辑类型判定的补充来源（真源 `EditKindSource`，:304-311）。
///
/// `inferEditOperation` 在 operationKinds 不足以判定时，用这里的工具名/标题兜底。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EditKindSource {
    pub tool_name: Option<String>,
    pub kind: Option<String>,
    pub title: Option<String>,
    pub input: Option<Value>,
    pub output: Option<Value>,
    pub raw: Option<Value>,
}

// ---------------------------------------------------------------------------
// WorkflowRunCardSummary（真源 :151-190）
// ---------------------------------------------------------------------------

/// 工作流 run 卡片的**入口与摘要**事实（真源 :151-190）。
///
/// 真源注释强调：刻意只有这几个字段——卡片承担入口与摘要，
/// 预算/事件日志/结果/失败面板都归详情页。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunCardSummary {
    /// run id（:158）。
    pub run_id: String,
    /// 发起这个 run 的 **CreateWorkflow 行** id（:164）——打开侧栏 run 视图必须带它。
    /// resume 行自己的 toolCallId **不是**这个值。
    pub tool_call_id: Option<String>,
    /// 状态（:160）。
    pub status: String,
    /// `stopped` 的原因（:161，投影带才带）。
    pub stop_reason: Option<String>,
    /// 已结算（`phase === "settled"`）的节点数，**含**撞界后没进表的（:163）。
    pub nodes_settled: u32,
    /// 已排程（observed）节点数，**不是**全程总数（:165-172）。
    ///
    /// 动态工作流的节点数由脚本运行时决定，静态总数不存在。
    /// 所以进度读作「已排程的里结算了几个」，绝不冒充完成百分比。
    pub nodes_total: u32,
    /// 子代理数（:175）。
    pub agents: Option<u32>,
    /// 活投影里的这条 run（:179）——卡片据此建时间线模型。
    /// journal 兜底命中时缺席，那时只有状态词，时间线画静态的。
    pub run: Option<Value>,
    /// 可恢复（:183）——Resume 按钮只在它在场时渲染，
    /// UI 绝不自行按 status + failureCode 推导。
    pub resumable: bool,
}

impl WorkflowRunCardSummary {
    /// 进度比（已排程里的结算数/ 已排程总数）。
    ///
    /// 真源注释：**不是完成百分比**——总数会随脚本运行时增长。
    /// 分母为 0 时给 0，避免除零。
    pub fn progress_ratio(&self) -> f32 {
        if self.nodes_total == 0 {
            return 0.0;
        }
        self.nodes_settled as f32 / self.nodes_total as f32
    }
}

/// 一条 CreateWorkflow / AmendWorkflow 行在编译反馈循环里的位置
/// （真源 `WorkflowDraftPosition`，:196-201）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowDraftPosition {
    /// 第几稿（:199）：同谱系、同一轮里自上次编过以来的第几次提交，从 1 起。
    pub ordinal: u32,
    /// 同谱系后面还有一行（:200）：反馈行的空环灯从警示色褪成中性。
    pub superseded: bool,
}

// ---------------------------------------------------------------------------
// diff 读取 helpers（真源 :34-133）
// ---------------------------------------------------------------------------

/// `isPlainRecord`（真源多处复用）：非 null 的非数组对象。
pub fn is_plain_record(value: &Value) -> bool {
    value.is_object()
}

/// `readChangesSources`（真源 :120-133）：从三处读 `changes` 字段。
///
/// 真源注释：`changes` 可能落在 `raw.changes` / `raw.rawInput.changes` /
/// `raw.rawOutput.changes` 三处，三处都要读。
pub struct ChangesSources {
    pub direct: Option<Value>,
    pub raw_input: Option<Value>,
    pub raw_output: Option<Value>,
}

/// 从 raw 载荷里读三处 changes（真源 :120-133）。
pub fn read_changes_sources(raw: &Value) -> ChangesSources {
    let raw_input = raw.get("rawInput").filter(|v| is_plain_record(v));
    let raw_output = raw.get("rawOutput").filter(|v| is_plain_record(v));
    ChangesSources {
        direct: raw.get("changes").filter(|v| is_plain_record(v)).cloned(),
        raw_input: raw_input
            .and_then(|r| r.get("changes"))
            .filter(|v| is_plain_record(v))
            .cloned(),
        raw_output: raw_output
            .and_then(|r| r.get("changes"))
            .filter(|v| is_plain_record(v))
            .cloned(),
    }
}

/// `normalizeSingleFilePatch`（真源 :60-64 一带）：单文件 patch 可能是字符串或
/// `{ patch: string }` 对象，统一取出字符串。
pub fn normalize_single_file_patch(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    value
        .get("patch")
        .and_then(|p| p.as_str())
        .map(|s| s.to_string())
}

/// `computeLineChangeStat`（真源 `packages/shared/src/lineChangeStat.ts:20`）：
/// 从新旧文本算增删行数。Rust 侧按行diff 语义统计——
/// 以新文本为准逐行比对，对齐常见的 LCS 之外的简单策略：
/// 相同行不算变更，新增行计 added，减少行计 removed。
///
/// 真源该函数用 LCS 动态规划（`computeLineChangeStat`），
/// 精确度要求高时需照抄 LCS 算法；此处先给出行级近似。
pub fn compute_line_change_stat(old_text: &str, new_text: &str) -> ChangeStat {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();
    // 简易 LCS 表（O(n*m)），行数大时换真源算法。
    let n = old_lines.len();
    let m = new_lines.len();
    if n.saturating_mul(m) > 4_000_000 {
        // 表太大时退化为「全删全增」，避免卡死。
        return ChangeStat {
            added: m as u32,
            removed: n as u32,
        };
    }
    let mut dp = vec![0u32; (n + 1) * (m + 1)];
    for i in 0..n {
        for j in 0..m {
            dp[(i + 1) * (m + 1) + j + 1] = if old_lines[i] == new_lines[j] {
                dp[i * (m + 1) + j] + 1
            } else {
                dp[i * (m + 1) + j + 1].max(dp[(i + 1) * (m + 1) + j])
            };
        }
    }
    let lcs = dp[n * (m + 1) + m] as usize;
    let added = m - lcs;
    let removed = n - lcs;
    ChangeStat {
        added: added as u32,
        removed: removed as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn edit_operation_kinds_parse_and_render() {
        // 真源 :313
        for k in ["write", "edit", "update", "delete"] {
            assert_eq!(EditOperationKind::parse(k).unwrap().as_str(), k);
        }
        assert_eq!(EditOperationKind::parse("nope"), None);
        // 未知回退由 RawToolCallFileSummary::operation 处理。
    }

    #[test]
    fn file_action_labels_map_to_operations() {
        // 真源 :137 + inferEditOperation 的基础映射
        assert_eq!(FileActionLabel::Created.to_operation(), EditOperationKind::Write);
        assert_eq!(FileActionLabel::Edited.to_operation(), EditOperationKind::Edit);
        assert_eq!(FileActionLabel::Deleted.to_operation(), EditOperationKind::Delete);
        assert_eq!(FileActionLabel::parse("nope"), None);
    }

    #[test]
    fn edit_kind_label_ids_match_source() {
        // 真源 :314-321
        assert_eq!(EditKindLabelId::Writing.as_str(), "chat.toolCall.edit.writing");
        assert_eq!(EditKindLabelId::KindWrite.as_str(), "chat.toolCall.kind.write");
        assert_eq!(EditKindLabelId::KindDelete.as_str(), "chat.toolCall.kind.delete");
        // 运行态变体判定。
        assert!(EditKindLabelId::Writing.is_running_variant());
        assert!(!EditKindLabelId::KindWrite.is_running_variant());
    }

    #[test]
    fn summary_operation_falls_back_to_edit() {
        // 未知 operationKind / actionLabel 回退（真源用兜底推断）。
        let s = RawToolCallFileSummary {
            path: "a.rs".into(),
            action_label: "Edited".into(),
            operation_kind: "bogus".into(),
            file_name: "a.rs".into(),
            file_path: None,
            file_icon_src: String::new(),
            change_stat: None,
            patch: None,
        };
        assert_eq!(s.operation(), EditOperationKind::Edit);
        assert_eq!(s.action(), FileActionLabel::Edited);
    }

    #[test]
    fn is_plain_record_rejects_arrays_and_nulls() {
        // 真源 isPlainRecord：非 null 的非数组对象。
        assert!(is_plain_record(&json!({})));
        assert!(!is_plain_record(&json!([])));
        assert!(!is_plain_record(&json!(null)));
        assert!(!is_plain_record(&json!("s")));
    }

    #[test]
    fn read_changes_sources_reads_all_three_places() {
        // 真源 :120-133 —— changes 可能在三处，三处都要读。
        let raw = json!({
            "changes": { "a": "1" },
            "rawInput": { "changes": { "b": "2" } },
            "rawOutput": { "changes": { "c": "3" } },
        });
        let sources = read_changes_sources(&raw);
        assert!(sources.direct.is_some());
        assert!(sources.raw_input.is_some());
        assert!(sources.raw_output.is_some());

        // 只有一处时其余为 None。
        let only_direct = json!({ "changes": { "a": "1" } });
        let sources = read_changes_sources(&only_direct);
        assert!(sources.direct.is_some());
        assert!(sources.raw_input.is_none());
        assert!(sources.raw_output.is_none());
    }

    #[test]
    fn normalize_patch_accepts_both_shapes() {
        // patch 可能是字符串或 { patch: string }。
        assert_eq!(
            normalize_single_file_patch(&json!("diff --git a/x b/x")).as_deref(),
            Some("diff --git a/x b/x")
        );
        assert_eq!(
            normalize_single_file_patch(&json!({ "patch": "P" })).as_deref(),
            Some("P")
        );
        assert_eq!(normalize_single_file_patch(&json!(null)), None);
        assert_eq!(normalize_single_file_patch(&json!(42)), None);
    }

    #[test]
    fn line_change_stat_counts_added_and_removed() {
        // 新增一行、删除一行。
        let stat = compute_line_change_stat("a\nb\nc", "a\nx\nc");
        assert_eq!(stat.added, 1);
        assert_eq!(stat.removed, 1);
        // 完全相同 → 无变更。
        let stat = compute_line_change_stat("a\nb", "a\nb");
        assert_eq!(stat, ChangeStat { added: 0, removed: 0 });
        // 空 → 全增。
        let stat = compute_line_change_stat("", "a\nb");
        assert_eq!(stat.added, 2);
        assert_eq!(stat.removed, 0);
    }

    #[test]
    fn workflow_card_progress_is_not_percentage() {
        // 真源注释：进度是「已排程里的结算数」，不是完成百分比。
        let s = WorkflowRunCardSummary {
            run_id: "r1".into(),
            tool_call_id: Some("tool-1".into()),
            status: "running".into(),
            stop_reason: None,
            nodes_settled: 3,
            nodes_total: 12,
            agents: Some(2),
            run: None,
            resumable: false,
        };
        assert!((s.progress_ratio() - 0.25).abs() < 1e-6);
        // 分母 0 时给 0，避免除零。
        let zero = WorkflowRunCardSummary {
            nodes_total: 0,
            ..s
        };
        assert_eq!(zero.progress_ratio(), 0.0);
    }
}