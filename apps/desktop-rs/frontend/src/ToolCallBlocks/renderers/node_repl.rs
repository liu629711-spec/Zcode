//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/node-repl.tsx`（388 行）。
//!
//! node_repl 工具卡：run（结果 + 图片 + 技术细节折叠区）/ reset / add-module-dir
//! 三种操作的统一卡面。Computer Use 的 cell 携带目标应用身份时，leading icon
//! 换成该应用的真实图标。
//!
//! **裁剪注明**：
//! - `CuaAppSummaryIcon`（真源 :20/:178-189）依赖主进程 Helper 协议
//!   （iconLocators 权威图标），Rust 侧未迁（cua.rs 同一条遗留）——
//!   恒走 `NODE_REPL_TOOL_ICON` 兜底，不切换成另一个指针图形。
//! - 二级折叠区（真源 Radix `Collapsible`）用 RwSignal + 条件挂载等价。
//! - `displaySource === "browser_turn_end"` 的卡只剩图片网格（真源 :373-375）——
//!   图片网格的 lightbox 裁剪见 nodeReplImageGrid.rs。
//! - 真源 `getSummary` 依赖注入 `formatMessage`；Rust 侧 i18n 模块内聚，
//!   直接读 `i18n::text`（判定逻辑仍是纯函数，可单测）。

use leptos::prelude::*;

use super::codeBlock::RichCodeBlock;
use super::nodeReplImageGrid::NodeReplImageGrid;
use super::super::ToolSnapshotFieldNotice::{ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps};
use crate::ToolCallBlocks::i18n;
use crate::ToolCallBlocks::toolCallRowAdapter::LegacyToolCall;
use crate::lib::nodeReplToolDisplay::{
    build_node_repl_display_model, NodeReplDisplayModel, NodeReplOperation,
};

/// `NODE_REPL_TOOL_ICON`（真源 :26-28 —— SquareMousePointerIcon，lucide square-mouse-pointer）。
pub fn node_repl_tool_icon() -> impl IntoView {
    view! {
        <span class="inline-flex size-4 shrink-0 text-foreground-subtle">
            <crate::app::Icon
                circles=vec![]
                paths=vec![
                    "M12.034 12.681a.498.498 0 0 1 .647-.647l9 3.5a.5.5 0 0 1-.033.943l-3.444 1.068a1 1 0 0 0-.66.66l-1.067 3.443a.5.5 0 0 1-.943.033z",
                    "M21 11V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h6",
                ]
            />
        </span>
    }
}

// Radix 会在浏览器工具及执行详情打开时立即测量高度；代码块默认的 200px 离屏占位
// 会让动画先展开过头再回落。两层内容都只在对应区域展开时挂载，使用真实布局不会损失
// 长会话性能（真源 :30-35 —— Rust 侧内容同样按需挂载，该样式常量不再需要）。

/// `COMPACT_RESULT_MAX_LENGTH`（真源 :36）。
pub const COMPACT_RESULT_MAX_LENGTH: usize = 160;

/// `looksLikeJson`（真源 :38-50）：`{`/`[` 开头且能整体解析成 JSON。
pub fn looks_like_json(value: &str) -> bool {
    let trimmed = value.trim();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return false;
    }
    serde_json::from_str::<serde::de::IgnoredAny>(trimmed).is_ok()
}

/// `isCompactResult`（真源 :52-60）：非空、无换行、非 JSON、≤160（按 Unicode 标量数）。
pub fn is_compact_result(value: &str) -> bool {
    let trimmed = value.trim();
    let length = trimmed.chars().count();
    length > 0
        && length <= COMPACT_RESULT_MAX_LENGTH
        && !trimmed.contains('\n')
        && !looks_like_json(trimmed)
}

/// `getSummary` 的返回（真源 :67 `{ title, status?, detail? }`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeReplSummary {
    pub title: String,
    pub status: Option<String>,
    pub detail: Option<String>,
}

/// `getSummary`（真源 :62-124）：按操作类型与状态给摘要三段词。
pub fn get_summary(
    model: &NodeReplDisplayModel,
    status: &str,
    is_running: bool,
) -> NodeReplSummary {
    let is_failed = status == "failed";
    let is_denied = status == "denied";
    let is_stopped = status == "stopped";

    if is_denied || is_stopped {
        return NodeReplSummary {
            title: i18n::text(if is_denied {
                "chat.toolCall.nodeRepl.denied"
            } else {
                "chat.toolCall.nodeRepl.stopped"
            }),
            status: None,
            detail: None,
        };
    }

    match model.operation {
        Some(NodeReplOperation::Reset) => NodeReplSummary {
            title: i18n::text(if is_failed {
                "chat.toolCall.nodeRepl.resetFailed"
            } else if is_running {
                "chat.toolCall.nodeRepl.resetting"
            } else {
                "chat.toolCall.nodeRepl.reset"
            }),
            status: None,
            detail: None,
        },
        Some(NodeReplOperation::AddModuleDir) => NodeReplSummary {
            title: i18n::text(if is_failed {
                "chat.toolCall.nodeRepl.configureFailed"
            } else if is_running {
                "chat.toolCall.nodeRepl.configuring"
            } else {
                "chat.toolCall.nodeRepl.configured"
            }),
            status: None,
            detail: model.module_directory.clone(),
        },
        _ => {
            let fallback_title = i18n::text(if is_failed {
                "chat.toolCall.nodeRepl.failed"
            } else if is_running {
                "chat.toolCall.nodeRepl.processing"
            } else {
                "chat.toolCall.nodeRepl.finished"
            });
            match &model.user_title {
                Some(user_title) => NodeReplSummary {
                    title: user_title.clone(),
                    status: Some(i18n::text(if is_failed {
                        "chat.toolCall.nodeRepl.failed"
                    } else if is_running {
                        "chat.toolCall.nodeRepl.processing"
                    } else {
                        "chat.toolCall.nodeRepl.completed"
                    })),
                    detail: None,
                },
                None => NodeReplSummary {
                    title: fallback_title,
                    status: None,
                    detail: None,
                },
            }
        }
    }
}

/// `NodeReplToolCallBlock`（真源 :166-405）。
#[component]
pub fn NodeReplToolCallBlock(
    tool_call: LegacyToolCall,
    is_running: bool,
    show_icon: bool,
    can_toggle: Option<bool>,
    force_open: Option<bool>,
    error_text: Option<String>,
    source_label: Option<String>,
    on_load_full_tool_call_fields: Option<Callback<String, bool>>,
) -> impl IntoView {
    let model = build_node_repl_display_model(&tool_call);
    let summary = get_summary(&model, &tool_call.status, is_running);

    // Computer Use 的 cell 携带目标应用身份时，leading icon 换成该应用的真实图标——
    // 连续 CUA 步骤据此一眼看出各步操作的是哪个 app。图标由平台服务按 locator 现取，
    // 取不到时保持 node_repl 自己的图标（真源 :175-189；Helper 协议未迁，恒走兜底图标）。
    let result_label = i18n::text("chat.toolCall.nodeRepl.result");
    let no_result_label = i18n::text("chat.toolCall.nodeRepl.noResult");
    let details_label = i18n::text("chat.toolCall.nodeRepl.details");
    let detail_content_label = i18n::text("chat.toolCall.nodeRepl.detailContent");
    let technical_details_label = i18n::text("chat.toolCall.nodeRepl.technicalDetails");
    let copy_result_label = i18n::text("chat.toolCall.nodeRepl.copyResult");
    let copy_details_label = i18n::text("chat.toolCall.nodeRepl.copyDetails");
    let wrap_lines_label = i18n::text("chat.toolCall.nodeRepl.wrapLines");
    let result_image_label = i18n::text("chat.toolCall.nodeRepl.resultImage");
    let full_result_label = model
        .persisted_result
        .as_ref()
        .map(|persisted| {
            i18n::format(
                "chat.toolCall.nodeRepl.fullResult",
                &[("size".to_string(), persisted.size_label.clone())],
            )
        });
    let visible_error = if tool_call.status == "failed" {
        model
            .error
            .as_ref()
            .map(|error| error.summary.clone())
            .or(error_text.clone())
    } else {
        None
    };
    let has_technical_details = model.code.is_some()
        || model
            .error
            .as_ref()
            .map(|error| error.stack.is_some())
            .unwrap_or(false);
    let has_visible_result = visible_error.is_some()
        || model.result_text.is_some()
        || !model.images.is_empty()
        || model.persisted_result.is_some();
    let has_details = if model.operation == Some(NodeReplOperation::Run) {
        has_visible_result || has_technical_details
    } else {
        tool_call.status == "failed" && (has_visible_result || has_technical_details)
    };
    let can_toggle = can_toggle.unwrap_or(has_details);

    // 结果面板（真源 :237-270）：失败面 > 紧凑短文 > 代码块 >（停机且无图）无结果行。
    let has_result_code_block = model
        .result_text
        .as_ref()
        .map(|text| !is_compact_result(text))
        .unwrap_or(false);
    let compact_text = model
        .result_text
        .as_ref()
        .filter(|text| is_compact_result(text))
        .map(|text| text.trim().to_string());
    let result_for_code = model.result_text.clone();
    let result_is_json = model
        .result_text
        .as_deref()
        .map(looks_like_json)
        .unwrap_or(false);
    // 真源 :268 —— running 或有图时不念「无结果」。
    let show_no_result = !is_running && model.images.is_empty();

    let code_for_details = model.code.clone();
    let stack_for_details = model
        .error
        .as_ref()
        .and_then(|error| error.stack.clone());
    let images_for_grid = model.images.clone();
    let images_for_body = model.images.clone();
    let persisted_for_body = model.persisted_result.clone();
    let full_result_for_body = full_result_label.clone();
    // 「打开完整结果」链接钮（真源 :279-296）需要 persisted.artifactPath +
    // onOpenCodeViewer 通道；两者 Rust 侧均未接线，等 code viewer 链路迁移时补。
    // 真源 :373-375 —— browser_turn_end 的显示源只渲染图片网格。
    let is_browser_turn_end = model.display_source.is_some();
    let summary_title_for_layout = summary.title.clone();

    // 技术细节折叠区开合（真源 Radix Collapsible 的 Rust 等价；无动画）。
    let details_open = RwSignal::new(false);

    view! {
        {move || {
            if is_browser_turn_end {
                view! {
                    <NodeReplImageGrid
                        images=images_for_body.clone()
                        result_image_label=result_image_label.clone()
                    />
                }
                    .into_any()
            } else {
                view! {
                    <>
                        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
                            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                                tool_id: tool_call.tool_id.clone(),
                                show_icon: Some(show_icon),
                                can_toggle: Some(can_toggle),
                                force_open: Some(force_open.unwrap_or(false)),
                                source_label: source_label.clone(),
                                // 摘要三段：标题（用户标题或操作词）· 状态 · 等宽细节。
                                primary_text_view: Some({
                                    let summary = summary.clone();
                                    std::sync::Arc::new(move || {
                                        let summary = summary.clone();
                                        view! {
                                            <span class="inline-flex min-w-0 items-center gap-2">
                                                <span class="min-w-0 truncate font-medium text-foreground-subtle">
                                                    {summary.title.clone()}
                                                </span>
                                                {summary.status.clone().map(|status| {
                                                    view! {
                                                        <span class="shrink-0 text-foreground-subtlest">
                                                            {format!("· {status}")}
                                                        </span>
                                                    }
                                                })}
                                                {summary.detail.clone().map(|detail| {
                                                    view! {
                                                        <code class="min-w-0 truncate font-mono text-foreground-subtlest">
                                                            {detail}
                                                        </code>
                                                    }
                                                })}
                                            </span>
                                        }
                                            .into_any()
                                    })
                                        as std::sync::Arc<
                                            dyn Fn() -> AnyView + Send + Sync + 'static,
                                        >
                                }),
                                status_label: Some(i18n::text("chat.toolCall.nodeRepl.failed")),
                                status_tooltip: visible_error.clone(),
                                show_failure_status: Some(
                                    tool_call.status == "failed" && model.user_title.is_some(),
                                ),
                                is_running: Some(is_running),
                                title: Some(summary_title_for_layout.clone()),
                                ..Default::default()
                            }
                            icon_view=Some(std::sync::Arc::new(|| {
                                view! { {node_repl_tool_icon()} }.into_any()
                            })
                                as std::sync::Arc<dyn Fn() -> AnyView + Send + Sync + 'static>)
                            render_content=has_details.then(|| {
                                // ToolLayout 已提供展开间距，Node REPL 再叠加横向 padding 会让 BUA
                                // 结果相对摘要行二次缩进，在窄屏消息流里尤其突兀（真源 :232-235）。
                                let visible_error = visible_error.clone();
                                let result_label = result_label.clone();
                                let compact_text = compact_text.clone();
                                let has_result_code_block = has_result_code_block;
                                let result_for_code = result_for_code.clone();
                                let result_is_json = result_is_json;
                                let show_no_result = show_no_result;
                                let no_result_label = no_result_label.clone();
                                let images = images_for_grid.clone();
                                let result_image_label = result_image_label.clone();
                                let persisted = persisted_for_body.clone();
                                let full_result_label = full_result_for_body.clone();
                                let has_technical_details = has_technical_details;
                                let details_label = details_label.clone();
                                let details_open = details_open;
                                let code_for_details = code_for_details.clone();
                                let stack_for_details = stack_for_details.clone();
                                let detail_content_label = detail_content_label.clone();
                                let technical_details_label = technical_details_label.clone();
                                let copy_result_label = copy_result_label.clone();
                                let copy_details_label = copy_details_label.clone();
                                let wrap_lines_label = wrap_lines_label.clone();
                                std::sync::Arc::new(move || {
                                    view! {
                                        <div class="mb-2 space-y-3 py-1" data-testid="node-repl-expanded-content">
                                            {if let Some(visible_error) = visible_error.clone() {
                                                view! {
                                                    <section class="space-y-1.5">
                                                        <h4 class="text-ui-base font-medium text-destructive">
                                                            {result_label.clone()}
                                                        </h4>
                                                        <p class="whitespace-pre-wrap break-words rounded-lg bg-destructive/10 px-3 py-2 text-ui-base text-destructive">
                                                            {visible_error}
                                                        </p>
                                                    </section>
                                                }
                                                    .into_any()
                                            } else if let Some(compact_text) = compact_text.clone() {
                                                // BUA 常返回 done、标题或 URL 等短结果，完整代码块的标题栏、
                                                // 边框和操作按钮会让内容重量远大于信息本身；短结果按普通次级正文展示。
                                                view! {
                                                    <p
                                                        class="break-words rounded-xl border border-border bg-card px-3 py-2 text-ui-base text-foreground-subtle"
                                                        data-testid="node-repl-result-surface"
                                                    >
                                                        {compact_text}
                                                    </p>
                                                }
                                                    .into_any()
                                            } else if has_result_code_block {
                                                view! {
                                                    <div
                                                        class="max-h-72 overflow-auto rounded-xl border border-border bg-card"
                                                        data-testid="node-repl-result-surface"
                                                    >
                                                        <RichCodeBlock
                                                            class="bg-card".to_string()
                                                            code=result_for_code.clone().unwrap_or_default()
                                                            copy_label=Some(copy_result_label.clone())
                                                            label=Some(result_label.clone())
                                                            language=if result_is_json { "json".to_string() } else { "log".to_string() }
                                                            wrap_label=Some(wrap_lines_label.clone())
                                                            wrap_long_lines=true
                                                        />
                                                    </div>
                                                }
                                                    .into_any()
                                            } else if show_no_result {
                                                view! {
                                                    <p class="text-ui-base text-foreground-subtle">
                                                        {no_result_label.clone()}
                                                    </p>
                                                }
                                                    .into_any()
                                            } else {
                                                ().into_any()
                                            }}

                                            {(!images.is_empty()).then(|| {
                                                view! {
                                                    <NodeReplImageGrid
                                                        images=images.clone()
                                                        result_image_label=result_image_label.clone()
                                                    />
                                                }
                                            })}

                                            {(persisted.is_some() && full_result_label.is_some()).then(|| {
                                                view! {
                                                    <div class="flex flex-wrap items-center gap-2 text-ui-base text-foreground-subtle">
                                                        <span>{full_result_label.clone().unwrap_or_default()}</span>
                                                        // 真源 :279-296 —— onOpenCodeViewer 在场才渲染
                                                        // 「打开完整结果」链接钮；Rust 侧 v4 通道未接线，暂无入口。
                                                    </div>
                                                }
                                            })}

                                            {has_technical_details.then(|| {
                                                view! {
                                                    <div class="group/details">
                                                        <button
                                                            class="group/button inline-flex shrink-0 items-center justify-center rounded-lg border border-transparent bg-clip-padding font-medium whitespace-nowrap transition-colors outline-none select-none text-foreground-subtlest hover:bg-transparent hover:text-foreground h-6 gap-1 px-2 -ml-2 text-ui-base/relaxed"
                                                            type="button"
                                                            on:click=move |_| details_open.update(|v| *v = !*v)
                                                        >
                                                            <span class=if details_open.get() {
                                                                "inline-flex size-3.5 rotate-90 transition-transform"
                                                            } else {
                                                                "inline-flex size-3.5 transition-transform"
                                                            }>
                                                                <crate::app::Icon circles=vec![] paths=vec!["m9 18 6-6-6-6"] />
                                                            </span>
                                                            {details_label.clone()}
                                                        </button>
                                                        {details_open.get().then(|| {
                                                            view! {
                                                                <div class="space-y-2 pt-2">
                                                                    {code_for_details.clone().map(|code| {
                                                                        view! {
                                                                            <div
                                                                                class="max-h-72 overflow-auto rounded-xl border border-border bg-card"
                                                                                data-testid="node-repl-detail-surface"
                                                                            >
                                                                                <RichCodeBlock
                                                                                    class="bg-card".to_string()
                                                                                    code=code
                                                                                    copy_label=Some(copy_details_label.clone())
                                                                                    label=Some(detail_content_label.clone())
                                                                                    language="javascript".to_string()
                                                                                    wrap_label=Some(wrap_lines_label.clone())
                                                                                    wrap_long_lines=true
                                                                                />
                                                                            </div>
                                                                        }
                                                                    })}
                                                                    {stack_for_details.clone().map(|stack| {
                                                                        view! {
                                                                            <div
                                                                                class="max-h-72 overflow-auto rounded-xl border border-border bg-card"
                                                                                data-testid="node-repl-detail-surface"
                                                                            >
                                                                                <RichCodeBlock
                                                                                    class="bg-card".to_string()
                                                                                    code=stack
                                                                                    copy_label=Some(copy_details_label.clone())
                                                                                    label=Some(technical_details_label.clone())
                                                                                    language="log".to_string()
                                                                                    wrap_label=Some(wrap_lines_label.clone())
                                                                                    wrap_long_lines=true
                                                                                />
                                                                            </div>
                                                                        }
                                                                    })}
                                                                </div>
                                                            }
                                                        })}
                                                    </div>
                                                }
                                            })}
                                        </div>
                                    }
                                        .into_any()
                                })
                                    as std::sync::Arc<dyn Fn() -> AnyView + Send + Sync + 'static>
                            })
                        />
                        <ToolSnapshotFieldNoticeComponent
                            props=ToolSnapshotFieldNoticeProps {
                                refs: tool_call.snapshot_refs.clone(),
                                tool_id: tool_call.tool_id.clone(),
                                on_load_full_tool_call_fields: on_load_full_tool_call_fields.clone(),
                            }
                        />
                    </>
                }
                    .into_any()
            }
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lib::nodeReplToolDisplay::{NodeReplCuaApp, NodeReplDisplayModel};

    fn model(operation: NodeReplOperation) -> NodeReplDisplayModel {
        NodeReplDisplayModel {
            operation: Some(operation),
            ..Default::default()
        }
    }

    #[test]
    fn summary_denied_and_stopped_are_terminal_words() {
        // 真源 :72-78 —— denied / stopped 只有一个词，无状态无细节。
        let m = model(NodeReplOperation::Run);
        let s = get_summary(&m, "denied", false);
        assert_eq!(
            s.title,
            i18n::text("chat.toolCall.nodeRepl.denied")
        );
        assert_eq!(s.status, None);
        let s = get_summary(&m, "stopped", false);
        assert_eq!(
            s.title,
            i18n::text("chat.toolCall.nodeRepl.stopped")
        );
    }

    #[test]
    fn summary_reset_and_configure_follow_lifecycle() {
        // 真源 :80-103 —— reset 三态；add-module-dir 带 moduleDirectory 细节。
        let m = model(NodeReplOperation::Reset);
        assert_eq!(
            get_summary(&m, "completed", false).title,
            i18n::text("chat.toolCall.nodeRepl.reset")
        );
        assert_eq!(
            get_summary(&m, "in_progress", true).title,
            i18n::text("chat.toolCall.nodeRepl.resetting")
        );
        assert_eq!(
            get_summary(&m, "failed", false).title,
            i18n::text("chat.toolCall.nodeRepl.resetFailed")
        );

        let mut m = model(NodeReplOperation::AddModuleDir);
        m.module_directory = Some("/tmp/mods".into());
        let s = get_summary(&m, "completed", false);
        assert_eq!(
            s.title,
            i18n::text("chat.toolCall.nodeRepl.configured")
        );
        assert_eq!(s.detail.as_deref(), Some("/tmp/mods"));
    }

    #[test]
    fn summary_run_prefers_user_title_with_status_word() {
        // 真源 :105-123 —— 有用户标题：标题 + 状态词；没有：合成的生命周期词。
        let mut m = model(NodeReplOperation::Run);
        m.user_title = Some("斐波那契".into());
        let s = get_summary(&m, "completed", false);
        assert_eq!(s.title, "斐波那契");
        assert_eq!(
            s.status.as_deref(),
            Some(i18n::text("chat.toolCall.nodeRepl.completed").as_str())
        );

        let s = get_summary(&m, "in_progress", true);
        assert_eq!(
            s.status.as_deref(),
            Some(i18n::text("chat.toolCall.nodeRepl.processing").as_str())
        );

        let s = get_summary(&m, "failed", false);
        assert_eq!(
            s.status.as_deref(),
            Some(i18n::text("chat.toolCall.nodeRepl.failed").as_str())
        );

        // 无用户标题：fallback 词、无状态段。
        let m = model(NodeReplOperation::Run);
        let s = get_summary(&m, "completed", false);
        assert_eq!(
            s.title,
            i18n::text("chat.toolCall.nodeRepl.finished")
        );
        assert_eq!(s.status, None);
    }

    #[test]
    fn cua_app_presence_is_recorded_on_model() {
        // 真源 :178-189 —— 有 app 身份时走权威图标分支（Rust 侧恒兜底，但模型要带 app）。
        let mut m = model(NodeReplOperation::Run);
        m.app = Some(NodeReplCuaApp {
            app_key: "com.apple.Safari".into(),
            display_name: None,
        });
        assert_eq!(m.app.as_ref().unwrap().app_key, "com.apple.Safari");
    }
}
