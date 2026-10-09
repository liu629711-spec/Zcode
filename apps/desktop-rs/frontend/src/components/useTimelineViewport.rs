//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/use-timeline-viewport.ts`（101 行）。
//!
//! 滚动视口的三个数（scrollLeft / clientWidth / scrollWidth）与「正在滚」标记。
//! 滚动事件按帧合并；尺寸变化走 ResizeObserver（容器与内容层都看，草稿的笔每揭示一站
//! 内容就变宽）。量不到（宽 0）时消费者什么都不折——静态测试结果逐像素保持不变。
//!
//! ★真源 :7-14 注释——以**元素**而不是 ref 对象为依赖：元素由消费者放进状态交过来，
//! 出现就接线、离开就拆。量到的数没变就不换对象（真源 :15-16，多余的更新曾让时间线白渲染）。

use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// `TimelineViewport`（真源 :16-22）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TimelineViewport {
    pub scroll_left: f64,
    pub client_width: f64,
    pub scroll_width: f64,
    /// 最近 800ms 内滚过（滚动条据此露出）。
    pub scrolling: bool,
}

/// `SCROLLING_MS`（真源 :24）。
pub const SCROLLING_MS: u32 = 800;

/// `EMPTY`（真源 :25）。
pub fn empty_viewport() -> TimelineViewport {
    TimelineViewport::default()
}

/// 一次测量（真源 `measure`，:27-33）。量不到（节点已离场）给全零。
fn measure(element: &web_sys::Element) -> (f64, f64, f64) {
    (
        element.scroll_left() as f64,
        element.client_width() as f64,
        element.scroll_width() as f64,
    )
}

/// `merge`（真源 :36-47）：把一次测量并进上一份视口；三个数与 scrolling 都没变就返回
/// 上一份（真源：React 据此跳过这轮渲染；Leptos 侧信号不变同样不触发重渲染）。
pub fn merge(
    previous: TimelineViewport,
    measured: (f64, f64, f64),
    scrolling: bool,
) -> TimelineViewport {
    if previous.client_width == measured.1
        && previous.scroll_left == measured.0
        && previous.scroll_width == measured.2
        && previous.scrolling == scrolling
    {
        previous
    } else {
        TimelineViewport {
            scroll_left: measured.0,
            client_width: measured.1,
            scroll_width: measured.2,
            scrolling,
        }
    }
}

/// `useTimelineViewport`（真源 :51-100）。
///
/// - `element`：滚动层元素的**只读信号**（真源以元素为 effect 依赖：出现就接线、
///   离开就拆）；还没挂上（草稿首帧）或已卸下时为 `None`。
/// - `content_key`：内容变化的抓手（站数、宽度）；变了就重新量一次。
///
/// Rust 侧监听接线用 web_sys 事件回调，与真源 addEventListener / ResizeObserver 同构；
/// 滚动按帧合并走 requestAnimationFrame；`scrolling` 标记 800ms 后复位。
pub fn use_timeline_viewport(
    element: ReadSignal<Option<web_sys::Element>>,
    content_key: usize,
) -> Memo<TimelineViewport> {
    let viewport = RwSignal::new(empty_viewport());

    // 真源 :59-62（useIsomorphicLayoutEffect）：元素在场 / 内容变化 → 立即量一次。
    Effect::new(move |_| {
        let element = element.get();
        let _ = content_key;
        let Some(element) = element else {
            return;
        };
        viewport.set(merge(
            viewport.get_untracked(),
            measure(&element),
            viewport.get_untracked().scrolling,
        ));
    });

    // 真源 :64-98：滚动 / 尺寸监听。元素变化即重挂（effect 以元素为依赖）。
    Effect::new(move |_| {
        let Some(element) = element.get() else {
            return;
        };
        let target: web_sys::EventTarget = element.clone().into();

        // 滚动：按帧合并（真源 onScroll 的 rAF 节流）。
        let frame = StoredValue::new(None::<i32>);
        let scrolling_timer = StoredValue::new(None::<i32>);
        let viewport_for_scroll = viewport;
        let element_for_scroll = element.clone();
        let on_scroll: Closure<dyn Fn()> = Closure::new(move || {
            if frame.get_value().is_some() {
                return;
            }
            let viewport = viewport_for_scroll;
            let element = element_for_scroll.clone();
            let scrolling_timer = scrolling_timer;
            let frame = frame;
            let cb = Closure::once(move || {
                frame.set_value(None);
                viewport.set(merge(
                    viewport.get_untracked(),
                    measure(&element),
                    true,
                ));
                // 800ms 后复位 scrolling 标记（真源 :73-79）。
                let viewport = viewport;
                let scrolling_timer = scrolling_timer;
                let cb = Closure::once(move || {
                    scrolling_timer.set_value(None);
                    let previous = viewport.get_untracked();
                    if previous.scrolling {
                        viewport.set(TimelineViewport {
                            scrolling: false,
                            ..previous
                        });
                    }
                });
                if let Some(w) = web_sys::window() {
                    if let Ok(id) = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                        cb.as_ref().unchecked_ref(),
                        SCROLLING_MS as i32,
                    ) {
                        scrolling_timer.set_value(Some(id));
                    }
                }
                std::mem::forget(cb);
            });
            if let Some(w) = web_sys::window() {
                if let Ok(id) = w.request_animation_frame(cb.as_ref().unchecked_ref()) {
                    frame.set_value(Some(id));
                }
            }
            std::mem::forget(cb);
        });
        let _ = target.add_event_listener_with_callback("scroll", on_scroll.as_ref().unchecked_ref());

        // 尺寸：ResizeObserver 看容器与内容层（真源 :83-91）。
        let mut observer: Option<web_sys::ResizeObserver> = None;
        let viewport_for_resize = viewport;
        let element_for_resize = element.clone();
        let on_resize: Closure<dyn Fn()> = {
            let viewport = viewport_for_resize;
            let element = element_for_resize;
            Closure::new(move || {
                viewport.set(merge(
                    viewport.get_untracked(),
                    measure(&element),
                    viewport.get_untracked().scrolling,
                ));
            })
        };
        if let Ok(ro) = web_sys::ResizeObserver::new(on_resize.as_ref().unchecked_ref()) {
            ro.observe(&element);
            if let Some(content) = element.first_element_child() {
                ro.observe(&content);
            }
            observer = Some(ro);
        }

        // 清理要 Send（leptos on_cleanup 的约束）；DOM 闭包用 SendWrapper 装进来。
        let on_scroll_cleanup = send_wrapper::SendWrapper::new(on_scroll);
        let ro_cleanup = send_wrapper::SendWrapper::new(observer);
        let target_cleanup = send_wrapper::SendWrapper::new(target);
        on_cleanup(move || {
            let _ = target_cleanup.remove_event_listener_with_callback(
                "scroll",
                on_scroll_cleanup.as_ref().unchecked_ref(),
            );
            if let Some(ro) = ro_cleanup.take() {
                ro.disconnect();
            }
        });
        // on_scroll 已交给 SendWrapper（cleanup 持引用）；on_resize 由 ResizeObserver 持有。
        std::mem::forget(on_resize);
    });

    Memo::new(move |_| viewport.get())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_source() {
        assert_eq!(SCROLLING_MS, 800);
        assert_eq!(empty_viewport(), TimelineViewport::default());
        assert_eq!(empty_viewport().client_width, 0.0);
    }

    #[test]
    fn merge_keeps_previous_identity_when_nothing_changed() {
        // 真源 :36-47 —— 三个数与 scrolling 都没变就返回上一份。
        let previous = TimelineViewport {
            scroll_left: 12.0,
            client_width: 100.0,
            scroll_width: 300.0,
            scrolling: true,
        };
        let merged = merge(previous, (12.0, 100.0, 300.0), true);
        assert_eq!(merged.scroll_left, previous.scroll_left);
        assert_eq!(merged.scrolling, previous.scrolling);
    }

    #[test]
    fn merge_updates_when_scroll_changes() {
        let previous = TimelineViewport {
            scroll_left: 0.0,
            client_width: 100.0,
            scroll_width: 300.0,
            scrolling: false,
        };
        let merged = merge(previous, (40.0, 100.0, 300.0), true);
        assert_eq!(merged.scroll_left, 40.0);
        assert!(merged.scrolling);
    }

    #[test]
    fn merge_updates_when_scrolling_flag_clears() {
        // 真源 :76-79 —— 只翻转 scrolling 也要换对象（滚动条据此收起）。
        let previous = TimelineViewport {
            scroll_left: 40.0,
            client_width: 100.0,
            scroll_width: 300.0,
            scrolling: true,
        };
        let merged = merge(previous, (40.0, 100.0, 300.0), false);
        assert!(!merged.scrolling);
        assert_eq!(merged.scroll_left, 40.0);
    }

    #[test]
    fn merge_updates_when_content_grows() {
        // 草稿的笔每揭示一站内容变宽：scrollWidth 变化必须穿透。
        let previous = TimelineViewport {
            scroll_left: 0.0,
            client_width: 100.0,
            scroll_width: 300.0,
            scrolling: false,
        };
        let merged = merge(previous, (0.0, 100.0, 456.0), false);
        assert_eq!(merged.scroll_width, 456.0);
    }
}
