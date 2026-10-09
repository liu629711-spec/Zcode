//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/workflow-draft-row.tsx`（186 行）。
//!
//! 编译反馈行在 ToolLayout 各槽位里说的话 + 展开后的内容
//! （`WorkflowFeedbackContent` 组件在文件末——上一批迁这个文件时只落了纯判定，
//! 组件本体随 create-workflow 这批补齐）。
//! 真源从 `create-workflow.tsx` 拆出（oxlint max-lines 400 门）。

use leptos::prelude::*;

use crate::ToolCallBlocks::i18n;

use super::createWorkflowDisplay::{
    CreateWorkflowDisplay, format_workflow_feedback_tooltip, workflow_diagnostic_lines,
};
use super::workflow_diagnostics::workflow_feedback_lede_message_id;

// ---------------------------------------------------------------------------
// 稿号判定（真源 :145-155）
// ---------------------------------------------------------------------------

/// `workflowDraftOrdinalShown`（真源 :147-155）。
///
/// ★真源注释：编不过的行总是写（宿主给了位置时）；在途行从第 2 稿起写。
/// 运行卡、启动摘要与确认窗不走这里，**永远不带稿号**。
pub fn workflow_draft_ordinal_shown(
    draft_ordinal: Option<i64>,
    compile_errors: bool,
    in_flight: bool,
) -> Option<i64> {
    let ordinal = draft_ordinal?;
    if compile_errors {
        return Some(ordinal);
    }
    // 在途行第 2 稿起才编号——第 1 稿不预告还会有第 2 稿。
    if in_flight && ordinal >= 2 {
        Some(ordinal)
    } else {
        None
    }
}

/// 稿号文案（真源 `chat.toolCall.workflow.draftOrdinal`）。
pub fn draft_ordinal_text(ordinal: i64) -> String {
    i18n::format(
        "chat.toolCall.workflow.draftOrdinal",
        &[("ordinal".to_string(), ordinal.to_string())],
    )
}

// ---------------------------------------------------------------------------
// 槽位推导（真源 :38-95）
// ---------------------------------------------------------------------------

/// `WorkflowDraftRowInput`（真源 :31-37）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowDraftRowInput {
    pub draft_ordinal: Option<i64>,
    /// 有更新的稿之后，本稿不再是最新的（真源 `draft.superseded`）。
    /// 只影响空环灯颜色：最新一稿警示色，已被超越的褪成中性。
    pub superseded: bool,
    pub compile_errors: bool,
    /// 在途行（编写、待确认）：第 2 稿起才编号。
    pub in_flight: bool,
    pub error_count: i64,
    pub saved: bool,
    /// 诊断位置（供 tooltip 逐条列出）。
    pub diagnostics: Vec<(i64, i64, String)>,
}

/// `WorkflowDraftRowSlots`（真源 :19-30）——纯文本形态。
///
/// 真源注释：只用行已有的槽位（种类词、名字、细节、状态），
/// **不往对话里加新元素**——上一轮通知重设计在真机里被判「花」，
/// 正是因为引入了新的视觉语法。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowDraftRowSlots {
    /// 细节槽：「第 n 稿」。
    pub secondary_text: Option<String>,
    /// 状态词「n 处待修正 · 未运行」，编不过时才有。
    pub status_label: Option<String>,
    /// 状态词前的空环灯，编不过时才有。
    pub status_indicator: Option<DraftLampState>,
    /// 悬停提示：那句话 + 逐条诊断；编不过时才有。
    pub status_tooltip: Option<String>,
    /// 在途行（校验中的卡片表头）从第 2 稿起写在细节位的稿号文字。
    pub in_flight_ordinal_text: Option<String>,
}

/// 空环灯的状态（真源 `WorkflowDraftLamp` :169-186）。
///
/// ★真源注释：形状说「什么都没跑」，颜色说注意力是否还悬着
/// （最新一稿警示色，有更新的一稿后褪成中性）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftLampState {
    /// 最新一稿：警示色。
    Open,
    /// 有更新的一稿后：褪成中性。
    Settled,
}

impl DraftLampState {
    /// `data-draft-lamp` 属性值。
    pub fn data_attr(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Settled => "settled",
        }
    }

    /// 颜色类名（真源 `DRAFT_FEEDBACK_DOT`）。
    pub fn class(self) -> &'static str {
        match self {
            Self::Open => "bg-destructive",
            Self::Settled => "bg-foreground-subtlest",
        }
    }
}

/// `useWorkflowDraftRowSlots`（真源 :43-95）的纯逻辑形态。
pub fn build_workflow_draft_row_slots(input: &WorkflowDraftRowInput) -> WorkflowDraftRowSlots {
    let ordinal = workflow_draft_ordinal_shown(
        input.draft_ordinal,
        input.compile_errors,
        input.in_flight,
    );
    // 在途行的稿号**不看 compileErrors**（真源 :45）——
    // 它写在「校验中的卡片表头」的细节位，与编不过那条路径无关。
    let in_flight_ordinal = workflow_draft_ordinal_shown(input.draft_ordinal, false, true);
    let in_flight_ordinal_text = in_flight_ordinal.map(draft_ordinal_text);

    let lede = i18n::text(workflow_feedback_lede_message_id(input.saved));

    // 真源 :53-57 ——「{n} 处待修正 · 未运行」。
    let words = if input.compile_errors {
        Some(format!(
            "{} · {}",
            i18n::format(
                "chat.toolCall.workflow.toFix",
                &[("count".to_string(), input.error_count.to_string())]
            ),
            i18n::text("chat.toolCall.workflow.notRun")
        ))
    } else {
        None
    };

    let status_tooltip = if input.compile_errors {
        Some(format_workflow_feedback_tooltip(&lede, &input.diagnostics))
    } else {
        None
    };

    WorkflowDraftRowSlots {
        secondary_text: ordinal.map(draft_ordinal_text),
        status_label: words,
        status_indicator: input.compile_errors.then_some(if input.superseded {
            DraftLampState::Settled
        } else {
            DraftLampState::Open
        }),
        status_tooltip,
        in_flight_ordinal_text,
    }
}

// ---------------------------------------------------------------------------
// 展开后的内容（真源 :98-141）
// ---------------------------------------------------------------------------

/// `WorkflowFeedbackContent` 的判定部分（真源 :112-116）。
///
/// 真源注释：按内容记忆——display 不变就是同一个数组，
/// 代码块的注入样式不会跟着重建。
pub fn feedback_flagged_lines(
    display: Option<&CreateWorkflowDisplay>,
) -> Option<Vec<i64>> {
    let display = display?;
    if display.ok {
        return None;
    }
    let positions: Vec<(i64, i64, String)> = display
        .diagnostics
        .iter()
        .map(|d| (d.line, d.column, d.message.clone()))
        .collect();
    Some(workflow_diagnostic_lines(&positions))
}

/// 是否渲染编译反馈卡（真源 :127-134）。
pub fn should_render_diagnostics(display: Option<&CreateWorkflowDisplay>) -> bool {
    display.is_some_and(|d| !d.diagnostics.is_empty())
}

/// 是否退回纯文本输出（真源 :135-140）。
///
/// 真源：只有 `!display` 时才退（display 在但无诊断时也走卡片）。
pub fn should_render_fallback_output(
    display: Option<&CreateWorkflowDisplay>,
    fallback_output_text: Option<&str>,
) -> bool {
    display.is_none() && fallback_output_text.is_some_and(|t| !t.is_empty())
}

/// `WorkflowFeedbackContent`（真源 :96-143）——编译反馈行展开后的内容。
///
/// 三段按在场情况叠：脚本代码块（行号 + 被诊断点名的行着色）→ 诊断表 →
/// 没有 display 时的有界纯文本面板。
#[component]
pub fn WorkflowFeedbackContent(
    // 真源 `display`（可空：还没结算的在途行没有 display）。
    display: Option<CreateWorkflowDisplay>,
    // 真源 `fallbackOutputText`（`readFallbackOutputText(toolCall.output)`）。
    fallback_output_text: Option<String>,
    // 脚本来自保存的工作流文件（真源 `saved`：那句话点名文件而不是这次调用）。
    saved: bool,
    // 真源 `scriptText`（`readWorkflowScript(toolCall.input)`）。
    script_text: Option<String>,
) -> impl IntoView {
    // 真源 :107-111 —— 按内容记忆：display 不变就是同一个数组，
    // 代码块的注入样式不会跟着重建。
    let flagged = feedback_flagged_lines(display.as_ref());
    let wrap_label = i18n::text("codeBlock.wrapLines");
    let diagnostics = display
        .as_ref()
        .map(|d| {
                d.diagnostics
                    .iter()
                    .map(|entry| super::workflow_diagnostics::WorkflowDiagnosticEntry {
                        line: entry.line,
                        column: entry.column,
                        code: entry.code,
                        message: entry.message.clone(),
                    })
                    .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let error_count = display.as_ref().map(|d| d.error_count);
    let truncated = display.as_ref().and_then(|d| d.truncated) == Some(true);
    let has_diagnostics = !diagnostics.is_empty();
    let show_fallback = should_render_fallback_output(display.as_ref(), fallback_output_text.as_deref());
    let fallback_text = fallback_output_text.unwrap_or_default();
    let script_for_block = script_text.clone();
    let show_script = script_text.is_some();

    view! {
        <div class="mb-2 space-y-3">
            {show_script.then(|| {
                let marked = flagged.clone();
                view! {
                    <div data-testid="workflow-script-codeblock">
                        // 真源 :116-125 —— CodeBlock + CodeBlockHeader(language)。
                        // 容器类照真源 className / contentClassName 两处。
                        <div class="max-h-80 overflow-auto border border-border bg-card">
                            <super::codeBlock::RichCodeBlock
                                code=script_for_block.clone().unwrap_or_default()
                                language="typescript".to_string()
                                label=Some("typescript".to_string())
                                copy_label=None
                                wrap_label=Some(wrap_label.clone())
                                wrap_long_lines=false
                                class="border-border bg-card".to_string()
                                show_line_numbers=true
                                marked_lines=marked.unwrap_or_default()
                            />
                        </div>
                    </div>
                }
            })}
            {has_diagnostics.then(|| view! {
                <super::workflow_diagnostics::WorkflowDiagnosticsSectionComponent
                    count=error_count
                    diagnostics=diagnostics.clone()
                    saved=saved
                    truncated=truncated
                />
            })}
            {show_fallback.then(|| view! {
                // 没有 display（老会话 / 读不出装饰载荷）时退回有界纯文本，不做 JSON dump。
                <pre class="max-h-60 overflow-auto whitespace-pre-wrap break-words rounded-xl border border-border bg-panel px-3 py-2 font-mono text-ui-base text-foreground-subtle">
                    {fallback_text.clone()}
                </pre>
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolCallBlocks::toolResultDisplay::WorkflowDiagnostic;

    fn diags() -> Vec<(i64, i64, String)> {
        vec![
            (3, 7, "Expected ';'".to_string()),
            (3, 1, "Another".to_string()),
        ]
    }

    fn input(compile_errors: bool, in_flight: bool, ordinal: Option<i64>) -> WorkflowDraftRowInput {
        WorkflowDraftRowInput {
            draft_ordinal: ordinal,
            superseded: false,
            compile_errors,
            in_flight,
            error_count: 2,
            saved: false,
            diagnostics: diags(),
        }
    }

    fn display(ok: bool, diagnostics: Vec<WorkflowDiagnostic>) -> CreateWorkflowDisplay {
        CreateWorkflowDisplay {
            ok,
            error_count: diagnostics.len() as i64,
            diagnostics,
            causality_graph: None,
            truncated: None,
        }
    }

    // ── 稿号 ──

    #[test]
    fn no_draft_position_means_no_ordinal() {
        assert_eq!(workflow_draft_ordinal_shown(None, true, true), None);
        assert_eq!(workflow_draft_ordinal_shown(None, false, false), None);
    }

    #[test]
    fn compile_errors_always_show_ordinal() {
        // 真源 :150 —— 编不过总是写。
        for ordinal in [1, 2, 5] {
            assert_eq!(
                workflow_draft_ordinal_shown(Some(ordinal), true, false),
                Some(ordinal)
            );
        }
    }

    #[test]
    fn in_flight_shows_ordinal_from_second_draft() {
        // ★真源 :151-152 —— 第 1 稿不预告还会有第 2 稿。
        assert_eq!(workflow_draft_ordinal_shown(Some(1), false, true), None);
        assert_eq!(workflow_draft_ordinal_shown(Some(2), false, true), Some(2));
        assert_eq!(workflow_draft_ordinal_shown(Some(7), false, true), Some(7));
    }

    #[test]
    fn terminal_never_shows_ordinal() {
        // 真源 :144-146 —— 运行卡/启动摘要/确认窗永远不带稿号。
        for ordinal in [1, 2, 5] {
            assert_eq!(
                workflow_draft_ordinal_shown(Some(ordinal), false, false),
                None,
                "非在途非编不过时不该有稿号"
            );
        }
    }

    // ── 槽位 ──

    #[test]
    fn compile_error_slots_are_all_present() {
        let slots = build_workflow_draft_row_slots(&input(true, false, Some(3)));
        assert_eq!(slots.secondary_text.as_deref(), Some("第 3 稿").as_deref().map(|_| "第 3 稿").or(Some("第 3 稿")).map(|x| x));
        assert!(slots.status_label.is_some(), "编不过要有状态词");
        assert_eq!(slots.status_indicator, Some(DraftLampState::Open));
        assert!(slots.status_tooltip.is_some(), "编不过要有悬停提示");
    }

    #[test]
    fn healthy_slots_are_empty() {
        let slots = build_workflow_draft_row_slots(&input(false, false, Some(1)));
        assert_eq!(slots.secondary_text, None);
        assert_eq!(slots.status_label, None);
        assert_eq!(slots.status_indicator, None);
        assert_eq!(slots.status_tooltip, None);
    }

    #[test]
    fn status_label_reads_fix_count_and_not_run() {
        // 真源 :53-57 ——「{n} 处待修正 · 未运行」。
        let slots = build_workflow_draft_row_slots(&input(true, false, Some(1)));
        let label = slots.status_label.unwrap();
        assert!(label.contains("2"), "含待修正数：{label}");
        assert!(label.contains('·'), "含分隔符：{label}");
    }

    #[test]
    fn status_tooltip_contains_lede_and_diagnostics() {
        let slots = build_workflow_draft_row_slots(&input(true, false, Some(1)));
        let tip = slots.status_tooltip.unwrap();
        assert!(tip.contains("L3:C7 Expected ';'"), "含逐条诊断：{tip}");
        assert!(tip.lines().count() >= 3, "首行那句 + 两条诊断：{tip}");
    }

    #[test]
    fn saved_flag_changes_lede() {
        let mut inp = input(true, false, Some(1));
        inp.saved = true;
        let saved_tip = build_workflow_draft_row_slots(&inp).status_tooltip.unwrap();
        let normal_tip = build_workflow_draft_row_slots(&input(true, false, Some(1)))
            .status_tooltip
            .unwrap();
        assert_ne!(saved_tip, normal_tip, "保存来源那句话要点名文件");
    }

    #[test]
    fn in_flight_ordinal_text_ignores_compile_errors() {
        // 真源 :45 —— 在途稿号不看 compileErrors。
        let slots = build_workflow_draft_row_slots(&input(false, true, Some(4)));
        assert!(
            slots.in_flight_ordinal_text.is_some(),
            "在途第 4 稿应有稿号文字"
        );
        // 第 1 稿没有。
        let first = build_workflow_draft_row_slots(&input(false, true, Some(1)));
        assert_eq!(first.in_flight_ordinal_text, None);
    }

    #[test]
    fn superseded_draft_lamp_fades_to_neutral() {
        // ★真源 :169-171 —— 最新一稿警示色，有更新的一稿后褪成中性。
        let mut fresh = input(true, false, Some(2));
        fresh.superseded = false;
        assert_eq!(
            build_workflow_draft_row_slots(&fresh).status_indicator,
            Some(DraftLampState::Open)
        );

        fresh.superseded = true;
        assert_eq!(
            build_workflow_draft_row_slots(&fresh).status_indicator,
            Some(DraftLampState::Settled),
            "被超越的稿褪成中性"
        );
    }

    #[test]
    fn lamp_states_differ_in_color_and_attr() {
        // 真源 :171-178 —— 形状说「什么都没跑」，颜色说注意力是否悬着。
        assert_eq!(DraftLampState::Open.data_attr(), "open");
        assert_eq!(DraftLampState::Settled.data_attr(), "settled");
        assert_ne!(DraftLampState::Open.class(), DraftLampState::Settled.class());
    }

    // ── 展开内容 ──

    #[test]
    fn flagged_lines_only_when_display_failed() {
        let failed = display(false, vec![WorkflowDiagnostic {
            line: 3,
            column: 7,
            code: 9001,
            message: "x".into(),
        }]);
        assert_eq!(feedback_flagged_lines(Some(&failed)), Some(vec![3]));

        let ok = display(true, Vec::new());
        assert_eq!(feedback_flagged_lines(Some(&ok)), None, "成功不着色");
        assert_eq!(feedback_flagged_lines(None), None);
    }

    #[test]
    fn diagnostics_section_shown_when_has_diagnostics() {
        assert!(should_render_diagnostics(Some(&display(
            false,
            vec![WorkflowDiagnostic {
                line: 1,
                column: 1,
                code: 9001,
                message: "x".into()
            }]
        ))));
        assert!(!should_render_diagnostics(Some(&display(false, Vec::new()))));
        assert!(!should_render_diagnostics(None));
    }

    #[test]
    fn fallback_only_when_no_display() {
        // ★真源 :135 —— 有 display 时（即便无诊断）也不退纯文本。
        assert!(should_render_fallback_output(None, Some("文本")));
        assert!(!should_render_fallback_output(Some(&display(true, Vec::new())), Some("文本")));
        assert!(!should_render_fallback_output(None, None));
        assert!(!should_render_fallback_output(None, Some("")));
    }

    #[test]
    fn draft_ordinal_text_uses_i18n() {
        let text = draft_ordinal_text(2);
        assert!(text.contains('2'), "含稿号：{text}");
    }
}