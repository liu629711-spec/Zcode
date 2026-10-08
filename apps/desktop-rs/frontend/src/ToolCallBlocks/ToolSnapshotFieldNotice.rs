//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/ToolSnapshotFieldNotice.tsx`（80 行）。
//!
//! 工具大字段被快照预算裁剪后的提示条：描述 + 「加载完整工具数据」按钮。
//! 各 renderer 在 `ToolLayout` 之后渲染（真源 Fragment 第二子元素）。
//!
//! **恒不渲染语义**：真源 `refs.length === 0 || !onLoadFullToolCallFields`
//! 时 `return null`。`snapshotRefs` 的生产者在 legacy 服务层
//! （`zcodeTaskServiceAdapter.ts:852` 合并快照工具 + 实时工具），
//! `v4/toolCallRowAdapter.ts` **不产**该字段——因此当前 v4 投影下
//! 该组件恒不渲染（真源同）。组件随迁，通道就位后自动亮起。
//!
//! **裁剪注明**：真源 `onLoadFullToolCallFields` 返回
//! `Promise<boolean | void> | boolean | void`（resolve(false) 或 reject →
//! 失败态，「加载失败，重试」）。Rust 通道以同步 `Callback<String, bool>`
//! 承接（false = 失败）；后端 IPC 未迁（rg `snapshot_refs` 无生产调用），
//! 真实异步结算待通道迁入后升级为 spawn_local + 信号回写。

use leptos::prelude::*;
use serde_json::Value;

/// `ZCodeTaskSnapshotToolFieldRef`（真源 shared `zcode-task-types-core.ts:1135-1141`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotFieldRef {
    /// `"input" | "output" | "raw"`（真源 `ZCodeTaskSnapshotToolField`）。
    pub field: String,
    pub ref_id: String,
    pub hash: String,
    pub full_bytes: u64,
    pub preview_bytes: u64,
}

/// 从 v4 行 JSON 防御性解析 `snapshotRefs`（前向兼容点）。
///
/// 真源 v4 adapter 不传播该字段（见模块注释）——当前恒空，与真源行为一致；
/// 若将来 v4 行携带 `snapshotRefs`（或宿主注入），此处即为唯一入口。
/// 任一项五字段类型不匹配则跳过该项（严格解析，格式坏不渲染）。
pub fn read_snapshot_refs(value: &Value) -> Vec<SnapshotFieldRef> {
    let Some(items) = value.get("snapshotRefs").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let field = item.get("field").and_then(|v| v.as_str())?;
            let ref_id = item.get("refId").and_then(|v| v.as_str())?;
            let hash = item.get("hash").and_then(|v| v.as_str())?;
            let full_bytes = item.get("fullBytes").and_then(|v| v.as_u64())?;
            let preview_bytes = item.get("previewBytes").and_then(|v| v.as_u64())?;
            Some(SnapshotFieldRef {
                field: field.to_string(),
                ref_id: ref_id.to_string(),
                hash: hash.to_string(),
                full_bytes,
                preview_bytes,
            })
        })
        .collect()
}

/// `Intl.NumberFormat` 近似：zh-CN 逗号分组 + 至多 1 位小数 + 去尾随零。
///
/// 真源 `new Intl.NumberFormat(locale, { maximumFractionDigits: 1 })`；
/// Rust 侧 locale 固定 zh-CN（分组逗号）。**先 `round()` 再格式化**：
/// Rust 的 `{:.N}` 用 round-half-even，而 Intl 用 half-away-from-zero
/// （`f64::round` 恰为 half-away-from-zero），1.25 这类 tie 才能对齐。
fn format_intl_number(value: f64, max_frac_digits: u32) -> String {
    let factor = 10f64.powi(max_frac_digits as i32);
    let rounded = (value * factor).round() / factor;
    let text = format!("{:.*}", max_frac_digits as usize, rounded);
    let (int_part, frac_part) = match text.split_once('.') {
        Some((i, f)) => (i.to_string(), f.trim_end_matches('0').to_string()),
        None => (text, String::new()),
    };
    let int_part = group_digits(&int_part);
    if frac_part.is_empty() {
        int_part
    } else {
        format!("{int_part}.{frac_part}")
    }
}

/// 千分位分组（`Intl.NumberFormat` 的 useGrouping 行为）。
fn group_digits(digits: &str) -> String {
    let (sign, body) = match digits.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", digits),
    };
    let mut out = String::with_capacity(body.len() + body.len() / 3);
    for (i, c) in body.chars().enumerate() {
        if i > 0 && (body.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("{sign}{out}")
}

/// `formatBytes`（真源 :70-80）：MB → KB → B 三档（无 GB 档）。
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let value = bytes as f64;
    if value >= MB {
        return format!("{} MB", format_intl_number(value / MB, 1));
    }
    if value >= KB {
        return format!("{} KB", format_intl_number(value / KB, 1));
    }
    format!("{} B", format_intl_number(value, 0))
}

/// 描述文案（真源 :21-31 的 `chat.toolCall.snapshot.notice` 模板）。
pub fn build_snapshot_notice_description(refs: &[SnapshotFieldRef]) -> String {
    let preview_bytes: u64 = refs.iter().map(|r| r.preview_bytes).sum();
    let full_bytes: u64 = refs.iter().map(|r| r.full_bytes).sum();
    format!(
        "该工具有 {} 个字段被裁剪，当前显示预览 {} / {}",
        refs.len(),
        format_bytes(preview_bytes),
        format_bytes(full_bytes)
    )
}

/// Button 类名（真源 `components/ui/button.tsx` base + `secondary` + `sm`
/// + `className="h-7 self-start px-2 text-ui-base sm:self-auto"` 的
/// tailwind-merge 结果：h-6→h-7、text-ui-base/relaxed→text-ui-base、
/// `svg size-4`→`size-3`，冲突项取最后）。
const SNAPSHOT_LOAD_BUTTON_CLASS: &str = "group/button inline-flex shrink-0 items-center justify-center rounded-lg border border-transparent bg-clip-padding text-ui-base font-medium whitespace-nowrap transition-colors outline-none select-none disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-2 aria-invalid:ring-destructive/20 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-3 bg-secondary text-foreground hover:bg-secondary/80 aria-expanded:bg-secondary aria-expanded:text-foreground h-7 gap-1 px-2 self-start sm:self-auto";

/// `ToolSnapshotFieldNotice` 的 props。
#[derive(Debug, Clone)]
pub struct ToolSnapshotFieldNoticeProps {
    /// 快照字段引用（真源 `refs`）。
    pub refs: Vec<SnapshotFieldRef>,
    /// 工具 ID（点击时透传给回调；真源在 renderer 侧已包好 toolId 的无参闭包）。
    pub tool_id: String,
    /// 「加载完整工具数据」回调；`None` 时整条不渲染（真源 `!onLoadFullToolCallFields`）。
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
}

/// `ToolSnapshotFieldNotice`（真源 :8-67）。
///
/// 组件名带 `Component` 后缀：`#[component]` 宏会为 `ToolSnapshotFieldNotice`
/// 自动生成同名 Props 类型，与手写的 `ToolSnapshotFieldNoticeProps` 冲突
/// （`ToolLayoutComponent` + `ToolLayoutProps` 同款规避）。
#[component]
pub fn ToolSnapshotFieldNoticeComponent(props: ToolSnapshotFieldNoticeProps) -> AnyView {
    // 真源 :17-19 —— refs 空或回调缺省时 return null。
    let Some(callback) = props.on_load_full_tool_call_fields else {
        return ().into_any();
    };
    if props.refs.is_empty() {
        return ().into_any();
    }

    let description = build_snapshot_notice_description(&props.refs);
    let loading = RwSignal::new(false);
    let failed = RwSignal::new(false);
    let tool_id = props.tool_id.clone();

    let button_label = move || {
        if loading.get() {
            "正在加载..."
        } else if failed.get() {
            "加载失败，重试"
        } else {
            "加载完整工具数据"
        }
    };

    view! {
        <div class="flex flex-col gap-2 rounded-xl border border-border/70 bg-muted/40 px-3 py-2 text-ui-base text-foreground-subtle sm:flex-row sm:items-center sm:justify-between">
            <span>{description}</span>
            <button
                type="button"
                class=SNAPSHOT_LOAD_BUTTON_CLASS
                disabled=move || loading.get()
                on:click={
                    let callback = callback.clone();
                    let tool_id = tool_id.clone();
                    move |_| {
                        // 真源 :50-63 —— setLoading(true) → 调用 → measure
                        // 结果（resolve(false)/reject → failed）→ finally 清
                        // loading。同步通道下三步同帧完成（loading 视觉上不
                        // 可感，语义等价）。
                        loading.set(true);
                        failed.set(false);
                        let ok = callback.run(tool_id.clone());
                        failed.set(!ok);
                        loading.set(false);
                    }
                }
            >
                {move || {
                    loading.get().then(|| view! {
                        // Loader2Icon（lucide）：`mr-1 size-3 animate-spin`。
                        <svg class="mr-1 size-3 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M21 12a9 9 0 1 1-6.219-8.56"></path>
                        </svg>
                    })
                }}
                {button_label}
            </button>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_bytes_branches() {
        // 真源 :70-80 —— MB / KB / B 三档。
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        // Intl.NumberFormat 默认 useGrouping → 1023 带千分位。
        assert_eq!(format_bytes(1023), "1,023 B");
        assert_eq!(format_bytes(1024), "1 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(999_999), "976.6 KB");
        assert_eq!(format_bytes(1024 * 1024), "1 MB");
        assert_eq!(format_bytes(2_621_440), "2.5 MB");
        assert_eq!(format_bytes(1_234_567), "1.2 MB");
    }

    #[test]
    fn format_bytes_groups_thousands() {
        // Intl.NumberFormat(zh-CN) 千分位：123456789 / 1024 / 1024 ≈ 117.7 MB。
        assert_eq!(format_bytes(123_456_789), "117.7 MB");
        // B 档整数字节带分组。
        assert_eq!(format_bytes(999), "999 B");
    }

    #[test]
    fn format_intl_number_trims_trailing_zeros() {
        // maximumFractionDigits: 1 时 1.0 显示 "1"（minimumFractionDigits 默认 0）。
        assert_eq!(format_intl_number(1.0, 1), "1");
        assert_eq!(format_intl_number(1.25, 1), "1.3");
        assert_eq!(format_intl_number(1234.0, 0), "1,234");
        assert_eq!(format_intl_number(1234.5, 1), "1,234.5");
    }

    #[test]
    fn description_sums_refs() {
        let refs = vec![
            SnapshotFieldRef {
                field: "input".to_string(),
                ref_id: "a".to_string(),
                hash: "h1".to_string(),
                full_bytes: 1024,
                preview_bytes: 64,
            },
            SnapshotFieldRef {
                field: "output".to_string(),
                ref_id: "b".to_string(),
                hash: "h2".to_string(),
                full_bytes: 4096,
                preview_bytes: 128,
            },
        ];
        assert_eq!(
            build_snapshot_notice_description(&refs),
            "该工具有 2 个字段被裁剪，当前显示预览 192 B / 5 KB"
        );
    }

    #[test]
    fn read_snapshot_refs_strict_parse() {
        // 完整项解析。
        let refs = read_snapshot_refs(&json!({
            "snapshotRefs": [
                {"field": "input", "refId": "r1", "hash": "abc", "fullBytes": 100, "previewBytes": 10},
                // 缺 hash → 跳过。
                {"field": "output", "refId": "r2", "fullBytes": 1, "previewBytes": 1},
                // bytes 非数字 → 跳过。
                {"field": "raw", "refId": "r3", "hash": "h", "fullBytes": "x", "previewBytes": 1},
            ]
        }));
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].field, "input");
        assert_eq!(refs[0].full_bytes, 100);
        // 无该键 / 非数组 → 恒空（真源 v4 adapter 不产）。
        assert!(read_snapshot_refs(&json!({})).is_empty());
        assert!(read_snapshot_refs(&json!({"snapshotRefs": "x"})).is_empty());
    }
}
