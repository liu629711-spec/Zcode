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

/// `readStringField`（真源 :16-28）：按候选键顺序取第一个非空字符串。
pub fn read_string_field(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(candidate) = value.get(*key).and_then(|v| v.as_str()) {
            if !candidate.trim().is_empty() {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

/// 文件展示描述符（真源 `FileDisplayDescriptor`，fileDisplay.tsx:117-135 的返回）。
///
/// **裁剪注明**：真源的 `filePath` 是 `relativePath + fileName` 的组装结果
/// （依赖 `defaultFileDisplayBasePath` 全局状态做 base 剥离）。桌面端 v1 无该
/// 全局基准，`file_path` 直接给原路径——残留绝对路径前缀（若有）属已知差异。
pub struct FileDisplayDescriptor {
    pub file_name: String,
    pub file_path: Option<String>,
    pub file_icon_src: String,
}

/// `resolveFileDisplayDescriptor`（真源 fileDisplay.tsx:117-135，无 options 路径）。
pub fn resolve_file_display_descriptor(file_path: &str) -> FileDisplayDescriptor {
    let file_name = super::renderers::get_path_leaf(file_path).to_string();
    let icon = crate::file_icons::resolve_icon_name(&file_name);
    FileDisplayDescriptor {
        file_icon_src: crate::file_icons::icon_src(&icon),
        file_name,
        file_path: (!file_path.is_empty()).then(|| file_path.to_string()),
    }
}

/// `readUnifiedDiffField`（真源 :30-37）：从 change 对象取 unified diff 文本。
pub fn read_unified_diff_field(value: &Value) -> Option<String> {
    if !is_plain_record(value) {
        return None;
    }
    read_string_field(value, &["unified_diff", "unifiedDiff", "patch", "diff"])
}

/// `readRawToolCallInput`（真源 :82-91）：优先 `rawInput`，回退 `input`。
///
/// 真源注释：ZCode protocol 的 permission/request payload 按 schema 把工具参数
/// 放在 `input`，旧 UI 只读 `rawInput` 时 Write/Edit 会退化成整段 JSON 展示。
pub fn read_raw_tool_call_input(raw: &Value) -> Value {
    if !is_plain_record(raw) {
        return Value::Null;
    }
    if let Some(v) = raw.get("rawInput") {
        if !v.is_null() {
            return v.clone();
        }
    }
    raw.get("input").cloned().unwrap_or(Value::Null)
}

/// `readStructuredDiffBlock`（真源 :96-113）：`{type:"diff", newText, path?, oldText?}` 块。
pub struct StructuredDiffBlock {
    pub path: Option<String>,
    pub old_text: String,
    pub new_text: String,
}

pub fn read_structured_diff_block(value: &Value) -> Option<StructuredDiffBlock> {
    if !is_plain_record(value) {
        return None;
    }
    if value.get("type").and_then(|t| t.as_str()) != Some("diff") {
        return None;
    }
    let new_text = value.get("newText").and_then(|v| v.as_str())?;
    Some(StructuredDiffBlock {
        path: value
            .get("path")
            .and_then(|v| v.as_str())
            .filter(|p| !p.trim().is_empty())
            .map(|p| p.to_string()),
        // oldText 非字符串时：null → ""，其他类型 → String() 强转（真源 :106-111）。
        old_text: match value.get("oldText") {
            Some(Value::String(s)) => s.clone(),
            None | Some(Value::Null) => String::new(),
            Some(other) => other.to_string(),
        },
        new_text: new_text.to_string(),
    })
}

/// `readRawToolCallChanges`（真源 :115-133）的三路 changes。
pub struct RawToolCallChanges {
    pub direct: Option<Value>,
    pub raw_input: Option<Value>,
    pub raw_output: Option<Value>,
}

pub fn read_raw_tool_call_changes(raw: &Value) -> RawToolCallChanges {
    if !is_plain_record(raw) {
        return RawToolCallChanges {
            direct: None,
            raw_input: None,
            raw_output: None,
        };
    }
    let raw_input_source = read_raw_tool_call_input(raw);
    let raw_input = raw_input_source.is_object().then_some(raw_input_source);
    let raw_output = raw.get("rawOutput").filter(|v| is_plain_record(v)).cloned();
    RawToolCallChanges {
        direct: raw.get("changes").filter(|v| is_plain_record(v)).cloned(),
        raw_input: raw_input
            .as_ref()
            .and_then(|r| r.get("changes"))
            .filter(|v| is_plain_record(v))
            .cloned(),
        raw_output: raw_output
            .as_ref()
            .and_then(|r| r.get("changes"))
            .filter(|v| is_plain_record(v))
            .cloned(),
    }
}

/// `normalizeSingleFilePatch`（真源 :38-84）：
/// 提取出的 patch 文本做单文件化净化——多段 diff 拒收、`@@` 片段补文件头。
///
/// 关键分支（真源注释逐条对应）：
/// - 多文件 patch 丢弃（:46-52）：PatchDiff 只能解析单文件，直接渲染会让
///   「点击文件名查看变更」失败；
/// - 有 `---/+++` 头原样返回（:54-56）；
/// - 只有 `@@` hunk 片段：按首个 hunk 的行数判定新建/删除，补 `/dev/null`
///   头（:58-70）——大整文件新增按普通 change 解析会把页面拖死；
/// - 其余（如 apply_patch 原文）原样返回，不强补头（:72-76）。
pub fn normalize_single_file_patch(patch: Option<&str>, file_label: &str) -> Option<String> {
    let patch = patch?;
    let trimmed = patch.trim();
    if trimmed.is_empty() {
        return None;
    }

    if crate::ToolCallBlocks::toolDiffPreview::count_patch_file_diffs(trimmed) > 1 {
        return None;
    }

    let has_file_header = trimmed.lines().any(|l| l.starts_with("--- "));
    let has_plus_header = trimmed.lines().any(|l| l.starts_with("+++ "));
    if has_file_header && has_plus_header {
        return Some(trimmed.to_string());
    }

    if trimmed.starts_with("@@ ") {
        // 首个 hunk 头：`@@ -old[,n] +new[,m] @@`。
        let (deleted_count, added_count) = parse_first_hunk_counts(trimmed).unwrap_or((1, 1));
        let left = if deleted_count == 0 && added_count > 0 {
            "--- /dev/null".to_string()
        } else {
            format!("--- a/{file_label}")
        };
        let right = if added_count == 0 && deleted_count > 0 {
            "+++ /dev/null".to_string()
        } else {
            format!("+++ b/{file_label}")
        };
        return Some(format!("{left}\n{right}\n{trimmed}"));
    }

    Some(trimmed.to_string())
}

/// 首个 hunk 头的行数（真源 :64-66 `firstHeader` 捕获组的语义）。
fn parse_first_hunk_counts(patch: &str) -> Option<(usize, usize)> {
    let first_line = patch.lines().next()?;
    let rest = first_line.strip_prefix("@@ -")?;
    let (old_part, rest) = rest.split_once(" +")?;
    let new_part = rest.split(" @@").next()?;
    let count_of = |part: &str| -> Option<usize> {
        match part.split_once(',') {
            Some((_, n)) => n.trim().parse().ok(),
            None => Some(1),
        }
    };
    Some((count_of(old_part)?, count_of(new_part)?))
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
        assert_eq!(
            FileActionLabel::Created.to_operation(),
            EditOperationKind::Write
        );
        assert_eq!(
            FileActionLabel::Edited.to_operation(),
            EditOperationKind::Edit
        );
        assert_eq!(
            FileActionLabel::Deleted.to_operation(),
            EditOperationKind::Delete
        );
        assert_eq!(FileActionLabel::parse("nope"), None);
    }

    #[test]
    fn edit_kind_label_ids_match_source() {
        // 真源 :314-321
        assert_eq!(
            EditKindLabelId::Writing.as_str(),
            "chat.toolCall.edit.writing"
        );
        assert_eq!(
            EditKindLabelId::KindWrite.as_str(),
            "chat.toolCall.kind.write"
        );
        assert_eq!(
            EditKindLabelId::KindDelete.as_str(),
            "chat.toolCall.kind.delete"
        );
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
    fn normalize_patch_hunk_only_gets_headers() {
        // 只有 @@ 片段（无 ---/+++ 头）：补文件头（真源 :58-70）。
        let hunk = "@@ -1,1 +1,2 @@\n a\n+b";
        assert_eq!(
            normalize_single_file_patch(Some(hunk), "f.rs").as_deref(),
            Some("--- a/f.rs\n+++ b/f.rs\n@@ -1,1 +1,2 @@\n a\n+b")
        );
        // 纯新增（deleted=0）→ 左头 /dev/null。
        let add_only = "@@ -0,0 +1,2 @@\n+x\n+y";
        assert_eq!(
            normalize_single_file_patch(Some(add_only), "new.rs").as_deref(),
            Some("--- /dev/null\n+++ b/new.rs\n@@ -0,0 +1,2 @@\n+x\n+y")
        );
        // 纯删除（added=0）→ 右头 /dev/null。
        let del_only = "@@ -1,2 +0,0 @@\n-x\n-y";
        assert_eq!(
            normalize_single_file_patch(Some(del_only), "old.rs").as_deref(),
            Some("--- a/old.rs\n+++ /dev/null\n@@ -1,2 +0,0 @@\n-x\n-y")
        );
    }

    #[test]
    fn normalize_patch_rejects_multi_file_patch() {
        // 多文件 patch 丢弃（真源 :46-52：PatchDiff 只能解析单文件）。
        let multi = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,1 +1,1 @@\n-a\n+b\ndiff --git a/y b/y\n--- a/y\n+++ b/y\n@@ -1,1 +1,1 @@\n-c\n+d";
        assert_eq!(normalize_single_file_patch(Some(multi), "x"), None);
    }

    #[test]
    fn normalize_patch_passthrough_and_empty() {
        // 有 ---/+++ 头：原样返回。
        let with_headers = "--- a/x\n+++ b/x\n@@ -1,1 +1,1 @@\n-a\n+b";
        assert_eq!(
            normalize_single_file_patch(Some(with_headers), "x").as_deref(),
            Some(with_headers)
        );
        // 非标准内容（apply_patch 原文）：原样，不强补头（真源 :72-76）。
        assert_eq!(
            normalize_single_file_patch(Some("*** Begin Patch\n"), "x").as_deref(),
            Some("*** Begin Patch")
        );
        // 空/None。
        assert_eq!(normalize_single_file_patch(None, "x"), None);
        assert_eq!(normalize_single_file_patch(Some("   "), "x"), None);
    }

    #[test]
    fn line_change_stat_counts_added_and_removed() {
        // 新增一行、删除一行。
        let stat = compute_line_change_stat("a\nb\nc", "a\nx\nc");
        assert_eq!(stat.added, 1);
        assert_eq!(stat.removed, 1);
        // 完全相同 → 无变更。
        let stat = compute_line_change_stat("a\nb", "a\nb");
        assert_eq!(
            stat,
            ChangeStat {
                added: 0,
                removed: 0
            }
        );
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
