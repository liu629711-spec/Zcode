//! 1:1 翻译 `packages/ui/src/lib/workflowObservationFormat.ts`（79 行）。
//!
//! 观察类工作流工具卡的数值/时间格式化（GetWorkflowRun 与 ListWorkflowRuns 共用）。
//! 真源 :1-4：纯函数、无 i18n 依赖——数字与时间走 Intl 默认 locale，文案才走 message 表。
//!
//! ★时刻格式的偏离（真源 :19-30 用 `Intl.DateTimeFormat(undefined, {month/day/hour/minute
//! 全 2-digit})`）：js-sys 不暴露 `Intl`，本仓库既有做法是手工格式化 + 注明
//! （见 `components/WorkflowSettingsChangeRow.rs` 的 HH:MM）。这里钉死 zh-CN 下的输出形状
//! `MM/DD HH:MM`（斜杠分隔、24 小时制、无年份）。真源在 en-US 下会多出 `, ` 与 am/pm，
//! 那是 runtime locale 的差异，不是逻辑差异；本应用的文案表本就是 zh-CN 单语（见
//! `ToolCallBlocks/i18n.rs`），所以钉死 zh-CN 形状反而更贴近实际渲染结果。

/// `trimTrailingZero`（真源 :15-17）：去掉 `.0` 尾巴。
fn trim_trailing_zero(value: &str) -> &str {
    if value.ends_with(".0") {
        &value[..value.len() - 2]
    } else {
        value
    }
}

/// `Number.prototype.toFixed(1)` 的等价实现。
///
/// ★Rust 的 `{:.1}` 是「正确舍入 + 平局取偶」，JS 的 `toFixed` 是「平局远离 0」
/// （V8 实测 `(1.25).toFixed(1) === "1.3"`、`(-1.25).toFixed(1) === "-1.3"`）。
/// 真实 token 数是整数，`n/1000` 落在 `.25` / `.75` 上是常事（1250 → 1.25），
/// 直接写 `{:.1}` 会让卡上的读数比模型面少一档。
///
/// 两者只在**精确平局**上不同。平局 ⇔ 该值恰好是 奇数/4，而 `×4` 在二进制浮点下是
/// 精确运算（乘 2 的幂不产生舍入），所以 `value * 4` 为奇整数即精确平局。
/// 不能用 `value * 10`/`value * 20` 来判：那一步本身就带舍入，会把 0.15
/// （真值 0.1499999999999999944）误判成平局。
fn to_fixed_1(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let quarter = value * 4.0;
    let is_exact_tie = quarter.fract() == 0.0
        && quarter.abs() <= 9_000_000_000_000_000.0
        && (quarter as i64) % 2 != 0;
    if !is_exact_tie {
        return format!("{:.1}", value);
    }
    // 平局：按 JS 取远离 0 的那一档。value*20 = 5*(value*4) 仍是精确奇整数，
    // (|奇数| + 1) / 2 即向上（按量级）取到的十分位。
    let tenths = ((value * 20.0).abs() + 1.0) / 2.0;
    let signed = if value < 0.0 { -tenths } else { tenths };
    format!("{:.1}", signed / 10.0)
}

/// `Math.round` 的等价实现（JS 平局朝 +∞，Rust `.round()` 平局远离 0）。
fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// `String(number)` 的等价实现：`Infinity` / `-Infinity` 是 JS 拼写，
/// Rust 的 `to_string()` 给 `inf`。只在非有限值与取整后的整数上调用。
fn js_string_number(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "Infinity".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else {
        // -0 与 0 都要写成 "0"（JS `String(Math.round(-0.5)) === "0"`）。
        format!("{}", value as i64)
    }
}

/// `formatWorkflowTokenCount`（真源 :6-13）。
///
/// 三档：千以内取整；千到百万一位小数加 `k`；百万以上一位小数加 `M`。
/// 非有限值原样转字符串（真源 :7 的 `String(value)`）。
pub fn format_workflow_token_count(value: f64) -> String {
    if !value.is_finite() {
        return js_string_number(value);
    }
    let abs = value.abs();
    if abs < 1_000.0 {
        return js_string_number(js_round(value));
    }
    if abs < 1_000_000.0 {
        return format!("{}k", trim_trailing_zero(&to_fixed_1(value / 1_000.0)));
    }
    format!("{}M", trim_trailing_zero(&to_fixed_1(value / 1_000_000.0)))
}

const SECOND_MS: f64 = 1_000.0;
const MINUTE_MS: f64 = 60_000.0;
const HOUR_MS: f64 = 3_600_000.0;
const DAY_MS: f64 = 86_400_000.0;

/// `padTwo`（真源 :39-41）。
fn pad_two(value: i64) -> String {
    format!("{:02}", value)
}

/// `formatWorkflowDuration`（真源 :50-63）。
///
/// `40s` / `5m 10s` / `2h 15m` / `3d 2h`：情势截面里每一段时长与年龄的写法。
///
/// 真源 :43-49 的刻意约束——与 GetWorkflowRun 模型面的 `formatWorkflowRunDuration`
/// （`apps/zcode-cli/packages/core/src/tool/handlers/workflow-run-introspection.ts`）
/// **逐字同款**：同一份快照的同一个数，模型读到的和卡上画的不能长得不一样。
///
/// 非有限值与非正数都归零（真源 :51 `Number.isFinite(ms) && ms > 0 ? ms : 0`）→ `"0s"`。
pub fn format_workflow_duration(ms: f64) -> String {
    let total = if ms.is_finite() && ms > 0.0 { ms } else { 0.0 };
    if total < MINUTE_MS {
        format!("{}s", (total / SECOND_MS).floor())
    } else if total < HOUR_MS {
        let minutes = (total / MINUTE_MS).floor();
        let seconds = ((total % MINUTE_MS) / SECOND_MS).floor();
        format!("{}m {}s", minutes, pad_two(seconds as i64))
    } else if total < DAY_MS {
        let hours = (total / HOUR_MS).floor();
        let minutes = ((total % HOUR_MS) / MINUTE_MS).floor();
        format!("{}h {}m", hours, pad_two(minutes as i64))
    } else {
        let days = (total / DAY_MS).floor();
        let hours = ((total % DAY_MS) / HOUR_MS).floor();
        format!("{}d {}h", days, hours)
    }
}

/// `formatWorkflowTimestamp` 的纯核（真源 :19-24 的四项 2-digit 形状）：
/// 月/日/时/分由调用方给出（本地时区的分解只能靠 `js_sys::Date`，见
/// `format_workflow_timestamp`；拆分手法同 `taskListItemPresentation.rs`）。
fn timestamp_text_from_parts(month: i64, day: i64, hours: i64, minutes: i64) -> String {
    format!(
        "{}/{} {}:{}",
        pad_two(month),
        pad_two(day),
        pad_two(hours),
        pad_two(minutes),
    )
}

/// `formatWorkflowTimestamp`（真源 :27-30）：列表行的短时间戳，密排行里只保留月日 + 时分。
///
/// 非有限值原样转字符串（真源 :28）。见文件头的 locale 偏离说明。
pub fn format_workflow_timestamp(epoch_ms: f64) -> String {
    if !epoch_ms.is_finite() {
        return js_string_number(epoch_ms);
    }
    let date = js_sys::Date::new(&epoch_ms.into());
    // get_month 是 0 基，真源的 2-digit 形态要补 1。
    timestamp_text_from_parts(
        date.get_month() as i64 + 1,
        date.get_date() as i64,
        date.get_hours() as i64,
        date.get_minutes() as i64,
    )
}

/// `formatWorkflowAge`（真源 :72-79）：「多久以前」的裸时长（文案里的「前」由 message 表拼）。
///
/// 基准是快照时刻 `generatedAt`，**不是** `Date.now()`（真源 :66-71）：一条三天前的 transcript
/// 重新打开时，卡上的年龄仍该是当时那个年龄，否则同一张卡每次重渲染都在漂。
/// 没有基准或没有那个时刻就回 `None`——读侧据此整段省略这个年龄，
/// 绝不用 0 或「未知」顶替一件不知道的事。
pub fn format_workflow_age(generated_at: Option<f64>, at: Option<f64>) -> Option<String> {
    let generated_at = generated_at.filter(|v| v.is_finite())?;
    let at = at.filter(|v| v.is_finite())?;
    Some(format_workflow_duration(generated_at - at))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_count_three_tiers() {
        // 真源 :6-13 逐档，值按真源手算。
        assert_eq!(format_workflow_token_count(0.0), "0");
        assert_eq!(format_workflow_token_count(999.0), "999");
        assert_eq!(format_workflow_token_count(1_000.0), "1k");
        assert_eq!(format_workflow_token_count(1_500.0), "1.5k");
        assert_eq!(format_workflow_token_count(12_345.0), "12.3k");
        assert_eq!(format_workflow_token_count(999_999.0), "1000k");
        assert_eq!(format_workflow_token_count(1_000_000.0), "1M");
        assert_eq!(format_workflow_token_count(2_500_000.0), "2.5M");
    }

    #[test]
    fn token_count_ties_round_away_from_zero_like_to_fixed() {
        // ★1250 / 1000 = 1.25 是**精确**平局（奇数/4）：`(1.25).toFixed(1) === "1.3"`，
        // 直接写 {:.1} 会得到 "1.2"（round-half-to-even），卡上少画一档。
        assert_eq!(format_workflow_token_count(1_250.0), "1.3k");
        assert_eq!(format_workflow_token_count(2_250.0), "2.3k");
        assert_eq!(format_workflow_token_count(1_250_000.0), "1.3M");
        // 负数同样远离 0（V8 实测 (-1.25).toFixed(1) === "-1.3"）。
        assert_eq!(format_workflow_token_count(-1_250.0), "-1.3k");
        // ★12350 / 1000 的 double 真值是 12.3499999999999996447，**不是**精确平局，
        // 正确舍入即 12.3——用 ×10+0.5 的土办法会误判成 12.4。
        assert_eq!(format_workflow_token_count(12_350.0), "12.3k");
        assert_eq!(format_workflow_token_count(150.0), "150", "千以内走取整档，不做十分位舍入");
    }

    #[test]
    fn token_count_rounds_below_a_thousand() {
        // 真源 :8 `String(Math.round(value))` —— 取整发生在分档**之后**，
        // 所以 999.6 仍是 "1000" 而不是 "1k"。
        assert_eq!(format_workflow_token_count(999.6), "1000");
        assert_eq!(format_workflow_token_count(12.4), "12");
        assert_eq!(format_workflow_token_count(12.5), "13");
        assert_eq!(format_workflow_token_count(-0.5), "0", "Math.round(-0.5) 是 -0，显示为 0");
    }

    #[test]
    fn token_count_passes_non_finite_through() {
        // 真源 :7 `String(value)` —— NaN / Infinity 不崩，原样写。
        assert_eq!(format_workflow_token_count(f64::NAN), "NaN");
        assert_eq!(format_workflow_token_count(f64::INFINITY), "Infinity");
        assert_eq!(format_workflow_token_count(f64::NEG_INFINITY), "-Infinity");
    }

    #[test]
    fn trim_trailing_zero_only_strips_dot_zero() {
        // 真源 :15-17 —— 只吃结尾的 ".0"，".05" 与 "10.0" 之外的都不动。
        assert_eq!(trim_trailing_zero("1.0"), "1");
        assert_eq!(trim_trailing_zero("1.5"), "1.5");
        assert_eq!(trim_trailing_zero("10.0"), "10");
        assert_eq!(trim_trailing_zero("100"), "100");
    }

    #[test]
    fn duration_matches_source_examples() {
        // 真源 :44 注释给的四个样子 + 各档边界。
        assert_eq!(format_workflow_duration(40_000.0), "40s");
        assert_eq!(format_workflow_duration(310_000.0), "5m 10s");
        assert_eq!(format_workflow_duration(8_100_000.0), "2h 15m");
        assert_eq!(format_workflow_duration(267_120_000.0), "3d 2h");
    }

    #[test]
    fn duration_boundaries_and_padding() {
        // 档位在整点上切换；不足两位补零（真源 :39-41 padTwo）。
        assert_eq!(format_workflow_duration(0.0), "0s");
        assert_eq!(format_workflow_duration(999.0), "0s");
        assert_eq!(format_workflow_duration(59_999.0), "59s");
        assert_eq!(format_workflow_duration(60_000.0), "1m 00s");
        assert_eq!(format_workflow_duration(3_599_999.0), "59m 59s");
        assert_eq!(format_workflow_duration(3_600_000.0), "1h 00m");
        assert_eq!(format_workflow_duration(86_399_999.0), "23h 59m");
        assert_eq!(format_workflow_duration(86_400_000.0), "1d 0h");
        // 「d 档」的小时不补零（真源 :62 直接 `${...}h`，没走 padTwo）。
        assert_eq!(format_workflow_duration(90_000_000.0), "1d 1h");
    }

    #[test]
    fn duration_clamps_non_positive_and_non_finite() {
        // 真源 :51 —— 不是有限正数就当 0，绝不给负时长。
        assert_eq!(format_workflow_duration(-5_000.0), "0s");
        assert_eq!(format_workflow_duration(f64::NAN), "0s");
        assert_eq!(format_workflow_duration(f64::NEG_INFINITY), "0s");
    }

    #[test]
    fn age_requires_both_endpoints() {
        // 真源 :76-77 —— 缺任一或非法都回 undefined，读侧整段省略这个年龄。
        assert_eq!(format_workflow_age(None, Some(1_000.0)), None);
        assert_eq!(format_workflow_age(Some(1_000.0), None), None);
        assert_eq!(format_workflow_age(None, None), None);
        assert_eq!(format_workflow_age(Some(f64::NAN), Some(1.0)), None);
        assert_eq!(format_workflow_age(Some(1.0), Some(f64::NAN)), None);
        // 都有：差值走同一套时长格式。
        assert_eq!(
            format_workflow_age(Some(310_000.0), Some(0.0)),
            Some("5m 10s".to_string())
        );
        // ★未来时刻（差值为负）不写成负时长，也不省略——真源照样给出 "0s"。
        assert_eq!(
            format_workflow_age(Some(0.0), Some(5_000.0)),
            Some("0s".to_string())
        );
    }

    #[test]
    fn timestamp_shape_is_two_digit_month_day_hour_minute() {
        // 真源 :19-24 的四项全 2-digit，zh-CN 下 `Intl.DateTimeFormat` 实测输出
        // "10/09 14:05"（斜杠分隔、24 小时制、无年份）。纯核只吃分解后的数字，
        // 不经 js_sys（原生 cargo test 下调用 wasm 导入会直接崩）。
        assert_eq!(timestamp_text_from_parts(10, 9, 14, 5), "10/09 14:05");
        assert_eq!(timestamp_text_from_parts(1, 1, 0, 0), "01/01 00:00");
        assert_eq!(timestamp_text_from_parts(12, 31, 23, 59), "12/31 23:59");
    }

    #[test]
    fn timestamp_passes_non_finite_through_without_js() {
        // 真源 :28 —— 非有限值走 String(value)，这条路不碰 Date，可在原生下测。
        assert_eq!(format_workflow_timestamp(f64::NAN), "NaN");
        assert_eq!(format_workflow_timestamp(f64::INFINITY), "Infinity");
        assert_eq!(format_workflow_timestamp(f64::NEG_INFINITY), "-Infinity");
    }
}
