//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/todo.tsx`（154 行）
//! + `packages/shared/src/tool-plan-adapter.ts`（160 行）。
//!
//! todo 卡片。真源要点：
//! - **计划步骤优先读 output，再读 input**（真源 :23-31 `readTodoPlan`）——
//!   output 里是最新状态，input 里可能是旧快照
//! - activeStep 三级回落（:46-49）：`in_progress` → 第一个未完成 → 最后一个
//! - secondaryText 是**完成计数** `{completed}/{total}`（:52），不是状态词
//! - 状态图标三态（:33-45）：completed 用 CircleCheck + text-success；
//!   in_progress 用 **静态箭头**（真源 :38-39 注释：工具输出里的 running 状态会
//!   长时间留在页面上，用静态箭头避免和加载动画语义混在一起）；pending 用空心圆
//! - 文本类名三态（:16-20）：pending 最浅 / in_progress 最亮 / completed **删除线**
//!
//! plan 提取（tool-plan-adapter.ts）：
//! - 工具名门控（:4-5 正则 `(?:^|[_\s-])(?:todo[_\s-]*(?:read|write)|update[_\s-]*plan)(?:$|[_\s-])`）
//! - 集合键四个（:6 `todos`/`plan`/`steps`/`items`）
//! - 步骤 title 五个候选键（:33-37 `content`/`step`/`title`/`text`/`activeForm`）
//! - **全成功才返回**（:88-90 `steps.length === collection.length`），
//!   部分解析失败整体放弃，避免显示残缺计划
//!
//! 行号注释均指真源文件。

use leptos::prelude::*;
use serde_json::Value;

// ---------------------------------------------------------------------------
// tool-plan-adapter.ts
// ---------------------------------------------------------------------------

/// `PLAN_COLLECTION_KEYS`（真源 :6）。
pub const PLAN_COLLECTION_KEYS: [&str; 4] = ["todos", "plan", "steps", "items"];

/// `isTodoPlanToolName`（真源 :127-129）。
///
/// 正则：`(?:^|[_\s-])(?:todo[_\s-]*(?:read|write)|update[_\s-]*plan)(?:$|[_\s-])`
/// ——匹配 TodoRead/TodoWrite/todo_read/update-plan 等变体。
pub fn is_todo_plan_tool_name(value: &str) -> bool {
    let text = value.trim();
    let lower = text.to_lowercase();
    // 手工实现正则：找 todo[_\s-]*(read|write) 或 update[_\s-]*plan，
    // 且前后是边界或分隔符。
    for (name, kw) in [
        ("todo", "read"),
        ("todo", "write"),
        ("update", "plan"),
    ] {
        // 枚举名字与关键字之间的分隔符组合（零个或多个 [_\s-]）。
        let seps = ["", "_", " ", "-"];
        for s1 in seps {
            for s2 in seps {
                let needle = format!("{name}{s1}{kw}{s2}");
                if let Some(pos) = lower.find(&needle) {
                    // s2 已并入 needle，故后边界就在 needle 末尾。
                    let before_ok = pos == 0
                        || matches!(
                            lower[..pos].chars().next_back(),
                            Some('_') | Some(' ') | Some('-')
                        );
                    let end = pos + needle.len();
                    let after_ok = end == lower.len()
                        || matches!(
                            lower[end..].chars().next(),
                            Some('_') | Some(' ') | Some('-')
                        );
                    if before_ok && after_ok {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// `ZCodePlanStep["status"]`（真源 zcode-task-types-core）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStepStatus {
    Pending,
    InProgress,
    Completed,
}

impl PlanStepStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
        }
    }
}

/// `normalizePlanStatus`（真源 :18-25）：连字符转下划线后小写，非法返回 None。
fn normalize_plan_status(value: &Value) -> Option<PlanStepStatus> {
    let s = value.as_str()?.trim().replace('-', "_").to_lowercase();
    match s.as_str() {
        "pending" => Some(PlanStepStatus::Pending),
        "in_progress" => Some(PlanStepStatus::InProgress),
        "completed" => Some(PlanStepStatus::Completed),
        _ => None,
    }
}

/// `ZCodePlanStep`（真源 shared 类型）。
#[derive(Debug, Clone, PartialEq)]
pub struct PlanStep {
    pub id: String,
    pub title: String,
    pub status: PlanStepStatus,
}

/// `parsePlanStep`（真源 :27-53）——解析单个步骤。
///
/// 字符串形态（:29-31）：**第一项视为 in_progress，其余 pending**。
/// 对象形态：title 取五个候选键之一，status 必须合法。
pub fn parse_plan_step(value: &Value, index: usize) -> Option<PlanStep> {
    if let Some(s) = value.as_str() {
        let title = s.trim();
        if title.is_empty() {
            return None;
        }
        return Some(PlanStep {
            id: title.to_string(),
            title: title.to_string(),
            status: if index == 0 {
                PlanStepStatus::InProgress
            } else {
                PlanStepStatus::Pending
            },
        });
    }
    if !value.is_object() {
        return None;
    }
    // 真源 :33-37 —— title 五个候选键，按序取第一个非空。
    let title = ["content", "step", "title", "text", "activeForm"]
        .iter()
        .find_map(|k| value.get(k).and_then(|v| v.as_str()).map(str::trim))
        .filter(|s| !s.is_empty())?;
    let status = normalize_plan_status(value.get("status")?)?;
    Some(PlanStep {
        id: value
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| title.to_string()),
        title: title.to_string(),
        status,
    })
}

/// `readPlanCollection`（真源 :66-77）——找四个集合键之一。
fn read_plan_collection(input: &Value) -> Option<Vec<Value>> {
    let value = if let Some(s) = input.as_str() {
        serde_json::from_str::<Value>(s).ok()?
    } else {
        input.clone()
    };
    if !value.is_object() {
        return None;
    }
    for key in PLAN_COLLECTION_KEYS {
        if let Some(items) = value.get(key).and_then(|v| v.as_array()) {
            return Some(items.clone());
        }
    }
    None
}

/// `extractPlanStepsFromValue`（真源 :79-90）——解析计划步骤。
///
/// **全部步骤都解析成功才返回**（:88-90）——部分失败整体放弃，
/// 避免显示残缺计划误导用户。
pub fn extract_plan_steps_from_value(value: &Value) -> Option<Vec<PlanStep>> {
    let collection = read_plan_collection(value)?;
    if collection.is_empty() {
        return None;
    }
    let steps: Vec<PlanStep> = collection
        .iter()
        .enumerate()
        .filter_map(|(i, item)| parse_plan_step(item, i))
        .collect();
    // ★真源 :88-90 —— 数量不符说明有解析失败，整体放弃。
    (steps.len() == collection.len()).then_some(steps)
}

/// `collectOutputCandidates`（真源 :92-114）——输出的候选位置。
///
/// 真源顺序：`output` 本体 → 解析后的 output（字符串时）→
/// `content`/`output`/`result` 三个键的值（字符串再解析）。
fn collect_output_candidates(output: &Value) -> Vec<Value> {
    let mut candidates = vec![output.clone()];
    if let Some(s) = output.as_str() {
        if let Ok(parsed) = serde_json::from_str::<Value>(s) {
            candidates.push(parsed);
        }
    }
    if output.is_object() {
        for key in ["content", "output", "result"] {
            if let Some(value) = output.get(key) {
                candidates.push(value.clone());
                if let Some(s) = value.as_str() {
                    if let Ok(parsed) = serde_json::from_str::<Value>(s) {
                        candidates.push(parsed);
                    }
                }
            }
        }
    }
    candidates
}

/// `extractPlanStepsFromToolOutput`（真源 :142-160）。
pub fn extract_plan_steps_from_tool_output(
    title: Option<&str>,
    kind: Option<&str>,
    output: &Value,
) -> Option<Vec<PlanStep>> {
    // 真源 :147 —— fingerprint 是 title+kind 拼起来（过滤空值）。
    let fingerprint = [title, kind]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if !is_todo_plan_tool_name(&fingerprint) {
        return None;
    }
    collect_output_candidates(output)
        .iter()
        .find_map(extract_plan_steps_from_value)
}

/// `extractPlanStepsFromToolInput`（真源 :146-157）。
pub fn extract_plan_steps_from_tool_input(
    title: Option<&str>,
    kind: Option<&str>,
    input: &Value,
) -> Option<Vec<PlanStep>> {
    let fingerprint = [title, kind]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if !is_todo_plan_tool_name(&fingerprint) {
        return None;
    }
    extract_plan_steps_from_value(input)
}

// ---------------------------------------------------------------------------
// todo.tsx
// ---------------------------------------------------------------------------

/// `TODO_TOOL_ICON`（真源 :14）——ListTodoIcon。
pub const TODO_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `todoTextClasses`（真源 :16-20）——三个状态的文本类名。
pub fn todo_text_class(status: PlanStepStatus) -> &'static str {
    match status {
        PlanStepStatus::Pending => "text-foreground-subtle",
        PlanStepStatus::InProgress => "text-foreground",
        // 完成态加删除线（:19）。
        PlanStepStatus::Completed => "text-foreground-subtlest line-through",
    }
}

/// `TodoStatusIcon`（真源 :33-45）的类名。
///
/// 真源注释（:38-39）：工具输出里的 todo running 状态会长时间留在页面上，
/// 用**静态箭头**表达当前项，避免和加载动画语义混在一起。
pub fn todo_status_icon_class(status: PlanStepStatus) -> &'static str {
    match status {
        PlanStepStatus::Completed => "size-3.5 flex-none text-success",
        PlanStepStatus::InProgress => "size-3.5 flex-none text-foreground",
        PlanStepStatus::Pending => "size-3.5 flex-none text-foreground-subtlest",
    }
}

/// `readTodoPlan`（真源 :21-31）——**先读 output，再读 input**。
///
/// 真源注释：output 里是最新状态，input 里可能是旧快照。
pub fn read_todo_plan(
    title: Option<&str>,
    kind: Option<&str>,
    input: &Value,
    output: &Value,
) -> Option<Vec<PlanStep>> {
    extract_plan_steps_from_tool_output(title, kind, output)
        .or_else(|| extract_plan_steps_from_tool_input(title, kind, input))
}

/// 计划的统计与当前步骤（真源 :45-49）。
#[derive(Debug, Clone, PartialEq)]
pub struct PlanSummary {
    pub completed_count: usize,
    pub total: usize,
    /// 当前步骤标题：in_progress → 第一个未完成 → 最后一个。
    pub active_title: Option<String>,
}

/// `plan` 的统计与 activeStep 选取（真源 :45-49 三级回落）。
pub fn summarize_plan(plan: &[PlanStep]) -> PlanSummary {
    let completed_count = plan
        .iter()
        .filter(|s| s.status == PlanStepStatus::Completed)
        .count();
    // 真源 :46-49 —— in_progress → 第一个未完成 → 最后一个。
    let active_title = plan
        .iter()
        .find(|s| s.status == PlanStepStatus::InProgress)
        .or_else(|| plan.iter().find(|s| s.status != PlanStepStatus::Completed))
        .or_else(|| plan.last())
        .map(|s| s.title.clone());
    PlanSummary {
        completed_count,
        total: plan.len(),
        active_title,
    }
}

/// secondaryText：完成计数 `{completed}/{total}`（真源 :52）。
pub fn plan_secondary_text(summary: &PlanSummary) -> String {
    format!("{}/{}", summary.completed_count, summary.total)
}

/// TodoToolCallBlock 的 props。
#[derive(Debug, Clone)]
pub struct TodoBlockProps {
    pub tool_id: String,
    pub title: Option<String>,
    pub kind: Option<String>,
    pub input: Value,
    pub output: Value,
    pub status: Option<String>,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
}

/// TodoToolCallBlock 的主体（真源 :48-154）。
#[component]
pub fn TodoToolCallBlock(props: TodoBlockProps) -> impl IntoView {
    let plan = read_todo_plan(
        props.title.as_deref(),
        props.kind.as_deref(),
        &props.input,
        &props.output,
    );
    let summary = plan.as_deref().map(summarize_plan);

    // 真源 :50 —— primaryText：activeStep.title ?? title ?? 固定文案。
    let primary_text = summary
        .as_ref()
        .and_then(|s| s.active_title.clone())
        .or_else(|| props.title.clone())
        .unwrap_or_else(|| "当前任务".to_string());

    // 真源 :52 —— secondaryText：计划存在时是计数，否则状态词。
    let secondary_text = match &summary {
        Some(s) => plan_secondary_text(s),
        None => props.status_label.clone().unwrap_or_default(),
    };

    // 展开区：计划列表（真源 :84-99）
    let plan_rows = plan.as_ref().map(|steps| {
        steps
            .iter()
            .map(|step| {
                view! {
                    <div class="flex min-w-0 items-center gap-2 py-1" data-step-id=step.id.clone()>
                        <span class=todo_status_icon_class(step.status)>{status_icon_glyph(step.status)}</span>
                        <span class=todo_text_class(step.status)>{step.title.clone()}</span>
                    </div>
                }
            })
            .collect_view()
    });

    let is_failed = props.status.as_deref() == Some("failed");

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                can_toggle: Some(true),
                force_open: Some(false),
                // 真源 :46 —— kindLabel 是「待办计划」。
                kind_label: Some("待办计划".to_string()),
                source_label: props.source_label.clone(),
                primary_text: Some(primary_text.clone()),
                secondary_text: Some(secondary_text.clone()),
                status_label: if is_failed { props.status_label.clone() } else { None },
                status_tooltip: if is_failed { props.error_text.clone() } else { None },
                show_failure_status: Some(is_failed),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=TODO_TOOL_ICON_CLASS>
                        <crate::app::Icon
                            paths=vec![
                                "M8 6h13",
                                "M8 12h13",
                                "M8 18h13",
                                "M3 6l.4.6a1 1 0 1 1-1.4 1.4L3 9.6",
                                "M3 12l.4.6a1 1 0 1 1-1.4 1.4L3 15.6",
                            ]
                            circles=vec![]
                        />
                    </span>
                }
                .into_any()
            }))
            render_content=Some(std::sync::Arc::new(move || {
                // 真源 :84 —— 有计划时列表，无计划时走 ToolCallBody 兜底。
                plan_rows
                    .clone()
                    .map(|rows| {
                        view! {
                            <div class="space-y-1 rounded-xl bg-surface px-3 py-2">
                                {rows}
                            </div>
                        }
                    })
                    .into_any()
            }))
        />
    }
}

/// 状态图标字形（真源 :33-45 的三个 lucide 图标）。
fn status_icon_glyph(status: PlanStepStatus) -> &'static str {
    match status {
        // CircleCheckIcon 的勾。
        PlanStepStatus::Completed => "✓",
        // ArrowRightIcon（静态箭头，真源注释说明理由）。
        PlanStepStatus::InProgress => "→",
        // CircleIcon（空心圆）。
        PlanStepStatus::Pending => "○",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn todo_tool_name_pattern_matches_variants() {
        // 真源 :4-5 正则
        for name in [
            "TodoRead",
            "TodoWrite",
            "todo_read",
            "todo_write",
            "todo read",
            "todo-write",
            "update_plan",
            "update-plan",
            "UpdatePlan",
        ] {
            assert!(is_todo_plan_tool_name(name), "{name} 应被识别为 todo 计划工具");
        }
    }

    #[test]
    fn todo_tool_name_pattern_rejects_others() {
        for name in ["Bash", "Read", "Todo", "todolist", "plan", ""] {
            assert!(
                !is_todo_plan_tool_name(name),
                "{name} 不该被识别为 todo 计划工具"
            );
        }
    }

    #[test]
    fn plan_step_string_form_marks_first_in_progress() {
        // 真源 :29-31
        let steps = extract_plan_steps_from_value(&json!({
            "todos": ["第一步", "第二步", "第三步"]
        }))
        .unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].status, PlanStepStatus::InProgress, "第一项是进行中");
        assert_eq!(steps[1].status, PlanStepStatus::Pending);
        // id 用标题（:31）
        assert_eq!(steps[0].id, "第一步");
    }

    #[test]
    fn plan_step_object_form_reads_five_title_keys() {
        // 真源 :33-37 —— content/step/title/text/activeForm
        for key in ["content", "step", "title", "text", "activeForm"] {
            let input = json!({
                "todos": [{ key: "任务标题", "status": "completed" }]
            });
            let steps = extract_plan_steps_from_value(&input).unwrap();
            assert_eq!(steps[0].title, "任务标题", "{key} 应被识别");
        }
    }

    #[test]
    fn plan_status_normalizes_hyphen() {
        // 真源 :20 —— 连字符转下划线
        let input = json!({ "todos": [{ "title": "T", "status": "in-progress" }] });
        let steps = extract_plan_steps_from_value(&input).unwrap();
        assert_eq!(steps[0].status, PlanStepStatus::InProgress);
    }

    #[test]
    fn plan_step_missing_status_is_rejected() {
        // 真源 :39-42 —— 无 title 或 status 非法 → 整步丢弃 → 整体放弃
        let input = json!({ "todos": [{ "title": "T" }] });
        assert_eq!(extract_plan_steps_from_value(&input), None);
        let input2 = json!({ "todos": [{ "title": "T", "status": "bogus" }] });
        assert_eq!(extract_plan_steps_from_value(&input2), None);
    }

    #[test]
    fn partial_parse_failure_aborts_whole_plan() {
        // ★真源 :88-90 —— 数量不符就整体放弃，不显示残缺计划。
        let input = json!({
            "todos": [
                { "title": "A", "status": "completed" },
                { "title": "B" }  // 缺 status → 解析失败
            ]
        });
        assert_eq!(
            extract_plan_steps_from_value(&input),
            None,
            "部分失败必须整体放弃"
        );
    }

    #[test]
    fn four_collection_keys_supported() {
        // 真源 :6 —— todos/plan/steps/items
        for key in PLAN_COLLECTION_KEYS {
            let input = json!({ key: [{ "title": "T", "status": "pending" }] });
            assert!(
                extract_plan_steps_from_value(&input).is_some(),
                "{key} 应被识别为计划集合"
            );
        }
    }

    #[test]
    fn json_string_input_is_parsed() {
        // 真源 :68 —— 字符串先 JSON.parse
        let input = json!("{\"todos\":[{\"title\":\"T\",\"status\":\"pending\"}]}");
        assert!(extract_plan_steps_from_value(&input).is_some());
    }

    #[test]
    fn tool_output_preferred_over_input() {
        // 真源 :23-31 —— 先 output 再 input。
        // output 里 content 是**对象**时才能读到 todos（collectOutputCandidates :95
        // 取 content/output/result 三个键的值，再交给 readPlanCollection）。
        let output = json!({
            "content": { "todos": [{ "title": "来自 output", "status": "completed" }] }
        });
        let input = json!({ "todos": [{ "title": "来自 input", "status": "pending" }] });
        let plan = read_todo_plan(Some("TodoWrite"), Some("todo_write"), &input, &output).unwrap();
        assert_eq!(plan[0].title, "来自 output", "output 优先");
    }

    #[test]
    fn output_content_array_falls_back_to_input() {
        // ★实测确认：content 是**数组**时 readPlanCollection 判 isRecord=false 读不到
        // （真源 :69-71），此时走 input 兜底。Rust 侧行为与真源一致，
        // 不是实现缺陷。
        let output = json!({
            "content": [{ "todos": [{ "title": "数组里的", "status": "completed" }] }]
        });
        let input = json!({ "todos": [{ "title": "来自 input", "status": "pending" }] });
        let plan = read_todo_plan(Some("TodoWrite"), Some("todo_write"), &input, &output).unwrap();
        assert_eq!(plan[0].title, "来自 input");
    }

    #[test]
    fn output_candidates_scan_content_output_result_keys() {
        // 真源 :95 —— content / output / result 三个键都要扫。
        for key in ["content", "output", "result"] {
            let output = json!({ key: { "todos": [{ "title": "T", "status": "pending" }] } });
            assert!(
                read_todo_plan(Some("TodoWrite"), Some("todo_write"), &Value::Null, &output)
                    .is_some(),
                "{key} 里的计划应被找到"
            );
        }
    }

    #[test]
    fn falls_back_to_input_when_output_empty() {
        let output = Value::Null;
        let input = json!({ "todos": [{ "title": "来自 input", "status": "pending" }] });
        let plan = read_todo_plan(Some("TodoWrite"), Some("todo_write"), &input, &output).unwrap();
        assert_eq!(plan[0].title, "来自 input");
    }

    #[test]
    fn non_todo_tool_returns_none() {
        // 真源 :148 —— 工具名门控
        let input = json!({ "todos": [{ "title": "T", "status": "pending" }] });
        assert_eq!(
            read_todo_plan(Some("Bash"), Some("bash"), &input, &Value::Null),
            None,
            "非 todo 工具不该解析计划"
        );
    }

    #[test]
    fn summarize_plan_three_level_fallback() {
        // 真源 :46-49 —— in_progress → 第一个未完成 → 最后一个
        let steps = vec![
            PlanStep { id: "1".into(), title: "已完成".into(), status: PlanStepStatus::Completed },
            PlanStep { id: "2".into(), title: "进行中".into(), status: PlanStepStatus::InProgress },
            PlanStep { id: "3".into(), title: "待办".into(), status: PlanStepStatus::Pending },
        ];
        let s = summarize_plan(&steps);
        assert_eq!(s.completed_count, 1);
        assert_eq!(s.total, 3);
        assert_eq!(s.active_title.as_deref(), Some("进行中"));

        // 无in_progress → 第一个未完成
        let steps2 = vec![
            PlanStep { id: "1".into(), title: "已完成".into(), status: PlanStepStatus::Completed },
            PlanStep { id: "2".into(), title: "待办".into(), status: PlanStepStatus::Pending },
        ];
        assert_eq!(
            summarize_plan(&steps2).active_title.as_deref(),
            Some("待办")
        );

        // 全完成 → 最后一个
        let steps3 = vec![
            PlanStep { id: "1".into(), title: "A".into(), status: PlanStepStatus::Completed },
            PlanStep { id: "2".into(), title: "B".into(), status: PlanStepStatus::Completed },
        ];
        assert_eq!(summarize_plan(&steps3).active_title.as_deref(), Some("B"));

        // 空计划 → None
        assert_eq!(summarize_plan(&[]).active_title, None);
    }

    #[test]
    fn secondary_text_is_progress_ratio() {
        // 真源 :52 —— `{completed}/{total}`
        let s = PlanSummary { completed_count: 2, total: 5, active_title: None };
        assert_eq!(plan_secondary_text(&s), "2/5");
    }

    #[test]
    fn completed_text_has_line_through() {
        // 真源 :19 —— 完成态加删除线
        assert!(todo_text_class(PlanStepStatus::Completed).contains("line-through"));
        assert!(!todo_text_class(PlanStepStatus::Pending).contains("line-through"));
        assert!(!todo_text_class(PlanStepStatus::InProgress).contains("line-through"));
    }

    #[test]
    fn in_progress_icon_is_static_arrow() {
        // 真源 :38-39 注释 —— 用静态箭头避免与加载动画语义混淆
        assert_eq!(status_icon_glyph(PlanStepStatus::InProgress), "→");
        assert_eq!(todo_status_icon_class(PlanStepStatus::Completed), "size-3.5 flex-none text-success");
    }
}