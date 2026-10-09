//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/workflow-face-motion.ts`（178 行）。
//!
//! 瓦片脸的动作引擎：一张脸只有一个待执行计时器；DOM 动作不触发整个工作流的
//! 渲染。`faceState` / `BASE_EXPRESSION` / 表情池 / 停顿区间是纯映射（有单测）；
//! `startFaceMotion` 是纯 DOM 编排（data-* 属性 + CSS 自定义属性 + 计时器链）。
//!
//! JS 用互指的具名闭包表达循环（cycle → pause → cycle）；Rust 侧把这张转移图
//! 折叠成**显式的步骤枚举 + 单槽调度器**（`Step` / `later`）——同一时刻只有一个
//! 待执行计时器，转移条件与概率逐字对齐真源。签名与清理契约相同：
//! 返回的清理函数清计时器、断观察器、摘监听、复位基础表情。

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::components::workflow_graph::types::StepRunStatus;

/// `FaceState`（真源 :3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceState {
    Waiting,
    Scanning,
    Content,
    Sad,
}

impl FaceState {
    pub fn as_str(self) -> &'static str {
        match self {
            FaceState::Waiting => "waiting",
            FaceState::Scanning => "scanning",
            FaceState::Content => "content",
            FaceState::Sad => "sad",
        }
    }
}

/// `faceState`（真源 :5-13）。
pub fn face_state(status: Option<StepRunStatus>) -> FaceState {
    match status {
        Some(StepRunStatus::Running) => FaceState::Scanning,
        Some(StepRunStatus::Done) => FaceState::Content,
        Some(StepRunStatus::Failed) => FaceState::Sad,
        _ => FaceState::Waiting,
    }
}

/// `EyeExpression`（真源 :4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EyeExpression {
    Dots,
    Pill,
    Happy,
    Sleepy,
    Focused,
    Sad,
    Confused,
}

impl EyeExpression {
    pub fn as_str(self) -> &'static str {
        match self {
            EyeExpression::Dots => "dots",
            EyeExpression::Pill => "pill",
            EyeExpression::Happy => "happy",
            EyeExpression::Sleepy => "sleepy",
            EyeExpression::Focused => "focused",
            EyeExpression::Sad => "sad",
            EyeExpression::Confused => "confused",
        }
    }
}

/// `BASE_EXPRESSION`（真源 :14-19）：状态定义基础表情。
pub fn base_expression(state: FaceState) -> EyeExpression {
    match state {
        FaceState::Waiting => EyeExpression::Dots,
        FaceState::Scanning => EyeExpression::Pill,
        FaceState::Content => EyeExpression::Happy,
        FaceState::Sad => EyeExpression::Sad,
    }
}

/// 偶发表情池（真源 `SPECIAL`，:20-25）：只有 scanning 有偶发表情。
fn special(state: FaceState) -> &'static [EyeExpression] {
    match state {
        FaceState::Scanning => &[EyeExpression::Focused, EyeExpression::Confused],
        _ => &[],
    }
}

/// 停顿区间（真源 pause :48-60，毫秒）。
fn pause_range(state: FaceState) -> (u32, u32) {
    match state {
        FaceState::Waiting => (1500, 3000),
        FaceState::Scanning => (1500, 3500),
        FaceState::Sad => (1800, 4000),
        FaceState::Content => (6000, 10000),
    }
}

fn random_in(min: u32, max: u32) -> u32 {
    min + (js_sys::Math::random() * (max - min) as f64) as u32
}

/// 编排的一步。真源里每个具名闭包在这里是一个变体；`morph(expression, next)`
/// 的续延参数用 `Box<Step>` 表达。
enum Step {
    /// `base`（真源 :42-47）：复位基础表情与视线。
    Base,
    /// `pause`（真源 :48-60）：歇一会儿再进 cycle。
    Pause,
    /// `cycle`（真源 :95-148）：按状态起一段动作。
    Cycle,
    /// `glance`（真源 :88-94）：瞥一眼，再决定要不要换表情。
    Glance,
    /// `maybeExpression`（真源 :72-87）：攒够两轮后按概率换偶发表情。
    MaybeExpression,
    /// `morph`（真源 :62-71）的第一拍：motion=morph，80ms 后换轮廓。
    Morph(EyeExpression, Box<Step>),
    /// morph 的第二拍：换 data-expression，100ms 后复位 idle 并走续延。
    SetExpression(EyeExpression, Box<Step>),
    /// 直接换表情（content 眨眼结束那一下：expression=happy; pause()）。
    SetExpressionNow(EyeExpression, Box<Step>),
    /// motion=idle 后立即执行续延。
    IdleThen(Box<Step>),
    /// 纯延时后执行续延（对应内联的 setTimeout）。
    Delay(u32, Box<Step>),
    /// sad 浮动落定（真源 :105-119）：40% 概率接 focused → 抖动。
    SadFloat,
    /// sad 的抖动段：3-5 下 shake。
    SadShake,
    /// content 跳跃落定（真源 :122-139）：表情换 pill，停 250ms 后眨一次眼。
    ContentHop,
    /// content 的眨眼段。
    ContentBlink,
    /// scanning 的眨眼段（真源 :140-147）：眨 2-3 下后瞥一眼。
    ScanningBlink,
}

struct Inner {
    face: web_sys::Element,
    state: FaceState,
    timer: Option<i32>,
    next: Option<Step>,
    rounds: u32,
    looking_left: bool,
    previous: Option<EyeExpression>,
    /// restart 时采样的三面镜子（真源 :155）：页面可见、未减动效、画面内。
    page_visible: bool,
    reduced: bool,
    intersecting: bool,
    restart_listeners: Vec<ListenerGuard>,
    observer: Option<web_sys::IntersectionObserver>,
}

/// 监听的清理（removeEventListener 需要把闭包活着拿出来，包一层 guard）。
struct ListenerGuard {
    target: web_sys::EventTarget,
    kind: &'static str,
    callback: Closure<dyn Fn()>,
}

impl Drop for ListenerGuard {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.kind, self.callback.as_ref().unchecked_ref());
    }
}

fn set_style(face: &web_sys::Element, name: &str, value: &str) {
    let _ = face
        .unchecked_ref::<web_sys::HtmlElement>()
        .style()
        .set_property(name, value);
}

fn motion(face: &web_sys::Element, value: &str) {
    let _ = face.set_attribute("data-motion", value);
}

fn set_expression(face: &web_sys::Element, expression: EyeExpression) {
    let _ = face.set_attribute("data-expression", expression.as_str());
}

/// 单槽调度：同一时刻只有一个待执行计时器（真源「一张脸只有一个待执行计时器」）。
fn later(handle: &Rc<RefCell<Inner>>, delay: u32, step: Step) {
    let run_handle = handle.clone();
    let callback: Closure<dyn Fn()> = Closure::new(move || run_step(&run_handle));
    let id = web_sys::window().and_then(|w| {
        w.set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            delay as i32,
        )
        .ok()
    });
    let mut inner = handle.borrow_mut();
    if let Some(old) = inner.timer.take() {
        if let Some(w) = web_sys::window() {
            let _ = w.clear_timeout_with_handle(old);
        }
    }
    inner.next = Some(step);
    inner.timer = id;
    // 每次调度换新闭包；闭包随计时器回调被浏览器持有，执行后自然释放。
    std::mem::forget(callback);
}

fn run_step(handle: &Rc<RefCell<Inner>>) {
    let step = handle.borrow_mut().next.take();
    if let Some(step) = step {
        exec(handle, step);
    }
}

fn exec(handle: &Rc<RefCell<Inner>>, step: Step) {
    let face = handle.borrow().face.clone();
    match step {
        Step::Base => {
            let state = handle.borrow().state;
            set_expression(&face, base_expression(state));
            motion(&face, "idle");
            set_style(&face, "--wf-face-x", "0px");
            set_style(
                &face,
                "--wf-face-y",
                if state == FaceState::Sad { "0.3px" } else { "0px" },
            );
        }
        Step::Pause => {
            let state = handle.borrow().state;
            motion(&face, "idle");
            let (lo, hi) = pause_range(state);
            later(handle, random_in(lo, hi), Step::Cycle);
        }
        Step::Glance => {
            motion(&face, "glance");
            // 平移要换到脸的另一侧；旧的 ±0.65 微移只像眼睛抖动（真源 :90-92）。
            let looking_left = {
                let mut inner = handle.borrow_mut();
                inner.looking_left = !inner.looking_left;
                inner.looking_left
            };
            set_style(&face, "--wf-face-x", if looking_left { "-4px" } else { "0px" });
            later(handle, 360, Step::MaybeExpression);
        }
        Step::MaybeExpression => {
            let state = handle.borrow().state;
            let rounds = {
                let mut inner = handle.borrow_mut();
                inner.rounds += 1;
                inner.rounds
            };
            let probability = if state == FaceState::Scanning { 0.45 } else { 0.25 };
            if rounds < 2 || js_sys::Math::random() >= probability {
                later(handle, 0, Step::Pause);
                return;
            }
            // 排除上一个表情；只一种结果态也要经过两轮基础动作，不能连续换脸（真源 :80-81）。
            let previous = handle.borrow().previous;
            let pool_all = special(state);
            let mut choices: Vec<EyeExpression> =
                pool_all.iter().copied().filter(|e| Some(*e) != previous).collect();
            if choices.is_empty() {
                choices = pool_all.to_vec();
            }
            let expression = choices[(js_sys::Math::random() * choices.len() as f64) as usize];
            {
                let mut inner = handle.borrow_mut();
                inner.previous = Some(expression);
                inner.rounds = 0;
            }
            // hold：confused 停得更久（真源 :85）。
            let hold = if expression == EyeExpression::Confused {
                random_in(2200, 3800)
            } else {
                random_in(800, 1800)
            };
            later(
                handle,
                0,
                Step::Morph(
                    expression,
                    Box::new(Step::Delay(
                        hold,
                        Box::new(Step::Morph(base_expression(state), Box::new(Step::Pause))),
                    )),
                ),
            );
        }
        Step::Morph(expression, next) => {
            motion(&face, "morph");
            later(handle, 80, Step::SetExpression(expression, next));
        }
        Step::SetExpression(expression, next) => {
            set_expression(&face, expression);
            later(handle, 100, Step::IdleThen(next));
        }
        Step::SetExpressionNow(expression, next) => {
            set_expression(&face, expression);
            exec(handle, *next);
        }
        Step::IdleThen(next) => {
            motion(&face, "idle");
            exec(handle, *next);
        }
        Step::Delay(ms, next) => later(handle, ms, Step::IdleThen(next)),
        Step::Cycle => match handle.borrow().state {
            FaceState::Waiting => {
                motion(&face, "dots-wave");
                later(handle, 900, Step::Pause);
            }
            FaceState::Sad => {
                let floats = if js_sys::Math::random() < 0.5 { 1 } else { 2 };
                set_style(&face, "--wf-face-floats", &floats.to_string());
                motion(&face, "float");
                later(handle, floats * 600, Step::SadFloat);
            }
            FaceState::Content => {
                let hops = if js_sys::Math::random() < 0.5 { 2 } else { 3 };
                set_style(&face, "--wf-face-hops", &hops.to_string());
                motion(&face, "hop");
                later(handle, hops * 240, Step::ContentHop);
            }
            FaceState::Scanning => {
                let blinks = if js_sys::Math::random() < 0.5 { 2 } else { 3 };
                let duration = random_in(200, 260);
                set_style(&face, "--wf-face-blinks", &blinks.to_string());
                set_style(&face, "--wf-face-blink-time", &format!("{duration}ms"));
                motion(&face, "blink");
                later(handle, duration * blinks, Step::ScanningBlink);
            }
        },
        Step::SadFloat => {
            if js_sys::Math::random() >= 0.4 {
                later(handle, 0, Step::Pause);
                return;
            }
            later(handle, 0, Step::Morph(EyeExpression::Focused, Box::new(Step::SadShake)));
        }
        Step::SadShake => {
            let shakes = 3 + (js_sys::Math::random() * 3.0) as u32;
            set_style(&face, "--wf-face-shakes", &shakes.to_string());
            motion(&face, "shake");
            later(
                handle,
                shakes * 90,
                Step::IdleThen(Box::new(Step::Delay(
                    random_in(500, 1000),
                    Box::new(Step::Morph(EyeExpression::Sad, Box::new(Step::Pause))),
                ))),
            );
        }
        Step::ContentHop => {
            set_expression(&face, EyeExpression::Pill);
            motion(&face, "idle");
            later(handle, 250, Step::ContentBlink);
        }
        Step::ContentBlink => {
            let duration = random_in(260, 320);
            set_style(&face, "--wf-face-blinks", "1");
            set_style(&face, "--wf-face-blink-time", &format!("{duration}ms"));
            motion(&face, "blink");
            later(
                handle,
                duration,
                Step::SetExpressionNow(EyeExpression::Happy, Box::new(Step::Pause)),
            );
        }
        Step::ScanningBlink => later(handle, 0, Step::Glance),
    }
}

fn restart(handle: &Rc<RefCell<Inner>>) {
    {
        let mut inner = handle.borrow_mut();
        if let Some(old) = inner.timer.take() {
            if let Some(w) = web_sys::window() {
                let _ = w.clear_timeout_with_handle(old);
            }
        }
        inner.next = None;
        inner.rounds = 0;
        inner.looking_left = false;
        inner.page_visible = web_sys::window()
            .map(|w| !w.document().map(|d| d.hidden()).unwrap_or(false))
            .unwrap_or(true);
        inner.reduced = web_sys::window()
            .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
            .map(|m| m.matches())
            .unwrap_or(false);
    }
    exec(handle, Step::Base);
    let should_schedule = {
        let inner = handle.borrow();
        inner.page_visible && !inner.reduced && inner.intersecting
    };
    if should_schedule {
        later(handle, random_in(1200, 2800), Step::Cycle);
    }
}

/// `startFaceMotion`（真源 :28-177）。返回清理函数（真源 :171-177 的契约）：
/// 清计时器、断观察器、摘监听、复位基础表情。
pub fn start_face_motion(face: &web_sys::Element, state: FaceState) -> Box<dyn FnOnce()> {
    let inner = Rc::new(RefCell::new(Inner {
        face: face.clone(),
        state,
        timer: None,
        next: None,
        rounds: 0,
        looking_left: false,
        previous: None,
        page_visible: true,
        reduced: false,
        intersecting: true,
        restart_listeners: Vec::new(),
        observer: None,
    }));

    // visibilitychange / media change → restart（真源 :168-169）。
    let mut listeners: Vec<ListenerGuard> = Vec::new();
    if let Some(w) = web_sys::window() {
        if let Some(doc) = w.document() {
            let doc_target: web_sys::EventTarget = doc.into();
            let restart_handle = inner.clone();
            let cb: Closure<dyn Fn()> = Closure::new(move || restart(&restart_handle));
            let _ = doc_target.add_event_listener_with_callback(
                "visibilitychange",
                cb.as_ref().unchecked_ref(),
            );
            listeners.push(ListenerGuard {
                target: doc_target,
                kind: "visibilitychange",
                callback: cb,
            });
        }
        if let Ok(Some(media)) = w.match_media("(prefers-reduced-motion: reduce)") {
            if let Some(target) = media.dyn_ref::<web_sys::EventTarget>().cloned() {
                let restart_handle = inner.clone();
                let cb: Closure<dyn Fn()> = Closure::new(move || restart(&restart_handle));
                let _ =
                    target.add_event_listener_with_callback("change", cb.as_ref().unchecked_ref());
                listeners.push(ListenerGuard {
                    target,
                    kind: "change",
                    callback: cb,
                });
            }
        }
    }
    inner.borrow_mut().restart_listeners = listeners;

    // IntersectionObserver（真源 :157-166）：离开画面就停，回来就重启。
    if web_sys::window().is_some() {
        let observe_handle = inner.clone();
        let face_for_observe = face.clone();
        let cb: Closure<dyn FnMut(Vec<web_sys::IntersectionObserverEntry>)> =
            Closure::new(move |entries: Vec<web_sys::IntersectionObserverEntry>| {
            let next = entries
                .first()
                .map(|entry| entry.is_intersecting())
                .unwrap_or(true);
            let changed = observe_handle.borrow().intersecting != next;
            if changed {
                observe_handle.borrow_mut().intersecting = next;
                restart(&observe_handle);
            }
        });
        if let Ok(observer) =
            web_sys::IntersectionObserver::new(cb.as_ref().unchecked_ref())
        {
            observer.observe(&face_for_observe);
            inner.borrow_mut().observer = Some(observer);
            // observer 存进 inner，回调闭包随它活到清理。
            std::mem::forget(cb);
        } else {
            drop(cb);
        }
    }

    restart(&inner);

    let cleanup_handle = inner.clone();
    Box::new(move || {
        let face = cleanup_handle.borrow().face.clone();
        {
            let mut inner = cleanup_handle.borrow_mut();
            if let Some(old) = inner.timer.take() {
                if let Some(w) = web_sys::window() {
                    let _ = w.clear_timeout_with_handle(old);
                }
            }
            inner.next = None;
            if let Some(observer) = inner.observer.take() {
                observer.disconnect();
            }
            inner.restart_listeners.clear();
        }
        // 复位基础表情（真源 :176 base()）。
        let state = state;
        set_expression(&face, base_expression(state));
        motion(&face, "idle");
        set_style(&face, "--wf-face-x", "0px");
        set_style(
            &face,
            "--wf-face-y",
            if state == FaceState::Sad { "0.3px" } else { "0px" },
        );
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_state_maps_status_the_same_way() {
        // 真源 :5-13 —— running → scanning、done → content、failed → sad、其余 waiting。
        assert_eq!(face_state(Some(StepRunStatus::Running)), FaceState::Scanning);
        assert_eq!(face_state(Some(StepRunStatus::Done)), FaceState::Content);
        assert_eq!(face_state(Some(StepRunStatus::Failed)), FaceState::Sad);
        assert_eq!(face_state(Some(StepRunStatus::Pending)), FaceState::Waiting);
        assert_eq!(face_state(None), FaceState::Waiting);
    }

    #[test]
    fn base_expression_table() {
        // 真源 :14-19。
        assert_eq!(base_expression(FaceState::Waiting), EyeExpression::Dots);
        assert_eq!(base_expression(FaceState::Scanning), EyeExpression::Pill);
        assert_eq!(base_expression(FaceState::Content), EyeExpression::Happy);
        assert_eq!(base_expression(FaceState::Sad), EyeExpression::Sad);
    }

    #[test]
    fn special_pool_only_for_scanning() {
        // 真源 :20-25 —— 只有 scanning 有偶发表情（focused / confused）。
        assert_eq!(special(FaceState::Scanning).len(), 2);
        assert!(special(FaceState::Waiting).is_empty());
        assert!(special(FaceState::Content).is_empty());
        assert!(special(FaceState::Sad).is_empty());
    }

    #[test]
    fn pause_ranges_match_truth_source() {
        // 真源 :48-60 的 random(min, max) 区间。
        assert_eq!(pause_range(FaceState::Waiting), (1500, 3000));
        assert_eq!(pause_range(FaceState::Scanning), (1500, 3500));
        assert_eq!(pause_range(FaceState::Sad), (1800, 4000));
        assert_eq!(pause_range(FaceState::Content), (6000, 10000));
    }

    #[test]
    fn expression_strings_are_wire_values() {
        // data-expression 的取值要与 CSS 选择器对上。
        assert_eq!(EyeExpression::Dots.as_str(), "dots");
        assert_eq!(EyeExpression::Confused.as_str(), "confused");
        assert_eq!(EyeExpression::Happy.as_str(), "happy");
    }
}
