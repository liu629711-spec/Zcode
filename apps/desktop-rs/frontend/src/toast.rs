//! Toast 提示（1:1 迁移`packages/ui/src/components/ui/toast.tsx` 的核心契约）。
//!
//! ## 为什么不是整个文件逐行翻译
//!
//! 真源 466 行里包含：React Context + 模块级单例、4 个位置的堆叠定位、
//! 拖拽手势（swipe dismiss）、`update` 变体的原地更新、路由变化时清空队列等。
//! Rust 侧当前只有「操作失败提示用户」这一个用例，故只迁**契约与视觉**，
//! 不迁那套堆叠/手势基础设施。
//!
//! **必须对齐的部分**（照抄真源值，见下）：
//! - 默认时长 `3000ms`（真源 :70）
//! - 进出场动画 `200ms`（真源 :71 `TOAST_TRANSITION_DURATION_MS`）
//! - 容器定位类名（真源 :51-57）：`fixed top-16 ... z-[9999] flex flex-col gap-2`
//! - 单条卡片类名（真源 :310-317）：`rounded-2xl border bg-toast/60 ... shadow-lg
//!   backdrop-blur-xl` + 过渡 `transition-[transform,opacity] duration-200`
//! - 消息约定：首行是标题，其余行是正文（真源 :263-264按 `\n` 切分）
//!
//! ## 与真源的差异（有意）
//!
//! 真源有 `default / update / info / warning` 四变体，且不同变体宽度策略不同
//! （notice 类 `w-[min(536px,...)]`，default 类 `px-4 py-3`）。Rust 侧先只实现
//! `info` / `warning` 两个 notice 变体（错误提示用），保留变体字段以便后续补齐。

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;

/// 默认展示时长（真源 `DEFAULT_TOAST_DURATION_MS`，toast.tsx:70）。
pub const DEFAULT_TOAST_DURATION_MS: i64 = 3000;

/// 进出场动画时长（真源 `TOAST_TRANSITION_DURATION_MS`，toast.tsx:71）。
pub const TOAST_TRANSITION_DURATION_MS: i64 = 200;

/// Toast 变体（真源 `ToastVariant`，toast.tsx:13）。
///
/// `default` / `update` Rust 侧暂未使用，但保留字段以便对齐真源枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastVariant {
    #[default]
    Default,
    Update,
    Info,
    Warning,
}

/// Toast 显示位置（真源 `ToastPosition`，toast.tsx:12）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastPosition {
    #[default]
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
}

impl ToastPosition {
    /// 容器类名（照抄真源 toast.tsx:49-58）。
    ///
    /// 真源 bottom 两个变体带 `bottomInset`（动态计算），Rust侧先固定
    /// `bottom-4` —— 当前只有 top 位置在用，不影响显示。
    fn container_class(self) -> &'static str {
        match self {
            ToastPosition::TopCenter => {
                "fixed top-16 left-1/2 z-[9999] flex -translate-x-1/2 flex-col items-center gap-2"
            }
            ToastPosition::TopRight => {
                "fixed right-4 top-16 z-[9999] flex max-w-[calc(100vw-2rem)] flex-col items-end gap-2"
            }
            ToastPosition::BottomLeft => {
                "fixed bottom-4 left-4 z-[9999] flex flex-col items-start gap-2"
            }
            ToastPosition::BottomCenter => {
                "fixed bottom-4 left-1/2 z-[9999] flex -translate-x-1/2 flex-col items-center gap-2"
            }
        }
    }
}

/// 单条 toast（真源 `ToastItem`，toast.tsx:28-35 的子集）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastItem {
    pub id: u64,
    pub message: String,
    pub variant: ToastVariant,
    pub position: ToastPosition,
    pub duration_ms: i64,
}

impl ToastItem {
    /// 标题 = 消息首行，正文 = 其余行（真源 toast.tsx:263-264按 `\n` 切分）。
    pub fn title(&self) -> &str {
        self.message.split('\n').next().unwrap_or_default()
    }

    /// 正文 = 去掉首行后的部分（空则返回空串）。
    pub fn body(&self) -> String {
        let mut it = self.message.split('\n');
        it.next();
        it.collect::<Vec<_>>().join("\n").trim().to_string()
    }

    /// 卡片类名（照抄真源 toast.tsx:310-317 的变体分支）。
    fn card_class(&self) -> String {
        let base = "rounded-2xl border bg-toast/60 text-ui-base shadow-lg backdrop-blur-xl \
                    transition-[transform,opacity] duration-200 ease-[cubic-bezier(0.77,0,0.175,1)] \
                    motion-reduce:transform-none motion-reduce:transition-opacity";
        let variant = match self.variant {
            // 真源 update 变体：固定宽度、左下角变换原点。
            ToastVariant::Update => {
                "origin-bottom-left w-[min(300px,calc(100vw-1rem))] max-w-[min(300px,calc(100vw-1rem))] border-popover-border text-foreground shadow-lg"
            }
            // 真源 notice 变体（info / warning）：更宽、无内边距（内容自带）。
            ToastVariant::Info | ToastVariant::Warning => {
                "w-[min(536px,calc(100vw-2rem))] border-border text-foreground"
            }
            // 真源 default 变体：内容自带内边距。
            ToastVariant::Default => "border-border px-4 py-3 text-foreground whitespace-pre-line",
        };
        format!("{base} {variant}")
    }
}

/// 全局 toast 队列（挂载时由 `<Toaster/>` 消费）。
#[derive(Debug, Clone, Default)]
pub struct ToastStore {
    pub items: RwSignal<Vec<ToastItem>>,
    next_id: RwSignal<u64>,
}

impl ToastStore {
    /// 推入一条 toast，返回其 id。
    ///
    /// `duration_ms <= 0` 表示常驻不自动消失（真源 toast.tsx:247-248 的
    /// `!Number.isFinite || <= 0` 分支）。
    pub fn push(&self, message: impl Into<String>, variant: ToastVariant) -> u64 {
        self.push_with(
            message,
            variant,
            ToastPosition::default(),
            DEFAULT_TOAST_DURATION_MS,
        )
    }

    /// 完整版推入（可指定位置与时长）。
    pub fn push_with(
        &self,
        message: impl Into<String>,
        variant: ToastVariant,
        position: ToastPosition,
        duration_ms: i64,
    ) -> u64 {
        let id = self.next_id.get_untracked();
        self.next_id.set(id + 1);
        let item = ToastItem {
            id,
            message: message.into(),
            variant,
            position,
            duration_ms,
        };
        self.items.update(|list| list.push(item));
        id
    }

    /// 移除一条（真源 `removeToast`）。
    pub fn remove(&self, id: u64) {
        self.items.update(|list| list.retain(|i| i.id != id));
    }

    /// 清空队列（真源在路由变化时清空，见 `dismissedBeforeMountToastIds` 段）。
    pub fn clear(&self) {
        self.items.set(Vec::new());
    }

    /// 便捷方法：错误提示（warning 变体）。
    pub fn error(&self, message: impl Into<String>) {
        self.push(message, ToastVariant::Warning);
    }

    /// 便捷方法：普通信息（info 变体）。
    pub fn info(&self, message: impl Into<String>) {
        self.push(message, ToastVariant::Info);
    }
}

/// `Toaster` 根组件：渲染队列 + 负责自动消失。
///
/// 真源用 `setTimeout` + `setVisible(false)` 两段式（先淡出，等动画结束再移除），
/// Rust 侧用两个定时器实现同样的两段式，避免 Toast 突然消失。
#[component]
pub fn Toaster(store: ToastStore) -> impl IntoView {
    let items = store.items;
    view! {
        <div class="pointer-events-none fixed inset-0 z-[9998]">
            {move || {
                let list = items.get();
                // 按位置分组渲染（真源每种位置一个容器）。
                let mut by_position: HashMap<&'static str, Vec<&ToastItem>> = HashMap::new();
                for item in &list {
                    by_position
                        .entry(position_key(item.position))
                        .or_default()
                        .push(item);
                }
                by_position
                    .into_iter()
                    .map(|(key, group)| {
                        let pos = parse_position(key);
                        view! {
                            <div class=pos.container_class()>
                                {group
                                    .into_iter()
                                    .map(|item| {
                                        let store = store.clone();
                                        view! {
                                            <ToastCard item=item.clone() store=store />
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                    })
                    .collect_view()
            }}
        </div>
    }
}

fn position_key(p: ToastPosition) -> &'static str {
    match p {
        ToastPosition::TopCenter => "top-center",
        ToastPosition::TopRight => "top-right",
        ToastPosition::BottomLeft => "bottom-left",
        ToastPosition::BottomCenter => "bottom-center",
    }
}

fn parse_position(key: &str) -> ToastPosition {
    match key {
        "top-right" => ToastPosition::TopRight,
        "bottom-left" => ToastPosition::BottomLeft,
        "bottom-center" => ToastPosition::BottomCenter,
        _ => ToastPosition::TopCenter,
    }
}

/// 单条 toast 卡片：负责自动消失与手动关闭。
#[component]
fn ToastCard(item: ToastItem, store: ToastStore) -> impl IntoView {
    let id = item.id;
    let duration = item.duration_ms;
    let visible = RwSignal::new(true);

    // 自动消失：duration 计时 → 淡出 → 等动画 → 移除（真源 :247-259 三段式）。
    // store 是非 Copy 的信号容器，Effect 与 Callback 各自 clone 一份。
    let store_for_effect = store.clone();
    Effect::new(move |_| {
        let Some(duration) = (duration > 0).then_some(duration) else {
            // duration<=0 = 常驻，不自动消失（真源 :247-248）。
            return;
        };
        let store = store_for_effect.clone();
        let visible = visible;
        spawn_local(async move {
            sleep_ms(duration as u32).await;
            visible.set(false);
            // 等退场动画结束再移除（真源 :250-251）。
            sleep_ms(TOAST_TRANSITION_DURATION_MS as u32).await;
            store.remove(id);
        });
    });

    let dismiss = Callback::new(move |_: ()| {
        let store = store.clone();
        let visible = visible;
        visible.set(false);
        spawn_local(async move {
            sleep_ms(TOAST_TRANSITION_DURATION_MS as u32).await;
            store.remove(id);
        });
    });

    let title = item.title().to_string();
    let body = item.body();

    view! {
        <div
            class=item.card_class()
            style=move || {
                // 退场时缩小并淡出（真源 dismissWithTransition :266-269）。
                if visible.get() {
                    "opacity-100 translate-y-0"
                } else {
                    "opacity-0 -translate-y-1"
                }
            }
            role="status"
        >
            <div class="flex items-start gap-3 px-4 py-3">
                <div class="min-w-0 flex-1">
                    <div class="font-medium">{title}</div>
                    {(!body.is_empty())
                        .then(|| view! { <div class="mt-1 whitespace-pre-line text-ui-sm text-foreground-subtle">{body}</div> })}
                </div>
                <button
                    type="button"
                    class="flex-none cursor-pointer rounded p-1 text-foreground-subtlest hover:bg-surface-hover"
                    aria-label="关闭"
                    on:click=move |_| dismiss.run(())
                >
                    <span class="block size-3">
                        <svg viewBox="0 0 12 12" class="size-3" fill="none" stroke="currentColor" stroke-width="1.5">
                            <path d="M1 1L11 11M11 1L1 11" stroke-linecap="round"/>
                        </svg>
                    </span>
                </button>
            </div>
        </div>
    }
}

/// `setTimeout` 的 Promise 包装（项目内已有同款实现）。
async fn sleep_ms(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_source() {
        // 真源 toast.tsx:70-71。这两个值影响手感和测试稳定性，改了要同步。
        assert_eq!(DEFAULT_TOAST_DURATION_MS, 3000);
        assert_eq!(TOAST_TRANSITION_DURATION_MS, 200);
    }

    #[test]
    fn message_splits_into_title_and_body() {
        // 真源 :263-264：首行标题，其余行正文。
        let item = ToastItem {
            id: 1,
            message: "标题\n正文第一行\n正文第二行".into(),
            variant: ToastVariant::Info,
            position: ToastPosition::TopCenter,
            duration_ms: DEFAULT_TOAST_DURATION_MS,
        };
        assert_eq!(item.title(), "标题");
        assert_eq!(item.body(), "正文第一行\n正文第二行");
    }

    #[test]
    fn single_line_message_has_empty_body() {
        let item = ToastItem {
            id: 1,
            message: "只有标题".into(),
            variant: ToastVariant::Warning,
            position: ToastPosition::TopCenter,
            duration_ms: DEFAULT_TOAST_DURATION_MS,
        };
        assert_eq!(item.title(), "只有标题");
        assert_eq!(item.body(), "");
    }

    #[test]
    fn container_classes_match_source() {
        // 真源 :51-57 的四个容器类名。
        assert!(
            ToastPosition::TopCenter
                .container_class()
                .contains("fixed top-16 left-1/2")
        );
        assert!(
            ToastPosition::TopCenter
                .container_class()
                .contains("z-[9999]")
        );
        assert!(
            ToastPosition::TopRight
                .container_class()
                .contains("fixed right-4 top-16")
        );
        assert!(
            ToastPosition::BottomLeft
                .container_class()
                .contains("fixed bottom-4 left-4")
        );
        assert!(
            ToastPosition::BottomCenter
                .container_class()
                .contains("left-1/2")
        );
    }

    #[test]
    fn card_class_contains_source_tokens() {
        let item = ToastItem {
            id: 1,
            message: "x".into(),
            variant: ToastVariant::Info,
            position: ToastPosition::TopCenter,
            duration_ms: DEFAULT_TOAST_DURATION_MS,
        };
        let cls = item.card_class();
        // 真源 :310-312 的公共基础类
        for token in [
            "rounded-2xl",
            "bg-toast/60",
            "shadow-lg",
            "backdrop-blur-xl",
            "transition-[transform,opacity]",
            "duration-200",
        ] {
            assert!(cls.contains(token), "卡片类名应含 {token}");
        }
        // info/warning 走 notice 宽度
        assert!(cls.contains("w-[min(536px,calc(100vw-2rem))]"));
    }

    #[test]
    fn default_variant_uses_padding_instead_of_width() {
        // 真源：default 变体用 px-4 py-3，notice 变体用固定宽度。
        let item = ToastItem {
            id: 1,
            message: "x".into(),
            variant: ToastVariant::Default,
            position: ToastPosition::TopCenter,
            duration_ms: DEFAULT_TOAST_DURATION_MS,
        };
        let cls = item.card_class();
        assert!(cls.contains("px-4 py-3"));
        assert!(!cls.contains("w-[min(536px"));
    }

    #[test]
    fn store_push_increments_id() {
        let store = ToastStore::default();
        let a = store.push("一", ToastVariant::Info);
        let b = store.push("二", ToastVariant::Warning);
        assert_ne!(a, b);
        assert_eq!(store.items.get_untracked().len(), 2);
    }

    #[test]
    fn store_remove_and_clear() {
        let store = ToastStore::default();
        let a = store.push("一", ToastVariant::Info);
        store.push("二", ToastVariant::Info);
        store.remove(a);
        assert_eq!(store.items.get_untracked().len(), 1);
        store.clear();
        assert!(store.items.get_untracked().is_empty());
    }

    #[test]
    fn error_uses_warning_variant() {
        // 落库失败等错误提示用 warning（真源 notice 变体）。
        let store = ToastStore::default();
        store.error("保存失败");
        let items = store.items.get_untracked();
        assert_eq!(items[0].variant, ToastVariant::Warning);
    }
}
