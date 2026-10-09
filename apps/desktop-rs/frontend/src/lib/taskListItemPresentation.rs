//! 1:1 翻译 `packages/ui/src/lib/taskListItemPresentation.ts`（53 行）。
//!
//! 当前只迁 workflow 渲染层消费的 `formatTaskRelativeTime`。
//! `deriveTaskLeadingIndicator` 的两个入参类型（`ZCodeTaskMeta`、`TaskListRowActivity`）
//! 属于任务清单/协议面，尚未迁到 Rust——任务清单迁移时补全，不在此处造半截类型。

use crate::ToolCallBlocks::i18n;

/// `formatTaskRelativeTime`（真源 :33-53）：相对时间的四档措辞。
/// 真源用 `Date.now()`；wasm 侧同样取 `js_sys::Date::now()`。
pub fn format_task_relative_time(timestamp: i64) -> String {
    let now = js_sys::Date::now() as i64;
    format_task_relative_time_at(timestamp, now)
}

/// 纯核（测试用）：同一规则，时刻由调用方注入。
fn format_task_relative_time_at(timestamp: i64, now: i64) -> String {
    let diff = now - timestamp;
    let minutes = diff / 60_000;
    if minutes < 1 {
        return i18n::text("taskList.justNow");
    }
    if minutes < 60 {
        return i18n::format(
            "taskList.minutesAgo",
            &[("minutes".to_string(), minutes.to_string())],
        );
    }
    let hours = minutes / 60;
    if hours < 24 {
        return i18n::format(
            "taskList.hoursAgo",
            &[("hours".to_string(), hours.to_string())],
        );
    }
    let days = hours / 24;
    i18n::format("taskList.daysAgo", &[("days".to_string(), days.to_string())])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_buckets_match_truth_source() {
        // 真源 :40-52：justNow / minutesAgo / hoursAgo / daysAgo。
        let now = 1_700_000_000_000;
        let just = format_task_relative_time_at(now - 30_000, now);
        assert!(!just.is_empty());
        // 五分钟前 → minutesAgo 桶，数字来自参数而不是 Date.now。
        let mins = format_task_relative_time_at(now - 5 * 60_000, now);
        assert_ne!(mins, just);
        assert!(mins.contains('5'), "minutesAgo 应插值 minutes：{mins}");
        // 两小时前 → hoursAgo 桶。
        let hours = format_task_relative_time_at(now - 2 * 3_600_000, now);
        assert_ne!(hours, mins);
        assert!(hours.contains('2'), "hoursAgo 应插值 {hours}：{hours}");
        // 两天前 → daysAgo 桶。
        let days = format_task_relative_time_at(now - 2 * 86_400_000, now);
        assert_ne!(days, hours);
        assert!(days.contains('2'), "daysAgo 应插值 {days}：{days}");
    }
}
