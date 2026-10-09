//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/WorkflowTruncatedNotice.tsx`（40 行）。
//!
//! 「仅展示 {shown}/{total} 步的详情」：run 撞过 `WORKFLOW_RUNS_LIMITS.maxNodes` 之后，
//! 实例表停在界上，而它上面那些数（步数、结算数）已经把表外的算进来了。这一行说的
//! 正是这个差额——**没停的是 run，停的是每一步的详情**。
//!
//! 卡与详情页共用一个实现：同一条 run 在两个面上必须说同一句话。
//! 只在真有实例被拒之表外时出现：`truncated` 本身还会被 reports / artifacts / phases
//! 等小表触界置位（workflow-runs.ts），那时步数一个不少，再念一句「仅展示 40/40 步」
//! 是句废话。

use leptos::prelude::*;

use crate::components::workflow_graph::run_state::{workflow_run_step_counts, WorkflowRunState};
use crate::ToolCallBlocks::i18n;

/// 截断提示（真源 `WorkflowTruncatedNotice`，:15-40）。
#[component]
pub fn WorkflowTruncatedNotice(
    #[prop(optional, into)] class: String,
    /// 活投影里的这条 run；缺席（run 已被淘汰出投影）即无从谈起。
    run: Option<WorkflowRunState>,
    test_id: String,
) -> impl IntoView {
    let Some(run) = run.filter(|run| run.truncated == Some(true)) else {
        return view! { <span class="hidden" /> }.into_any();
    };
    let shown = run.nodes.len();
    let counts = workflow_run_step_counts(&run);
    if shown as i64 >= counts.total {
        return view! { <span class="hidden" /> }.into_any();
    }
    view! {
        <p
            class=format!("min-w-0 text-ui-xs text-foreground-subtlest {class}")
            data-testid=test_id
        >
            // 两个数不加千分位：紧挨着的摘要行那一段就是裸数字（真源 :35-37）。
            {i18n::format(
                "chat.toolCall.workflow.run.truncated",
                &[
                    ("shown".to_string(), shown.to_string()),
                    ("total".to_string(), counts.total.to_string()),
                ],
            )}
        </p>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_with(truncated: Option<bool>, node_count: usize, unlisted: i64) -> WorkflowRunState {
        let nodes: Vec<serde_json::Value> = (0..node_count)
            .map(|i| {
                serde_json::json!({
                    "siteId": format!("s{i}"), "ordinal": i as i64, "phase": "settled"
                })
            })
            .collect();
        serde_json::from_value(serde_json::json!({
            "runId": "r1", "status": "running",
            "truncated": truncated,
            "actors": [], "nodes": nodes,
            "usage": {
                "spentTokens": 0, "nodesUsed": node_count as i64,
                "nodesUnlisted": unlisted
            }
        }))
        .unwrap()
    }

    #[test]
    fn shown_covers_total_means_no_notice() {
        // 真源 :28-29 —— shown >= total：不念废话。40 表内 + 0 表外 → total=40。
        let run = run_with(Some(true), 40, 0);
        let counts = workflow_run_step_counts(&run);
        assert_eq!(counts.total, 40);
        assert!(run.nodes.len() as i64 >= counts.total);
    }

    #[test]
    fn notice_appears_only_when_instances_were_refused() {
        // 40 表内 + 3 表外 → total=43 > shown=40：差额真实存在。
        let run = run_with(Some(true), 40, 3);
        let counts = workflow_run_step_counts(&run);
        assert_eq!(counts.total, 43);
        assert!((run.nodes.len() as i64) < counts.total);
    }

    #[test]
    fn absent_run_or_truncated_false_is_silent() {
        // 缺席 / truncated ≠ true：组件整个缺席。
        assert_eq!(run_with(None, 40, 3).truncated, None);
        assert_eq!(run_with(Some(false), 40, 3).truncated, Some(false));
    }
}
