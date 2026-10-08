//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/fileSummaries.ts`（231 行）。
//!
//! `readRawToolCallFileSummaries` 是 edit 卡「文件 chip + diff 计数 + patch」
//! 的唯一来源。读取优先级（真源同序）：
//!
//! 1. **display 结构化事实**（`rawOutput.display` 的 file_diff/file_diffs）——
//!    真源注释（:79-81）：agent 会把真实 diff 放这里，summary 与工具轨迹必须
//!    复用同一份事实，否则会出现「工具行能展开 diff、摘要却显示无法预览」的分叉；
//! 2. `raw.content` 的结构化 diff 块（type:"diff"）；
//! 3. `changes` 三路（raw.changes / rawInput.changes / rawOutput.changes）的
//!    old/new 文本（无结构化 diff 时的兜底，自己 build unified diff）；
//! 4. 全部落空 → `buildFallbackRawToolCallFileSummary` 的启发式兜底。

use serde_json::Value;

use super::fileSummaryHeuristics::{
    build_fallback_raw_tool_call_file_summary, has_writable_tool_semantic,
};
use super::fileSummaryTypes::{
    EditKindSource, RawToolCallFileSummary, compute_line_change_stat, is_plain_record,
    normalize_single_file_patch, read_raw_tool_call_changes, read_raw_tool_call_input,
    read_string_field, read_structured_diff_block, read_unified_diff_field,
    resolve_file_display_descriptor,
};

/// `readFileDiffDisplays`（真源 :25-38）：从值里取 file_diff / file_diffs 展示块。
///
/// `value.display` 优先于 value 本身（真源 :30：display 若是 record 就用它）。
fn read_file_diff_displays(value: &Value) -> Vec<Value> {
    if !is_plain_record(value) {
        return Vec::new();
    }
    let display = value
        .get("display")
        .filter(|d| is_plain_record(d))
        .unwrap_or(value);
    match display.get("kind").and_then(|k| k.as_str()) {
        Some("file_diff") => vec![display.clone()],
        Some("file_diffs") => display
            .get("files")
            .and_then(|f| f.as_array())
            .map(|files| {
                files
                    .iter()
                    .filter(|f| is_plain_record(f))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// `readDisplayStructuredPatch`（真源 :40-64）：把 structuredPatch 数组
/// 拼成 unified diff 文本（含 ---/+++ 头）。
fn read_display_structured_patch(display: &Value, file_label: &str) -> Option<String> {
    let structured = display.get("structuredPatch").and_then(|p| p.as_array())?;
    let mut hunks: Vec<String> = Vec::new();
    for hunk in structured {
        if !is_plain_record(hunk) {
            continue;
        }
        let Some(lines) = hunk.get("lines").and_then(|l| l.as_array()) else {
            continue;
        };
        let number_field = |key: &str| -> usize {
            hunk.get(key)
                .and_then(|v| v.as_f64())
                .map(|n| n as usize)
                .unwrap_or(1)
        };
        let old_start = number_field("oldStart");
        let old_lines = number_field("oldLines");
        let new_start = number_field("newStart");
        let new_lines = number_field("newLines");
        hunks.push(format!(
            "@@ -{old_start},{old_lines} +{new_start},{new_lines} @@"
        ));
        for line in lines {
            if let Some(s) = line.as_str() {
                hunks.push(s.to_string());
            }
        }
    }
    if hunks.is_empty() {
        return None;
    }
    let mut out = vec![format!("--- a/{file_label}"), format!("+++ b/{file_label}")];
    out.extend(hunks);
    Some(out.join("\n"))
}

/// `readDisplayFileDiffSummaries`（真源 :66-101）：display 主路摘要。
///
/// 去重按 path（先到先得）；actionLabel 恒 "Edited"、operationKind 恒 "edit"
/// （display 是 agent 归一化后的变更事实，不再做增删判型）。
fn read_display_file_diff_summaries(values: &[&Value]) -> Vec<RawToolCallFileSummary> {
    let mut summaries: Vec<RawToolCallFileSummary> = Vec::new();
    let mut seen_paths: std::collections::HashSet<String> = std::collections::HashSet::new();

    for value in values {
        for display in read_file_diff_displays(value) {
            let Some(path) = read_string_field(&display, &["filePath", "file_path", "path"]) else {
                continue;
            };
            if !seen_paths.insert(path.clone()) {
                continue;
            }
            let descriptor = resolve_file_display_descriptor(&path);
            let additions = display
                .get("additions")
                .and_then(|v| v.as_f64())
                .map(|n| n.round().max(0.0) as u32)
                .unwrap_or(0);
            let deletions = display
                .get("deletions")
                .and_then(|v| v.as_f64())
                .map(|n| n.round().max(0.0) as u32)
                .unwrap_or(0);
            summaries.push(RawToolCallFileSummary {
                path: path.clone(),
                action_label: "Edited".to_string(),
                operation_kind: "edit".to_string(),
                file_name: descriptor.file_name,
                file_path: descriptor.file_path,
                file_icon_src: descriptor.file_icon_src,
                change_stat: Some(super::fileSummaryTypes::ChangeStat {
                    added: additions,
                    removed: deletions,
                }),
                patch: read_display_structured_patch(
                    &display,
                    super::renderers::get_path_leaf(&path),
                ),
            });
        }
    }
    summaries
}

/// `readRawToolCallFileSummaries`（真源 :103-231）：edit 卡文件摘要主入口。
pub fn read_raw_tool_call_file_summaries(
    raw: &Value,
    source: Option<&EditKindSource>,
) -> Vec<RawToolCallFileSummary> {
    if source.is_some() && !has_writable_tool_semantic(source) {
        // 真源注释（:107-111）：renderer 分流把「存在 rawFileSummaries」视为
        // edit 证据；非写类工具若也从 raw.changes 抽摘要，会把 search/explore/
        // execute 误导到 edit/delete。先按 kind/toolName 过滤。
        return Vec::new();
    }

    let source_output = source.and_then(|s| s.output.clone()).unwrap_or(Value::Null);

    if !is_plain_record(raw) {
        let display_summaries = read_display_file_diff_summaries(&[&source_output]);
        return if display_summaries.is_empty() {
            build_fallback_raw_tool_call_file_summary(source)
        } else {
            display_summaries
        };
    }

    let raw_input = read_raw_tool_call_input(raw);
    let raw_output = raw
        .get("rawOutput")
        .filter(|v| is_plain_record(v))
        .cloned()
        .unwrap_or(Value::Null);

    // display 主路（真源 :121-126）。
    let display_summaries =
        read_display_file_diff_summaries(&[raw, &raw_input, &raw_output, &source_output]);
    if !display_summaries.is_empty() {
        return display_summaries;
    }

    // raw.content 的结构化 diff 块（真源 :128-135）。
    let raw_diffs: Vec<super::fileSummaryTypes::StructuredDiffBlock> = raw
        .get("content")
        .and_then(|c| c.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(read_structured_diff_block)
                .filter(|d| d.path.is_some())
                .collect()
        })
        .unwrap_or_default();

    // changes 三路（真源 :137）。
    let changes = read_raw_tool_call_changes(raw);

    // orderedPaths：rawDiffs 先、再三路 changes 的键（真源 :139-159 的 Set 语义）。
    let mut ordered_paths: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let push_path =
        |path: &str, ordered: &mut Vec<String>, seen: &mut std::collections::HashSet<String>| {
            if seen.insert(path.to_string()) {
                ordered.push(path.to_string());
            }
        };
    for diff in &raw_diffs {
        if let Some(p) = &diff.path {
            push_path(p, &mut ordered_paths, &mut seen);
        }
    }
    for changes_value in [&changes.direct, &changes.raw_input, &changes.raw_output] {
        if let Some(record) = changes_value.as_ref().and_then(|v| v.as_object()) {
            for key in record.keys() {
                push_path(key, &mut ordered_paths, &mut seen);
            }
        }
    }

    let mut summaries: Vec<RawToolCallFileSummary> = Vec::new();
    for path in ordered_paths {
        let diff = raw_diffs
            .iter()
            .find(|d| d.path.as_deref() == Some(path.as_str()));
        // 三路 changes 的优先级：rawOutput > rawInput > direct（真源 :164）。
        let change = changes
            .raw_output
            .as_ref()
            .and_then(|c| c.get(&path))
            .or_else(|| changes.raw_input.as_ref().and_then(|c| c.get(&path)))
            .or_else(|| changes.direct.as_ref().and_then(|c| c.get(&path)));
        let change_type = change
            .filter(|c| is_plain_record(c))
            .and_then(|c| c.get("type"))
            .and_then(|t| t.as_str());

        // 变更类型 → 操作/动作（真源 :167-176）。
        let (operation_kind, action_label) = match change_type {
            Some("add") => ("write", "Created"),
            Some("update") => ("update", "Edited"),
            Some("delete") => ("delete", "Deleted"),
            _ => ("edit", "Edited"),
        };

        let old_text = diff.map(|d| d.old_text.clone()).or_else(|| {
            change.filter(|c| is_plain_record(c)).and_then(|c| {
                read_string_field(
                    c,
                    &[
                        "oldText",
                        "old_string",
                        "oldString",
                        "before",
                        "old_content",
                        "oldContent",
                    ],
                )
            })
        });
        let new_text = diff.map(|d| d.new_text.clone()).or_else(|| {
            change.filter(|c| is_plain_record(c)).and_then(|c| {
                read_string_field(
                    c,
                    &[
                        "newText",
                        "new_string",
                        "newString",
                        "after",
                        "new_content",
                        "newContent",
                        "content",
                    ],
                )
            })
        });

        let descriptor = resolve_file_display_descriptor(&path);
        let leaf = super::renderers::get_path_leaf(&path).to_string();

        // 行数统计（真源 :203-210）。
        let change_stat = match (old_text.as_deref(), new_text.as_deref(), change_type) {
            (Some(old), Some(new), _) => Some(compute_line_change_stat(old, new)),
            (None, Some(new), Some("add")) => Some(compute_line_change_stat("", new)),
            (Some(old), None, Some("delete")) => Some(compute_line_change_stat(old, "")),
            _ => None,
        };

        // patch：显式字段优先，否则从 old/new 自建（真源 :211-216）。
        let explicit_patch = change
            .filter(|c| is_plain_record(c))
            .and_then(read_unified_diff_field)
            .and_then(|p| normalize_single_file_patch(Some(&p), &leaf));
        let patch = explicit_patch.or_else(|| {
            (old_text.is_some() || new_text.is_some()).then(|| {
                super::toolDiffPreview::build_unified_diff(
                    old_text.as_deref().unwrap_or(""),
                    new_text.as_deref().unwrap_or(""),
                    &leaf,
                )
            })
        });

        summaries.push(RawToolCallFileSummary {
            path,
            action_label: action_label.to_string(),
            operation_kind: operation_kind.to_string(),
            file_name: descriptor.file_name,
            file_path: descriptor.file_path,
            file_icon_src: descriptor.file_icon_src,
            change_stat,
            patch,
        });
    }

    if summaries.is_empty() {
        build_fallback_raw_tool_call_file_summary(source)
    } else {
        summaries
    }
}

#[cfg(test)]
mod tests {
    use super::super::fileSummaryTypes::EditKindSource;
    use super::*;
    use serde_json::json;

    fn source(tool_name: &str, input: Value) -> EditKindSource {
        EditKindSource {
            tool_name: Some(tool_name.into()),
            kind: Some(tool_name.into()),
            title: None,
            input: Some(input),
            output: None,
            raw: None,
        }
    }

    #[test]
    fn display_file_diff_wins_over_changes() {
        // display 主路优先（真源 :121-126）。
        let raw = json!({
            "rawOutput": {
                "display": {
                    "kind": "file_diff",
                    "filePath": "src/a.rs",
                    "additions": 3,
                    "deletions": 1,
                    "structuredPatch": [
                        { "oldStart": 1, "oldLines": 2, "newStart": 1, "newLines": 4,
                          "lines": [" fn main() {", "-    old();", "+    new();", "+    more();", " }"] }
                    ]
                }
            },
            "changes": { "src/a.rs": { "type": "update" } }
        });
        let src = source("Edit", json!({"file_path": "src/a.rs"}));
        let summaries = read_raw_tool_call_file_summaries(&raw, Some(&src));
        assert_eq!(summaries.len(), 1);
        let s = &summaries[0];
        assert_eq!(s.path, "src/a.rs");
        assert_eq!(s.action_label, "Edited");
        assert_eq!(
            s.change_stat.as_ref().map(|c| (c.added, c.removed)),
            Some((3, 1))
        );
        // patch 来自 structuredPatch 拼装。
        let patch = s.patch.as_deref().unwrap();
        assert!(patch.contains("--- a/a.rs"));
        assert!(patch.contains("@@ -1,2 +1,4 @@"));
        assert!(patch.contains("-    old();"));
    }

    #[test]
    fn changes_path_produces_patch_from_old_new() {
        // 无 display：changes 的 old/new 自建 patch（真源 :189-216）。
        let raw = json!({
            "changes": {
                "src/b.rs": { "type": "update", "oldText": "a\nb\n", "newText": "a\nB\n" }
            }
        });
        let src = source("Edit", json!({"file_path": "src/b.rs"}));
        let summaries = read_raw_tool_call_file_summaries(&raw, Some(&src));
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].operation_kind, "update");
        assert_eq!(summaries[0].action_label, "Edited");
        let patch = summaries[0].patch.as_deref().unwrap();
        assert!(patch.contains("-b") && patch.contains("+B"));
        assert_eq!(
            summaries[0]
                .change_stat
                .as_ref()
                .map(|c| (c.added, c.removed)),
            Some((1, 1))
        );
    }

    #[test]
    fn add_type_maps_to_created_and_counts_from_content() {
        // type:"add" → write/Created；changeStat 从 null→newText 计（真源 :206-208）。
        let raw = json!({
            "changes": { "new.rs": { "type": "add", "content": "line1\nline2\n" } }
        });
        let src = source("Write", json!({"file_path": "new.rs"}));
        let summaries = read_raw_tool_call_file_summaries(&raw, Some(&src));
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].operation_kind, "write");
        assert_eq!(summaries[0].action_label, "Created");
        assert_eq!(
            summaries[0]
                .change_stat
                .as_ref()
                .map(|c| (c.added, c.removed)),
            Some((2, 0))
        );
        // 新建文件 patch 左头 /dev/null。
        assert!(
            summaries[0]
                .patch
                .as_deref()
                .unwrap()
                .contains("--- /dev/null")
        );
    }

    #[test]
    fn delete_type_uses_deleted_label() {
        let raw = json!({
            "changes": { "old.rs": { "type": "delete", "oldText": "x\n" } }
        });
        let src = source("Edit", json!({"file_path": "old.rs"}));
        let summaries = read_raw_tool_call_file_summaries(&raw, Some(&src));
        assert_eq!(summaries[0].operation_kind, "delete");
        assert_eq!(summaries[0].action_label, "Deleted");
    }

    #[test]
    fn content_diff_block_feeds_paths_before_changes() {
        // raw.content 的 type:"diff" 块（真源 :128-135）：路径序在 changes 前。
        let raw = json!({
            "content": [
                { "type": "diff", "path": "first.rs", "oldText": "a\n", "newText": "b\n" },
                { "type": "diff", "path": "second.rs", "newText": "new\n" }
            ],
            "changes": { "third.rs": { "type": "update", "oldText": "1\n", "newText": "2\n" } }
        });
        let src = source("Edit", json!({"file_path": "first.rs"}));
        let summaries = read_raw_tool_call_file_summaries(&raw, Some(&src));
        let paths: Vec<&str> = summaries.iter().map(|s| s.path.as_str()).collect();
        assert_eq!(paths, vec!["first.rs", "second.rs", "third.rs"]);
        // 第二块无 oldText：oldText 缺省空串 → patch 是纯新增。
        assert!(summaries[1].patch.as_deref().unwrap().contains("+new"));
    }

    #[test]
    fn non_writable_tool_gets_empty() {
        // 门控（真源 :107-112）：search/read 类不给摘要。
        let raw = json!({ "changes": { "x.rs": { "type": "update" } } });
        let src = source("Grep", json!({"pattern": "x"}));
        assert!(read_raw_tool_call_file_summaries(&raw, Some(&src)).is_empty());
    }

    #[test]
    fn fallback_used_when_no_structured_data() {
        // 结构化全落空 → 启发式兜底（真源 :230）。
        let raw =
            json!({ "rawInput": { "file_path": "fb.rs", "old_string": "a", "new_string": "b" } });
        let src = source("Edit", json!({"file_path": "fb.rs"}));
        let summaries = read_raw_tool_call_file_summaries(&raw, Some(&src));
        assert_eq!(summaries.len(), 1, "兜底应产出文件摘要");
        assert_eq!(summaries[0].path, "fb.rs");
    }

    #[test]
    fn display_file_diffs_array_handles_multiple_files() {
        let raw = json!({
            "rawOutput": {
                "display": {
                    "kind": "file_diffs",
                    "files": [
                        { "filePath": "a1.rs", "additions": 1, "deletions": 0 },
                        { "filePath": "a2.rs", "additions": 0, "deletions": 2 }
                    ]
                }
            }
        });
        let src = source("Edit", json!({"file_path": "a1.rs"}));
        let summaries = read_raw_tool_call_file_summaries(&raw, Some(&src));
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[1].path, "a2.rs");
    }

    #[test]
    fn display_paths_dedupe_keeps_first() {
        // 同一 path 出现两次：先到先得（真源 seenPaths）。
        let raw = json!({
            "display": { "kind": "file_diff", "path": "dup.rs", "additions": 5 },
            "rawOutput": {
                "display": { "kind": "file_diff", "path": "dup.rs", "additions": 9 }
            }
        });
        let src = source("Edit", json!({"file_path": "dup.rs"}));
        let summaries = read_raw_tool_call_file_summaries(&raw, Some(&src));
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].change_stat.as_ref().map(|c| c.added), Some(5));
    }
}
