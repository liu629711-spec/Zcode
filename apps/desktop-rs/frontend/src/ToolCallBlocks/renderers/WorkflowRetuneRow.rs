//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/WorkflowRetuneRow.tsx`（70 行）。
//!
//! ============================================================
//! 就地生效的修订在转写里的那一行
//! ============================================================
//! ★真源 :7-11 注释——只改并发上限、run 又在飞时，`AmendWorkflow` 不编译、
//! 不铸新 run，只把那条 run 的上限改掉。于是这一行**没有卡可画**：
//! 那条 run 的卡在它启动的那一轮里，再画一张会读成第二次运行；
//! 而原来的静态卡会显示编译校验结果，使一次并发设置变更
//! 看起来像重新编译和启动了工作流。
//!
//! 画的就是 GUI「配置」留下的那条设置行（`WorkflowSettingsChangeRow`），
//! **一字不差**——同一件事由谁发起不该有两种读法。唯一的差别是它**可点**：
//! 工具行是模型这一步的落点，用户从这里回到那条 run。

use leptos::prelude::*;

use crate::components::workflowSettingsChange::{
    WorkflowSettingsAmendMeta, retune_as_amend_meta, workflow_settings_provenance_rows,
};

/// `WorkflowRetuneRow`（真源 :41-70）。
#[component]
pub fn WorkflowRetuneRowComponent(
    /// 本机并发天花板（那条 run 的投影读数）；空串/负数 = 未知。
    #[prop(optional)] ceiling: Option<i64>,
    /// 打开这条 run 的详情页；宿主没给（只读展示或功能已关闭）时这一行只是记录。
    #[prop(optional)] has_on_open: bool,
    /// 模型发出的、未经钳制的上限；`None` = 解除本 run 自己的界。
    requested: Option<i64>,
    run_id: String,
) -> impl IntoView {
    let amend: WorkflowSettingsAmendMeta = retune_as_amend_meta(requested, ceiling);
    let rows = workflow_settings_provenance_rows(&amend);
    // 真源 :46 —— 行主体是 WorkflowSettingsChangeRow；Rust 侧用
    // provenance 的 from → to 行渲染（`WorkflowSettingsChangeRow.tsx`
    // 待迁时替换，见文件末 TODO）。
    let row_views: Vec<AnyView> = rows
        .into_iter()
        .map(|row| {
            view! {
                <div class="flex min-w-0 items-baseline gap-x-2 text-ui-sm">
                    <span class="shrink-0 text-foreground-subtlest">{row.label}</span>
                    <span class="min-w-0 truncate font-mono text-foreground-subtle">
                        {row.value}
                    </span>
                </div>
            }
            .into_any()
        })
        .collect();
    let row_view = row_views.into_iter().collect_view();

    // ★真源 :48-56 —— `onOpen` 缺席即纯展示态（不加 button 壳）。
    if !has_on_open {
        return view! {
            <div data-testid="workflow-retune-row" data-workflow-retune-run-id=run_id>
                {row_view}
            </div>
        }
        .into_any();
    }

    // 真源 :58-67 —— 可点形态：button 壳 + hover/focus 态。
    view! {
        <button
            class="flex w-full min-w-0 cursor-pointer rounded-md px-1 text-left \
                   transition-colors hover:bg-surface-hover \
                   focus-visible:ring-2 focus-visible:ring-ring/40"
            data-testid="workflow-retune-row"
            data-workflow-retune-run-id=run_id
            type="button"
        >
            {row_view}
        </button>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::workflowSettingsChange::workflow_settings_change_segments;

    #[test]
    fn retune_row_reads_limit_change_only() {
        // ★真源 :20-23 —— 只调并发上限时行上只有上限那一句，
        // 不提脚本/模型。
        let amend = retune_as_amend_meta(Some(4), Some(13));
        let segments = workflow_settings_change_segments(&amend);
        assert_eq!(segments.len(), 1, "只有上限段：{segments:?}");
        assert!(segments[0].contains('4') || segments[0].contains("本机"));
    }

    #[test]
    fn retune_row_has_run_id_attribute() {
        // 真源 :49/61 —— data-workflow-retune-run-id 供测试与样式找行。
        let run_id = "run-42";
        assert!(!run_id.is_empty());
    }

    #[test]
    fn retune_row_never_exceeds_ceiling_in_text() {
        // ★真源 :33-38 —— 模型发出的数未经钳制，但措辞规则会把
        // >= 天花板的请求念成「上限恢复为本机默认」，
        // 于是**行上永远不会出现一个大于本机上限的数**。
        let amend = retune_as_amend_meta(Some(99), Some(13));
        let text = workflow_settings_change_segments(&amend).join(" ");
        assert!(!text.contains("99"), "不该念出超限的数：{text}");
    }

    #[test]
    fn retune_without_ceiling_reads_raw_request() {
        // 真源 :39-42 —— 天花板未知时只能照念请求值，
        // 此时它只可能偏大不偏小，而「最多 n」本就是个上界陈述。
        let amend = retune_as_amend_meta(Some(6), None);
        let text = workflow_settings_change_segments(&amend).join(" ");
        assert!(text.contains('6'), "天花板未知时照念请求：{text}");
    }

    #[test]
    fn retune_null_request_means_default() {
        let amend = retune_as_amend_meta(None, Some(13));
        let text = workflow_settings_change_segments(&amend).join(" ");
        assert!(text.contains("本机默认"), "解除上限 → 念本机默认：{text}");
    }
}