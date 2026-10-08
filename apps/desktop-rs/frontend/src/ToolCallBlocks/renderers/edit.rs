//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/edit.tsx`（480 行）。
//!
//! edit 卡片。真源要点：
//! - **默认可折叠**（`EDIT_SINGLE_LAYOUT`，:24-27）——与 read 卡相反
//! - `hideDiffCountWhenOpen={hasMultipleFiles}`（:325）——多文件时展开后隐藏 diff 计数
//! - `expandedPrimaryText`（:325）：多文件收起显示文件 chips，**展开显示「N 个文件」**
//!   （真源 zh-CN.ts:4861 `chat.toolCall.edit.multipleFiles` = "{count} 个文件"）
//! - `renderContent` 三态（:188-284）：
//!   ① 无摘要 → `ToolCallBody` 兜底（失败编辑）
//!   ② 多文件 → `ml-2 space-y-2 border-l pl-3.5` 缩进容器，逐文件拆块
//!   ③ 单文件 → `space-y-3` + `EditInlineDiffContent` 内联 diff
//!
//! 行号注释均指真源文件。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolCallBody::ToolCallBody;
use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use super::super::codeViewer::{CodeViewerWorkspaceScope, PatchCodeViewerSource};
use super::super::fileDisplay::get_file_display_path;
use super::super::toolCallRowAdapter::{LegacyToolCall, LegacyToolCallNode};
use super::super::toolDisplay::{ToolDisplayModel, ToolInlinePreview};
use super::editInlineDiffContent::{EditInlineDiffContentComponent, EditInlineDiffContentProps};

use crate::ToolCallBlocks::fileSummaryTypes::{
    ChangeStat, EditKindSource, EditOperationKind, RawToolCallFileSummary,
};

/// `EDIT_TOOL_ICON`（真源 :22）——PencilIcon。
pub const EDIT_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `EDIT_SINGLE_LAYOUT`（真源 :24-27）与 `EDIT_CHILD_LAYOUT`（:29-32）取值相同，
/// 都可折叠、不强制展开。
pub const EDIT_LAYOUT_CAN_TOGGLE: bool = true;
pub const EDIT_LAYOUT_FORCE_OPEN: bool = false;

/// `isRawToolCallFailed`（真源 :35-43）：raw.status == "failed"。
pub fn is_raw_tool_call_failed(raw: &Value) -> bool {
    if !raw.is_object() {
        return false;
    }
    raw.get("status").and_then(|v| v.as_str()) == Some("failed")
}

/// `buildEditCodeViewerSource`（真源 :45-64）的结果。
#[derive(Debug, Clone, PartialEq)]
pub enum CodeViewerSource {
    /// 无 patch → 只给文件路径。
    File { title: String, path: String },
    /// 有 patch → 给 patch（侧栏按目录解析图标）。
    Patch {
        title: String,
        path: String,
        patch: String,
    },
}

/// `buildEditCodeViewerSource`（真源 :45-64）。
///
/// 真源注释（:58-60）：**summary.filePath 是 fileDisplay 生成的目录展示路径，
/// 不是可打开的文件路径**。传给 side pane 会按目录解析图标，导致 .ts/.tsx 的
/// diff tab 退回通用文件图标。所以这里必须用 `summary.path`。
pub fn build_edit_code_viewer_source(summary: &RawToolCallFileSummary) -> CodeViewerSource {
    match summary.patch.as_deref().filter(|p| !p.is_empty()) {
        None => CodeViewerSource::File {
            title: summary.file_name.clone(),
            path: summary.path.clone(),
        },
        Some(patch) => CodeViewerSource::Patch {
            title: summary.file_name.clone(),
            path: summary.path.clone(),
            patch: patch.to_string(),
        },
    }
}

/// `buildEditInlinePreview`（真源 :66-82）：把当前文件的 patch 转成 inline preview。
///
/// 真源注释（:70-72）：edit 摘要层已能从 rawFileSummaries 拿到单文件 patch，
/// 但展开区之前只认 displayModel.inlinePreview，导致卡片自动展开后看不到 diff。
/// 这里把 patch 直接转成 content 里的 diff 块，避免 ToolCallBody 再次抢走这份预览。
pub fn build_edit_inline_preview(
    summary: Option<&RawToolCallFileSummary>,
) -> Option<CodeViewerSource> {
    let s = summary?;
    if s.patch.as_deref().is_none_or(str::is_empty) {
        return None;
    }
    match build_edit_code_viewer_source(s) {
        CodeViewerSource::Patch { .. } => Some(build_edit_code_viewer_source(s)),
        CodeViewerSource::File { .. } => None,
    }
}

/// `totalChangeStat`（真源 :124-141）：所有文件的增删累加。
pub fn total_change_stat(summaries: &[RawToolCallFileSummary]) -> ChangeStat {
    summaries
        .iter()
        .fold(ChangeStat::default(), |acc, s| match s.change_stat {
            Some(stat) => ChangeStat {
                added: acc.added + stat.added,
                removed: acc.removed + stat.removed,
            },
            None => acc,
        })
}

/// `expandedPrimaryText` 的文案（真源 :170-183 + zh-CN.ts:4861）。
///
/// 多文件时展开显示「N 个文件」，单文件时沿用收起态（文件 chips）。
pub fn expanded_primary_text(count: usize) -> Option<String> {
    (count > 1).then(|| format!("{count} 个文件"))
}

/// EditToolCallBlock 的 props（对应真源从 context 解构的字段）。
#[derive(Debug, Clone)]
pub struct EditBlockProps {
    pub tool_id: String,
    pub raw_file_summaries: Vec<RawToolCallFileSummary>,
    /// 失败判定（真源 :91-92）：status==="failed" || raw.status==="failed" || 有 errorText。
    pub is_failed: bool,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub kind: Option<String>,
    /// 编辑类型判定来源（真源 :147-154）。
    pub kind_source: EditKindSource,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub is_office_mode: bool,
    /// 工具调用本体（展开区 ToolCallBody 消费 input/output/error/kind）。
    pub legacy: LegacyToolCall,
    /// 展示模型（真源 `context.displayModel`）。
    pub display_model: ToolDisplayModel,
    /// 子工具列表（真源 `context.childToolList`；v4 行没有子树，恒空）。
    pub child_tool_calls: Vec<LegacyToolCallNode>,
    pub workspace_path: String,
    /// 应用主题（内联 diff 高亮口径）。
    pub theme: Option<String>,
    /// system 主题下的 OS 明暗偏好。
    pub prefers_dark: bool,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
}

/// `kindLabel` 的解析（真源 :142-157）。
///
/// 先算操作类型（门控在 fileSummaryHeuristics 里），再按运行态选 i18n id，
/// 最后取中文文案。
pub fn edit_kind_label(props: &EditBlockProps) -> &'static str {
    let kinds: Vec<EditOperationKind> = props
        .raw_file_summaries
        .iter()
        .map(|s| s.operation())
        .collect();
    let labels: Vec<&str> = props
        .raw_file_summaries
        .iter()
        .map(|s| s.action_label.as_str())
        .collect();
    super::super::renderers::get_edit_kind_label_id(
        &kinds,
        &labels,
        props.is_running,
        Some(props.kind_source.clone()),
    )
}

/// i18n id → 中文文案（zh-CN.ts 对应条目）。
fn kind_label_text(id: &str) -> &'static str {
    match id {
        "chat.toolCall.edit.writing" => "正在写入",
        "chat.toolCall.edit.updating" => "正在更新",
        "chat.toolCall.edit.deleting" => "正在删除",
        "chat.toolCall.edit.editing" => "正在编辑",
        "chat.toolCall.kind.write" => "写入",
        "chat.toolCall.kind.delete" => "删除",
        _ => "编辑",
    }
}

/// 本文件的简化 `CodeViewerSource`（上方既有）→ codeViewer 模块的 patch 源
/// （EditInlineDiffContent 消费的形态）。
fn edit_patch_source(summary: &RawToolCallFileSummary) -> Option<PatchCodeViewerSource> {
    match build_edit_code_viewer_source(summary) {
        CodeViewerSource::Patch { title, path, patch } => Some(PatchCodeViewerSource {
            scope: CodeViewerWorkspaceScope::default(),
            title,
            path: Some(path),
            patch,
        }),
        CodeViewerSource::File { .. } => None,
    }
}

/// `EditFileSummaryBlock` 的 props（真源 :341-462 的形参子集）。
#[derive(Debug, Clone)]
pub struct EditFileSummaryBlockProps {
    pub summary: RawToolCallFileSummary,
    pub is_running: bool,
    pub status_label: String,
    pub error_text: Option<String>,
    pub tool_id: String,
    pub is_failed: bool,
    pub show_icon: bool,
    pub source_label: Option<String>,
    pub display_model: ToolDisplayModel,
    pub legacy: LegacyToolCall,
    pub child_tool_calls: Vec<LegacyToolCallNode>,
    pub workspace_path: String,
    pub theme: Option<String>,
    pub prefers_dark: bool,
}

/// `EditFileSummaryBlock`（真源 :341-475）——多文件 edit 的单个文件块。
///
/// 真源注释（:454-455）：多文件 edit 的每个子文件都有自己的 diff content，
/// 必须允许单独展开查看（EDIT_CHILD_LAYOUT）。
#[component]
pub fn EditFileSummaryBlockComponent(props: EditFileSummaryBlockProps) -> impl IntoView {
    let kind_label = kind_label_text(&crate::ToolCallBlocks::renderers::get_edit_kind_label_id(
        &[props.summary.operation()],
        &[props.summary.action_label.as_str()],
        props.is_running,
        None,
    ));
    let patch_source = edit_patch_source(&props.summary);
    let title = get_file_display_path(
        props
            .summary
            .file_path
            .as_deref()
            .unwrap_or(props.summary.path.as_str()),
        Some(&props.workspace_path),
    );
    let status_tooltip = if props.is_failed {
        props.error_text.clone()
    } else {
        None
    };
    let diff_view = crate::ToolCallBlocks::renderers::diff_count_view(
        props.summary.change_stat.map(|s| (s.added, s.removed)),
    );
    let file_path = props.summary.file_path.clone();

    let legacy = props.legacy.clone();
    let display_model = props.display_model.clone();
    let workspace_path = props.workspace_path.clone();
    let patch_for_preview = patch_source.clone();
    let theme = props.theme.clone();
    let prefers_dark = props.prefers_dark;
    let child_tool_calls = props.child_tool_calls.clone();
    let patch_override = patch_source.as_ref().map(|_| ToolInlinePreview::None);
    let render_content = move || {
        let inline_diff = patch_for_preview.clone().map(|preview| {
            view! {
                <EditInlineDiffContentComponent
                    props=EditInlineDiffContentProps {
                        preview,
                        theme: theme.clone(),
                        prefers_dark,
                        code_preview_settings: Default::default(),
                    }
                />
            }
        });
        view! {
            <div class="space-y-3">
                {inline_diff}
                <ToolCallBody
                    display_model=display_model.clone()
                    inline_preview_override=patch_override.clone()
                    tool_call=legacy.clone()
                    workspace_path=workspace_path.clone()
                    child_tool_list=child_render(&child_tool_calls)
                />
            </div>
        }
        .into_any()
    };

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                can_toggle: Some(EDIT_LAYOUT_CAN_TOGGLE),
                force_open: Some(EDIT_LAYOUT_FORCE_OPEN),
                kind_label: Some(kind_label.to_string()),
                source_label: props.source_label.clone(),
                primary_text: None,
                primary_file_chip: Some(crate::ToolCallBlocks::ToolLayout::FileChipData {
                    path: props.summary.path.clone(),
                    file_name: props.summary.file_name.clone(),
                    icon_src: props.summary.file_icon_src.clone(),
                    // 真源 :392 —— clickable = Boolean(onOpenCodeViewer)，Rust 侧无
                    // 码查看器回调通道 → 恒静态 chip。
                    clickable: false,
                }),
                prioritize_primary_text: Some(true),
                secondary_text: None,
                secondary_text_view: Some(std::sync::Arc::new(move || {
                    view! {
                        <crate::ToolCallBlocks::renderers::RenderFilePath path=file_path.clone() />
                    }
                    .into_any()
                })),
                diff_count: diff_view.map(|v| (v.added, v.removed)),
                status_label: Some(props.status_label.clone()),
                status_tooltip,
                show_failure_status: Some(props.is_failed),
                is_running: Some(props.is_running),
                title: Some(title),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=EDIT_TOOL_ICON_CLASS>
                        <crate::app::Icon
                            paths=vec![
                                "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z",
                                "m15 5 4 4",
                            ]
                            circles=vec![]
                        />
                    </span>
                }
                .into_any()
            }))
            render_content=Some(std::sync::Arc::new(render_content))
        />
    }
}

/// 子工具列表 → `childToolList` 节点（真源由 ToolCallBlocks 组装传入）。
fn child_render(children: &[LegacyToolCallNode]) -> Option<ChildrenFn> {
    if children.is_empty() {
        return None;
    }
    let nodes = children.to_vec();
    Some(std::sync::Arc::new(move || {
        nodes
            .iter()
            .map(|node| {
                let tool_id = node.tool_call.tool_id.clone();
                view! {
                    <div class="min-w-0" data-tool-id=tool_id>
                        <crate::ToolCallBlocks::ToolCallBlock::ToolCallBlock
                            node=node.clone()
                            context=crate::ToolCallBlocks::ToolCallBlock::ToolCallBlockContext::default()
                        />
                    </div>
                }
                .into_any()
            })
            .collect_view()
            .into_any()
    }))
}

/// EditToolCallBlock 的主体（真源 :85-338）。
#[component]
pub fn EditToolCallBlock(props: EditBlockProps) -> impl IntoView {
    let summaries = props.raw_file_summaries.clone();
    let has_multiple_files = summaries.len() > 1;

    // 真源 :93-95 —— 失败时状态词强制显示「执行失败」。
    let effective_status_label = if props.is_failed {
        Some("执行失败".to_string())
    } else {
        props.status_label.clone()
    };

    // primaryText：多文件时是文件 chip 串（真源 :107-119）。
    // ToolLayout 的 primary_file_chip 只支持单文件 chip，多文件走展开态文案，
    // 这里把首个 chip 作为主文本载体（完整多文件 chip 串在展开区呈现）。
    let chip = summaries
        .first()
        .map(|s| crate::ToolCallBlocks::ToolLayout::FileChipData {
            path: s.path.clone(),
            file_name: s.file_name.clone(),
            icon_src: s.file_icon_src.clone(),
            clickable: true,
        });

    // secondaryText：单文件时显示路径（真源 :120-127）。
    let secondary_text = if summaries.len() == 1 {
        summaries.first().and_then(|s| s.file_path.clone())
    } else {
        None
    };

    // diff 计数（真源 :142-146），office 模式不显示（:322）。
    let total_stat = total_change_stat(&summaries);
    let show_diff = !props.is_office_mode
        && crate::ToolCallBlocks::renderers::diff_count_view(Some((
            total_stat.added,
            total_stat.removed,
        )))
        .is_some();
    let diff_view = crate::ToolCallBlocks::renderers::diff_count_view(Some((
        total_stat.added,
        total_stat.removed,
    )));

    let kind_label = kind_label_text(&edit_kind_label(&props));
    let expanded_primary = expanded_primary_text(summaries.len());

    let status_tooltip = if props.is_failed {
        props.error_text.clone()
    } else {
        None
    };

    // ── renderContent 三态（真源 :188-284）──
    // 闭包捕获：move 闭包不能直接读 props（所有权），全部提前绑定。
    let legacy = props.legacy.clone();
    let display_model = props.display_model.clone();
    let workspace_path = props.workspace_path.clone();
    let child_tool_calls = props.child_tool_calls.clone();
    let theme = props.theme.clone();
    let prefers_dark = props.prefers_dark;
    let notice_refs = props.snapshot_refs.clone();
    let tool_id_base = props.tool_id.clone();
    let notice_cb = props.on_load_full_tool_call_fields.clone();
    let is_running = props.is_running;
    let is_failed = props.is_failed;
    let error_text = props.error_text.clone();
    let source_label = props.source_label.clone();
    let status_label_for_children = effective_status_label.clone().unwrap_or_default();
    // 真源 :285 —— 只有恰好单文件时才构造内联预览（多文件走子块）。
    let single_file_patch = (summaries.len() == 1)
        .then(|| summaries.first().and_then(edit_patch_source))
        .flatten();
    // 真源 :277-279 —— patch 存在时给 ToolCallBody 传 {type:"none"}，
    // 避免 ToolCallBody 再次抢走这份预览。
    let single_file_patch_override = single_file_patch.as_ref().map(|_| ToolInlinePreview::None);

    let render_content = move || {
        let notice = view! {
            <ToolSnapshotFieldNoticeComponent
                props=ToolSnapshotFieldNoticeProps {
                    refs: notice_refs.clone(),
                    tool_id: tool_id_base.clone(),
                    on_load_full_tool_call_fields: notice_cb.clone(),
                }
            />
        };
        if summaries.is_empty() {
            // ① 无摘要 → ToolCallBody 兜底（失败编辑）+ 快照提示。
            view! {
                <ToolCallBody
                    display_model=display_model.clone()
                    inline_preview_override=None
                    tool_call=legacy.clone()
                    workspace_path=workspace_path.clone()
                    child_tool_list=child_render(&child_tool_calls)
                />
                {notice}
            }
            .into_any()
        } else if has_multiple_files {
            // ② 多文件 → 缩进容器逐文件拆块（每块自带 diff content，
            // 可单独展开）+ 快照提示。
            let summary_items = summaries
                .iter()
                .enumerate()
                .map(|(index, summary)| {
                    let block_props = EditFileSummaryBlockProps {
                        summary: summary.clone(),
                        is_running,
                        status_label: status_label_for_children.clone(),
                        error_text: error_text.clone(),
                        // 真源 :224 —— 子块 toolId 带 index 后缀。
                        tool_id: format!("{tool_id_base}:{index}"),
                        is_failed,
                        // 真源 :232 —— 子块不重复显示图标。
                        show_icon: false,
                        source_label: source_label.clone(),
                        display_model: display_model.clone(),
                        legacy: legacy.clone(),
                        child_tool_calls: child_tool_calls.clone(),
                        workspace_path: workspace_path.clone(),
                        theme: theme.clone(),
                        prefers_dark,
                    };
                    view! { <EditFileSummaryBlockComponent props=block_props /> }.into_any()
                })
                .collect_view();
            view! {
                <div class="ml-2 space-y-2 border-border border-l pl-3.5 border-border">
                    {summary_items}
                    {notice}
                </div>
            }
            .into_any()
        } else {
            // ③ 单文件 → 内联 diff + ToolCallBody + 快照提示。
            let inline_diff = single_file_patch.clone().map(|preview| {
                view! {
                    <EditInlineDiffContentComponent
                        props=EditInlineDiffContentProps {
                            preview,
                            theme: theme.clone(),
                            prefers_dark,
                            code_preview_settings: Default::default(),
                        }
                    />
                }
            });
            view! {
                <div class="space-y-3">
                    {inline_diff}
                    <ToolCallBody
                        display_model=display_model.clone()
                        inline_preview_override=single_file_patch_override.clone()
                        tool_call=legacy.clone()
                        workspace_path=workspace_path.clone()
                        child_tool_list=child_render(&child_tool_calls)
                    />
                    {notice}
                </div>
            }
            .into_any()
        }
    };

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                // 真源 :24-27 —— edit 默认可折叠（与 read 相反）。
                can_toggle: Some(EDIT_LAYOUT_CAN_TOGGLE && !props.is_office_mode),
                force_open: Some(EDIT_LAYOUT_FORCE_OPEN),
                kind_label: Some(kind_label.to_string()),
                source_label: props.source_label.clone(),
                primary_text: None,
                primary_file_chip: chip,
                prioritize_primary_text: Some(true),
                expanded_primary_text: expanded_primary.clone(),
                secondary_text: secondary_text.clone(),
                diff_count: if show_diff { diff_view.map(|v| (v.added, v.removed)) } else { None },
                // 真源 :325 —— 多文件时展开后隐藏 diff 计数。
                hide_diff_count_when_open: Some(has_multiple_files),
                status_label: effective_status_label.clone(),
                status_tooltip: status_tooltip.clone(),
                show_failure_status: Some(props.is_failed),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=EDIT_TOOL_ICON_CLASS>
                        <crate::app::Icon
                            paths=vec![
                                "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z",
                                "m15 5 4 4",
                            ]
                            circles=vec![]
                        />
                    </span>
                }.into_any()
            }))
            render_content=Some(std::sync::Arc::new(render_content))
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(
        path: &str,
        patch: Option<&str>,
        stat: Option<(u32, u32)>,
    ) -> RawToolCallFileSummary {
        RawToolCallFileSummary {
            path: path.into(),
            action_label: "Edited".into(),
            operation_kind: "edit".into(),
            file_name: crate::ToolCallBlocks::renderers::get_path_leaf(path).to_string(),
            file_path: Some(crate::ToolCallBlocks::renderers::get_path_leaf(path).to_string()),
            file_icon_src: String::new(),
            change_stat: stat.map(|(added, removed)| ChangeStat { added, removed }),
            patch: patch.map(|p| p.to_string()),
        }
    }

    #[test]
    fn raw_failed_detection() {
        // 真源 :35-43
        assert!(is_raw_tool_call_failed(
            &serde_json::json!({ "status": "failed" })
        ));
        assert!(!is_raw_tool_call_failed(
            &serde_json::json!({ "status": "completed" })
        ));
        assert!(!is_raw_tool_call_failed(&serde_json::json!([])));
        assert!(!is_raw_tool_call_failed(&serde_json::json!(null)));
    }

    #[test]
    fn code_viewer_source_uses_path_not_file_path() {
        // ★真源 :58-60 —— filePath 是展示用目录路径，**不能**传给 side pane。
        let s = summary("/a/b/main.rs", Some("diff"), None);
        match build_edit_code_viewer_source(&s) {
            CodeViewerSource::Patch { path, patch, .. } => {
                assert_eq!(path, "/a/b/main.rs", "必须是真实文件路径");
                assert_eq!(patch, "diff");
            }
            other => panic!("期望 Patch，实际 {other:?}"),
        }
    }

    #[test]
    fn code_viewer_source_falls_back_to_file() {
        // 无 patch → File 形态。
        let s = summary("/a/b/main.rs", None, None);
        assert_eq!(
            build_edit_code_viewer_source(&s),
            CodeViewerSource::File {
                title: "main.rs".into(),
                path: "/a/b/main.rs".into(),
            }
        );
    }

    #[test]
    fn inline_preview_only_for_patch() {
        // 真源 :66-82 —— 无 patch 时没有 inline preview。
        assert!(build_edit_inline_preview(Some(&summary("/a.rs", None, None))).is_none());
        assert!(build_edit_inline_preview(Some(&summary("/a.rs", Some("d"), None))).is_some());
        assert!(build_edit_inline_preview(None).is_none());
    }

    #[test]
    fn total_change_stat_accumulates() {
        // 真源 :124-141
        let summaries = vec![
            summary("a.rs", None, Some((3, 1))),
            summary("b.rs", None, Some((2, 4))),
            summary("c.rs", None, None), // 无 stat 不参与累加
        ];
        let total = total_change_stat(&summaries);
        assert_eq!(total.added, 5);
        assert_eq!(total.removed, 5);
        // 空列表。
        assert_eq!(
            total_change_stat(&[]),
            ChangeStat {
                added: 0,
                removed: 0
            }
        );
    }

    #[test]
    fn expanded_primary_text_only_for_multiple() {
        // 真源 :170-183 —— 多文件展开显「N 个文件」。
        assert_eq!(expanded_primary_text(1), None, "单文件展开沿用收起态");
        assert_eq!(expanded_primary_text(3).as_deref(), Some("3 个文件"));
        // zh-CN.ts:4861 `chat.toolCall.edit.multipleFiles` = "{count} 个文件"
    }

    #[test]
    fn edit_layout_is_collapsible_unlike_read() {
        // 真源 :24-27 —— edit 默认可折叠；read 是 canToggle={false}。
        assert!(EDIT_LAYOUT_CAN_TOGGLE);
        assert!(!EDIT_LAYOUT_FORCE_OPEN);
    }

    #[test]
    fn kind_label_text_covers_all_ids() {
        // 七个 i18n id 都要有中文（zh-CN.ts:4838-4843 + edit 系列）。
        assert_eq!(kind_label_text("chat.toolCall.edit.writing"), "正在写入");
        assert_eq!(kind_label_text("chat.toolCall.edit.updating"), "正在更新");
        assert_eq!(kind_label_text("chat.toolCall.edit.deleting"), "正在删除");
        assert_eq!(kind_label_text("chat.toolCall.edit.editing"), "正在编辑");
        assert_eq!(kind_label_text("chat.toolCall.kind.write"), "写入");
        assert_eq!(kind_label_text("chat.toolCall.kind.delete"), "删除");
        assert_eq!(kind_label_text("chat.toolCall.kind.edit"), "编辑");
        // 未知 id 兜底到「编辑」。
        assert_eq!(kind_label_text("unknown"), "编辑");
    }
}
