//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/subagent-model-label.ts`（129 行）。
//!
//! 子代理模型的**词**。
//!
//! ★真源 :6-15 注释——run 上存的是规范串 `providerId/modelId[$reasoningLevel]`：
//! 那是给机器回填用的，不是给人读的。团队套餐的 providerId 是一个 UUID，
//! 原样贴到屏幕上，用户第一眼看到的就是一串十六进制。
//!
//! 所以三个面（确认窗、运行卡、详情侧板）共用这一个纯函数：拼名规则直接复用模型菜单那一条
//! （`formatProviderModelLabel`：内置家族只显示模型名，自定义 provider 才显示「名字/模型」），
//! 思考强度复用思考控件的词表。规范串本身只住在 tooltip 里。
//!
//! 纯函数 + 注入的 `providerName`：与 `timeline_summary.rs` 同一条纪律，本文件不碰 store。
//! 注入点是可选闭包——`None` 等价于「会话清单里查不到」，退回裸 modelId。

/// `WorkflowSubagentModelLabel`（真源 :21-28）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSubagentModelLabel {
    /// 屏幕上的模型名；**永远不含 providerId**。
    pub name: String,
    /// 本地化的思考强度词；规范串没有 `$level` 时缺席。
    pub level: Option<String>,
    /// 规范串原文（trim 过），只进 tooltip。
    pub canonical: String,
}

/// 卡与侧板要的两样东西（真源 :119-128 的返回）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSubagentModelCardLabel {
    pub name: String,
    pub title: String,
}

/// `BUILTIN_MODEL_PROVIDER_IDS`（真源 model-provider-types.ts:7-14）。
///
/// ★真源 model-provider-family.ts:22-23 —— Z.ai / BigModel 的**内置连接名**
/// 属于产品固定入口，拼进模型文案会重复展示「Coding Plan」等连接信息。
pub const BUILTIN_MODEL_PROVIDER_IDS: [&str; 6] = [
    "account:zai-individual-coding-plan",
    "account:zai-team-coding-plan",
    "account:zai-start-plan",
    "account:bigmodel-individual-coding-plan",
    "account:bigmodel-team-coding-plan",
    "account:bigmodel-start-plan",
];

/// `resolveModelProviderFamilyIdByProviderId`（真源 model-provider-family.ts:72-76）。
pub fn resolve_model_provider_family_id_by_provider_id(provider_id: &str) -> Option<&'static str> {
    if provider_id.starts_with("account:zai-") {
        Some("zai")
    } else if provider_id.starts_with("account:bigmodel-") {
        Some("bigmodel")
    } else {
        None
    }
}

/// `getModelProviderFamilySpec(familyId).label`（真源 model-provider-family.ts:26-47, 66-70）。
///
/// ★这张表只有两个内置家族，词表是产品固定入口名，不是 providerId
/// ——组名/标签名一律走这里，屏幕上绝不出现 `account:zai-*` 这类连接 id。
pub fn model_provider_family_label(family_id: &str) -> Option<&'static str> {
    match family_id {
        "zai" => Some("Z.ai"),
        "bigmodel" => Some("BigModel"),
        _ => None,
    }
}

/// `resolveModelProviderFamilyLabelByProviderId`（真源 model-provider-family.ts:106-108）：
/// providerId → 家族名；不是内置家族时回 None（调用方继续往 providerLabel / 会话清单找）。
pub fn resolve_model_provider_family_label_by_provider_id(provider_id: &str) -> Option<&'static str> {
    resolve_model_provider_family_id_by_provider_id(provider_id)
        .and_then(model_provider_family_label)
}

/// `formatProviderModelLabel`（真源 modelTriggerDisplay.ts:41-54）。
///
/// ★真源 :46-50 注释——内置家族只显示模型名（连接名是产品固定入口，
/// 拼进文案会重复展示）；自定义 provider 才显示「名字/模型」。
pub fn format_provider_model_label(
    provider_id: Option<&str>,
    provider_name: Option<&str>,
    model_name: &str,
) -> String {
    if provider_id.and_then(resolve_model_provider_family_id_by_provider_id).is_some() {
        return model_name.to_string();
    }
    match provider_name.map(str::trim).filter(|n| !n.is_empty()) {
        Some(name) => format!("{name}/{model_name}"),
        None => model_name.to_string(),
    }
}

/// `parseModelPickerValue`（真源 model-selection.ts:43-59）的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSelection {
    pub provider_id: String,
    pub model_id: String,
    pub reasoning_level: Option<String>,
}

/// `ZCODE_MODEL_REASONING_SEPARATOR`（真源 model-selection.ts:32）。
pub const ZCODE_MODEL_REASONING_SEPARATOR: char = '$';

/// `parseModelPickerValue`（真源 model-selection.ts:43-59）。
///
/// 解析不出结构时返回 `None`（真源抛错，UI 侧包try/catch 降级——见
/// `describe_workflow_subagent_model`）。
pub fn parse_model_picker_value(value: &str) -> Option<ModelSelection> {
    let normalized = value.trim();
    let provider_separator = normalized.find('/')?;
    if provider_separator == 0 {
        return None;
    }
    let provider_id = normalized[..provider_separator].trim().to_string();
    if provider_id.is_empty() {
        return None;
    }
    let raw_model_id = &normalized[provider_separator + 1..];
    // ★真源 :52 —— reasoningSeparatorIndex <= 0 || >= rawModelId.length - 1
    // 时不当作档位（`$` 在开头或结尾都不成结构）。
    match raw_model_id.find(ZCODE_MODEL_REASONING_SEPARATOR) {
        Some(i) if i > 0 && i < raw_model_id.len() - 1 => {
            let model_id = raw_model_id[..i].trim().to_string();
            let level = raw_model_id[i + 1..].trim().to_string();
            if model_id.is_empty() || level.is_empty() {
                return None;
            }
            Some(ModelSelection {
                provider_id,
                model_id,
                reasoning_level: Some(level),
            })
        }
        _ => {
            let model_id = raw_model_id.trim().to_string();
            if model_id.is_empty() {
                return None;
            }
            Some(ModelSelection {
                provider_id,
                model_id,
                reasoning_level: None,
            })
        }
    }
}

/// `THOUGHT_LEVEL_LABEL_IDS`（真源 thoughtLevelOptions.ts:19-44）。
///
/// ★真源 :46-48 注释——工具条之外也有人要说这个词（工作流的子代理模型），
/// 两处必须查同一张表。表里没有的值原样显示（provider 自定义的档位名）。
const THOUGHT_LEVEL_LABEL_IDS: [(&str, &str); 21] = [
    ("disabled", "chat.toolbar.thoughtLevel.value.off"),
    ("false", "chat.toolbar.thoughtLevel.value.off"),
    ("no", "chat.toolbar.thoughtLevel.value.off"),
    ("none", "chat.toolbar.thoughtLevel.value.off"),
    ("nothink", "chat.toolbar.thoughtLevel.value.off"),
    ("no-think", "chat.toolbar.thoughtLevel.value.off"),
    ("no_think", "chat.toolbar.thoughtLevel.value.off"),
    ("off", "chat.toolbar.thoughtLevel.value.off"),
    ("enable", "chat.toolbar.thoughtLevel.value.on"),
    ("enabled", "chat.toolbar.thoughtLevel.value.on"),
    ("on", "chat.toolbar.thoughtLevel.value.on"),
    ("true", "chat.toolbar.thoughtLevel.value.on"),
    ("low", "chat.toolbar.thoughtLevel.value.low"),
    ("minimal", "chat.toolbar.thoughtLevel.value.minimal"),
    ("medium", "chat.toolbar.thoughtLevel.value.medium"),
    ("high", "chat.toolbar.thoughtLevel.value.high"),
    ("extra-high", "chat.toolbar.thoughtLevel.value.xhigh"),
    ("extra_high", "chat.toolbar.thoughtLevel.value.xhigh"),
    ("xhigh", "chat.toolbar.thoughtLevel.value.xhigh"),
    ("max", "chat.toolbar.thoughtLevel.value.max"),
    ("ultra", "chat.toolbar.thoughtLevel.value.ultra"),
];

/// `thoughtLevelLabelId`（真源 thoughtLevelOptions.ts:50-52）。
pub fn thought_level_label_id(value: &str) -> Option<&'static str> {
    let normalized = value.trim().to_lowercase();
    THOUGHT_LEVEL_LABEL_IDS
        .iter()
        .find(|(k, _)| *k == normalized)
        .map(|(_, id)| *id)
}

/// `fallbackName`（真源 :40-46）：解析不出结构时的兜底取名。
///
/// 砍掉 `providerId/` 前缀与 `$level` 后缀，剩下的就是人能读的那截。
fn fallback_name(canonical: &str) -> String {
    let separator_index = canonical.find('/').filter(|i| *i > 0);
    let rest = match separator_index {
        Some(i) => &canonical[i + 1..],
        None => canonical,
    };
    let level_index = rest.find('$').filter(|i| *i > 0);
    let name = match level_index {
        Some(i) => &rest[..i],
        None => rest,
    };
    if name.is_empty() {
        canonical.to_string()
    } else {
        name.to_string()
    }
}

/// `describeWorkflowSubagentModel`（真源 :52-85）：规范串 → 屏幕上的词。
///
/// ★真源 :48-51 注释——解析失败（串缺 provider 段、或形状不合 schema）**不抛**：
/// UI 不是第二个解析器，拿不准就退回裸 modelId。
pub fn describe_workflow_subagent_model(
    canonical: &str,
    provider_name: Option<&dyn Fn(&str) -> Option<String>>,
) -> WorkflowSubagentModelLabel {
    let trimmed = canonical.trim().to_string();
    let Some(parsed) = parse_model_picker_value(&trimmed) else {
        return WorkflowSubagentModelLabel {
            canonical: trimmed.clone(),
            name: fallback_name(&trimmed),
            level: None,
        };
    };
    // ★真源 :67-71 —— 会话清单里 providerName 查不到时会退回 providerId 本身
    // （见 zcodeSessionSettingsToConfigOptions）；那种「名字」正是我们要挡的东西，
    // 当作没查到。
    let resolved = provider_name.and_then(|f| f(&parsed.provider_id));
    let provider_name = resolved
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty() && n != &parsed.provider_id);
    let name = format_provider_model_label(
        Some(&parsed.provider_id),
        provider_name.as_deref(),
        &parsed.model_id,
    );

    let Some(raw_level) = parsed.reasoning_level else {
        return WorkflowSubagentModelLabel {
            canonical: trimmed,
            name,
            level: None,
        };
    };
    // 档位词与思考控件同一张表；表里没有的值原样显示（provider 自定义的档位名）。
    let level = match thought_level_label_id(&raw_level) {
        None => raw_level,
        Some(id) => crate::ToolCallBlocks::i18n::text(id),
    };
    WorkflowSubagentModelLabel {
        canonical: trimmed,
        level: Some(level),
        name,
    }
}

/// `workflowSubagentModelText`（真源 :88-98）：一句话说清模型与强度。
///
/// 没有档位时就是模型名本身（确认窗与 tooltip 共用）。
pub fn workflow_subagent_model_text(label: &WorkflowSubagentModelLabel) -> String {
    match &label.level {
        None => label.name.clone(),
        Some(level) => crate::ToolCallBlocks::i18n::format(
            "chat.toolCall.workflow.subagentModel.withLevel",
            &[
                ("level".to_string(), level.clone()),
                ("model".to_string(), label.name.clone()),
            ],
        ),
    }
}

/// `workflowSubagentModelTooltip`（真源 :104-113）：三个面共用的 tooltip。
///
/// 一句解释（子代理跑在哪儿、主代理没变）+ 换行 + 规范串。
/// **规范串是给机器回填用的，它只该在这里出现。**
pub fn workflow_subagent_model_tooltip(label: &WorkflowSubagentModelLabel) -> String {
    let explained = crate::ToolCallBlocks::i18n::format(
        "chat.toolCall.workflow.subagentModel.tooltip",
        &[(
            "model".to_string(),
            workflow_subagent_model_text(label),
        )],
    );
    format!("{explained}\n{}", label.canonical)
}

/// `workflowSubagentModelCardLabel`（真源 :119-128）。
///
/// 卡与侧板要的两样东西：屏幕上的名字（只有名字，档位留给 tooltip）与 tooltip。
/// run 没指定过模型时**缺席**——跟随会话模型是常态，没有可说的。
pub fn workflow_subagent_model_card_label(
    canonical: Option<&str>,
    provider_name: Option<&dyn Fn(&str) -> Option<String>>,
) -> Option<WorkflowSubagentModelCardLabel> {
    let canonical = canonical?;
    let label = describe_workflow_subagent_model(canonical, provider_name);
    let title = workflow_subagent_model_tooltip(&label);
    Some(WorkflowSubagentModelCardLabel {
        name: label.name,
        title,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 解析 ──

    #[test]
    fn parses_provider_model_and_level() {
        let s = parse_model_picker_value("anthropic/claude-sonnet-4$high").unwrap();
        assert_eq!(s.provider_id, "anthropic");
        assert_eq!(s.model_id, "claude-sonnet-4");
        assert_eq!(s.reasoning_level.as_deref(), Some("high"));
    }

    #[test]
    fn parses_without_level() {
        let s = parse_model_picker_value("anthropic/claude-sonnet-4").unwrap();
        assert_eq!(s.reasoning_level, None);
    }

    #[test]
    fn rejects_shapes_the_schema_would_throw_on() {
        // ★真源 model-selection.ts:46 —— 缺 provider 段抛错。
        assert!(parse_model_picker_value("claude-sonnet-4").is_none());
        assert!(parse_model_picker_value("/claude").is_none(), "provider 段为空");
        assert!(parse_model_picker_value("anthropic/").is_none(), "model 段为空");
        assert!(parse_model_picker_value("").is_none());
    }

    #[test]
    fn edge_dollars_are_part_of_the_model_name() {
        // ★真源 :52 —— reasoningSeparatorIndex <= 0 || >= length-1 时**都**走
        // 「没有档位」那条分支，于是首尾的 `$` 就是模型名的一部分。真源确实这样，
        // UI 侧不该自己加规则把它当档位。
        let leading = parse_model_picker_value("anthropic/$high").unwrap();
        assert_eq!(leading.model_id, "$high");
        assert_eq!(leading.reasoning_level, None);

        let trailing = parse_model_picker_value("anthropic/claude$").unwrap();
        assert_eq!(trailing.model_id, "claude$");
        assert_eq!(trailing.reasoning_level, None);
    }

    // ── 拼名 ──

    #[test]
    fn builtin_family_shows_model_name_only() {
        // ★真源 modelTriggerDisplay.ts:46-50 —— 内置连接名是产品固定入口，
        // 拼进文案会重复展示「Coding Plan」。
        for provider in BUILTIN_MODEL_PROVIDER_IDS {
            assert_eq!(
                format_provider_model_label(Some(provider), Some("Z.ai Coding Plan"), "glm-4"),
                "glm-4",
                "{provider} 是内置家族"
            );
        }
    }

    #[test]
    fn custom_provider_shows_name_slash_model() {
        assert_eq!(
            format_provider_model_label(Some("acme"), Some("Acme"), "model-x"),
            "Acme/model-x"
        );
    }

    #[test]
    fn missing_or_blank_provider_name_falls_back_to_model_only() {
        assert_eq!(
            format_provider_model_label(Some("acme"), None, "model-x"),
            "model-x"
        );
        assert_eq!(
            format_provider_model_label(Some("acme"), Some("   "), "model-x"),
            "model-x",
            "空白名当没有"
        );
        assert_eq!(
            format_provider_model_label(None, Some("Acme"), "model-x"),
            "Acme/model-x",
            "无 providerId 时按有名字处理"
        );
    }

    // ── providerId 挡住 ──

    #[test]
    fn uuid_provider_id_never_reaches_the_screen() {
        // ★真源 :10-12 —— 团队套餐的 providerId 是一个 UUID，
        // 原样贴到屏幕上用户第一眼看到的就是一串十六进制。
        let label = describe_workflow_subagent_model(
            "9f8e7d6c-5b4a-3210-9876-543210fedcba/team-model",
            Some(&|_: &str| Some("Acme 团队套餐".to_string())),
        );
        assert_eq!(label.name, "Acme 团队套餐/team-model");
        assert!(!label.name.contains("9f8e7d6c"), "★屏幕上不得出现 providerId");
    }

    #[test]
    fn provider_lookup_returning_the_id_counts_as_not_found() {
        // ★真源 :67-71 —— 会话清单查不到时会退回 providerId 本身；
        // 那种「名字」正是我们要挡的东西，当作没查到。
        let id = "acme-provider";
        let label = describe_workflow_subagent_model(
            &format!("{id}/model-x"),
            Some(&move |_: &str| Some(id.to_string())),
        );
        assert_eq!(label.name, "model-x", "退回裸 modelId");
    }

    // ── 兜底 ──

    #[test]
    fn unparsable_string_falls_back_to_human_part() {
        // 真源 :63-65 —— 解析不出来就砍掉前缀与后缀。
        assert_eq!(describe_workflow_subagent_model("no-separator", None).name, "no-separator");
        assert_eq!(describe_workflow_subagent_model("acme/model$high", None).name.is_empty(), false);
    }

    #[test]
    fn fallback_name_strips_provider_prefix_and_level_suffix() {
        assert_eq!(fallback_name("acme/model-x"), "model-x");
        assert_eq!(fallback_name("acme/model-x$high"), "model-x");
        // `$` 在开头不算档位（真源 :43 用 `levelIndex > 0` 判）。
        assert_eq!(fallback_name("acme/$high"), "$high");
        // 全部砍光时退回整串。
        assert_eq!(fallback_name("acme/"), "acme/");
    }

    // ── 档位词 ──

    #[test]
    fn thought_levels_come_from_the_shared_table() {
        // ★真源 :46-48 —— 两处必须查同一张表。
        for value in ["off", "OFF", " none ", "no-think"] {
            assert_eq!(
                thought_level_label_id(value),
                Some("chat.toolbar.thoughtLevel.value.off"),
                "{value} → off"
            );
        }
        for value in ["on", "enabled", "true"] {
            assert_eq!(
                thought_level_label_id(value),
                Some("chat.toolbar.thoughtLevel.value.on")
            );
        }
        assert_eq!(thought_level_label_id("high"), Some("chat.toolbar.thoughtLevel.value.high"));
        assert_eq!(
            thought_level_label_id("extra-high"),
            Some("chat.toolbar.thoughtLevel.value.xhigh")
        );
        assert_eq!(
            thought_level_label_id("xhigh"),
            Some("chat.toolbar.thoughtLevel.value.xhigh")
        );
    }

    #[test]
    fn unknown_level_shows_provider_own_name() {
        // ★真源 :78 —— 表里没有的值原样显示。
        assert_eq!(thought_level_label_id("turbo"), None);
        let label = describe_workflow_subagent_model("acme/model-x$turbo", None);
        assert_eq!(label.level.as_deref(), Some("turbo"));
    }

    // ── tooltip ──

    #[test]
    fn tooltip_ends_with_the_canonical_string() {
        // ★真源 :100-103 —— 规范串是给机器回填用的，它只该在这里出现。
        let label = describe_workflow_subagent_model("acme/model-x", None);
        let tooltip = workflow_subagent_model_tooltip(&label);
        assert!(tooltip.ends_with("acme/model-x"));
        assert!(tooltip.contains('\n'), "解释与规范串之间换行");
    }

    #[test]
    fn text_with_level_uses_the_shared_phrase() {
        let label = describe_workflow_subagent_model("acme/model-x$high", None);
        let text = workflow_subagent_model_text(&label);
        assert!(text.contains("model-x"));
        assert!(text.contains(&label.level.unwrap()), "带档位时那句话里有档位");
    }

    #[test]
    fn text_without_level_is_just_the_model_name() {
        let label = describe_workflow_subagent_model("acme/model-x", None);
        assert_eq!(workflow_subagent_model_text(&label), "model-x");
    }

    // ──卡标签 ──

    #[test]
    fn card_label_absent_without_canonical() {
        // ★真源 :124-126 —— 没指定过模型的 run 缺席：跟随会话模型是常态，
        // 没有可说的。
        assert!(workflow_subagent_model_card_label(None, None).is_none());
    }

    #[test]
    fn card_label_carries_name_and_tooltip() {
        let label = workflow_subagent_model_card_label(Some("acme/model-x$high"), None).unwrap();
        assert_eq!(label.name, "model-x", "只有名字，档位留给 tooltip");
        assert!(label.title.contains("acme/model-x$high"), "规范串在 tooltip 里");
    }
}