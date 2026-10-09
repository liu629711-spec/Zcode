//! 1:1 翻译 `packages/ui/src/lib/workDuration.ts`（47 行）。
//!
//! 「工作了 1 分 42 秒」的时长写法（`chat.history.workedFor`），从 `ConversationTurnGroup`
//! 抽出，好让工作流完成卡的「时间」格与轮头的折叠标签一字不差：同一段时间在两处
//! 必须写成同一个样子。
//!
//! 规则：秒向最近取整、至少 1 秒；天 / 时 / 分 / 秒里只写非零的，**最多两段**（`1h 3m`，
//! 不写秒）；英文单位紧贴数字，中文单位前留一个空格。

use crate::ToolCallBlocks::i18n;

/// 一段拆出的时长（真源 `WorkDurationPart`）：数字与本地化单位词分开排，
/// 完成卡的大数字格要把两者放进不同的 span。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkDurationPart {
    pub value: i64,
    /// 本地化后的单位词（`m` / `分`）。
    pub unit: String,
}

/// 单位词的 message id（真源 `UNIT_IDS`，:17-22）。
const UNIT_DAY: &str = "chat.history.duration.day";
const UNIT_HOUR: &str = "chat.history.duration.hour";
const UNIT_MINUTE: &str = "chat.history.duration.minute";
const UNIT_SECOND: &str = "chat.history.duration.second";

/// `workDurationParts`（真源 :25-42）：拆成 `[{value, unit}]`，给要把数字与单位
/// 分开排的地方（完成卡的大数字）。秒向最近取整、至少 1 秒；最多两段。
pub fn work_duration_parts(duration_ms: i64) -> Vec<WorkDurationPart> {
    // Math.round(ms / 1000)：加 500 再整除，与四舍五入同值；最少 1 秒。
    let total_seconds = ((duration_ms + 500) / 1000).max(1);
    let days = total_seconds / 86_400;
    let hours = (total_seconds % 86_400) / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;
    let mut parts: Vec<WorkDurationPart> = Vec::new();
    if days > 0 {
        parts.push(WorkDurationPart {
            value: days,
            unit: i18n::text(UNIT_DAY),
        });
    }
    if hours > 0 {
        parts.push(WorkDurationPart {
            value: hours,
            unit: i18n::text(UNIT_HOUR),
        });
    }
    if minutes > 0 {
        parts.push(WorkDurationPart {
            value: minutes,
            unit: i18n::text(UNIT_MINUTE),
        });
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(WorkDurationPart {
            value: seconds,
            unit: i18n::text(UNIT_SECOND),
        });
    }
    parts.into_iter().take(2).collect()
}

/// 中文时长单位与数字之间留空格以保持可读；英文缩写单位紧贴数字（真源 :45-47）。
pub fn work_duration_unit_separator(locale: &str) -> &'static str {
    if locale == "zh-CN" {
        " "
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_to_nearest_second_and_never_below_one() {
        // 真源 :29 —— Math.round(ms/1000)，最少 1 秒。
        assert_eq!(work_duration_parts(0)[0].value, 1);
        assert_eq!(work_duration_parts(499)[0].value, 1);
        // 102.4s → 102s = 1 分 42 秒（两段）。
        let parts = work_duration_parts(102_400);
        assert_eq!(parts[0].value, 1);
        assert_eq!(parts[1].value, 42);
    }

    #[test]
    fn minute_only_duration_is_one_segment() {
        // 45s：没有分钟以上的段 → 只有「45 秒」一段。
        let parts = work_duration_parts(45_000);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].value, 45);
        assert_eq!(parts[0].unit, "秒");
    }

    #[test]
    fn hour_and_minute_take_two_segments_and_drop_seconds() {
        // 3740s = 1 时 2 分 20 秒：取两段后秒被裁掉（真源 :41 take 2）。
        let parts = work_duration_parts(3_740_000);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].value, 1);
        assert_eq!(parts[1].value, 2);
    }

    #[test]
    fn day_overflows_into_two_segments() {
        // 1 天 2 小时 3 分 → 「1 天 2 时」（真源最多两段）。
        let parts = work_duration_parts(86_400_000 + 2 * 3_600_000 + 3 * 60_000);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].unit, "天");
        assert_eq!(parts[1].value, 2);
    }

    #[test]
    fn zero_parts_fall_back_to_seconds() {
        // 全零不可能（至少 1 秒），但保底分支与真源同在。
        let parts = work_duration_parts(1_000);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].value, 1);
    }

    #[test]
    fn separator_is_space_only_for_chinese() {
        // 真源 :45-47 —— 中文单位前留空格，英文紧贴。
        assert_eq!(work_duration_unit_separator("zh-CN"), " ");
        assert_eq!(work_duration_unit_separator("en-US"), "");
    }
}
