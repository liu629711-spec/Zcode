//! 1:1 翻译 `packages/ui/src/components/workflow-graph/phase-name.ts`（56 行）。
//!
//! 阶段显示名的唯一策略点，与 `lane_name.rs` 逐条同构（同一件事只有一套规则）：
//! **作者原词 > 本地化兜底**。

use crate::ToolCallBlocks::i18n;

use super::types::{IMPLICIT_PHASE_ID, UNPHASED_PHASE_ID};

/// 做一个阶段显示名所需的全部输入——不含 id 之外的身份信息。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhaseNaming {
    pub id: String,
    /// 脚本里 `phase("preflight")` 给出的名字；`unphased` 没有。
    pub name: Option<String>,
}

/// `phaseDisplayName`（真源 :29-43）。
///
/// ★真源 :10-18 注释——兜底必须发生在**渲染时**：投影是被 memo 住的纯函数、
/// 与语言无关，把文案烘进去，用户中途切语言那串字就过期了。
/// `unphased` 是唯一拿不到 `name` 的阶段——它不是作者写下的词，
/// 而是「首个标记之前的那些 step」，所以它的名字永远本地化。
pub fn phase_display_name(phase: &PhaseNaming) -> String {
    if let Some(name) = &phase.name {
        return name.clone();
    }
    if phase.id == UNPHASED_PHASE_ID {
        return i18n::text("chat.toolCall.workflow.graph.phase.unphased");
    }
    // 无标记脚本的隐式唯一模块：整个脚本就是一个阶段。
    if phase.id == IMPLICIT_PHASE_ID {
        return i18n::text("chat.toolCall.workflow.graph.phase.workflow");
    }
    // ★真源 :40-42 —— 到不了这里（`unphased` 之外的阶段都带作者原词）。
    // 真出现时报 **id**（身份）而不是「未分组」——把一个有名字的阶段说成
    // 兜底阶段是撒谎，露出 id 至少是可排查的。
    phase.id.clone()
}

/// `DISPLAY_PHASE_NAME_BOUND`（真源 :49）。
///
/// display 的阶段名经 `boundGraphText` 截到这么多字符；运行时的名字
/// （进入记录、实例的出生戳）则带完整名字（reducer 自己截到同一上界）。
pub const DISPLAY_PHASE_NAME_BOUND: usize = 128;

/// `phaseNameMatches`（真源 :59-64）。
///
/// ★真源 :52-58 注释——display 阶段名 ↔ 运行时阶段名的**唯一**关联规则
/// 是精确匹配，display 名**恰好顶到上界**时才按前缀兜底——前缀只在截断
/// 真的发生过时才开，否则「计划」会误认「计划修复」。时间线的进入记录、
/// `currentPhase` 与实例绑定三处共用它。
///
/// 任一侧缺席 → false：关联需要两个名字，「无名」不是一个可匹配的名字。
pub fn phase_name_matches(
    display_name: Option<&str>,
    runtime_name: Option<&str>,
) -> bool {
    let (Some(display), Some(runtime)) = (display_name, runtime_name) else {
        return false;
    };
    if display == runtime {
        return true;
    }
    // ★只在恰好顶到上界时开前缀兜底。
    display.chars().count() == DISPLAY_PHASE_NAME_BOUND && runtime.starts_with(display)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn author_name_wins() {
        // 真源 :30 —— 作者原词原样显示：本地化它就是改作者的话。
        let phase = PhaseNaming {
            id: "preflight".into(),
            name: Some("预备".into()),
        };
        assert_eq!(phase_display_name(&phase), "预备");
    }

    #[test]
    fn unphased_is_always_localized() {
        // ★真源 :16-18 —— unphased 不是作者写下的词，永远本地化。
        let phase = PhaseNaming {
            id: UNPHASED_PHASE_ID.into(),
            name: None,
        };
        let shown = phase_display_name(&phase);
        assert_ne!(shown, UNPHASED_PHASE_ID, "应是本地化文案");
        assert!(!shown.is_empty());
    }

    #[test]
    fn implicit_workflow_phase_is_localized() {
        let phase = PhaseNaming {
            id: IMPLICIT_PHASE_ID.into(),
            name: None,
        };
        let shown = phase_display_name(&phase);
        assert_ne!(shown, IMPLICIT_PHASE_ID, "应是本地化文案");
    }

    #[test]
    fn unknown_nameless_phase_shows_its_id() {
        // ★真源 :40-42 —— 把一个有名字的阶段说成兜底阶段是撒谎，
        // 露出 id 至少是可排查的。
        let phase = PhaseNaming {
            id: "mystery".into(),
            name: None,
        };
        assert_eq!(phase_display_name(&phase), "mystery");
    }

    #[test]
    fn unphased_and_workflow_get_different_labels() {
        // 两个兜底阶段的文案不该相同（一个是「未分组前」，一个是「整个工作流」）。
        let a = phase_display_name(&PhaseNaming {
            id: UNPHASED_PHASE_ID.into(),
            name: None,
        });
        let b = phase_display_name(&PhaseNaming {
            id: IMPLICIT_PHASE_ID.into(),
            name: None,
        });
        assert_ne!(a, b, "两个兜底阶段的文案必须不同");
    }

    // ── 名字匹配 ──

    #[test]
    fn exact_match_succeeds() {
        assert!(phase_name_matches(Some("计划"), Some("计划")));
    }

    #[test]
    fn different_names_do_not_match() {
        // ★真源 :56 —— 前缀只在「恰好顶到上界」时开，
        // 否则「计划」会误认「计划修复」。
        assert!(!phase_name_matches(Some("计划"), Some("计划修复")));
    }

    #[test]
    fn truncated_display_name_matches_by_prefix() {
        // ★真源 :55-56 —— 恰好顶到上界时按前缀兜底。
        let long: String = "计".repeat(DISPLAY_PHASE_NAME_BOUND);
        let runtime = format!("{long}补充");
        assert_eq!(long.chars().count(), DISPLAY_PHASE_NAME_BOUND);
        assert!(phase_name_matches(Some(&long), Some(&runtime)));
    }

    #[test]
    fn short_prefix_never_matches() {
        // 未顶到上界的 display 名不走前缀兜底。
        assert!(!phase_name_matches(Some("计划"), Some("计划修复")));
    }

    #[test]
    fn absent_side_is_never_matchable() {
        // ★真源 :58 —— 关联需要两个名字，「无名」不是一个可匹配的名字。
        assert!(!phase_name_matches(None, Some("计划")));
        assert!(!phase_name_matches(Some("计划"), None));
        assert!(!phase_name_matches(None, None));
    }

    #[test]
    fn display_name_bound_is_128() {
        assert_eq!(DISPLAY_PHASE_NAME_BOUND, 128);
    }
}