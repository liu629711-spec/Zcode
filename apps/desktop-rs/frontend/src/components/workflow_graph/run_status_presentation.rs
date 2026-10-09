//! 1:1 翻译 `packages/ui/src/components/workflow-graph/run-status-presentation.ts`（87 行）。
//!
//! 状态灯的全部形与色。消费方自带尺寸与 `rounded-full`；这里只给形状差异（脉冲 / 实点 /
//! 晕圈 / 空环）与颜色。时间线的站灯、侧栏清单、run 目录、任务列表共用——一个特性只有一套
//! 「运行中」的画法。
//!
//! running 用 warning（活动色）而不是 primary：primary 跨主题反色（亮色下近黑），读作「强调」
//! 而非「在动」；活动色让给真正在动的东西（灯、行进虚线、转圈）。

use super::run_state::WorkflowRunStatus;
use super::types::StepRunStatus;

/// `STATUS_DOT`（真源 :12-17）：站点灯的四态。
pub fn status_dot_class(status: StepRunStatus) -> &'static str {
    match status {
        StepRunStatus::Done => "bg-success",
        StepRunStatus::Failed => "bg-destructive ring-2 ring-destructive/30",
        StepRunStatus::Pending => "border-[1.5px] border-foreground-subtlest bg-transparent",
        StepRunStatus::Running => "animate-pulse bg-warning motion-reduce:animate-none",
    }
}

/// `DRAFT_FEEDBACK_DOT`（真源 :24-27）：编译反馈行的空环灯。形状与「compiled」、
/// 待启动的站同为空环：什么都没跑。绝不用 destructive：这个特性里红色只属于出错的 run。
pub fn draft_feedback_dot_class(open: bool) -> &'static str {
    if open {
        "border-[1.5px] border-warning bg-transparent"
    } else {
        status_dot_class(StepRunStatus::Pending)
    }
}

/// `RUN_STATUS_DOT`（真源 :35-41）：run **整体**状态的视觉词汇表（五值），从四值 `STATUS_DOT`
/// 派生。`stopped` 走中性色而不是 destructive：停下（用户取消、进程亡故、模型侧错误）是可恢复的
/// 状态，不是脚本故障；只有 `errored` 才是 destructive。
pub fn run_status_dot_class(status: WorkflowRunStatus) -> &'static str {
    match status {
        WorkflowRunStatus::Pending => status_dot_class(StepRunStatus::Pending),
        WorkflowRunStatus::Running => status_dot_class(StepRunStatus::Running),
        WorkflowRunStatus::Completed => status_dot_class(StepRunStatus::Done),
        WorkflowRunStatus::Errored => status_dot_class(StepRunStatus::Failed),
        // stopped：中性空环，与「尚未开始」同一形状。
        WorkflowRunStatus::Stopped => status_dot_class(StepRunStatus::Pending),
    }
}

/// `RUN_STATUS_TEXT`（真源 :44-50）：状态词的语义色。与状态点同一套判断，只是换成文字通道
/// （状态永远有词，不只靠颜色）。
pub fn run_status_text_class(status: WorkflowRunStatus) -> &'static str {
    match status {
        WorkflowRunStatus::Pending => "text-foreground-subtle",
        WorkflowRunStatus::Running => "text-warning",
        WorkflowRunStatus::Completed => "text-success",
        WorkflowRunStatus::Errored => "text-destructive",
        WorkflowRunStatus::Stopped => "text-foreground-subtle",
    }
}

/// `readWorkflowRunStopReason`（真源 :67-77）：`stopped` 的原因词。按结构读一个可选键，
/// 不绑死某一个协议类型——读侧对象可能是投影 run、发现查询摘要或工具卡 display。
/// 真源的 run 入参在这里拆成两个字段（status + stopReason 文本）。
pub fn read_workflow_run_stop_reason(
    status: WorkflowRunStatus,
    stop_reason: Option<&str>,
) -> Option<&'static str> {
    if status != WorkflowRunStatus::Stopped {
        return None;
    }
    let reason = stop_reason?;
    match reason {
        "user" => Some("user"),
        "model" => Some("model"),
        "provider" => Some("provider"),
        "interrupted" => Some("interrupted"),
        // 被一次 AmendWorkflow 停下并替代：灯仍是 stopped 的中性空环，差别在词与那条指向后继的链接。
        "superseded" => Some("superseded"),
        _ => None,
    }
}

/// `workflowRunStopReasonMessageId`（真源 :80-82）：原因词的 i18n key。
pub fn workflow_run_stop_reason_message_id(reason: &str) -> String {
    format!("chat.toolCall.workflow.run.stopReason.{reason}")
}

/// `isWorkflowRunSuperseded`（真源 :85-87）：run 是否被一次修订停下并替代——卡与详情页据此
/// 换种类词、藏 Resume 位、画指向后继的链接。
pub fn is_workflow_run_superseded(
    status: WorkflowRunStatus,
    stop_reason: Option<&str>,
) -> bool {
    read_workflow_run_stop_reason(status, stop_reason) == Some("superseded")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_dot_palette_matches_truth_source() {
        // 真源 :12-17 —— 四态各自的形状与颜色。
        assert_eq!(status_dot_class(StepRunStatus::Done), "bg-success");
        assert_eq!(
            status_dot_class(StepRunStatus::Failed),
            "bg-destructive ring-2 ring-destructive/30"
        );
        assert_eq!(
            status_dot_class(StepRunStatus::Pending),
            "border-[1.5px] border-foreground-subtlest bg-transparent"
        );
        assert_eq!(
            status_dot_class(StepRunStatus::Running),
            "animate-pulse bg-warning motion-reduce:animate-none"
        );
    }

    #[test]
    fn run_dot_derives_from_station_dot() {
        // 真源 :35-41 —— 终态映射：stopped 走 pending 的中性空环。
        assert_eq!(run_status_dot_class(WorkflowRunStatus::Stopped), status_dot_class(StepRunStatus::Pending));
        assert_eq!(run_status_dot_class(WorkflowRunStatus::Completed), status_dot_class(StepRunStatus::Done));
        assert_eq!(run_status_dot_class(WorkflowRunStatus::Errored), status_dot_class(StepRunStatus::Failed));
        assert_eq!(run_status_dot_class(WorkflowRunStatus::Running), status_dot_class(StepRunStatus::Running));
    }

    #[test]
    fn run_text_palette() {
        // 真源 :44-50 —— stopped 与 pending 同为次要色，errored 红色。
        assert_eq!(run_status_text_class(WorkflowRunStatus::Stopped), "text-foreground-subtle");
        assert_eq!(run_status_text_class(WorkflowRunStatus::Running), "text-warning");
        assert_eq!(run_status_text_class(WorkflowRunStatus::Completed), "text-success");
        assert_eq!(run_status_text_class(WorkflowRunStatus::Errored), "text-destructive");
    }

    #[test]
    fn stop_reason_reads_only_for_stopped_runs() {
        // 真源 :67-77 —— 只有 stopped 有原因词；词必须在白名单里。
        assert_eq!(read_workflow_run_stop_reason(WorkflowRunStatus::Stopped, Some("user")), Some("user"));
        assert_eq!(read_workflow_run_stop_reason(WorkflowRunStatus::Stopped, Some("superseded")), Some("superseded"));
        assert_eq!(read_workflow_run_stop_reason(WorkflowRunStatus::Stopped, Some("mystery")), None);
        assert_eq!(read_workflow_run_stop_reason(WorkflowRunStatus::Completed, Some("user")), None);
        assert_eq!(read_workflow_run_stop_reason(WorkflowRunStatus::Stopped, None), None);
    }

    #[test]
    fn superseded_predicate_matches_truth_source() {
        assert!(is_workflow_run_superseded(WorkflowRunStatus::Stopped, Some("superseded")));
        assert!(!is_workflow_run_superseded(WorkflowRunStatus::Stopped, Some("user")));
        assert!(!is_workflow_run_superseded(WorkflowRunStatus::Completed, Some("superseded")));
    }

    #[test]
    fn reason_message_id_prefix() {
        assert_eq!(
            workflow_run_stop_reason_message_id("user"),
            "chat.toolCall.workflow.run.stopReason.user"
        );
    }
}
