//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/execute-group.tsx`（130 行）。
//!
//! executeGroup 聚合卡——把多个连续命令聚成一个可折叠组。
//!
//! 真源要点：
//! - `ACTIVE_STATUSES = { pending, in_progress }`（:19）——注释说明 V4 row 进共享
//!   renderer 前会把 inputStreaming/pendingApproval 统一适配为 pending
//! - **找最后一个活跃子命令**（:47-49 `findLast`），没有就用最后一个（:50）
//! - 运行时主文本 =「正在执行」，完成态 = 统计摘要（:74）
//! - `expandedSecondaryText={null}`（:86）——真源注释：ToolLayout 默认会在展开态
//!   沿用 secondaryText，导致命令数量后残留当前命令；父组展开后由子tool summary
//!   表达当前命令，故必须显式清空
//! - `summaryContentSeparator="·"`（:80）
//! - `summaryContentKey`（:90-95）——运行中带 latestChild 参与，保证滚轮动画
//!   在命令切换时重新触发
//! - runningSecondaryText 显式 `font-sans`（:66-68 真源注释：Tailwind v4 preflight
//!   给 code 默认 mono，不显式指定会与 execute/explore 收起态的 sans 约定不一致）

use leptos::prelude::*;
use serde_json::Value;

use super::execute::get_execute_secondary_text;

/// `EXECUTE_GROUP_ICON`（真源 :14）——SquareTerminalIcon。
pub const EXECUTE_GROUP_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `ACTIVE_STATUSES`（真源 :19）。
pub const ACTIVE_STATUSES: [&str; 2] = ["pending", "in_progress"];

/// 子工具调用（真源 childToolCalls 的 `child.toolCall` + 完整节点）。
///
/// `input`/`status` 是主摘要用的便捷字段；`node` 供 children 递归渲染
/// （真源 :56-70 逐项 `<ToolCallBlock toolCallNode={child} />`）。
#[derive(Debug, Clone)]
pub struct ChildToolCall {
    pub tool_id: String,
    pub input: Value,
    pub status: String,
    pub node: crate::ToolCallBlocks::toolCallRowAdapter::LegacyToolCallNode,
}

/// 状态是否活跃（真源 :19）。
pub fn is_active_status(status: &str) -> bool {
    ACTIVE_STATUSES.contains(&status)
}

/// 找最后一个活跃子命令，没有则取最后一个（真源 :47-50）。
pub fn latest_child<'a>(children: &'a [ChildToolCall]) -> Option<&'a ChildToolCall> {
    children
        .iter()
        .rev()
        .find(|c| is_active_status(&c.status))
        .or_else(|| children.last())
}

/// `formatCompletedSummary`（真源 :23-44）——完成态统计摘要。
///
/// 格式：`{count} 个命令` + 可选 `, {failed} 个失败` + 可选 `, {stopped} 个已停止`。
/// （zh-CN.ts:4878-4881）
pub fn format_completed_summary(child_statuses: &[String]) -> String {
    let count = child_statuses.len();
    let mut parts = vec![format!("{count} 个命令")];
    let failed = child_statuses.iter().filter(|s| *s == "failed").count();
    let stopped = child_statuses.iter().filter(|s| *s == "stopped").count();
    if failed > 0 {
        parts.push(format!("{failed} 个失败"));
    }
    if stopped > 0 {
        parts.push(format!("{stopped} 个已停止"));
    }
    parts.join(", ")
}

/// `summaryContentKey`（真源 :90-95）——滚轮动画的key。
///
/// 运行中带 latestChild 参与，命令切换时 key 变化触发动画；
/// 完成态用 done + 统计摘要。
pub fn summary_content_key(
    tool_id: &str,
    is_running: bool,
    latest_child_id: Option<&str>,
    running_action_label: Option<&str>,
    latest_command: Option<&str>,
    completed_summary: &str,
) -> String {
    if is_running && latest_child_id.is_some() {
        format!(
            "execute:{tool_id}:{}:{}:{}",
            latest_child_id.unwrap_or_default(),
            running_action_label.unwrap_or("running"),
            latest_command.unwrap_or("command")
        )
    } else {
        format!("execute:{tool_id}:done:{completed_summary}")
    }
}

/// ExecuteGroupToolCallBlock 的 props。
#[derive(Debug, Clone)]
pub struct ExecuteGroupProps {
    pub tool_id: String,
    pub title: Option<String>,
    pub child_tool_calls: Vec<ChildToolCall>,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub is_office_mode: bool,
    pub can_toggle: Option<bool>,
    pub force_open: Option<bool>,
}

/// ExecuteGroupToolCallBlock 的主体（真源 :46-130）。
#[component]
pub fn ExecuteGroupToolCallBlock(props: ExecuteGroupProps) -> impl IntoView {
    let latest = latest_child(&props.child_tool_calls);
    // 真源 :51 —— office 模式不取命令。
    let latest_command = if props.is_office_mode {
        None
    } else {
        latest.map(|c| get_execute_secondary_text(&c.input))
    };
    let latest_command = latest_command.flatten();

    let running_action_label = latest_command.as_ref().map(|_| "正在执行");
    let completed_summary = format_completed_summary(
        &props
            .child_tool_calls
            .iter()
            .map(|c| c.status.clone())
            .collect::<Vec<_>>(),
    );

    // 闭包要用 child_tool_calls —— 提前收集数据（View 非 Clone，需在闭包内重建）。
    // 真源 :56-70：每个 child 是完整的 ToolCallBlock 递归（showIcon=false）；
    // 容器类名 `ml-2 space-y-2 border-border border-l pl-3.5` 照抄。
    let child_context = crate::ToolCallBlocks::ToolCallBlock::ToolCallBlockContext {
        show_icon: false,
        ..Default::default()
    };
    let children_data: Vec<(
        String,
        crate::ToolCallBlocks::toolCallRowAdapter::LegacyToolCallNode,
    )> = props
        .child_tool_calls
        .iter()
        .map(|child| (child.tool_id.clone(), child.node.clone()))
        .collect();
    let key = summary_content_key(
        &props.tool_id,
        props.is_running,
        latest.map(|c| c.tool_id.as_str()),
        running_action_label,
        latest_command.as_deref(),
        &completed_summary,
    );

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                // 真源 :72-73 —— office 模式强制不可折叠/不展开。
                can_toggle: Some(!props.is_office_mode && props.can_toggle.unwrap_or(true)),
                force_open: Some(!props.is_office_mode && props.force_open.unwrap_or(false)),
                kind_label: Some("终端".to_string()),
                // 真源 :74 —— 展开态 kindLabel 同（终端）。
                expanded_kind_label: Some("终端".to_string()),
                // 真源 :79 —— 运行中显「正在执行」，完成态显统计摘要。
                primary_text: if props.is_running && running_action_label.is_some() {
                    Some("正在执行".to_string())
                } else {
                    Some(completed_summary.clone())
                },
                // 真源 :80 —— 运行态显示当前命令。
                secondary_text: if props.is_running { latest_command.clone() } else { None },
                summary_content_separator: Some("·".to_string()),
                expanded_primary_text: Some(completed_summary.clone()),
                // 真源 :86 —— 展开态必须显式清空次文本（默认会沿用，导致
                // 命令数量后残留当前命令）。
                expanded_secondary_text: Some(String::new()),
                status_label: props.status_label.clone(),
                is_running: Some(props.is_running),
                title: if props.is_office_mode {
                    None
                } else if props.is_running && latest_command.is_some() {
                    latest_command.clone()
                } else {
                    props.title.clone()
                },
                expanded_title: props.title.clone(),
                summary_content_key: Some(key),
                ..Default::default()
            }
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class=EXECUTE_GROUP_ICON_CLASS>
                        <crate::app::Icon paths=vec!["M7 11l-4 3 4 3", "M12 17h6"] circles=vec![] />
                    </span>
                }
                .into_any()
            }))
            render_content=Some(std::sync::Arc::new(move || {
                // 真源 :89-104 —— 展开区是子卡列表，缩进容器 + 不显示图标。
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn child(id: &str, status: &str) -> ChildToolCall {
        use crate::ToolCallBlocks::toolCallRowAdapter::tool_call_row_to_legacy_node;
        let node = tool_call_row_to_legacy_node(&serde_json::json!({
            "kind": "toolCall", "rowId": 0, "toolCallId": id,
            "toolName": "Bash", "status": "success", "inputText": "",
        }));
        ChildToolCall {
            tool_id: id.into(),
            input: Value::Null,
            status: status.into(),
            node,
        }
    }

    #[test]
    fn active_statuses_match_source() {
        // 真源 :19 —— pending / in_progress
        assert!(is_active_status("pending"));
        assert!(is_active_status("in_progress"));
        assert!(!is_active_status("completed"));
        assert!(!is_active_status("failed"));
    }

    #[test]
    fn latest_child_prefers_last_active() {
        // 真源 :47-49 —— findLast 找活跃子命令。
        let children = vec![
            child("a", "completed"),
            child("b", "in_progress"),
            child("c", "completed"),
            child("d", "pending"),
        ];
        assert_eq!(latest_child(&children).unwrap().tool_id, "d");
    }

    #[test]
    fn latest_child_falls_back_to_last() {
        // 真源 :50 —— 无活跃则取最后一个。
        let children = vec![child("a", "completed"), child("b", "failed")];
        assert_eq!(latest_child(&children).unwrap().tool_id, "b");
        // 空数组 → None
        assert!(latest_child(&[]).is_none());
    }

    #[test]
    fn completed_summary_formats_counts() {
        // 真源 :23-44 + zh-CN.ts:4878-4881
        assert_eq!(format_completed_summary(&[]), "0 个命令");
        // 单个也用「{count} 个命令」（zh-CN 里 one/other 文案相同）。
        assert_eq!(format_completed_summary(&["completed".into()]), "1 个命令");
        assert_eq!(
            format_completed_summary(&["completed".into(), "completed".into()]),
            "2 个命令"
        );
    }

    #[test]
    fn completed_summary_includes_failed_and_stopped() {
        // 真源 :30-42 —— 失败/停止计数按需追加。
        let statuses = vec![
            "completed".to_string(),
            "failed".to_string(),
            "stopped".to_string(),
        ];
        let summary = format_completed_summary(&statuses);
        assert!(summary.starts_with("3 个命令"));
        assert!(summary.contains("1 个失败"));
        assert!(summary.contains("1 个已停止"));
        // 只有失败 / 只有停止。
        assert_eq!(
            format_completed_summary(&["failed".to_string()]),
            "1 个命令, 1 个失败"
        );
        assert_eq!(
            format_completed_summary(&["stopped".to_string()]),
            "1 个命令, 1 个已停止"
        );
    }

    #[test]
    fn summary_content_key_switches_on_running() {
        // 真源 :90-95 —— 运行中带 latestChild，完成态用 done。
        let running =
            summary_content_key("t1", true, Some("c1"), Some("正在执行"), Some("ls"), "S");
        assert!(
            running.starts_with("execute:t1:c1"),
            "运行态 key 含 latestChild：{running}"
        );
        assert!(running.contains("正在执行"));
        assert!(running.contains("ls"));

        let done = summary_content_key("t1", false, None, None, None, "2 个命令");
        assert_eq!(done, "execute:t1:done:2 个命令");
    }

    #[test]
    fn summary_content_key_uses_done_when_no_child() {
        // 运行中但没有子命令 → 走 done 分支（真源 `isRunning && latestChild` 才用运行态 key）。
        let key = summary_content_key("t1", true, None, None, None, "S");
        assert!(
            key.starts_with("execute:t1:done:"),
            "无子命令时用 done：{key}"
        );
    }
}
