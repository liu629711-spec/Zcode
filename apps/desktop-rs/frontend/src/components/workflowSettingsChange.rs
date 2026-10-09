//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/workflowSettingsChange.ts`
//! （95 行）。
//!
//! 设置轮的两处文字。
//!
//! ★真源 :1-5 注释——一次「配置」在两个面上留下记录：
//! 转写里 run 卡上方的一行「已调整设置 · 子代理改用 X · 最多 N 个同时运行」，
//! 与详情页的来龙去脉块「由你调整设置」下的 from → to 两行。
//! **两处读同一块 `amend` 元数据，措辞规则只在这里写一次。**

use crate::ToolCallBlocks::i18n;

/// `WorkflowSettingsAmendMeta`（真源协议类型，Rust 侧按字段建模）。
///
/// `subagent_model` / `max_concurrency` 缺席 = 那项没改。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkflowSettingsAmendMeta {
    pub subagent_model: Option<ModelChange>,
    pub max_concurrency: Option<ConcurrencyChange>,
    /// 本机并发天花板。`None` = 未知（run 已被淘汰出投影/ 老 CLI 没发过）。
    pub ceiling: Option<i64>,
}

/// `subagentModel` 的一端（真源 `{from?, to?}`）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModelChange {
    pub from: Option<String>,
    pub to: Option<String>,
}

/// `maxConcurrency` 的一端。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConcurrencyChange {
    pub from: Option<i64>,
    pub to: Option<i64>,
}

/// 模型的屏幕名：只要名字（档位留给 tooltip）。
///
/// Rust 侧没有 provider 名称解析服务（`WorkflowSubagentModelDeps`），
/// 故canonical 原样作屏幕名——档位与 provider 前缀的解析待
/// `subagent-model-label.ts` 迁移时接入（见文件末 TODO）。
fn model_name(canonical: &str) -> String {
    canonical.to_string()
}

/// `workflowSettingsChangeSegments`（真源 :18-46）。
///
/// 转写行的各段（不含开头的「已调整设置」与末尾时刻）：
/// 只有改过的设置在场，**模型在前、上限在后**。
///
/// ★上限的 `to` 缺席或不低于天花板，都读作「上限恢复为本机默认」——
/// 这正是钳制之后的真相（钳到天花板 = 这条 run 没有自己的界）。
/// 于是行上永远不会出现一个大于本机上限的数。
pub fn workflow_settings_change_segments(amend: &WorkflowSettingsAmendMeta) -> Vec<String> {
    let mut segments: Vec<String> = Vec::new();

    if let Some(model) = &amend.subagent_model {
        segments.push(match &model.to {
            None => i18n::text("chat.toolCall.workflow.settingsChange.modelSession"),
            Some(to) => i18n::format(
                "chat.toolCall.workflow.settingsChange.model",
                &[("model".to_string(), model_name(to))],
            ),
        });
    }

    if let Some(concurrency) = &amend.max_concurrency {
        let to = concurrency.to;
        let at_ceiling =
            to.is_none() || to.is_some_and(|t| amend.ceiling.is_some_and(|c| t >= c));
        segments.push(if at_ceiling {
            i18n::text("chat.toolCall.workflow.settingsChange.limitCeiling")
        } else {
            i18n::format(
                "chat.toolCall.workflow.settingsChange.limit",
                &[("n".to_string(), to.unwrap_or_default().to_string())],
            )
        });
    }

    segments
}

/// `WorkflowSettingsProvenanceRow`（真源 :48-53）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSettingsProvenanceRow {
    pub key: &'static str,
    pub label: String,
    /// 「{from} → {to}」。
    pub value: String,
}

/// `workflowSettingsProvenanceRows`（真源 :58-95）。
///
/// 详情页来龙去脉块的 from → to 行。**缺席的一端写默认**：
/// 模型写「会话模型」，上限写本机上限（知道天花板时带上数字，
/// 如「13（本机上限）→ 4」）。
pub fn workflow_settings_provenance_rows(
    amend: &WorkflowSettingsAmendMeta,
) -> Vec<WorkflowSettingsProvenanceRow> {
    let mut rows: Vec<WorkflowSettingsProvenanceRow> = Vec::new();

    if let Some(model) = &amend.subagent_model {
        let end = |canonical: &Option<String>| match canonical {
            None => i18n::text("chat.workflowLaunch.settings.sessionModel"),
            Some(c) => model_name(c),
        };
        rows.push(WorkflowSettingsProvenanceRow {
            key: "model",
            label: i18n::text("chat.workflowLaunch.settings.model"),
            value: format!("{} → {}", end(&model.from), end(&model.to)),
        });
    }

    if let Some(concurrency) = &amend.max_concurrency {
        let ceiling = amend.ceiling;
        let end = |bound: &Option<i64>| match bound {
            // 真源 :85 —— 有界且低于天花板才念数字。
            Some(b) if ceiling.is_none_or(|c| *b < c) => b.to_string(),
            _ => match ceiling {
                None => i18n::text("chat.workflowLaunch.settings.machineLimit"),
                Some(c) => i18n::format(
                    "chat.workflowLaunch.settings.machineLimitValue",
                    &[("n".to_string(), c.to_string())],
                ),
            },
        };
        rows.push(WorkflowSettingsProvenanceRow {
            key: "limit",
            label: i18n::text("chat.workflowLaunch.settings.limit"),
            value: format!("{} → {}", end(&concurrency.from), end(&concurrency.to)),
        });
    }

    rows
}

/// `retuneAsAmendMeta`（真源 `WorkflowRetuneRow.tsx:31-38`）。
///
/// 入参里那个数 → 设置轮那一块元数据。两者本来就是同一件事的两种记法
/// （GUI 走轮元数据，工具走入参），映射到同一个形状之后措辞只剩一份实现。
///
/// ★真源注释要点：
/// - `requested` 是**模型发出的、未经钳制**的数，而 CLI 会把它钳进 `[1, 天花板]`。
///   所以天花板一起交给措辞规则：`to >= ceiling` 与 `to` 缺席一视同仁，
///   都念「上限恢复为本机默认」——这正是钳制之后的真相。
///   于是**行上永远不会出现一个大于本机上限的数**。
/// - `from` 不填：入参不知道改之前是多少。
/// - `predecessorRunId` 不填：就地生效没有前驱。
pub fn retune_as_amend_meta(
    requested: Option<i64>,
    ceiling: Option<i64>,
) -> WorkflowSettingsAmendMeta {
    WorkflowSettingsAmendMeta {
        max_concurrency: Some(ConcurrencyChange {
            from: None,
            // 真源 `to: requested` —— `requested === null` 时是空对象（键缺席）。
            to: requested,
        }),
        subagent_model: None,
        ceiling,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_amend_yields_no_segments() {
        assert!(workflow_settings_change_segments(&WorkflowSettingsAmendMeta::default()).is_empty());
        assert!(workflow_settings_provenance_rows(&WorkflowSettingsAmendMeta::default()).is_empty());
    }

    #[test]
    fn model_change_segment_uses_model_name() {
        let amend = WorkflowSettingsAmendMeta {
            subagent_model: Some(ModelChange {
                from: None,
                to: Some("openai/gpt-5".into()),
            }),
            ..Default::default()
        };
        let segments = workflow_settings_change_segments(&amend);
        assert_eq!(segments.len(), 1);
        assert!(segments[0].contains("gpt-5"), "含模型名：{}", segments[0]);
    }

    #[test]
    fn model_change_to_absent_means_session_model() {
        // 真源 :26 —— `to === undefined` 念「子代理改回会话模型」。
        let amend = WorkflowSettingsAmendMeta {
            subagent_model: Some(ModelChange {
                from: Some("openai/gpt-5".into()),
                to: None,
            }),
            ..Default::default()
        };
        let segments = workflow_settings_change_segments(&amend);
        assert!(segments[0].contains("会话模型"), "{}", segments[0]);
    }

    #[test]
    fn concurrency_below_ceiling_reads_number() {
        let amend = WorkflowSettingsAmendMeta {
            max_concurrency: Some(ConcurrencyChange {
                from: None,
                to: Some(4),
            }),
            ceiling: Some(13),
            ..Default::default()
        };
        let segments = workflow_settings_change_segments(&amend);
        assert!(segments[0].contains('4'), "念实际数字：{}", segments[0]);
        assert!(
            !segments[0].contains("本机默认"),
            "低于天花板不该念默认"
        );
    }

    #[test]
    fn concurrency_at_or_above_ceiling_reads_machine_default() {
        // ★真源 :32 —— `to >= ceiling` 与 `to` 缺席一视同仁。
        for to in [Some(13), Some(20), None] {
            let amend = WorkflowSettingsAmendMeta {
                max_concurrency: Some(ConcurrencyChange {
                    from: None,
                    to,
                }),
                ceiling: Some(13),
                ..Default::default()
            };
            let segments = workflow_settings_change_segments(&amend);
            assert!(
                segments[0].contains("本机默认"),
                "to={to:?} 应念「上限恢复为本机默认」：{}",
                segments[0]
            );
        }
    }

    #[test]
    fn unknown_ceiling_reads_requested_number() {
        // ★真源 WorkflowRetuneRow 注释——天花板未知时只能照念请求值，
        // 此时它也只可能偏大不偏小，而「最多 n」本就是个上界陈述。
        let amend = WorkflowSettingsAmendMeta {
            max_concurrency: Some(ConcurrencyChange {
                from: None,
                to: Some(20),
            }),
            ceiling: None,
            ..Default::default()
        };
        let segments = workflow_settings_change_segments(&amend);
        assert!(segments[0].contains('2') && segments[0].contains('0'), "{}", segments[0]);
    }

    #[test]
    fn segments_order_model_then_limit() {
        // ★真源 :21 —— 模型在前、上限在后。
        let amend = WorkflowSettingsAmendMeta {
            subagent_model: Some(ModelChange {
                from: None,
                to: Some("gpt-5".into()),
            }),
            max_concurrency: Some(ConcurrencyChange {
                from: None,
                to: Some(4),
            }),
            ceiling: Some(13),
        };
        let segments = workflow_settings_change_segments(&amend);
        assert_eq!(segments.len(), 2);
        assert!(segments[0].contains("gpt-5"), "第一段是模型");
        assert!(segments[1].contains('4'), "第二段是上限");
    }

    #[test]
    fn provenance_rows_show_from_to_arrow() {
        let amend = WorkflowSettingsAmendMeta {
            subagent_model: Some(ModelChange {
                from: Some("a/1".into()),
                to: Some("b/2".into()),
            }),
            max_concurrency: Some(ConcurrencyChange {
                from: Some(13),
                to: Some(4),
            }),
            ceiling: Some(13),
        };
        let rows = workflow_settings_provenance_rows(&amend);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].key, "model");
        assert!(rows[0].value.contains(" → "), "{}", rows[0].value);
        assert_eq!(rows[1].key, "limit");
        assert!(rows[1].value.contains("13"), "from 是 13：{}", rows[1].value);
        assert!(rows[1].value.contains("4"), "to 是 4：{}", rows[1].value);
    }

    #[test]
    fn provenance_absent_end_writes_default() {
        // 真源 :72-75 —— 缺席的一端写默认。
        let amend = WorkflowSettingsAmendMeta {
            max_concurrency: Some(ConcurrencyChange {
                from: None,
                to: Some(4),
            }),
            ceiling: Some(13),
            ..Default::default()
        };
        let rows = workflow_settings_provenance_rows(&amend);
        // from 缺席且 known ceiling → 「13（本机上限） → 4」
        assert!(rows[0].value.contains("13"), "缺席端补天花板：{}", rows[0].value);
        assert!(rows[0].value.contains('4'));
    }

    #[test]
    fn provenance_from_at_ceiling_is_machine_default() {
        // 真源 :85 —— from >= ceiling 也念「本机上限」。
        let amend = WorkflowSettingsAmendMeta {
            max_concurrency: Some(ConcurrencyChange {
                from: Some(13),
                to: Some(4),
            }),
            ceiling: Some(13),
            ..Default::default()
        };
        let rows = workflow_settings_provenance_rows(&amend);
        assert!(
            rows[0].value.contains("本机"),
            "from 等于天花板时念本机上限：{}",
            rows[0].value
        );
    }

    #[test]
    fn retune_meta_maps_requested_to_limit() {
        let amend = retune_as_amend_meta(Some(4), Some(13));
        let segs = workflow_settings_change_segments(&amend);
        assert_eq!(segs.len(), 1, "只调并发上限 → 只有一段");
        assert!(segs[0].contains('4'), "{}", segs[0]);
        // 没有模型段。
        assert!(amend.subagent_model.is_none());
        // from 不填（入参不知道改之前是多少）。
        assert_eq!(amend.max_concurrency.as_ref().unwrap().from, None);
    }

    #[test]
    fn retune_meta_with_null_requested_reads_default() {
        // `requested === null` = 解除本 run 自己的界 → 念「上限恢复为本机默认」。
        let amend = retune_as_amend_meta(None, Some(13));
        let segs = workflow_settings_change_segments(&amend);
        assert!(segs[0].contains("本机默认"), "{}", segs[0]);
    }

    #[test]
    fn retune_meta_never_shows_number_above_ceiling() {
        // ★真源 WorkflowRetuneRow 注释——行上永远不会出现大于本机上限的数。
        let amend = retune_as_amend_meta(Some(99), Some(13));
        let segs = workflow_settings_change_segments(&amend);
        assert!(
            !segs[0].contains("99"),
            "超过天花板的请求被钳制后不该念出来：{}",
            segs[0]
        );
    }

    #[test]
    fn retune_meta_without_ceiling_reads_requested() {
        // 天花板未知时照念请求值（只可能偏大不偏小）。
        let amend = retune_as_amend_meta(Some(6), None);
        let segs = workflow_settings_change_segments(&amend);
        assert!(segs[0].contains('6'), "{}", segs[0]);
    }

    #[test]
    fn i18n_keys_are_migrated() {
        for id in [
            "chat.toolCall.workflow.settingsChange.model",
            "chat.toolCall.workflow.settingsChange.modelSession",
            "chat.toolCall.workflow.settingsChange.limit",
            "chat.toolCall.workflow.settingsChange.limitCeiling",
            "chat.workflowLaunch.settings.sessionModel",
            "chat.workflowLaunch.settings.machineLimit",
            "chat.workflowLaunch.settings.machineLimitValue",
        ] {
            assert_ne!(i18n::text(id), id, "{id} 词条应已迁");
        }
    }
}