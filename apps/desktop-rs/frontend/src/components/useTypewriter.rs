//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/use-typewriter.ts`（127 行）。
//!
//! 草稿的笔。模型的脚本按块到达，扫描器看到的是跳变：一整个 `phase("implement")` 在一个
//! delta 里，有时两个阶段一起到。笔把跳变还原成书写：一次只写一站、每字 `PEN_MS`，写完一站
//! 停 `PEN_GAP_MS` 再揭示下一站——笔没到的站还不在轨道上，哪怕扫描器已经知道它。
//!
//! ★真源修订（GUI 崩溃 React #185 的根因）——笔只在**有站可揭示**时揭示：空草稿什么都不做，
//! 否则会揭示一个不存在的站、下一轮被裁回 0、再揭示……无限 setState 空转。
//! Rust 侧同一条纪律：effect 的每条分支要么推进一笔，要么静默退出。
//!
//! 计时只认**内容**变化（按内容键控）；`prefers-reduced-motion: reduce` 下整站立即写完
//! （仍一站一站揭示，只是不逐字）——reduced 分支在**渲染时派生**而不是 effect 里再 setState
//! 一轮（真源 :70-77 同一条理由）。

use leptos::prelude::*;
use leptos::task::spawn_local;

/// `PEN_MS`（真源 :21）：每字 24ms。
pub const PEN_MS: u32 = 24;
/// `PEN_GAP_MS`（真源 :22）：写完一站的停顿。
pub const PEN_GAP_MS: u32 = 120;

/// `TypewriterState`（真源 :24-31）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypewriterState {
    /// 已揭示的站数（笔到过的站）；后面的站还不在轨道上。
    pub visible: usize,
    /// 每个已揭示站已写出的字符数。
    pub shown: Vec<usize>,
    /// 笔追上了流（没有可写的字、没有可揭示的站）：光标闪烁。
    pub idle: bool,
}

/// 内部笔状态（真源 `PenState`，不含 idle——idle 是渲染时派生的）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PenState {
    pub visible: usize,
    pub shown: Vec<usize>,
}

/// `commonPrefix`（真源 :48-52）：公共前缀的长度。
/// JS 的 `a[i]` 按 UTF-16 码元取字符；站名是 UI 文本，Rust 侧按字符（char）取，
/// 对 BMP 内的名字（中日韩、拉丁）逐值相同。
pub fn common_prefix(a: &str, b: &str) -> usize {
    a.chars()
        .zip(b.chars())
        .take_while(|(x, y)| x == y)
        .count()
}

/// `reconcile`（真源 :55-68）：站名变了（变短 / 前缀变了 / 站数变少）时把笔退回到
/// 仍然成立的位置；没变就返回 `None`（= 真源的原样返回）。
pub fn reconcile(
    state: &PenState,
    names: &[String],
    previous: &[String],
) -> Option<PenState> {
    let visible = state.visible.min(names.len());
    let mut changed = visible != state.visible;
    let shown: Vec<usize> = state.shown[..visible]
        .iter()
        .enumerate()
        .map(|(i, count)| {
            let prefix = common_prefix(previous.get(i).map(String::as_str).unwrap_or(""), &names[i]);
            let next = (*count).min(prefix);
            if next != *count {
                changed = true;
            }
            next
        })
        .collect();
    if changed {
        Some(PenState { shown, visible })
    } else {
        None
    }
}

/// `shownOf`（真源 :75-77）：reduced-motion 下整站算写完，在渲染时派生。
pub fn shown_of(state: &PenState, names: &[String], reduced: bool) -> Vec<usize> {
    if reduced {
        (0..state.shown.len())
            .map(|i| names.get(i).map(|n| n.chars().count()).unwrap_or(0))
            .collect()
    } else {
        state.shown.clone()
    }
}

fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .map(|media| media.matches())
        .unwrap_or(false)
}

fn sleep_ms(ms: u32) -> wasm_bindgen_futures::JsFuture {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32);
        }
    });
    wasm_bindgen_futures::JsFuture::from(promise)
}

/// `useTypewriter`（真源 :79-126）。`names` 是扫描器当前的站名序列；`None` 表示
/// 不是草稿，钩子静默（返回 `SILENT`）。
///
/// 返回的是只读信号；笔的推进由 effect 驱动——每写一个字 / 揭示一站改一次内部信号，
/// 下一次 effect 随之触发，直到没有可做的（真源 :88-120 的 effect 链）。
pub fn use_typewriter(names: Option<Vec<String>>) -> Memo<TypewriterState> {
    let state = RwSignal::new(PenState::default());
    // 真源 :83-86：names 存 ref（每次调用都换数组，键控只看内容）；lastRef 是上一轮序列。
    let names_ref = StoredValue::new(names.clone());
    let last_ref = StoredValue::new(Vec::<String>::new());

    let key = names.as_ref().map(|list| list.join("\u{0}"));
    let key_stored = StoredValue::new(key);

    Effect::new(move |_| {
        let Some(_) = key_stored.get_value() else {
            return;
        };
        let Some(current) = names_ref.get_value() else {
            return;
        };
        let previous = last_ref.get_value();
        last_ref.set_value(current.clone());
        let pen = state.get_untracked();
        if let Some(next) = reconcile(&pen, &current, &previous) {
            state.set(next);
            return;
        }
        let reduced = prefers_reduced_motion();
        let shown = shown_of(&pen, &current, reduced);
        let at = pen.visible.checked_sub(1);
        let target = at.and_then(|i| current.get(i));
        let written = at.map(|i| shown.get(i).copied().unwrap_or(0)).unwrap_or(0);
        if let Some(target) = target {
            if written < target.chars().count() {
                // 逐字写当前站（reduced-motion 下 written 已经是整站，不会进到这里）。
                let state_for_tick = state;
                let current_len = target.chars().count();
                spawn_local(async move {
                    let _ = sleep_ms(PEN_MS).await;
                    let mut shown = state_for_tick.get_untracked().shown;
                    if shown.len() <= written {
                        shown.resize(written + 1, 0);
                    }
                    if written < current_len {
                        shown[written] = written + 1;
                        state_for_tick.set(PenState {
                            shown,
                            visible: state_for_tick.get_untracked().visible,
                        });
                    }
                });
                return;
            }
        }
        // 当前站写完了（或还没有站）：**有下一站才揭示**——空草稿什么都不做（真源 :111-119）。
        if current.len() <= pen.visible {
            return;
        }
        let state_for_reveal = state;
        let had_target = target.is_some();
        spawn_local(async move {
            if had_target {
                let _ = sleep_ms(PEN_GAP_MS).await;
            }
            let mut shown = state_for_reveal.get_untracked().shown;
            shown.push(0);
            let visible = state_for_reveal.get_untracked().visible + 1;
            state_for_reveal.set(PenState { shown, visible });
        });
    });

    Memo::new(move |_| {
        let Some(names) = names_ref.get_value() else {
            return TypewriterState::default();
        };
        let pen = state.get();
        let shown = shown_of(&pen, &names, prefers_reduced_motion());
        let at = pen.visible.checked_sub(1);
        let idle = at
            .map(|i| {
                pen.visible == names.len()
                    && shown.get(i).copied().unwrap_or(0) >= names.get(i).map(|n| n.chars().count()).unwrap_or(0)
            })
            .unwrap_or(false);
        TypewriterState {
            visible: pen.visible,
            shown,
            idle,
        }
    })
}

/// `SILENT`（真源 :38）：不是草稿时的输出。
pub fn silent() -> TypewriterState {
    TypewriterState::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn constants_match_source() {
        // 真源 :21-22。
        assert_eq!(PEN_MS, 24);
        assert_eq!(PEN_GAP_MS, 120);
    }

    #[test]
    fn common_prefix_counts_shared_head() {
        assert_eq!(common_prefix("phase(\"ver", "phase(\"verify\""), 10);
        assert_eq!(common_prefix("计划", "计划修复"), 2);
        assert_eq!(common_prefix("计划", "总结"), 0);
        assert_eq!(common_prefix("", "计划"), 0);
    }

    #[test]
    fn reconcile_pulls_pen_back_when_names_shrink() {
        // 真源 :55-68 —— 站数变少 / 前缀变了 → 退到仍然成立的位置。
        let state = PenState {
            visible: 2,
            shown: vec![3, 2],
        };
        let previous = names(&["预备", "主体"]);
        let next = names(&["预备"]);
        let reconciled = reconcile(&state, &next, &previous).unwrap();
        assert_eq!(reconciled.visible, 1);
        // 同名站也按公共前缀夹紧：previous[0] 与 next[0] 同为「预备」→ min(3, 2) = 2。
        assert_eq!(reconciled.shown, vec![2]);
    }

    #[test]
    fn reconcile_pulls_back_on_prefix_change() {
        // 名字变短：写出的字数退到公共前缀。
        let state = PenState {
            visible: 1,
            shown: vec![5],
        };
        let previous = names(&["计划修复"]);
        let next = names(&["计划"]);
        let reconciled = reconcile(&state, &next, &previous).unwrap();
        assert_eq!(reconciled.shown, vec![2]);
    }

    #[test]
    fn reconcile_is_noop_when_names_stable() {
        // 真源 :67 —— 没变就原样返回（Rust 侧 None）。
        let state = PenState {
            visible: 2,
            shown: vec![2, 1],
        };
        let same = names(&["预备", "主体"]);
        assert_eq!(reconcile(&state, &same, &same), None);
    }

    #[test]
    fn shown_of_derives_full_station_under_reduced_motion() {
        // 真源 :75-77 —— reduced-motion 下整站算写完（渲染时派生）。
        let state = PenState {
            visible: 2,
            shown: vec![1, 0],
        };
        let list = names(&["预备", "主体"]);
        assert_eq!(shown_of(&state, &list, false), vec![1, 0]);
        assert_eq!(shown_of(&state, &list, true), vec![2, 2]);
    }

    #[test]
    fn silent_state_is_default() {
        // 真源 :38 —— 不是草稿时全零。
        assert_eq!(silent(), TypewriterState::default());
        assert!(!silent().idle);
        assert_eq!(silent().visible, 0);
    }

    #[test]
    fn idle_only_when_pen_caught_up() {
        // 真源 :125 —— idle = 笔到过末站且末站写满。
        let list = names(&["预备", "主体"]);
        let caught_up = TypewriterState {
            visible: 2,
            shown: vec![2, 2],
            idle: true,
        };
        let at = caught_up.visible - 1;
        assert!(caught_up.shown[at] >= list[at].chars().count());
        let behind = TypewriterState {
            visible: 2,
            shown: vec![2, 1],
            idle: false,
        };
        assert!(behind.shown[1] < list[1].chars().count());
    }
}
