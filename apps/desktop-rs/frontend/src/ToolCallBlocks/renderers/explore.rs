//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/explore.tsx`（490 行）。
//!
//! Explore 聚合卡：parent 摘要 = 子工具归桶统计（「3 搜索, 2 文件」），
//! 运行中折叠态显示**最新子工具**的实时摘要（读/搜/终端各自形态），
//! 展开 = 递归子卡列表。
//!
//! ## 裁剪注明
//!
//! **已接线**：`ToolSnapshotFieldNoticeComponent`——refs 空或宿主未接回调时
//! 不渲染（真源 `return null` 语义；v4 投影下 snapshotRefs 无生产者，恒不显示）。
//! - read chip 的点击（`onOpenCodeViewer`）：Rust 无代码查看器回调，
//!   chip 恒静态（真源 `canOpenPreview = entryType==="file" && onOpenCodeViewer`）；
//! - `collectCommandStrings`（:70-122）：与 `exploreToolCall.rs` 的
//!   `extract_tool_commands` 逐语义等价（unwrap + split + trim + dedupe），
//!   复用后者不重复迁移（唯一差异 " ; " join 在调用点处理）。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};

use super::super::toolCallRowAdapter::LegacyToolCallNode;
use super::read::build_read_summary;

/// 真源 :17 —— `SearchIcon className="size-4 shrink-0 text-foreground-subtle"`。
pub const EXPLORE_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `ExploreBucket`（真源 :124）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExploreBucket {
    Search,
    List,
    File,
}

/// `getBucketLabel`（真源 :126-150）——中文文案（zh-CN.ts:5629-5635，单复同形）。
pub fn get_bucket_label(bucket: ExploreBucket) -> &'static str {
    match bucket {
        ExploreBucket::Search => "搜索",
        ExploreBucket::List => "列表",
        ExploreBucket::File => "文件",
    }
}

/// `classifyExploreToolCall`（真源 :152-179）。
///
/// fingerprint = `kind title`，command = 命令串 join(" ; ")——
/// 三条正则判 search / list / 默认 file。
pub fn classify_explore_tool_call(kind: &str, title: Option<&str>, input: &Value) -> ExploreBucket {
    let fingerprint = format!("{} {}", kind, title.unwrap_or_default()).to_lowercase();
    let command = super::super::exploreToolCall::extract_tool_commands(input)
        .join(" ; ")
        .to_lowercase();

    if search_fingerprint_re().is_match(&fingerprint) || search_command_re().is_match(&command) {
        return ExploreBucket::Search;
    }
    if list_fingerprint_re().is_match(&fingerprint) || list_command_re().is_match(&command) {
        return ExploreBucket::List;
    }
    ExploreBucket::File
}

fn search_fingerprint_re() -> &'static regex_lite::Regex {
    static RE: std::sync::OnceLock<regex_lite::Regex> = std::sync::OnceLock::new();
    // 真源 :163 —— /(\bgrep\b|\bsearch\b|\bfetch\b|\bweb.?search\b|\bweb.?fetch\b)/i
    RE.get_or_init(|| {
        regex_lite::Regex::new(
            r"(?i)(\bgrep\b|\bsearch\b|\bfetch\b|\bweb.?search\b|\bweb.?fetch\b)",
        )
        .expect("search fingerprint 正则")
    })
}

fn search_command_re() -> &'static regex_lite::Regex {
    static RE: std::sync::OnceLock<regex_lite::Regex> = std::sync::OnceLock::new();
    // 真源 :164 —— /(^|\s)(rg|grep|ripgrep|git\s+grep)(\s|$)/i
    RE.get_or_init(|| {
        regex_lite::Regex::new(r"(?i)(^|\s)(rg|grep|ripgrep|git\s+grep)(\s|$)")
            .expect("search command 正则")
    })
}

fn list_fingerprint_re() -> &'static regex_lite::Regex {
    static RE: std::sync::OnceLock<regex_lite::Regex> = std::sync::OnceLock::new();
    // 真源 :171 —— /(\bglob\b|\bfind\b|\blist\b|\btree\b|\bdir\b|\bls\b)/i
    RE.get_or_init(|| {
        regex_lite::Regex::new(r"(?i)(\bglob\b|\bfind\b|\blist\b|\btree\b|\bdir\b|\bls\b)")
            .expect("list fingerprint 正则")
    })
}

fn list_command_re() -> &'static regex_lite::Regex {
    static RE: std::sync::OnceLock<regex_lite::Regex> = std::sync::OnceLock::new();
    // 真源 :172 —— /(^|\s)(ls|find|tree|dir)(\s|$)/i
    RE.get_or_init(|| {
        regex_lite::Regex::new(r"(?i)(^|\s)(ls|find|tree|dir)(\s|$)").expect("list command 正则")
    })
}

/// `formatExploreSummary`（真源 :181-199）——归桶统计拼接。
///
/// 空则 `chat.toolCall.explore.emptySummary` = "0 个文件"。
pub fn format_explore_summary(search: usize, list: usize, file: usize) -> String {
    let mut parts: Vec<String> = Vec::new();
    if search > 0 {
        parts.push(format!(
            "{search} {}",
            get_bucket_label(ExploreBucket::Search)
        ));
    }
    if list > 0 {
        parts.push(format!("{list} {}", get_bucket_label(ExploreBucket::List)));
    }
    if file > 0 {
        parts.push(format!("{file} {}", get_bucket_label(ExploreBucket::File)));
    }
    if parts.is_empty() {
        "0 个文件".to_string()
    } else {
        parts.join(", ")
    }
}

/// `getChildActionKindLabel`（真源 :201-216）——前缀动作词。
pub fn get_child_action_kind_label(
    family: super::super::resolveRenderer::ToolFamily,
) -> &'static str {
    use super::super::resolveRenderer::ToolFamily;
    match family {
        ToolFamily::FileRead => "正在读取",
        ToolFamily::Search => "正在搜索",
        ToolFamily::Shell => "正在执行",
        _ => "执行中",
    }
}

/// 子摘要的内容形态（对应真源 `ExploreChildSummary` 的 primaryText/secondaryText
/// ReactNode；Rust 侧拆成数据，由组件组装视图）。
#[derive(Debug, Clone)]
pub enum ExploreChildContent {
    /// file-read：chip（静态）+ 次要路径。
    Read {
        path: String,
        file_name: String,
        file_icon_src: String,
        file_path: Option<String>,
    },
    /// search：主文本。
    Search { text: String },
    /// shell：主文本仅动作词，命令进次要文本（code）。
    Shell { command: String },
    /// todo：动作词 + 进度（如「正在更新待办 2/5 · 跑测试」）。
    Todo {
        action_label: &'static str,
        progress_text: String,
    },
    /// 其他 family：标题/kind 兜底。
    Fallback { text: String },
}

/// 子摘要（真源 `ExploreChildSummary`，:22-27）。
#[derive(Debug, Clone)]
pub struct ExploreChildSummary {
    pub animation_key: String,
    /// 前缀动作词（includeChildActionKindLabel 时）；todo 内部自算不受此控。
    pub action_kind_label: Option<&'static str>,
    pub content: ExploreChildContent,
    pub title: Option<String>,
}

/// `readTodoPlanFromToolCall`（真源 :233-249）→ todo 子摘要（:250-288）。
fn get_todo_child_summary(node: &LegacyToolCallNode) -> ExploreChildSummary {
    let tc = &node.tool_call;
    let plan = super::todo::read_todo_plan(
        tc.title.as_deref(),
        Some(tc.kind.as_str()),
        tc.input.as_ref().unwrap_or(&Value::Null),
        tc.output
            .as_ref()
            .map(|o| Value::String(o.clone()))
            .as_ref()
            .unwrap_or(&Value::Null),
    );

    let Some(plan) = plan.filter(|p| !p.is_empty()) else {
        // 真源 :255-263 —— 无 plan：只显「正在更新待办」。
        return ExploreChildSummary {
            animation_key: format!("todo:{}:empty", tc.tool_id),
            action_kind_label: None,
            content: ExploreChildContent::Todo {
                action_label: "正在更新待办",
                progress_text: String::new(),
            },
            title: tc.title.clone(),
        };
    };

    let completed_count = plan
        .iter()
        .filter(|s| s.status == super::todo::PlanStepStatus::Completed)
        .count();
    // 真源 :270-273 —— in_progress → 第一个未完成 → 最后一个。
    let active = plan
        .iter()
        .find(|s| s.status == super::todo::PlanStepStatus::InProgress)
        .or_else(|| {
            plan.iter()
                .find(|s| s.status != super::todo::PlanStepStatus::Completed)
        })
        .or_else(|| plan.last());
    let is_complete = completed_count == plan.len();
    let action_label = if is_complete {
        "已更新待办"
    } else {
        "正在更新待办"
    };
    let progress_text = match active {
        Some(step) if !step.title.is_empty() => {
            format!("{completed_count}/{} · {}", plan.len(), step.title)
        }
        _ => format!("{completed_count}/{}", plan.len()),
    };

    ExploreChildSummary {
        animation_key: format!(
            "todo:{}:{}:{}:{}:{}",
            tc.tool_id,
            completed_count,
            plan.len(),
            active.map(|s| s.id.as_str()).unwrap_or("none"),
            active.map(|s| s.title.as_str()).unwrap_or(""),
        ),
        action_kind_label: None,
        content: ExploreChildContent::Todo {
            action_label,
            progress_text,
        },
        title: active
            .map(|s| s.title.clone())
            .filter(|t| !t.is_empty())
            .or_else(|| tc.title.clone()),
    }
}

/// `getLatestExploreChildSummary`（真源 :290-367）。
pub fn get_latest_explore_child_summary(
    node: &LegacyToolCallNode,
    include_child_action_kind_label: bool,
) -> Option<ExploreChildSummary> {
    use super::super::resolveRenderer::ToolFamily;
    let tc = &node.tool_call;
    let family = super::super::resolveRenderer::family_by_lower(
        tc.tool_name.as_deref().unwrap_or(tc.kind.as_str()),
    );
    let action_kind_label =
        include_child_action_kind_label.then(|| get_child_action_kind_label(family));

    if family == ToolFamily::Todo {
        return Some(get_todo_child_summary(node));
    }

    if family == ToolFamily::FileRead {
        // 真源 :307-325 —— buildReadSummary；无则 null。
        let summary = build_read_summary(tc.input.as_ref().unwrap_or(&Value::Null), &tc.raw)?;
        return Some(ExploreChildSummary {
            animation_key: format!(
                "read:{}:{}:{}",
                tc.tool_id,
                action_kind_label.unwrap_or("plain"),
                summary.path
            ),
            action_kind_label,
            content: ExploreChildContent::Read {
                path: summary.path.clone(),
                file_name: summary.file_name,
                file_icon_src: summary.file_icon_src,
                file_path: summary.file_path,
            },
            title: Some(summary.path),
        });
    }

    if family == ToolFamily::Search {
        let primary_text =
            super::search::get_search_primary_text(tc.input.as_ref().unwrap_or(&Value::Null));
        return Some(ExploreChildSummary {
            animation_key: format!(
                "search:{}:{}:{}",
                tc.tool_id,
                action_kind_label.unwrap_or("plain"),
                primary_text
            ),
            action_kind_label,
            content: ExploreChildContent::Search {
                text: primary_text.clone(),
            },
            title: Some(primary_text),
        });
    }

    if family == ToolFamily::Shell {
        // 真源 :342-347 —— 命令 → title → kind 三级回落。
        let command =
            super::execute::get_execute_secondary_text(tc.input.as_ref().unwrap_or(&Value::Null))
                .or_else(|| tc.title.clone())
                .unwrap_or_else(|| tc.kind.clone());
        return Some(ExploreChildSummary {
            animation_key: format!(
                "shell:{}:{}:{}",
                tc.tool_id,
                action_kind_label.unwrap_or("plain"),
                command
            ),
            action_kind_label,
            content: ExploreChildContent::Shell {
                command: command.clone(),
            },
            title: Some(command),
        });
    }

    // 真源 :358-367 —— 其他 family 兜底。
    let fallback_text = tc
        .title
        .clone()
        .or_else(|| Some(tc.kind.clone()))
        .unwrap_or_else(|| "0 个文件".to_string());
    Some(ExploreChildSummary {
        animation_key: format!(
            "tool:{}:{}:{}",
            tc.tool_id,
            action_kind_label.unwrap_or("plain"),
            fallback_text
        ),
        action_kind_label,
        content: ExploreChildContent::Fallback {
            text: fallback_text.clone(),
        },
        title: Some(fallback_text),
    })
}

/// `getLatestExploreChildSummaryFromChildren`（真源 :369-389）——
/// 倒序找第一个有摘要的子工具。
pub fn get_latest_explore_child_summary_from_children(
    children: &[LegacyToolCallNode],
    include_child_action_kind_label: bool,
) -> Option<ExploreChildSummary> {
    children
        .iter()
        .rev()
        .find_map(|child| get_latest_explore_child_summary(child, include_child_action_kind_label))
}

/// ExploreToolCallBlock 的 props（真源从 context 解构的字段）。
#[derive(Debug, Clone)]
pub struct ExploreBlockProps {
    pub tool_id: String,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
    pub child_tool_calls: Vec<LegacyToolCallNode>,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    /// 失败判定（真源 `toolCall.status === "failed"`）。
    pub is_failed: bool,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    pub can_toggle: Option<bool>,
    pub force_open: Option<bool>,
}

/// `ExploreToolCallBlock`（真源 :391-490）。
#[component]
pub fn ExploreToolCallBlock(props: ExploreBlockProps) -> impl IntoView {
    // counts 归桶（真源 :394-402）。
    let (mut n_search, mut n_list, mut n_file) = (0usize, 0usize, 0usize);
    for child in &props.child_tool_calls {
        match classify_explore_tool_call(
            &child.tool_call.kind,
            child.tool_call.title.as_deref(),
            child.tool_call.input.as_ref().unwrap_or(&Value::Null),
        ) {
            ExploreBucket::Search => n_search += 1,
            ExploreBucket::List => n_list += 1,
            ExploreBucket::File => n_file += 1,
        }
    }
    let summary = format_explore_summary(n_search, n_list, n_file);
    let kind_label = "查阅";

    // 折叠态实时摘要（真源 :403-407，运行中才取）。
    let collapsed = props
        .is_running
        .then(|| get_latest_explore_child_summary_from_children(&props.child_tool_calls, true))
        .flatten();

    // 折叠态主文本视图（消费型闭包，重建多次）。
    let primary_view_source = collapsed.clone();
    let primary_view: Option<ChildrenFn> = primary_view_source
        .map(|c| std::sync::Arc::new(move || explore_summary_primary_view(&c)) as ChildrenFn);
    let secondary_view_source = collapsed.clone();
    let secondary_view: Option<ChildrenFn> = secondary_view_source
        .map(|c| std::sync::Arc::new(move || explore_summary_secondary_view(&c)) as ChildrenFn);

    let summary_content_key = collapsed
        .as_ref()
        .map(|c| c.animation_key.clone())
        .unwrap_or(format!(
            "explore:{}:{}:{}",
            props.tool_id,
            if props.is_running { "running" } else { "done" },
            summary
        ));
    let title = collapsed
        .as_ref()
        .and_then(|c| c.title.clone())
        .unwrap_or_else(|| {
            if props.is_running {
                summary.clone()
            } else {
                props.title.clone().unwrap_or_else(|| summary.clone())
            }
        });

    // children 递归（View 非 Clone：存数据、闭包内重建）。
    let child_context = crate::ToolCallBlocks::ToolCallBlock::ToolCallBlockContext {
        show_icon: false,
        ..Default::default()
    };
    let children_data: Vec<(String, LegacyToolCallNode)> = props
        .child_tool_calls
        .iter()
        .map(|child| (child.tool_call.tool_id.clone(), child.clone()))
        .collect();
    let has_children = !children_data.is_empty();

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                can_toggle: Some(props.can_toggle.unwrap_or(true)),
                force_open: Some(props.force_open.unwrap_or(false)),
                kind_label: Some(kind_label.to_string()),
                expanded_kind_label: Some(kind_label.to_string()),
                source_label: props.source_label.clone(),
                // 折叠态无实时摘要时用归桶统计（真源 :403/:465 两分支）。
                primary_text: Some(summary.clone()),
                primary_text_view: primary_view,
                expanded_primary_text: Some(summary.clone()),
                secondary_text_view: secondary_view,
                // 真源 :469 —— 展开态显式清空次文本。
                expanded_secondary_text: Some(String::new()),
                summary_content_separator: Some("·".to_string()),
                animate_summary_content: Some(true),
                disable_summary_content_animation: Some(false),
                summary_content_key: Some(summary_content_key),
                status_label: props.status_label.clone(),
                status_tooltip: props.is_failed.then(|| props.error_text.clone()).flatten(),
                show_failure_status: Some(props.is_failed),
                is_running: Some(props.is_running),
                title: Some(title),
                expanded_title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=EXPLORE_TOOL_ICON_CLASS>
                        <crate::app::Icon
                            paths=vec!["m21 21-4.34-4.34"]
                            circles=vec![("11", "11", "8")]
                        />
                    </span>
                }
                .into_any()
            }))
            render_content=Some(std::sync::Arc::new(move || {
                if !has_children {
                    return ().into_any();
                }
                // 真源 :434-436 —— 缩进容器 + 递归子卡（showIcon=false）。
                let child_views = children_data
                    .iter()
                    .map(|(tool_id, node)| {
                        view! {
                            <div class="min-w-0" data-tool-id=tool_id.clone()>
                                <crate::ToolCallBlocks::ToolCallBlock::ToolCallBlock
                                    node=node.clone()
                                    context=child_context.clone()
                                />
                            </div>
                        }
                    })
                    .collect_view();
                view! {
                    <div class="ml-2 space-y-2 border-l border-border pl-3.5">
                        {child_views}
                    </div>
                }
                .into_any()
            }))
        />
        <ToolSnapshotFieldNoticeComponent
            props=ToolSnapshotFieldNoticeProps {
                refs: props.snapshot_refs.clone(),
                tool_id: props.tool_id.clone(),
                on_load_full_tool_call_fields: props.on_load_full_tool_call_fields.clone(),
            }
        />
    }
}

/// 折叠态主文本节点：按内容形态组装（真源 `primaryText` 的 ReactNode 等价）。
///
/// 公开供 agent.rs 复用（Agent 卡的折叠态实时摘要与 explore 同款——
/// 真源 agent.tsx:256 直接调用 `getLatestExploreChildSummaryFromChildren`）。
pub fn explore_summary_primary_view(summary: &ExploreChildSummary) -> AnyView {
    let label_prefix = summary.action_kind_label.map(|label| {
        view! { <span class="shrink-0 text-foreground-subtle">{label}</span> }
    });
    match &summary.content {
        ExploreChildContent::Read {
            file_name,
            file_icon_src,
            ..
        } => {
            // 真源 :318-322 —— ReadFileChip（静态分支：v1 无 onOpenCodeViewer，
            // clickable 恒 false，类名同 read.rs 的 READ_CHIP_STATIC_CLASS）。
            let name = file_name.clone();
            let icon = file_icon_src.clone();
            view! {
                <span class="inline-flex min-w-0 items-center gap-2">
                    {label_prefix}
                    <span class=super::read::READ_CHIP_STATIC_CLASS>
                        <img src=icon class="size-4 flex-none" alt="" />
                        <span class="min-w-0 truncate">{name}</span>
                    </span>
                </span>
            }
            .into_any()
        }
        ExploreChildContent::Search { text } => {
            let text = text.clone();
            view! {
                <span class="inline-flex min-w-0 items-center gap-2">
                    {label_prefix}
                    <span class="min-w-0 truncate">{text}</span>
                </span>
            }
            .into_any()
        }
        ExploreChildContent::Shell { .. } => {
            // 真源 :347-350 —— shell 的 primaryText 只有动作词（命令在次文本）。
            view! {
                <span class="inline-flex min-w-0 items-center gap-2">{label_prefix}</span>
            }
            .into_any()
        }
        ExploreChildContent::Todo {
            action_label,
            progress_text,
        } => {
            // 真源 :281-286 —— todo 的 primaryText 是「动作词 + 进度」。
            let action = *action_label;
            let progress = progress_text.clone();
            view! {
                <span class="inline-flex min-w-0 items-center gap-2">
                    <span class="shrink-0 text-foreground-subtle">{action}</span>
                    <span class="min-w-0 truncate">{progress}</span>
                </span>
            }
            .into_any()
        }
        ExploreChildContent::Fallback { text } => {
            let text = text.clone();
            view! {
                <span class="inline-flex min-w-0 items-center gap-2">
                    {label_prefix}
                    <span class="min-w-0 truncate">{text}</span>
                </span>
            }
            .into_any()
        }
    }
}

/// 折叠态次文本节点（真源 `secondaryText`）。公开供 agent.rs 复用。
pub fn explore_summary_secondary_view(summary: &ExploreChildSummary) -> AnyView {
    match &summary.content {
        ExploreChildContent::Read { file_path, .. } => match file_path {
            Some(p) if !p.is_empty() => {
                let p = p.clone();
                view! {
                    <span class="min-w-0 truncate text-foreground-subtlest">{p}</span>
                }
                .into_any()
            }
            _ => ().into_any(),
        },
        ExploreChildContent::Shell { command } => {
            let command = command.clone();
            // 真源 :351-353 —— Tailwind v4 preflight 的 mono 默认要显式改回
            // font-sans（运行态命令与收起态 sans 约定一致）。
            view! {
                <code class="min-w-0 truncate font-sans">{command}</code>
            }
            .into_any()
        }
        _ => ().into_any(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolCallBlocks::toolCallRowAdapter::tool_call_row_to_legacy_node;
    use serde_json::json;

    fn node(name: &str, input: Value) -> LegacyToolCallNode {
        tool_call_row_to_legacy_node(&json!({
            "kind": "toolCall", "rowId": 1, "toolCallId": format!("tc-{name}"),
            "toolName": name, "status": "success", "inputText": "",
            "input": input,
        }))
    }

    #[test]
    fn classify_matches_source_regexes() {
        // 真源 :163-172 —— fingerprint 与 command 两路。
        assert_eq!(
            classify_explore_tool_call("Grep", None, &json!({})),
            ExploreBucket::Search
        );
        assert_eq!(
            classify_explore_tool_call("WebSearch", None, &json!({})),
            ExploreBucket::Search
        );
        assert_eq!(
            classify_explore_tool_call("Bash", None, &json!({"command": "rg -n foo"})),
            ExploreBucket::Search
        );
        assert_eq!(
            classify_explore_tool_call("Glob", None, &json!({})),
            ExploreBucket::List
        );
        assert_eq!(
            classify_explore_tool_call("Bash", None, &json!({"command": "ls -la"})),
            ExploreBucket::List
        );
        // 默认 file。
        assert_eq!(
            classify_explore_tool_call("Read", None, &json!({})),
            ExploreBucket::File
        );
    }

    #[test]
    fn format_summary_joins_nonzero_buckets() {
        // 真源 :181-199。
        assert_eq!(format_explore_summary(3, 0, 2), "3 搜索, 2 文件");
        assert_eq!(format_explore_summary(0, 1, 0), "1 列表");
        assert_eq!(format_explore_summary(0, 0, 0), "0 个文件");
    }

    #[test]
    fn child_summary_read_produces_chip_content() {
        // 真源 :307-325 —— file-read 出 chip + 路径。
        let n = node("Read", json!({"file_path": "src/main.rs"}));
        let summary = get_latest_explore_child_summary(&n, true).unwrap();
        assert_eq!(summary.action_kind_label, Some("正在读取"));
        match summary.content {
            ExploreChildContent::Read { file_name, .. } => {
                assert_eq!(file_name, "main.rs");
            }
            other => panic!("期望 Read 内容，得 {other:?}"),
        }
    }

    #[test]
    fn child_summary_search_and_shell_texts() {
        let n = node("Grep", json!({"pattern": "foo"}));
        let summary = get_latest_explore_child_summary(&n, true).unwrap();
        assert_eq!(summary.action_kind_label, Some("正在搜索"));
        assert!(matches!(
            summary.content,
            ExploreChildContent::Search { .. }
        ));

        let n = node("Bash", json!({"command": "ls -la"}));
        let summary = get_latest_explore_child_summary(&n, true).unwrap();
        assert_eq!(summary.action_kind_label, Some("正在执行"));
        match summary.content {
            ExploreChildContent::Shell { command } => assert!(command.contains("ls -la")),
            other => panic!("期望 Shell 内容，得 {other:?}"),
        }
    }

    #[test]
    fn latest_from_children_scans_from_tail() {
        // 真源 :369-389 —— 倒序取第一个有摘要者。
        let children = vec![
            node("Read", json!({"file_path": "a.rs"})),
            node("Grep", json!({"pattern": "x"})),
        ];
        let summary = get_latest_explore_child_summary_from_children(&children, true).unwrap();
        assert!(matches!(
            summary.content,
            ExploreChildContent::Search { .. }
        ));
    }

    #[test]
    fn label_toggle_controls_prefix() {
        // includeChildActionKindLabel=false 时前缀为空（真源 :292-296）。
        let n = node("Read", json!({"file_path": "a.rs"}));
        let summary = get_latest_explore_child_summary(&n, false).unwrap();
        assert_eq!(summary.action_kind_label, None);
    }

    #[test]
    fn todo_child_without_plan_shows_updating() {
        // 真源 :255-263 —— 无 plan 只显「正在更新待办」。
        let n = node("TodoWrite", json!({}));
        let summary = get_latest_explore_child_summary(&n, true).unwrap();
        match summary.content {
            ExploreChildContent::Todo {
                action_label,
                ref progress_text,
            } => {
                assert_eq!(action_label, "正在更新待办");
                assert!(progress_text.is_empty());
            }
            other => panic!("期望 Todo 内容，得 {other:?}"),
        }
    }
}
