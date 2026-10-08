//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/fileSummaryHeuristics.ts`（403 行）。
//!
//! 本文件是**编辑操作的启发式判定**：从工具身份 + 文本片段推断
//! 「这次是写/改/删/更新」，用于决定 edit 卡的 kindLabel 文案与摘要图标。
//!
//! **最关键的一条门控**（真源 :347-351，注释原文）：
//! search / explore / execute 等非写类工具，只要标题或输出里带 delete/remove，
//! 就会被文本启发式误判成 delete。所以**先要求工具本身具备"写文件"语义**，
//! 再继续细分操作。这是 Rust 侧必须保留的门控，否则 search 的结果会被渲染成删除卡。
//!
//! 行号注释均指真源文件。

use super::fileSummaryTypes::{EditKindSource, EditOperationKind};
use super::resolveRenderer::{family_by_lower, ToolFamily};

/// `hasWritableToolSemantic`（真源 :21-29）：工具本身是否是「写文件」类。
///
/// 真源注释：TodoWrite/AskUserQuestion 这类固定工具名不能再靠字符串片段判断，
/// 统一走 tool identity。
pub fn has_writable_tool_semantic(source: Option<&EditKindSource>) -> bool {
    let Some(s) = source else { return false };
    // resolveToolCallIdentity(source) 的 family 判定（:28）
    identity_family_of(s) == ToolFamily::FileWrite
}

/// `hasReadLikeToolSemantic`（真源 :31-37）。
pub fn has_read_like_tool_semantic(source: Option<&EditKindSource>) -> bool {
    let Some(s) = source else { return false };
    identity_family_of(s) == ToolFamily::FileRead
}

/// 从 source 解析工具身份 family（真源 `resolveToolCallIdentity(source)`）。
/// 优先 toolName，回退 kind / title。
fn identity_family_of(source: &EditKindSource) -> ToolFamily {
    if let Some(name) = source.tool_name.as_deref() {
        if !name.trim().is_empty() {
            return family_by_lower(name);
        }
    }
    for candidate in [source.kind.as_deref(), source.title.as_deref()] {
        if let Some(c) = candidate {
            if !c.trim().is_empty() {
                return family_by_lower(c);
            }
        }
    }
    ToolFamily::Unknown
}

/// `normalizeActionText`（真源 :约 30 行）：归一化文本用于匹配——
/// 转小写并折叠空白。
fn normalize_action_text(value: &str) -> String {
    value
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// `inferEditOperationFromText`（真源 :39-66）：从一段文本推断操作类型。
///
/// **判定顺序不可换**（真源 :45-63）：delete → update → write → edit。
/// 因为「deleted」同时命中 delete 与 edit，先判 delete 才不会误分类。
fn infer_edit_operation_from_text(value: &str) -> Option<EditOperationKind> {
    let text = normalize_action_text(value);
    if text.is_empty() {
        return None;
    }
    // 真源 :45 delete|deleted|remove|removed|erase|erased|unlink|destroy|rm
    for kw in ["delete", "deleted", "remove", "removed", "erase", "erased", "unlink", "destroy", "rm"] {
        if contains_word(&text, kw) {
            return Some(EditOperationKind::Delete);
        }
    }
    // 真源 :49 update|updating|updated
    for kw in ["update", "updating", "updated"] {
        if contains_word(&text, kw) {
            return Some(EditOperationKind::Update);
        }
    }
    // 真源 :53 write|wrote|written|create|creating|created|add|added|save|saved|new
    for kw in [
        "write", "wrote", "written", "create", "creating", "created", "add", "added", "save", "saved",
        "new",
    ] {
        if contains_word(&text, kw) {
            return Some(EditOperationKind::Write);
        }
    }
    // 真源 :57-63 edit|editing|edited|modify|modifying|modified|change|changed|patch|replace|replaced|fix|fixed
    for kw in [
        "edit", "editing", "edited", "modify", "modifying", "modified", "change", "changed", "patch",
        "replace", "replaced", "fix", "fixed",
    ] {
        if contains_word(&text, kw) {
            return Some(EditOperationKind::Edit);
        }
    }
    None
}

/// 单词包含匹配（真源正则里的 `\b`：词边界）。
fn contains_word(text: &str, word: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|t| t == word)
}

/// `readEditOperationCandidates`（真源 :68-95 附近）：从 source 收集候选文本。
///
/// 真源读 title / kind，以及 input 里的description/action/operation/mode/title/kind。
fn read_edit_operation_candidates(source: &EditKindSource) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |s: Option<&String>| {
        if let Some(v) = s {
            if !v.trim().is_empty() {
                out.push(v.clone());
            }
        }
    };
    push(source.title.as_ref());
    push(source.kind.as_ref());
    if let Some(input) = source.input.as_ref() {
        if super::fileSummaryTypes::is_plain_record(input) {
            for key in ["description", "action", "operation", "mode", "title", "kind"] {
                push(input.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()).as_ref());
            }
        }
    }
    out
}

/// `inferEditOperation`（真源 :342-403）—— 编辑操作推断主函数。
///
/// 优先级：
/// 1. **门控**（:347-351）：source 存在但工具不是写类 → 直接返回 None
/// 2. `operationKinds` 全同 → 直接判定（write/update/delete，否则 edit）(:353-371)
/// 3. source 文本启发式（:373-380）
/// 4. `actionLabels` 全同 → write/delete/edit（:382-401）
pub fn infer_edit_operation(
    operation_kinds: &[EditOperationKind],
    action_labels: &[&str],
    source: Option<&EditKindSource>,
) -> Option<EditOperationKind> {
    // ── 门控（:347-351）──
    if source.is_some() && !has_writable_tool_semantic(source) {
        return None;
    }

    // ── operationKinds 全同判定（:353-371）──
    if !operation_kinds.is_empty() {
        let all_write = operation_kinds.iter().all(|k| *k == EditOperationKind::Write);
        let all_update = operation_kinds.iter().all(|k| *k == EditOperationKind::Update);
        let all_delete = operation_kinds.iter().all(|k| *k == EditOperationKind::Delete);
        if all_write {
            return Some(EditOperationKind::Write);
        }
        if all_update {
            return Some(EditOperationKind::Update);
        }
        if all_delete {
            return Some(EditOperationKind::Delete);
        }
        return Some(EditOperationKind::Edit);
    }

    // ── source 文本启发式（:373-380）──
    if let Some(s) = source {
        for candidate in read_edit_operation_candidates(s) {
            if let Some(inferred) = infer_edit_operation_from_text(&candidate) {
                return Some(inferred);
            }
        }
    }

    // ── actionLabels（:382-401）──
    if action_labels.is_empty() {
        return None;
    }
    let all_created = action_labels.iter().all(|l| *l == "Created");
    let all_deleted = action_labels.iter().all(|l| *l == "Deleted");
    let all_edited = action_labels.iter().all(|l| *l == "Edited");
    if all_created {
        return Some(EditOperationKind::Write);
    }
    if all_deleted {
        return Some(EditOperationKind::Delete);
    }
    if all_edited || !action_labels.is_empty() {
        return Some(EditOperationKind::Edit);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_writable_tool_is_gated_out() {
        // ★核心门控（:347-351）—— search 工具不该被文本启发式误判成 delete。
        let search_source = EditKindSource {
            tool_name: Some("Grep".into()),
            title: Some("delete everything".into()), // 标题里带 delete
            ..Default::default()
        };
        // 工具不是写类 → 直接 None，不进入文本启发式。
        assert_eq!(infer_edit_operation(&[], &[], Some(&search_source)), None);
    }

    #[test]
    fn operation_kinds_all_same_determine_result() {
        // 真源 :353-371
        let w = vec![EditOperationKind::Write, EditOperationKind::Write];
        assert_eq!(infer_edit_operation(&w, &[], None), Some(EditOperationKind::Write));

        let u = vec![EditOperationKind::Update, EditOperationKind::Update];
        assert_eq!(infer_edit_operation(&u, &[], None), Some(EditOperationKind::Update));

        let d = vec![EditOperationKind::Delete, EditOperationKind::Delete];
        assert_eq!(infer_edit_operation(&d, &[], None), Some(EditOperationKind::Delete));

        // 混合 → edit。
        let mixed = vec![EditOperationKind::Write, EditOperationKind::Edit];
        assert_eq!(infer_edit_operation(&mixed, &[], None), Some(EditOperationKind::Edit));
    }

    #[test]
    fn text_heuristics_order_delete_first() {
        // 真源 :45-63 —— 判定顺序 delete→update→write→edit 不可换。
        // "deleted" 同时命中 delete 与 edit，先判 delete。
        let s = EditKindSource {
            tool_name: Some("Edit".into()),
            title: Some("deleted file".into()),
            ..Default::default()
        };
        assert_eq!(
            infer_edit_operation(&[], &[], Some(&s)),
            Some(EditOperationKind::Delete)
        );

        let s = EditKindSource {
            tool_name: Some("Edit".into()),
            title: Some("updated file".into()),
            ..Default::default()
        };
        assert_eq!(
            infer_edit_operation(&[], &[], Some(&s)),
            Some(EditOperationKind::Update)
        );

        let s = EditKindSource {
            tool_name: Some("Write".into()),
            title: Some("created file".into()),
            ..Default::default()
        };
        assert_eq!(
            infer_edit_operation(&[], &[], Some(&s)),
            Some(EditOperationKind::Write)
        );
    }

    #[test]
    fn word_boundary_matching() {
        // 真源正则用 \b，词边界匹配。
        // "remove" 命中 delete。
        assert_eq!(
            infer_edit_operation_from_text("remove the file"),
            Some(EditOperationKind::Delete)
        );
        // 包含但非独立词（如 "removedx"）不该命中。
        assert_eq!(infer_edit_operation_from_text("removedx"), None);
        // 正常 edit 词。
        assert_eq!(
            infer_edit_operation_from_text("edit the file"),
            Some(EditOperationKind::Edit)
        );
    }

    #[test]
    fn writable_tool_allows_text_inference() {
        // 写类工具允许文本启发式。
        let s = EditKindSource {
            tool_name: Some("Write".into()),
            title: Some("创建新文件".into()),
            ..Default::default()
        };
        // 中文标题不含英文关键词 → 推断不出，回 None。
        assert_eq!(infer_edit_operation(&[], &[], Some(&s)), None);
    }

    #[test]
    fn action_labels_determine_result() {
        // 真源 :382-401
        assert_eq!(
            infer_edit_operation(&[], &["Created", "Created"], None),
            Some(EditOperationKind::Write)
        );
        assert_eq!(
            infer_edit_operation(&[], &["Deleted", "Deleted"], None),
            Some(EditOperationKind::Delete)
        );
        assert_eq!(
            infer_edit_operation(&[], &["Edited", "Edited"], None),
            Some(EditOperationKind::Edit)
        );
        // 混合 Created + Edited → edit（:398-400）。
        assert_eq!(
            infer_edit_operation(&[], &["Created", "Edited"], None),
            Some(EditOperationKind::Edit)
        );
        // 空 actionLabels → None。
        assert_eq!(infer_edit_operation(&[], &[], None), None);
    }

    #[test]
    fn has_writable_semantic_checks_family() {
        // 真源 :21-29
        assert!(has_writable_tool_semantic(Some(&EditKindSource {
            tool_name: Some("Edit".into()),
            ..Default::default()
        })));
        assert!(!has_writable_tool_semantic(Some(&EditKindSource {
            tool_name: Some("Grep".into()),
            ..Default::default()
        })));
        assert!(!has_writable_tool_semantic(None));
        // read 类不算 writable。
        assert!(!has_writable_tool_semantic(Some(&EditKindSource {
            tool_name: Some("Read".into()),
            ..Default::default()
        })));
        assert!(has_read_like_tool_semantic(Some(&EditKindSource {
            tool_name: Some("Read".into()),
            ..Default::default()
        })));
    }

    #[test]
    fn normalize_action_text_folds_whitespace_and_lowercases() {
        // 真源 normalizeActionText
        assert_eq!(normalize_action_text("  Hello   World  "), "hello world");
        assert_eq!(normalize_action_text("EDIT"), "edit");
    }
}