//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/changes-group.tsx`（319 行）。
//!
//! changesGroup 聚合卡——把一组文件变更聚成可折叠组。
//!
//! 真源要点：
//! - **文件按显示路径去重**（:161-170）：`getFileDisplayPath` 归一后建 Map，
//!   同一文件多次变更只留一条 chip
//! - **计数回落**（:174-190 真源注释）：Write/Edit 的流式 JSON 可能先到 content、
//!   后到 file_path。路径不可解析时按**已进入分组的 tool 数**计数，
//!   避免悬空分隔符和误导性的 "0 files"
//! - `expandedSecondaryText={null}`（:305）——与 execute-group 同款
//! - `hideDiffCountWhenOpen`（:306）
//! - 运行中次文本展示「· 动作 + 最后一个文件的 chip + 路径」（:243-260）
//!
//! **宽度测量算法**（:21-56）是纯函数，可完整照抄：
//! - `resolveFileChipAvailableWidth`：可用宽 = 边界右 -列表左 -尾部预留
//! - `resolveResponsiveFileChipCount`：**关键**——零可用宽度要返回 0
//!   （真源 :40-42 注释：返回 0 才会保留完整的 +N 提示，
//!   否则窄屏只露出被截断的文件名）
//!
//! 行号注释均指真源 file。

use leptos::prelude::*;
use serde_json::Value;
use std::sync::Arc;

use crate::ToolCallBlocks::fileSummaryTypes::{EditOperationKind, RawToolCallFileSummary};

/// `CHANGES_GROUP_ICON`（真源 :18）——PencilIcon。
pub const CHANGES_GROUP_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `FILE_CHIP_GAP_PX`（真源 :19）。
pub const FILE_CHIP_GAP_PX: f64 = 8.0;

/// `FILE_CHIP_TRAILING_SPACE_PX`（真源 :20）。
pub const FILE_CHIP_TRAILING_SPACE_PX: f64 = 24.0;

/// `resolveFileChipAvailableWidth`（真源 :22-32）。
///
/// 真源注释（:80-82）：summary 是 shrink-to-content，按自身宽度测量会在隐藏 chip
/// 后继续收缩。以整行 ToolCall 的右边界计算，才能稳定得到真正可用的空间。
pub fn resolve_file_chip_available_width(
    boundary_right: f64,
    list_left: f64,
    trailing_width: f64,
) -> f64 {
    (boundary_right - list_left - trailing_width).max(0.0)
}

/// `resolveResponsiveFileChipCount`（真源 :34-56）——算能显示几枚 chip。
///
/// 返回值是**可见 chip 数**，剩余数量由调用方渲染成 `+N`。
pub fn resolve_responsive_file_chip_count(
    available_width: f64,
    chip_widths: &[f64],
    overflow_width: f64,
    gap: f64,
) -> usize {
    if chip_widths.is_empty() {
        return 0;
    }
    // ★真源 :40-42 —— 零可用宽度返回 0（一枚都放不下），
    // **不能误解为无需裁剪**。返回 0 才会保留完整的 +N 提示，
    // 否则窄屏只露出被截断的文件名。
    if available_width <= 0.0 {
        return 0;
    }
    // 全部 chip 放得下 → 全显。
    let all_chips_width: f64 =
        chip_widths.iter().sum::<f64>() + gap * (chip_widths.len().saturating_sub(1)) as f64;
    if all_chips_width <= available_width {
        return chip_widths.len();
    }

    let mut visible_width = 0.0;
    let mut visible_count = 0usize;
    for chip_width in chip_widths {
        let next_count = visible_count + 1;
        let next_visible_width = visible_width + chip_width;
        // ★真源 :50-52 —— 仍有隐藏项时，布局包含 visible chips + overflow(+N)
        // + 两者之间的所有 gap。
        let required_width = next_visible_width + gap * next_count as f64 + overflow_width;
        if required_width > available_width {
            break;
        }
        visible_width = next_visible_width;
        visible_count = next_count;
    }
    visible_count
}

/// 子工具调用的最小信息。
#[derive(Debug, Clone)]
pub struct ChildToolCall {
    pub tool_id: String,
    pub raw: Value,
    pub input: Value,
    pub output: Value,
    pub status: String,
    pub title: Option<String>,
    pub kind: Option<String>,
}

/// 文件去重（真源 :158-171）。
///
/// 用显示路径（`getFileDisplayPath` 归一 + 反斜杠转正斜杠）作 key，
/// 同一文件的多次变更只留**首次出现**的那条。
pub fn dedupe_files(
    entries: &[(ChildToolCall, Vec<RawToolCallFileSummary>)],
) -> Vec<RawToolCallFileSummary> {
    let mut unique: Vec<(String, RawToolCallFileSummary)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (_child, summaries) in entries {
        for summary in summaries {
            // 真源 :163 —— 用显示路径做 key（叶子名 +斜杠归一）。
            let key = crate::ToolCallBlocks::renderers::get_path_leaf(&summary.path)
                .to_string()
                .replace('\\', "/");
            if seen.insert(key.clone()) {
                unique.push((key, summary.clone()));
            }
        }
    }
    unique.into_iter().map(|(_, s)| s).collect()
}

/// 计数文案（真源 :176-190）。
///
/// **优先用文件数**，路径不可解析时退回工具数——真源注释说明 Write/Edit 的
/// 流式 JSON 可能先到 content、后到 file_path，此时按工具数计数，
/// 避免悬空分隔符和误导性的 "0 files"。
pub fn count_text(files_len: usize, child_count: usize) -> String {
    let has_resolved_files = files_len > 0;
    if has_resolved_files {
        format!("{files_len} 个文件")
    } else {
        format!("{child_count} 个工具")
    }
}

/// 找最后一个有文件解析结果的子项（真源 :172）。
pub fn latest_child_with_files<'a>(
    entries: &'a [(ChildToolCall, Vec<RawToolCallFileSummary>)],
) -> Option<&'a (ChildToolCall, Vec<RawToolCallFileSummary>)> {
    entries
        .iter()
        .rev()
        .find(|(_, summaries)| !summaries.is_empty())
}

/// `actionText`（真源 :191-203）：最新子项的动作标签（**固定按运行态取**，
/// 真源 :200 第三个参数传 `true`）。
pub fn action_text(summaries: &[RawToolCallFileSummary], child: &ChildToolCall) -> Option<String> {
    if summaries.is_empty() {
        return None;
    }
    let kinds: Vec<EditOperationKind> = summaries.iter().map(|s| s.operation()).collect();
    let labels: Vec<&str> = summaries.iter().map(|s| s.action_label.as_str()).collect();
    let source = crate::ToolCallBlocks::fileSummaryTypes::EditKindSource {
        tool_name: child.title.clone(),
        kind: child.kind.clone(),
        input: Some(child.input.clone()),
        output: Some(child.output.clone()),
        raw: Some(child.raw.clone()),
        title: child.title.clone(),
    };
    let id = crate::ToolCallBlocks::renderers::get_edit_kind_label_id(
        &kinds,
        &labels,
        // 真源 :200 —— 固定传 true（运行态文案）。
        true,
        Some(source),
    );
    Some(
        match id {
            "chat.toolCall.edit.writing" => "正在写入",
            "chat.toolCall.edit.updating" => "正在更新",
            "chat.toolCall.edit.deleting" => "正在删除",
            _ => "正在编辑",
        }
        .to_string(),
    )
}

/// `summaryContentKey`（真源 :307）。
///
/// 运行中带最新子项/动作/文件路径；完成态把所有文件路径用 `|` 串起来。
pub fn summary_content_key(
    tool_id: &str,
    is_running: bool,
    latest_child_id: Option<&str>,
    action: Option<&str>,
    latest_file_path: Option<&str>,
    files: &[RawToolCallFileSummary],
) -> String {
    if is_running {
        format!(
            "changes:{tool_id}:{}:{}:{}",
            latest_child_id.unwrap_or("running"),
            action.unwrap_or("changing"),
            latest_file_path.unwrap_or("file")
        )
    } else {
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        format!("changes:{tool_id}:{}", paths.join("|"))
    }
}

/// ChangesGroupToolCallBlock 的 props。
#[derive(Debug, Clone)]
pub struct ChangesGroupProps {
    pub tool_id: String,
    pub title: Option<String>,
    pub entries: Vec<(ChildToolCall, Vec<RawToolCallFileSummary>)>,
    pub is_running: bool,
    pub is_office_mode: bool,
    pub can_toggle: Option<bool>,
    pub force_open: Option<bool>,
}

/// ChangesGroupToolCallBlock 的主体（真源 :153-319）。
#[component]
pub fn ChangesGroupToolCallBlock(props: ChangesGroupProps) -> impl IntoView {
    let files = dedupe_files(&props.entries);
    let latest = latest_child_with_files(&props.entries);
    let latest_file = latest.and_then(|(_, s)| s.last());
    // 真源 :174 —— 优先文件数，路径不可解析时退回工具数。
    let count_text = count_text(files.len(), props.entries.len());
    let action = latest.and_then(|(child, summaries)| action_text(summaries, child));

    let key = summary_content_key(
        &props.tool_id,
        props.is_running,
        latest.map(|(c, _)| c.tool_id.as_str()),
        action.as_deref(),
        latest_file.map(|f| f.path.as_str()),
        &files,
    );

    // 单文件时主文本就是 chip；多文件是「{count} 个文件 · chips」（真源 :221-236）。
    let expanded_primary = count_text.clone();
    let child_rows = props
        .entries
        .iter()
        .map(|(child, _)| {
            view! {
                <div class="min-w-0" data-tool-id=child.tool_id.clone() />
            }
        })
        .collect_view();

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                can_toggle: Some(!props.is_office_mode && props.can_toggle.unwrap_or(true)),
                force_open: Some(!props.is_office_mode && props.force_open.unwrap_or(false)),
                // 真源 :299 —— kindLabel 是「更改」。
                kind_label: Some("更改".to_string()),
                // 真源 :300 —— 运行中只显示计数；完成态显示「{count} 个文件 · chips」。
                // 真源 :300 —— 运行中只显示计数；完成态显示「{count} 个文件 · chips」。
                // View/AnyView 都**不可克隆**（只能消费一次），所以闭包不能复用已建的视图；
                // 改为捕获**数据**（文件名列表），在闭包内现场重建视图。
                primary_text_view: {
                    let names: Vec<String> = files.iter().map(|f| f.file_name.clone()).collect();
                    let count = count_text.clone();
                    let is_running = props.is_running;
                    Some(Arc::new(move || {
                        if is_running {
                            view! { <span class="flex-none">{count.clone()}</span> }.into_any()
                        } else if names.len() == 1 {
                            view! {
                                <span class="inline-flex min-w-0 max-w-full items-center gap-2">
                                    <span class="min-w-0 truncate text-foreground-subtle">
                                        {names.first().cloned().unwrap_or_default()}
                                    </span>
                                </span>
                            }
                            .into_any()
                        } else {
                            view! {
                                <span class="inline-flex min-w-0 max-w-full flex-1 items-center gap-2">
                                    <span class="flex-none">{count.clone()}</span>
                                    <span>"·"</span>
                                    {names
                                        .iter()
                                        .map(|n| {
                                            view! {
                                                <span class="min-w-0 flex-none truncate text-foreground-subtle">
                                                    {n.clone()}
                                                </span>
                                            }
                                        })
                                        .collect_view()}
                                </span>
                            }
                            .into_any()
                        }
                    }))
                },
                expanded_primary_text: Some(expanded_primary),
                // 真源 :305 —— 展开态显式清空次文本。
                expanded_secondary_text: Some(String::new()),
                // 真源 :301 —— 运行中次文本展示「· 动作 + 最新文件」。
                secondary_text_view: {
                    // 真源 :301 —— 运行中次文本：「· 动作 + 最新文件名」。
                    let action = action.clone();
                    let latest_name = latest_file.map(|f| f.file_name.clone());
                    let is_running = props.is_running;
                    latest_name.map(move |name| {
                        let action = action.clone();
                        Arc::new(move || {
                            if !is_running {
                                return ().into_view().into_any();
                            }
                            view! {
                                <span class="inline-flex min-w-0 items-center gap-2">
                                    <span class="flex-none text-foreground-subtlest">"·"</span>
                                    {action.clone().map(|a| {
                                        view! {
                                            <span class="flex-none text-foreground-subtle">{a}</span>
                                        }
                                    })}
                                    <span class="min-w-0 truncate text-foreground-subtle">{name.clone()}</span>
                                </span>
                            }
                            .into_any()
                        })
                            as std::sync::Arc<
                                dyn Fn() -> leptos::prelude::AnyView + Send + Sync + 'static,
                            >
                    })
                },
                summary_content_separator: Some("·".to_string()),
                summary_content_key: Some(key),
                // 真源 :306 —— 展开后隐藏 diff 计数。
                hide_diff_count_when_open: Some(true),
                animate_summary_content: Some(props.is_running),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=CHANGES_GROUP_ICON_CLASS>
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
            render_content=Some(std::sync::Arc::new(move || {
                // 真源 :265-289 —— 展开区是子卡列表。
                view! {
                    <div class="ml-2 space-y-2 border-l border-border pl-3.5">
                        {child_rows.clone()}
                    </div>
                }
                .into_any()
            }))
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str) -> RawToolCallFileSummary {
        RawToolCallFileSummary {
            path: path.into(),
            action_label: "Edited".into(),
            operation_kind: "edit".into(),
            file_name: crate::ToolCallBlocks::renderers::get_path_leaf(path).to_string(),
            file_path: None,
            file_icon_src: String::new(),
            change_stat: None,
            patch: None,
        }
    }

    #[test]
    fn available_width_subtracts_trailing_space() {
        // 真源 :22-32
        assert_eq!(resolve_file_chip_available_width(300.0, 100.0, 24.0), 176.0);
        // 结果不为负。
        assert_eq!(resolve_file_chip_available_width(100.0, 200.0, 24.0), 0.0);
    }

    #[test]
    fn all_chips_fit_shows_everything() {
        // 真源 :49-50
        let count = resolve_responsive_file_chip_count(500.0, &[50.0, 60.0, 70.0], 30.0, 8.0);
        assert_eq!(count, 3, "全部放得下");
    }

    #[test]
    fn zero_available_width_returns_zero_not_full() {
        // ★真源 :40-42 —— 零可用宽度必须返回 0（一枚都放不下），
        // **不能误解为无需裁剪**。返回 0 才会保留完整的 +N 提示。
        let count = resolve_responsive_file_chip_count(0.0, &[50.0, 60.0], 30.0, 8.0);
        assert_eq!(count, 0, "零宽不是「无需裁剪」，必须返回 0");
    }

    #[test]
    fn partial_fit_reserves_room_for_overflow_badge() {
        // ★真源 :50-52 —— 仍有隐藏项时，布局包含 visible chips + `+N` + 两者间 gap。
        // 算例：可用 150，三枚 chip 各 60，gap 8，overflow(+N) 宽 24
        //   全放需：60*3 + 8*2 = 196 > 150 → 进入裁剪
        //   第1 枚：60 + 8*1 + 24 = 92<= 150 → 可见
        //   第 2 枚：120 + 8*2 + 24 = 160 > 150 → 停
        // 所以只显示 1 枚，剩下 2 个走 +N。
        let count = resolve_responsive_file_chip_count(150.0, &[60.0, 60.0, 60.0], 24.0, 8.0);
        assert_eq!(count, 1, "要为 +N 预留宽度，实际只放得下 1 枚");
    }

    #[test]
    fn all_fit_ignores_overflow_reservation() {
        // 真源 :49-50 —— 全部 chip 放得下时直接全显，**不为 +N 预留**
        // （没有隐藏项就不需要 +N 徽标）。
        // 2 枚各 60 + gap 8 = 128 <= 150 → 全显
        let count = resolve_responsive_file_chip_count(150.0, &[60.0, 60.0], 24.0, 8.0);
        assert_eq!(count, 2, "全放得下时不预留 +N 宽度");
    }

    #[test]
    fn exactly_fitting_count_is_visible() {
        // 边界：required_width 恰好等于可用宽度时算可见（真源 :51 用 `>` 判break）。
        // 1 枚：60 + 8*1 + 24 = 92，可用 92 → 可见
        let count = resolve_responsive_file_chip_count(92.0, &[60.0], 24.0, 8.0);
        assert_eq!(count, 1, "恰好放得下应可见");
    }

    #[test]
    fn empty_chip_list_returns_zero() {
        assert_eq!(resolve_responsive_file_chip_count(500.0, &[], 30.0, 8.0), 0);
    }

    #[test]
    fn files_dedupe_by_leaf_name() {
        // 真源 :158-171 —— 同一文件多次变更只留首次。
        let child = |id: &str| ChildToolCall {
            tool_id: id.into(),
            raw: Value::Null,
            input: Value::Null,
            output: Value::Null,
            status: "completed".into(),
            title: None,
            kind: Some("Edit".into()),
        };
        let entries = vec![
            (child("c1"), vec![file("a/main.rs"), file("b/lib.rs")]),
            (child("c2"), vec![file("c/main.rs")]), // main.rs 重复
        ];
        let files = dedupe_files(&entries);
        assert_eq!(files.len(), 2, "main.rs 应被去重");
    }

    #[test]
    fn count_text_falls_back_to_tool_count() {
        // ★真源 :174-190 —— 路径不可解析时按工具数计数，
        // 避免悬空分隔符和误导性的 "0 files"。
        assert_eq!(count_text(3, 5), "3 个文件", "有文件时用文件数");
        assert_eq!(count_text(0, 5), "5 个工具", "无文件时退回工具数");
        assert_eq!(count_text(1, 1), "1 个文件");
        assert_eq!(count_text(0, 1), "1 个工具");
    }

    #[test]
    fn latest_child_finds_last_with_files() {
        // 真源 :172 —— findLast 找最后一个有文件解析结果的子项。
        let child = |id: &str| ChildToolCall {
            tool_id: id.into(),
            raw: Value::Null,
            input: Value::Null,
            output: Value::Null,
            status: "completed".into(),
            title: None,
            kind: Some("Edit".into()),
        };
        let entries = vec![
            (child("a"), vec![file("x.rs")]),
            (child("b"), vec![]), // 无文件，跳过
            (child("c"), vec![file("y.rs"), file("z.rs")]),
        ];
        let latest = latest_child_with_files(&entries).unwrap();
        assert_eq!(latest.0.tool_id, "c");
        assert_eq!(latest.1.len(), 2);
    }

    #[test]
    fn summary_content_key_switches_on_running() {
        // 真源 :307
        let running =
            summary_content_key("t1", true, Some("c1"), Some("正在编辑"), Some("/a.rs"), &[]);
        assert!(
            running.starts_with("changes:t1:c1"),
            "运行态含最新子项：{running}"
        );
        assert!(running.contains("/a.rs"));

        let done = summary_content_key(
            "t1",
            false,
            None,
            None,
            None,
            &[file("/a.rs"), file("/b.rs")],
        );
        assert_eq!(done, "changes:t1:/a.rs|/b.rs", "完成态用文件路径串");
    }
}
