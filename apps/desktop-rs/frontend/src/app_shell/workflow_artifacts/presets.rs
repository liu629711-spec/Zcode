//! 1:1 翻译 `packages/ui/src/app-shell/workflow-artifacts/presets/` 的 `PresetLabels`。
//!
//! 喂给四个预置渲染器的三句译文。渲染器自己不查 i18n（它要能被侧板 / tab / 中枢复用），
//! 所以每个表面在自己的 intl 语境里造一份（真源 `buildPresetLabels`，artifactPresentation.tsx :215-229）。
//!
//! 四个预置看板渲染器（chart / table / metrics / board）与 spec 解析层
//! （`parseArtifactPresetSpec` / `applyArtifactItems`）等 run 侧板 / 产物 tab 迁到时再进。

/// `PresetLabels`（真源 parts.ts）：两句静态译文 + 条数措辞。
/// 真源的 `itemsCount` 是闭包字段；Rust 侧用模板串 + 方法表达，插值规则相同（`{count}`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetLabels {
    pub other_column: String,
    pub empty: String,
    items_template: String,
}

impl PresetLabels {
    /// `itemsCount(count)`：预置看板的「大小」是数据量，条数不知道时上游不画细节槽。
    pub fn items_count(&self, count: i64) -> String {
        self.items_template.replace("{count}", &count.to_string())
    }
}

/// `buildPresetLabels`（真源 artifactPresentation.tsx :215-229）。
/// Rust 侧 i18n 是全局查表（`ToolCallBlocks::i18n`），无需传 formatMessage 进来。
pub fn build_preset_labels() -> PresetLabels {
    use crate::ToolCallBlocks::i18n;
    PresetLabels {
        other_column: i18n::text("chat.toolCall.workflow.run.artifacts.preset.otherColumn"),
        empty: i18n::text("chat.toolCall.workflow.run.artifacts.preset.empty"),
        items_template: i18n::text("chat.toolCall.workflow.run.artifacts.preset.items"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_resolve_from_the_zh_cn_table() {
        let labels = build_preset_labels();
        assert!(!labels.other_column.is_empty());
        assert!(!labels.empty.is_empty());
        let items = labels.items_count(3);
        assert!(items.contains('3'), "itemsCount 应插值条数：{items}");
        assert_ne!(labels.empty, labels.other_column);
    }
}
