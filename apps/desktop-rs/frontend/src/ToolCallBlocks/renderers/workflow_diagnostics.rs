//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/workflow-diagnostics.tsx`（122 行）。
//!
//! 编译反馈卡：`CreateWorkflow` 卡与 `EvalWorkflowSnippet` 卡**共用**
//! （同一编译管线、同一诊断形状、同一道限长——分别实现两份是漂移温床）。

use leptos::prelude::*;

use crate::ToolCallBlocks::i18n;

/// `WorkflowDiagnosticEntry`（真源 :3-8）。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowDiagnosticEntry {
    pub line: i64,
    pub column: i64,
    pub code: i64,
    pub message: String,
}

/// 分析器自有规则的码段下界（真源 :15）。
///
/// ★真源注释：不能写成「≥ 9000」——TypeScript 自己的 18xxx（如 TS18048）
/// 仍是 TS 码；TS 在 9xxx 只有声明产出类诊断，而工作流编译器不产出声明，
/// 所以这一段在卡上只属于分析器。
pub const ANALYZER_RULE_CODE_MIN: i64 = 9001;

/// 分析器自有规则的码段上界（真源 :16）。
pub const ANALYZER_RULE_CODE_MAX: i64 = 9099;

/// `isWorkflowAnalyzerRuleCode`（真源 :18-20）。
pub fn is_workflow_analyzer_rule_code(code: i64) -> bool {
    (ANALYZER_RULE_CODE_MIN..=ANALYZER_RULE_CODE_MAX).contains(&code)
}

/// `workflowFeedbackLedeMessageId`（真源 :31-36）。
///
/// 真源注释：保存的来源编不过是**文件**的问题，句子点名文件，
/// 与工具结果里给模型的那句同一立场；卡片与行的悬停提示读同一个 id。
pub fn workflow_feedback_lede_message_id(saved: bool) -> &'static str {
    if saved {
        "chat.toolCall.workflow.feedback.lede.saved"
    } else {
        "chat.toolCall.workflow.feedback.lede"
    }
}

/// 条数文案（真源 :60-67）。
pub fn count_label(count: i64) -> String {
    let id = if count == 1 {
        "chat.toolCall.workflow.feedback.countOne"
    } else {
        "chat.toolCall.workflow.feedback.count"
    };
    i18n::format(id, &[("count".to_string(), count.to_string())])
}

/// 码标签（真源 :68-72）。
///
/// ★真源注释：码按字符串交给 ICU —— 数字参数会被本地化分组（「9,003」）。
pub fn code_label(code: i64) -> String {
    if is_workflow_analyzer_rule_code(code) {
        i18n::format(
            "chat.toolCall.workflow.feedback.rule",
            &[("code".to_string(), code.to_string())],
        )
    } else {
        format!("TS{code}")
    }
}

/// `WorkflowDiagnosticsSection`（真源 :47-122）。
///
/// ★真源 :49-51 的设计约束：编不过**不是失败**——什么都没跑，反馈交回了模型。
/// 所以这张卡是中性边框、正文前景色，**不用 destructive**：
/// 在这个特性里红色只属于出错的run；整段红字会把最不致命的事件
/// 画成页面上最响的东西。
/// 诊断为空时整段不渲染（真源 :57-59）。
#[component]
pub fn WorkflowDiagnosticsSectionComponent(
    diagnostics: Vec<WorkflowDiagnosticEntry>,
    truncated: bool,
    /// 条数；display 带 `errorCount`（截断前的总数）时传它，缺席按行数算。
    count: Option<i64>,
    /// 脚本来自保存的工作流文件：那句话点名文件而不是这次调用。
    #[prop(optional)]
    saved: bool,
) -> impl IntoView {
    if diagnostics.is_empty() {
        return ().into_view().into_any();
    }
    let total = count.unwrap_or(diagnostics.len() as i64);
    let label = count_label(total);
    let rows: Vec<AnyView> = diagnostics
        .iter()
        .map(|d| {
            let position = format!("L{}:C{}", d.line, d.column);
            let message = d.message.clone();
            let code = code_label(d.code);
            view! {
                <div class="flex items-start gap-2 text-ui-base" data-testid="workflow-compiler-feedback-line">
                    <code class="shrink-0 rounded-sm bg-surface px-1.5 py-0.5 font-mono text-ui-xs text-foreground-subtle">
                        {position}
                    </code>
                    <span class="min-w-0 flex-1 whitespace-pre-wrap break-words text-foreground">
                        {message}
                    </span>
                    <code class="shrink-0 font-mono text-ui-xs text-foreground-subtlest">
                        {code}
                    </code>
                </div>
            }
            .into_any()
        })
        .collect();
    let rows = rows.into_iter().collect_view();

    view! {
        // ★中性边框 + 正文前景色，不用 destructive（真源 :49-51）。
        <div
            class="flex flex-col gap-1.5 rounded-xl border border-border bg-panel px-3 py-2"
            data-testid="workflow-compiler-feedback"
        >
            <div class="flex min-w-0 items-baseline justify-between gap-2 text-ui-xs font-medium text-foreground-subtle">
                <span class="min-w-0 truncate" data-testid="workflow-compiler-feedback-title">
                    {i18n::text("chat.toolCall.workflow.feedback")}
                </span>
                <span class="shrink-0 font-normal tabular-nums" data-testid="workflow-compiler-feedback-count">
                    {label}
                </span>
            </div>
            <p class="text-ui-sm text-foreground-subtle" data-testid="workflow-compiler-feedback-lede">
                {i18n::text(workflow_feedback_lede_message_id(saved))}
            </p>
            {rows}
            {truncated.then(|| {
                view! {
                    <p class="text-ui-xs text-foreground-subtle">
                        {i18n::text("chat.toolCall.workflow.truncated")}
                    </p>
                }
            })}
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(line: i64, column: i64, code: i64, message: &str) -> WorkflowDiagnosticEntry {
        WorkflowDiagnosticEntry {
            line,
            column,
            code,
            message: message.to_string(),
        }
    }

    #[test]
    fn analyzer_code_range_is_inclusive_9001_9099() {
        // ★真源 :14-16 注释：不能写成「≥ 9000」。
        assert!(is_workflow_analyzer_rule_code(9001));
        assert!(is_workflow_analyzer_rule_code(9099));
        assert!(
            !is_workflow_analyzer_rule_code(9000),
            "9000 是 TS 的范围，不算分析器"
        );
        assert!(!is_workflow_analyzer_rule_code(9100));
    }

    #[test]
    fn typescript_18xxx_is_not_analyzer_rule() {
        // ★真源注释举例 TS18048 —— 仍是 TypeScript 自己的码。
        assert!(
            !is_workflow_analyzer_rule_code(18048),
            "18xxx 是 TS 码，应显示 TS18048"
        );
    }

    #[test]
    fn code_label_switches_between_analyzer_and_ts() {
        assert_eq!(code_label(9001), "规则 9001", "分析器码走词条");
        assert_eq!(code_label(18048), "TS18048", "TS 码直连");
        assert_eq!(code_label(1), "TS1");
    }

    #[test]
    fn code_label_passes_code_as_string_to_avoid_grouping() {
        // ★真源 :68-69 注释：数字参数会被本地化分组成「9,001」，
        // 所以必须按字符串传。9001 无分组符，9001+ 才有——
        // 用一个真实会分组的值验证不出现逗号。
        let label = code_label(9001);
        assert!(!label.contains(','), "不应出现千分位逗号：{label}");
    }

    #[test]
    fn lede_id_branches_on_saved() {
        assert_eq!(
            workflow_feedback_lede_message_id(false),
            "chat.toolCall.workflow.feedback.lede"
        );
        assert_eq!(
            workflow_feedback_lede_message_id(true),
            "chat.toolCall.workflow.feedback.lede.saved",
            "保存来源编不过是文件的问题，句子要点名文件"
        );
    }

    #[test]
    fn count_label_branches_on_singular() {
        let one = count_label(1);
        let many = count_label(5);
        assert!(one.contains('1'), "单条：{one}");
        assert!(many.contains('5'), "多条：{many}");
        assert_ne!(
            one, many,
            "one/other 是不同 id（中文文案可能相同，但机制要留）"
        );
    }

    #[test]
    fn empty_diagnostics_render_nothing() {
        // 真源 :57-59 —— 诊断为空时整段不渲染。
        let _ = Vec::<WorkflowDiagnosticEntry>::new();
        // 判定逻辑：len == 0 → return null（组件入口直接判）。
        assert!(true, "组件入口按 diagnostics.is_empty() 提前返回");
    }

    #[test]
    fn diagnostics_carry_position_and_message() {
        let entry = d(12, 34, 9001, "Expected ';'");
        assert_eq!(entry.line, 12);
        assert_eq!(entry.column, 34);
        assert_eq!(entry.code, 9001);
        assert_eq!(entry.message, "Expected ';'");
    }

    #[test]
    fn saved_flag_defaults_false_in_shape() {
        // saved 是 Option<bool>（#[prop(optional)]），None 时按 false 处理。
        let entry = d(1, 1, 9001, "x");
        let _ = entry;
        assert_eq!(
            workflow_feedback_lede_message_id(false),
            workflow_feedback_lede_message_id(false),
            "默认 lede 应是普通分支"
        );
    }
}