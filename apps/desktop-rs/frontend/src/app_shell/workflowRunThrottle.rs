//! 1:1 翻译 `packages/ui/src/app-shell/workflowRunThrottle.ts`（181 行）中被工具卡消费的那一段。
//!
//! 真源文件有两块：① 限流原因的短标签（:15-40，纯表查 + i18n）；② run 详情页的自适应并发
//! 观察面（:42-181，收到时刻 WeakMap 登记 + `workflowRunConcurrencyView` +
//! `workflowRunConcurrencyEventLine`）。
//!
//! ★本文件只迁 ①。②的两个导出函数唯一的消费方是 `app-shell/workflowRunPanel.tsx`
//! （run 详情页），那张面板还没迁——现在搬过来就是无人调用的死代码，随面板那批一起进。
//! `GetWorkflowRun` 情势截面只用到 `throttleReasonLabel`
//! （见 `ToolCallBlocks/renderers/get-workflow-run-situation.tsx:16,164`）。
//!
//! `formatMessage` 在真源是注入参数（:19 的 `FormatMessage` 类型），Rust 侧走本仓库统一的
//! `ToolCallBlocks::i18n` 查表，与其他 renderer 一致。

use crate::ToolCallBlocks::i18n;

/// 真源 :16 的 `I18N_PREFIX`。
pub const I18N_PREFIX: &str = "chat.toolCall.workflow.run.";

/// `REASON_LABEL_KEY`（真源 :25-35）。
///
/// reason 是开放字符串，这里只映射已知的几类，其余归入「瞬态错误」；
/// 完全陌生的值原样显示——不认识不等于不显示（真源 :21-24 注释）。
const REASON_LABEL_KEY: &[(&str, &str)] = &[
    ("rate_limited", "throttle.reason.rateLimited"),
    ("provider_overloaded", "throttle.reason.overloaded"),
    ("offpeak_queued", "throttle.reason.offpeak"),
    ("server_error", "throttle.reason.transient"),
    ("network_error", "throttle.reason.transient"),
    ("timeout", "throttle.reason.transient"),
    ("stream_idle_timeout", "throttle.reason.transient"),
    ("stale_connection", "throttle.reason.transient"),
    ("proxy_error", "throttle.reason.transient"),
];

/// `throttleReasonLabel`（真源 :37-40）。
///
/// 命中表 → 查 `chat.toolCall.workflow.run.<key>`；不命中 → **原样返回 reason**。
pub fn throttle_reason_label(reason: &str) -> String {
    match REASON_LABEL_KEY.iter().find(|(key, _)| *key == reason) {
        Some((_, suffix)) => i18n::text(&format!("{I18N_PREFIX}{suffix}")),
        None => reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_reasons_map_to_their_labels() {
        // 真源 :25-35 的九个键映射到四个 message id；文案表里四条都在。
        assert_eq!(throttle_reason_label("rate_limited"), "限流");
        assert_eq!(throttle_reason_label("provider_overloaded"), "过载");
        assert_eq!(throttle_reason_label("offpeak_queued"), "闲时排队");
        for reason in [
            "server_error",
            "network_error",
            "timeout",
            "stream_idle_timeout",
            "stale_connection",
            "proxy_error",
        ] {
            assert_eq!(
                throttle_reason_label(reason),
                "瞬态错误",
                "{reason} 应归入瞬态错误"
            );
        }
    }

    #[test]
    fn unknown_reason_is_shown_verbatim() {
        // 真源 :39 —— `key === undefined ? reason : ...`，不认识不等于不显示。
        assert_eq!(throttle_reason_label("weird_new_reason"), "weird_new_reason");
        assert_eq!(throttle_reason_label(""), "");
        // ★表查按精确字符串，不做归一：下划线/连字符写法的键不同，缺省即原样。
        assert_eq!(throttle_reason_label("rate-limited"), "rate-limited");
    }

    #[test]
    fn table_covers_every_documented_reason() {
        // 九个键一条不落（真源 :26-34 逐条）。
        assert_eq!(REASON_LABEL_KEY.len(), 9);
        let keys: Vec<&str> = REASON_LABEL_KEY.iter().map(|(k, _)| *k).collect();
        for k in [
            "rate_limited",
            "provider_overloaded",
            "offpeak_queued",
            "server_error",
            "network_error",
            "timeout",
            "stream_idle_timeout",
            "stale_connection",
            "proxy_error",
        ] {
            assert!(keys.contains(&k), "{k} 应在 REASON_LABEL_KEY 里");
        }
    }

    #[test]
    fn prefix_matches_the_source_constant() {
        assert_eq!(I18N_PREFIX, "chat.toolCall.workflow.run.");
    }
}
