//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers.tsx`（175 行）+ `renderers/` 子目录。
//!
//! 真源里 `renderers.tsx` 与 `renderers/` 目录并存（TS 允许），Rust 不允许同名
//! `renderers.rs` + `renderers/` 并存，故本文件承载 `renderers.tsx` 的内容，
//! 子目录 `renderers/` 承载各 renderer 卡片。
//!
//! 本文件内容：
//! - `renderDiffCount`（:14-59）：diff 计数，两数都 ≤0 时返回 null
//! - `renderFileChip`（:61-107）：文件 chip，可点击/静态两个分支
//! - `renderFilePath`（:109-126）：次要路径，窄屏隐藏
//! - `getEditKindLabelMessageId`（:128-150）：编辑类 kindLabel 的 i18n id
//! - `renderJoinedFileChips`（:152-175）：多文件 chip 拼接（逗号分隔）

pub mod agent;
pub mod agentHelpers;
pub mod agentPromptSection;
pub mod ask_question;
pub mod changes_group;
pub mod edit;
pub mod execute;
pub mod execute_group;
pub mod explore;
pub mod fallback;
pub mod goal;
pub mod planToolCall;
pub mod plan_guidance;
pub mod read;
pub mod respond_to_coordinator;
pub mod search;
pub mod send_message;
pub mod skill;
pub mod subagentColors;
pub mod switch_mode;
pub mod task_output;
pub mod task_stop;
pub mod todo;
pub mod toolOutput;

use leptos::prelude::*;

use crate::ToolCallBlocks::fileSummaryHeuristics::infer_edit_operation;
use crate::ToolCallBlocks::fileSummaryTypes::{
    EditKindSource, EditOperationKind, RawToolCallFileSummary,
};
use crate::file_icons::{icon_src, resolve_icon_name};

// ---------------------------------------------------------------------------
// renderDiffCount（:14-59）
// ---------------------------------------------------------------------------

/// diff 计数的显示数据（真源函数返回 JSX，这里把判断逻辑与文本拆出来供渲染）。
#[derive(Debug, Clone, PartialEq)]
pub struct DiffCountView {
    pub added: u32,
    pub removed: u32,
}

/// `renderDiffCount` 的判定（:17-22）：
/// 无 changeStat、或增删都 ≤0 时返回 null（不渲染）。
pub fn diff_count_view(change_stat: Option<(u32, u32)>) -> Option<DiffCountView> {
    let (added, removed) = change_stat?;
    if added <= 0 && removed <= 0 {
        return None;
    }
    Some(DiffCountView { added, removed })
}

/// 计数行的类名（:26）——`font-mono` + `tabular-nums` 保证数字等宽不跳动。
pub const DIFF_COUNT_ROW_CLASS: &str =
    "inline-flex items-center gap-1 whitespace-nowrap font-mono leading-none tabular-nums";

/// 增行类名（:32）。
pub const DIFF_ADDED_CLASS: &str = "inline-flex items-center text-diff-added";

/// 删行类名（:44）。
pub const DIFF_REMOVED_CLASS: &str = "inline-flex items-center text-diff-removed";

/// diff 计数组件（真源 :25-57 的 JSX）。
///
/// 真源用 `FlipMetricValue` 做「一次短翻页」动画（注释说投影已按秒给出真实统计，
/// 数字只做一次短翻页，不再逐行 rAF 追赶）。该组件在
/// `components/ui/flip-metric-value.tsx`，属另一个模块，此处渲染静态数字。
#[component]
pub fn RenderDiffCount(view: DiffCountView) -> impl IntoView {
    view! {
        <span class=DIFF_COUNT_ROW_CLASS>
            {(view.added > 0).then(|| view! {
                <span
                    aria-label=format!("+{}", view.added)
                    class=DIFF_ADDED_CLASS
                    role="text"
                    title=format!("+{}", view.added)
                >
                    {format!("+{}", view.added)}
                </span>
            })}
            {(view.removed > 0).then(|| view! {
                <span
                    aria-label=format!("-{}", view.removed)
                    class=DIFF_REMOVED_CLASS
                    role="text"
                    title=format!("-{}", view.removed)
                >
                    {format!("-{}", view.removed)}
                </span>
            })}
        </span>
    }
}

// ---------------------------------------------------------------------------
// renderFileChip（:61-107）
// ---------------------------------------------------------------------------

/// `getPathLeaf`：取路径最后一段。
pub fn get_path_leaf(path: &str) -> &str {
    path.trim_end_matches(['\\', '/'])
        .rsplit(['\\', '/'])
        .find(|s| !s.is_empty())
        .unwrap_or(path)
}

/// chip 的类名。clickable 分支多`hover:underline`。
const CHIP_BASE: &str =
    "inline-flex min-w-0 max-w-full items-center gap-1.5 text-foreground-subtle";
const CHIP_CLICKABLE: &str =
    "inline-flex min-w-0 max-w-full items-center gap-1.5 text-foreground-subtle hover:underline";

/// 文件 chip 组件（真源 :75-105）。
#[component]
pub fn RenderFileChip(summary: RawToolCallFileSummary, clickable: bool) -> impl IntoView {
    let path = summary.path.clone();
    // chipTitle 默认是 display path（:69）。这里用叶子名（chip 内显示）与完整路径（title）。
    let file_name = get_path_leaf(&path).to_string();
    let icon = icon_src(&resolve_icon_name(&path));
    let title = path.clone();

    view! {
        <span class="contents">
            {if clickable {
                view! {
                    <button
                        type="button"
                        class=CHIP_CLICKABLE
                        title=title.clone()
                        on:mousedown=move |ev| {
                            // 真源 :84-87：mousedown 阻止默认，避免焦点转移打断对话区。
                            ev.prevent_default();
                            ev.stop_propagation();
                        }
                    >
                        <img src=icon class="size-4 flex-none" alt="" />
                        <span class="min-w-0 truncate text-foreground-subtle">{file_name.clone()}</span>
                    </button>
                }
                    .into_any()
            } else {
                // 真源 :98-104：静态分支（非 button，无事件）。
                view! {
                    <span class=CHIP_BASE title=title.clone()>
                        <img src=icon class="size-4 flex-none" alt="" />
                        <span class="min-w-0 truncate">{file_name.clone()}</span>
                    </span>
                }
                .into_any()
            }}
        </span>
    }
}

// ---------------------------------------------------------------------------
// renderFilePath（:109-126）
// ---------------------------------------------------------------------------

/// 次要路径的类名（:117）。窄会话流里隐藏，让文件名承担主识别信息。
pub const FILE_PATH_CLASS: &str =
    "min-w-0 truncate text-foreground-subtlest @max-[360px]/conversation:hidden";

/// 路径组件（真源 :119-124）。`path` 为空时返回 null（:111-113）。
#[component]
pub fn RenderFilePath(path: Option<String>) -> impl IntoView {
    match path {
        Some(p) if !p.is_empty() => view! { <span class=FILE_PATH_CLASS>{p}</span> }.into_any(),
        _ => ().into_any(),
    }
}

// ---------------------------------------------------------------------------
// getEditKindLabelMessageId（:128-150）
// ---------------------------------------------------------------------------

/// 编辑类 kindLabel 的 i18n id（真源 `EditKindLabelId`，fileSummaryTypes.ts:314-321）。
///
/// 渲染层再按 id 取中文文案；这里保留 id 语义以对齐真源。
pub fn get_edit_kind_label_id(
    operation_kinds: &[EditOperationKind],
    action_labels: &[&str],
    is_running: bool,
    source: Option<EditKindSource>,
) -> &'static str {
    let inferred = infer_edit_operation(operation_kinds, action_labels, source.as_ref());
    match inferred {
        Some(EditOperationKind::Write) => {
            if is_running {
                "chat.toolCall.edit.writing"
            } else {
                "chat.toolCall.kind.write"
            }
        }
        Some(EditOperationKind::Delete) => {
            if is_running {
                "chat.toolCall.edit.deleting"
            } else {
                "chat.toolCall.kind.delete"
            }
        }
        // 真源 :145-148 —— actionLabels 为空与否则走同一分支（两个 return 值相同），
        // 写成两个分支是为对齐真源，实际产出都是 edit。
        _ => {
            // 真源 :145-148 —— actionLabels 为空与否则走同一分支（两个 return 值相同）。
            // 写成两个分支是为对齐真源，实际产出都是 edit。
            if is_running {
                "chat.toolCall.edit.editing"
            } else {
                "chat.toolCall.kind.edit"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// renderJoinedFileChips（:152-175）
// ---------------------------------------------------------------------------

/// 多文件 chip 拼接组件（真源 :158-173）。
///
/// 第 2 项及之后每项前加逗号（真源 :167）。
#[component]
pub fn RenderJoinedFileChips(summaries: Vec<RawToolCallFileSummary>) -> impl IntoView {
    let items = summaries
        .into_iter()
        .enumerate()
        .map(|(index, summary)| {
            let comma = (index > 0).then(|| {
                view! {
                    <span class="mx-1 text-foreground-subtlest">{","}</span>
                }
            });
            let chip = view! { <RenderFileChip summary=summary clickable=false /> };
            view! {
                <span class="inline-flex min-w-0 items-center">
                    {comma}
                    {chip}
                </span>
            }
        })
        .collect_view();
    view! {
        <div class="inline-flex min-w-0 items-center">{items}</div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(path: &str) -> RawToolCallFileSummary {
        RawToolCallFileSummary {
            path: path.into(),
            action_label: "Edited".into(),
            operation_kind: "edit".into(),
            file_name: get_path_leaf(path).to_string(),
            file_path: None,
            file_icon_src: String::new(),
            change_stat: None,
            patch: None,
        }
    }

    #[test]
    fn diff_count_hidden_when_both_zero() {
        // 真源 :20-22 —— 增删都 ≤0 返回 null。
        assert_eq!(diff_count_view(None), None);
        assert_eq!(diff_count_view(Some((0, 0))), None);
        assert_eq!(
            diff_count_view(Some((0, 3))),
            Some(DiffCountView {
                added: 0,
                removed: 3
            })
        );
        assert_eq!(
            diff_count_view(Some((2, 0))),
            Some(DiffCountView {
                added: 2,
                removed: 0
            })
        );
    }

    #[test]
    fn diff_count_classes_match_source() {
        // 真源 :26/:32/:44
        assert!(DIFF_COUNT_ROW_CLASS.contains("font-mono"));
        assert!(DIFF_COUNT_ROW_CLASS.contains("tabular-nums"));
        assert!(DIFF_COUNT_ROW_CLASS.contains("whitespace-nowrap"));
        assert_eq!(DIFF_ADDED_CLASS, "inline-flex items-center text-diff-added");
        assert_eq!(
            DIFF_REMOVED_CLASS,
            "inline-flex items-center text-diff-removed"
        );
    }

    #[test]
    fn get_path_leaf_takes_last_segment() {
        // 真源 lib/path.js getPathLeaf
        assert_eq!(get_path_leaf("a/b/c.rs"), "c.rs");
        assert_eq!(get_path_leaf("a\\b\\c.rs"), "c.rs");
        assert_eq!(get_path_leaf("a/b/"), "b", "尾部斜杠要吃掉");
        assert_eq!(get_path_leaf("solo"), "solo");
    }

    #[test]
    fn chip_classes_differ_only_by_hover() {
        // 真源 :77（可点击）vs :98（静态）：只差 hover:underline。
        assert!(CHIP_CLICKABLE.contains("hover:underline"));
        assert!(!CHIP_BASE.contains("hover:underline"));
        // 两者共有的基础类名。
        for c in [CHIP_BASE, CHIP_CLICKABLE] {
            assert!(c.contains("min-w-0 max-w-full"));
            assert!(c.contains("gap-1.5"));
            assert!(c.contains("text-foreground-subtle"));
        }
    }

    #[test]
    fn file_path_class_hides_on_narrow() {
        // 真源 :117 —— 窄会话流隐藏次要路径。
        assert!(FILE_PATH_CLASS.contains("@max-[360px]/conversation:hidden"));
        assert!(FILE_PATH_CLASS.contains("text-foreground-subtlest"));
    }

    #[test]
    fn edit_kind_label_switches_on_running() {
        // 真源 :136-148。
        let write = vec![EditOperationKind::Write];
        assert_eq!(
            get_edit_kind_label_id(&write, &[], false, None),
            "chat.toolCall.kind.write"
        );
        assert_eq!(
            get_edit_kind_label_id(&write, &[], true, None),
            "chat.toolCall.edit.writing"
        );

        let delete = vec![EditOperationKind::Delete];
        assert_eq!(
            get_edit_kind_label_id(&delete, &[], false, None),
            "chat.toolCall.kind.delete"
        );
        assert_eq!(
            get_edit_kind_label_id(&delete, &[], true, None),
            "chat.toolCall.edit.deleting"
        );

        let edit = vec![EditOperationKind::Edit];
        assert_eq!(
            get_edit_kind_label_id(&edit, &["Edited"], false, None),
            "chat.toolCall.kind.edit"
        );
        assert_eq!(
            get_edit_kind_label_id(&edit, &["Edited"], true, None),
            "chat.toolCall.edit.editing"
        );
    }

    /// 冒烟：组件能构造出视图（真源逻辑在纯函数里已单测，这里只保证不崩）。
    #[test]
    fn components_render_without_panic() {
        let items = vec![summary("a/one.ts"), summary("b/two.ts")];
        let _joined = view! { <RenderJoinedFileChips summaries=items /> };
        let _chip_static =
            view! { <RenderFileChip summary=summary("src/main.rs") clickable=false /> };
        let _chip_clickable =
            view! { <RenderFileChip summary=summary("src/main.rs") clickable=true /> };
        // 真源 :111-113 —— 空路径返回 null。
        let _empty = view! { <RenderFilePath path=None /> };
        let _empty_str = view! { <RenderFilePath path=Some(String::new()) /> };
        let _ok = view! { <RenderFilePath path=Some("src/main.rs".into()) /> };
        let _diff = view! { <RenderDiffCount view=DiffCountView { added: 1, removed: 2 } /> };
    }
}
