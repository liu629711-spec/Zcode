//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowSettingsChangeRow.tsx`（73 行）。
//!
//! 设置轮的那一行：「配置」的一次修改在转写里留下一个控制轮：没有用户气泡，它的呈现
//! 是新 run 的卡，卡上方这一行说改了什么——工具行的单行样式：滑杆图标、「已调整设置」、
//! 每项改动一段、时刻，以 `·` 相隔。它是记录，不是控件。

use leptos::prelude::*;

use crate::components::workflowSettingsChange::workflow_settings_change_segments;
use crate::ToolCallBlocks::i18n;

/// 设置轮行（真源 `WorkflowSettingsChangeRow`，:14-73）。
///
/// `at` 的时刻格式真源用 `Intl.DateTimeFormat(hour/minute, hour12: false)`；
/// Rust 侧用 js_sys::Date + 手工 HH:MM（同一格式），`None` 即不写。
/// `providerName` 查找参数在纯层 `workflow_settings_change_segments` 里未开（见该模块
/// 文件头的偏离说明），Rust 侧相应省略；接上 provider 名后一并补。
#[component]
pub fn WorkflowSettingsChangeRow(
    amend: crate::components::workflowSettingsChange::WorkflowSettingsAmendMeta,
    /// 设置轮的时刻（毫秒时间戳）；缺席即不写。
    at: Option<i64>,
) -> impl IntoView {
    let segments = workflow_settings_change_segments(&amend);
    let time = at.map(|at| {
        let date = js_sys::Date::new(&at.into());
        format!("{:02}:{:02}", date.get_hours(), date.get_minutes())
    });
    // 段序：种类词 · 各改动 · 时刻（真源 :38-42）。
    let mut parts: Vec<String> = vec![i18n::text("chat.toolCall.workflow.settingsChange.kind")];
    parts.extend(segments);
    if let Some(time) = &time {
        parts.push(time.clone());
    }
    let parts_len = parts.len();
    view! {
        <div
            class="flex min-w-0 items-center gap-2 py-0.5 text-ui-base text-foreground-subtle"
            data-testid="workflow-settings-change-row"
        >
            <span aria-hidden="true" class="shrink-0 inline-flex size-4">
                {crate::components::workflowIcons::icon_sliders_horizontal()}
            </span>
            <span class="flex min-w-0 flex-wrap items-center gap-x-2">
                {parts
                    .iter()
                    .enumerate()
                    .map(|(index, part)| {
                        let tone = if index == 0 {
                            "font-medium"
                        } else if index == parts_len - 1 && time.is_some() {
                            "text-foreground-subtlest tabular-nums"
                        } else {
                            ""
                        };
                        view! {
                            <>
                                {(index > 0).then(|| {
                                    view! {
                                        <span aria-hidden="true" class="text-foreground-subtlest">
                                            {"\u{b7}"}
                                        </span>
                                    }
                                })}
                                <span class=tone>{part.clone()}</span>
                            </>
                        }
                        .into_any()
                    })
                    .collect_view()}
            </span>
        </div>
    }
    .into_any()
}
