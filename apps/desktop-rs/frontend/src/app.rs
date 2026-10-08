//! 桌面端应用壳 + 对话工作区。
//!
//! 布局 1:1 对齐 `packages/ui/src/app-shell/WorkspaceShellLayout.tsx` 的骨架：
//! DesktopWindowFrame → [sidebar | separator | content(header + main)]。
//! DOM 结构与 Tailwind 类名照抄 React 版，仅实现语言换成 Leptos。
//! 对话数据面：session/list → session/messages → session/send（v4 sendText 迁移待后续）。
//! 刷新驱动：主进程经 `agent://notification` 转发 agent 通知，state.updated 触发
//! 会话列表与当前会话消息自动刷新。

use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::Value;
use wasm_bindgen::prelude::*;

pub use crate::file_tree::FileTreePanel;

#[wasm_bindgen]
extern "C" {
    // Tauri 2 在 WebView 中注入 __TAURI_INTERNALS__，wasm 侧经此调用 Rust command。
    #[wasm_bindgen(js_namespace = ["window", "__TAURI_INTERNALS__"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

pub async fn invoke_json(cmd: &str, args: Value) -> Result<Value, String> {
    let args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let raw = invoke(cmd, args).await.map_err(js_err_text)?;
    serde_wasm_bindgen::from_value(raw).map_err(|e| e.to_string())
}

fn js_err_text(e: JsValue) -> String {
    e.as_string().unwrap_or_else(|| format!("{e:?}"))
}

// ---------------------------------------------------------------------------
// 协议视图模型（字段对齐 zcode-protocol legacy-types）
// ---------------------------------------------------------------------------

/// session/list 返回的会话行（zcodeSessionInfoSchema，legacy-types 146-166 行）。
/// 部分字段当前视图未消费，保留完整 wire 结构供后续视图直接使用。
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_id: String,
    pub title: String,
    pub status: String,
    pub session_kind: String,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub archived_at: Option<i64>,
    /// workspace ref（legacy-types 45-52），取 workspacePath 做文件树根。
    #[serde(default)]
    pub workspace: Option<Value>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionListResult {
    sessions: Vec<SessionInfo>,
}

/// 部分字段（has_more/时间戳）保留完整 wire 结构供分页与时间显示使用。
#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RowsRangeResult {
    #[serde(default)]
    rows: Vec<ConversationRowView>,
    #[serde(default)]
    has_more: bool,
}

/// v4 会话行（rows.ts rowBaseFields + 各 kind 字段的宽松视图）。
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationRowView {
    #[serde(default)]
    pub row_id: i64,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub created_at: Option<Value>,
    #[serde(default)]
    pub duration_ms: Option<i64>,
    #[serde(default)]
    pub origin: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    /// toolCall 行
    #[serde(default)]
    pub tool_name: Option<String>,
    /// 工具调用协议 id（ToolCallBlock 的 data-tool-call-id / 渲染分流键）。
    #[serde(default)]
    pub tool_call_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    /// 工具入参的 JSON 投影（agent 已解析时随行给出；流式中可能缺席）。
    #[serde(default)]
    pub input: Option<Value>,
    /// 工具入参的原始文本（流式半截也在）。
    #[serde(default)]
    pub input_text: Option<String>,
    /// 工具输出（{ text, display?, truncated? } 的宽松视图）。
    #[serde(default)]
    pub output: Option<Value>,
    /// 顶层装饰载荷（旧 Node REPL 图片通道；CUA 展示在 output.display）。
    #[serde(default)]
    pub display: Option<Value>,
    /// CUA 工具的应用身份（{ pid, name, bundleId? }）。
    #[serde(default)]
    pub cua_app: Option<Value>,
    /// 运行起始时刻（毫秒时间戳）。
    #[serde(default)]
    pub started_at: Option<Value>,
    #[serde(default)]
    pub error: Option<Value>,
    /// userInput 行：引擎尾注起点（正文 = text[..epilogueStart]）
    #[serde(default)]
    pub epilogue_start: Option<usize>,
    /// turnHeader 行
    #[serde(default)]
    pub turn_id: Option<String>,
    /// subagent 行的父工具调用 id（Agent 工具行配对用；真源 SubagentRow.parentToolCallId）。
    #[serde(default)]
    pub parent_tool_call_id: Option<String>,
    /// subagent 行的子代理类型（配对时作 authoritativeAgentType 注入 Agent 卡）。
    #[serde(default)]
    pub subagent_type: Option<String>,
    /// subagent 行的子会话 id（打开右侧 tab 用；Rust 侧未迁会话视图，暂存）。
    #[serde(default)]
    pub child_session_id: Option<String>,
    /// artifact 行
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub artifact_type: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<i64>,
}

/// 会话状态徽标配色（zcodeSessionStatusSchema 六态）。
/// 配色语义对齐 run-status-presentation.ts 的 STATUS_DOT：
/// running/waiting/paused 用活动色 warning（pulse），error 用 destructive，
/// completed 用 success，idle 空环（bg-transparent + subtlest 描边）。
pub fn status_dot_class(status: &str) -> &'static str {
    match status {
        "running" => "bg-warning animate-pulse motion-reduce:animate-none",
        "waiting" | "paused" => "bg-warning",
        "error" => "bg-destructive ring-2 ring-destructive/30",
        "completed" => "bg-success",
        _ => "border-[1.5px] border-foreground-subtlest bg-transparent",
    }
}

pub fn relative_time(ms: i64) -> String {
    let now = js_sys::Date::now() as i64;
    let diff = (now - ms).max(0) / 1000;
    match diff {
        s if s < 60 => format!("{s} 秒前"),
        m if m < 3600 => format!("{m} 分钟前"),
        h if h < 86400 => format!("{h} 小时前"),
        d => format!("{d} 天前"),
    }
}

/// markdown → HTML（模型回复渲染）。
/// 安全处理：模型输出不可信，原始 HTML 事件一律转义为文本，
/// 仅保留 markdown 结构标记（pulldown-cmark 对 Text 事件自动转义）。
pub fn render_markdown(src: &str) -> String {
    use pulldown_cmark::{Event, Options, Parser};
    let options =
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(src, options).map(|event| match event {
        Event::Html(h) => Event::Text(h),
        Event::InlineHtml(h) => Event::Text(h),
        other => other,
    });
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, parser);
    decorate_code_blocks(out)
}

/// 代码块语言标签：把 `<pre><code class="language-x">` 的语言名挂到 pre 的
/// data-lang 属性，由 CSS ::before 展示（不做 DOM 结构改写，避免嵌套闭合问题）。
fn decorate_code_blocks(mut html: String) -> String {
    let marker = "<pre><code class=\"language-";
    loop {
        let Some(idx) = html.find(marker) else { break };
        let after = &html[idx + marker.len()..];
        let Some(end) = after.find('"') else { break };
        let lang = after[..end].to_string();
        let replaced = format!("<pre data-lang=\"{lang}\"><code class=\"language-{lang}\"");
        html = format!("{}{}{}", &html[..idx], replaced, &after[end..]);
    }
    html
}

// ---------------------------------------------------------------------------
// 全局状态（context）
// ---------------------------------------------------------------------------

/// Agent 启动成功后递增：驱动会话列表首次加载。
#[derive(Clone, Copy)]
struct SessionRefreshTrigger(RwSignal<u64>);

/// agent 通知驱动的刷新：会话列表 + 当前消息。
#[derive(Clone, Copy)]
struct DataRefreshTriggers {
    sessions: RwSignal<u64>,
    messages: RwSignal<u64>,
}

/// 全局会话列表（侧栏与 header 共享）。
#[derive(Clone, Copy)]
pub struct SessionsStore {
    sessions: RwSignal<Vec<SessionInfo>>,
    loading: RwSignal<bool>,
    error: RwSignal<String>,
    /// 当前视图（对齐 taskViewMode 概念的轻量版）。
    view: RwSignal<&'static str>,
}

/// 任务视图定义。归档视图走 includeArchived 通道单独拉取。
/// 侧栏任务视图（真源 `WorkspaceSidebar.tsx:203` SidebarTaskViewMode：
/// "grouped" | "workspace" | "timeline" | "archived" | "agents"）。
///
/// 标签显示用中文，语义与真源枚举一一对应——不自造"分组"这类真源没有的名字。
const TASK_VIEWS: [&str; 5] = ["分组", "工作区", "时间线", "归档", "智能体"];

/// 视图 → SessionsStore.view 的中文标签（store内部按标签过滤）。
const VIEW_GROUPED: &str = "分组";
const VIEW_WORKSPACE: &str = "工作区";
const VIEW_ARCHIVED: &str = "归档";

impl SessionsStore {
    fn load(&self) {
        let include_archived = self.view.get_untracked() == VIEW_ARCHIVED;
        self.loading.set(true);
        self.error.set(String::new());
        let store = *self;
        spawn_local(async move {
            match invoke_json(
                "agent_request",
                serde_json::json!({
                    "method": "session/list",
                    "params": { "includeArchived": include_archived, "limit": 50 }
                }),
            )
            .await
            {
                Ok(v) => match serde_json::from_value::<SessionListResult>(v) {
                    Ok(mut list) => {
                        if include_archived {
                            // 归档视图只看 archivedAt 非空的行。
                            list.sessions.retain(|s| s.archived_at.is_some());
                        } else {
                            list.sessions.retain(|s| s.archived_at.is_none());
                        }
                        store.sessions.set(list.sessions);
                        store.error.set(String::new());
                    }
                    Err(e) => store.error.set(format!("解析失败: {e}")),
                },
                Err(e) => store.error.set(e),
            }
            store.loading.set(false);
        });
    }

    /// 按当前视图过滤会话（归档视图已在 load 时过滤）。
    fn visible(&self) -> Vec<SessionInfo> {
        // 分组视图的数据源是任务索引库（GroupedTasksView 自己拉），
        // 这里返回空即可——会话列表不在该视图渲染。
        if self.view.get() == VIEW_GROUPED {
            return Vec::new();
        }
        self.sessions
            .get()
            .into_iter()
            .filter(|s| {
                // 「工作区」视图排除出错任务（真源 workspace 视图口径）。
                self.view.get() != VIEW_WORKSPACE || s.status != "error"
            })
            .collect()
    }

    fn title_of(&self, session_id: &str) -> Option<String> {
        self.sessions
            .get_untracked()
            .iter()
            .find(|s| s.session_id == session_id)
            .map(|s| s.title.clone())
    }

    fn status_of(&self, session_id: &str) -> Option<String> {
        self.sessions
            .get_untracked()
            .iter()
            .find(|s| s.session_id == session_id)
            .map(|s| s.status.clone())
    }

    pub fn workspace_path_of(&self, session_id: &str) -> Option<String> {
        self.sessions
            .get_untracked()
            .iter()
            .find(|s| s.session_id == session_id)
            .and_then(|s| s.workspace.as_ref())
            .and_then(|w| w["workspacePath"].as_str())
            .map(|p| p.to_string())
    }

    /// 全部会话快照（分组视图要 join 任务标题/状态，真源走 sessions-index join）。
    pub fn all(&self) -> Vec<SessionInfo> {
        self.sessions.get_untracked()
    }

    /// 会话列表投影成`groupedTasks::join::SessionRow`（join 层的输入）。
    ///
    /// `workspace` 在 wire 上是宽松 JSON（可能缺失或字段不全），这里按
    /// `workspacePath` / `workspaceIdentity` 两个标准键提取，与
    /// `workspace_path_of` 同一口径。**identity 缺失时保持 `None`**，绝不用
    /// path顶替——那会让远程 workspace 的 taskKey 算错。
    pub fn join_rows(&self) -> Vec<crate::groupedTasks::join::SessionRow> {
        self.sessions
            .get_untracked()
            .iter()
            .map(|s| {
                let ws = s.workspace.as_ref();
                crate::groupedTasks::join::SessionRow {
                    session_id: s.session_id.clone(),
                    title: s.title.clone(),
                    status: s.status.clone(),
                    created_at: s.created_at,
                    updated_at: s.updated_at,
                    workspace_path: ws
                        .and_then(|w| w["workspacePath"].as_str())
                        .unwrap_or_default()
                        .to_string(),
                    workspace_identity: ws
                        .and_then(|w| w["workspaceIdentity"].as_str())
                        .map(|s| s.to_string()),
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// 根组件
// ---------------------------------------------------------------------------

#[component]
pub fn App() -> impl IntoView {
    let session_refresh: RwSignal<u64> = RwSignal::new(0);
    provide_context(SessionRefreshTrigger(session_refresh));

    let selected_session: RwSignal<Option<String>> = RwSignal::new(None);
    provide_context(selected_session);

    let store = SessionsStore {
        sessions: RwSignal::new(Vec::new()),
        loading: RwSignal::new(false),
        error: RwSignal::new(String::new()),
        view: RwSignal::new("全部"),
    };
    provide_context(store);

    let triggers = DataRefreshTriggers {
        sessions: session_refresh,
        messages: RwSignal::new(0),
    };
    provide_context(triggers);

    // 命令中心（⌘K / Ctrl+K 唤起，对齐 WorkspaceSidebar 的 onOpenCommandCenter）。
    let show_command_center: RwSignal<bool> = RwSignal::new(false);
    provide_context(show_command_center);
    {
        let show = show_command_center;
        let handler =
            Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
                if (e.meta_key() || e.ctrl_key()) && e.key() == "k" {
                    e.prevent_default();
                    show.update(|v| *v = !*v);
                }
                if e.key() == "Escape" {
                    show.set(false);
                }
            });
        web_sys::window().map(|w| {
            w.add_event_listener_with_callback(
                "keydown",
                handler.as_ref().unchecked_ref::<js_sys::Function>(),
            )
            .ok();
        });
        handler.forget();
    }

    // 文件树滑出面板开关（对齐真实侧栏行内的 onOpenFileTree）。
    let show_file_tree: RwSignal<bool> = RwSignal::new(false);
    provide_context(show_file_tree);

    // 逐token 流式累积器：v4 conversation 帧只累积 row.delta，不整页重拉。
    let stream_store = crate::stream::StreamStore::new();
    provide_context(stream_store);

    // 全局 toast 队列（真源 toast.tsx）。挂在 App 根部供各视图 expect_context。
    let toast_store = crate::toast::ToastStore::default();
    provide_context(toast_store.clone());

    // agent 通知桥（主进程 emit `agent://notification`）：
    // - v4/conversation/frame → 喂给流式累积器，只有结构变化才触发重拉
    // - session/event / state.updated → 刷新会话列表与消息
    // 其余通知方法（遥测/存储启动等）与视图无关，忽略。
    spawn_local(async move {
        let handler = Closure::<dyn FnMut(JsValue)>::new(move |_event: JsValue| {
            // 事件 payload: { payload: ProtocolNotification }。
            let payload = js_sys::Reflect::get(&_event, &"payload".into())
                .ok()
                .and_then(|v| serde_wasm_bindgen::from_value::<Value>(v).ok());
            let Some(note) = payload else { return };
            let method = note["method"].as_str().unwrap_or("");
            match method {
                "v4/conversation/frame" => {
                    // 帧的 topic 决定归属会话；累积后仅结构变化才重拉，
                    // 纯文本增量靠 ChatView 的流式覆盖渲染（逐 token 的关键）。
                    let Some(params) = note["params"].as_object() else {
                        return;
                    };
                    let topic = params
                        .get("topic")
                        .and_then(|t| t.as_str())
                        .unwrap_or_default();
                    let Some(session_id) = topic.strip_prefix("conversation/") else {
                        return;
                    };
                    match stream_store.feed(session_id, &note["params"]) {
                        // 纯文本增量：不重拉，ChatView 直接读累积文本渲染。
                        crate::stream::FrameOutcome::TextDeltas { .. } => {
                            triggers.messages.update(|n| *n += 1);
                        }
                        // 结构变化 / 快照：需要重拉 rowsRange 拿权威结构。
                        crate::stream::FrameOutcome::Structural
                        | crate::stream::FrameOutcome::Snapshot => {
                            triggers.messages.update(|n| *n += 1);
                            triggers.sessions.update(|n| *n += 1);
                        }
                        // 分片帧本轮未实现重组：退回 session 事件流兜底。
                        crate::stream::FrameOutcome::NeedsResync => {
                            triggers.messages.update(|n| *n += 1);
                        }
                        crate::stream::FrameOutcome::Ignored => {}
                    }
                }
                // legacy 订阅事件流（每次 turn 事件一条）与状态变更。
                "session/event" | "state.updated" => {
                    triggers.sessions.update(|n| *n += 1);
                    triggers.messages.update(|n| *n += 1);
                }
                _ => {}
            }
        });

        // handler 是 JsValue（函数），无法走 serde 序列化，手工构造 invoke 参数。
        let args = js_sys::Object::new();
        let target = js_sys::Object::new();
        js_sys::Reflect::set(&args, &"event".into(), &"agent://notification".into()).ok();
        js_sys::Reflect::set(&args, &"kind".into(), &"Any".into()).ok();
        js_sys::Reflect::set(&target, &"kind".into(), &"Any".into()).ok();
        js_sys::Reflect::set(&args, &"target".into(), &target.into()).ok();
        js_sys::Reflect::set(&args, &"handler".into(), handler.as_ref()).ok();

        // 订阅失败不致命（退化为手动刷新），只打控制台。
        let subscribed = invoke("plugin:event|listen", args.into()).await;
        if subscribed.is_err() {
            web_sys::console::error_1(&"agent 通知订阅失败，将退化为手动刷新".into());
        }
        // Closure 生命周期与监听器一致，泄漏即常驻。
        handler.forget();
    });

    view! {
        <div class="relative flex h-full min-h-0 w-full overflow-hidden">
            <SidebarPanel />
            <SidebarSeparator />
            <ContentPanel />
            <CommandCenter show=show_command_center />
            // toast 浮层挂在最外层：真源 z-[9999]，须盖住所有面板。
            <crate::toast::Toaster store=toast_store />
        </div>
    }
}

/// 命令中心：⌘K 快速搜索并跳转任务。
/// 对齐真实产品 command-center 目录的能力面（会话跳转先行，命令执行后续接入）。
#[component]
fn CommandCenter(show: RwSignal<bool>) -> impl IntoView {
    let (query, set_query) = signal(String::new());
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let store = expect_context::<SessionsStore>();

    let results = move || {
        let q = query.get().to_lowercase();
        store
            .sessions
            .get()
            .into_iter()
            .filter(|s| q.is_empty() || s.title.to_lowercase().contains(&q))
            .take(8)
            .collect::<Vec<_>>()
    };

    view! {
        {move || {
            if !show.get() {
                return ().into_any();
            }
            let open = Callback::new(move |id: String| {
                selected_session.set(Some(id));
                show.set(false);
                set_query.set(String::new());
            });
            view! {
                // 覆盖层：点击遮罩关闭。
                <div
                    class="fixed inset-0 z-50 flex items-start justify-center bg-black/30 pt-[12vh]"
                    on:click=move |_| show.set(false)
                >
                    <div
                        class="w-[520px] max-w-[90%] overflow-hidden rounded-xl border border-border bg-panel shadow-xl"
                        on:click=move |e| e.stop_propagation()
                    >
                        <input
                            class="w-full border-b border-border bg-transparent px-4 py-3 text-sm outline-none"
                            placeholder="搜索任务…"
                            autofocus=true
                            prop:value=move || query.get()
                            on:input=move |e| set_query.set(event_target_value(&e))
                        />
                        <div class="max-h-[320px] overflow-y-auto p-1">
                            {move || {
                                let list = results();
                                if list.is_empty() {
                                    return view! { <p class="px-3 py-2 text-sm text-muted">"无匹配任务"</p> }.into_any();
                                }
                                list.iter()
                                    .map(|s| {
                                        let sid = s.session_id.clone();
                                        let sid_active = s.session_id.clone();
                                        let title = s.title.clone();
                                        let status = s.status.clone();
                                        let dot = status_dot_class(&status).to_string();
                                        let is_active = move || {
                                            selected_session.get().as_deref()
                                                == Some(sid_active.as_str())
                                        };
                                        let row_class = move || {
                                            if is_active() {
                                                "flex w-full items-center gap-2 rounded-lg bg-selected px-3 py-2 text-left text-sm"
                                            } else {
                                                "flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm hover:bg-surface-hover"
                                            }
                                        };
                                        view! {
                                            <button class=row_class on:click=move |_| open.run(sid.clone())>
                                                <span class=format!("size-1.5 flex-none rounded-full {dot}")></span>
                                                <span class="min-w-0 flex-1 truncate">{title}</span>
                                                <span class="flex-none text-[11px] text-muted">{status}</span>
                                            </button>
                                        }
                                    })
                                    .collect_view()
                                    .into_any()
                            }}
                        </div>
                    </div>
                </div>
            }
            .into_any()
        }}
    }
}

// ---------------------------------------------------------------------------
// 侧栏
// ---------------------------------------------------------------------------

/// lucide 图标内联渲染（path 数据取自 node_modules/lucide-react/dist/esm/icons,
/// 与 React 版完全同源；stroke 规则统一 lucide 默认 2px round）。
///
/// 部分图标含 `rect` 节点（archive-x 等），故额外接收矩形列表。
#[component]
pub fn Icon(
    paths: Vec<&'static str>,
    circles: Vec<(&'static str, &'static str, &'static str)>,
    #[prop(optional)] rects: Vec<(&'static str, &'static str, &'static str, &'static str)>,
) -> impl IntoView {
    view! {
        <svg
            class="size-4 flex-none"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            {rects.into_iter()
                .map(|(x, y, w, h)| view! { <rect x=x y=y width=w height=h rx="1" /> })
                .collect_view()}
            {circles.into_iter()
                .map(|(cx, cy, r)| view! { <circle cx=cx cy=cy r=r /> })
                .collect_view()}
            {paths.into_iter()
                .map(|d| view! { <path d=d /> })
                .collect_view()}
        </svg>
    }
}

/// lucide "search"。
#[component]
fn IconSearch() -> impl IntoView {
    view! {
        <Icon
            paths=vec!["m21 21-4.34-4.34"]
            circles=vec![("11", "11", "8")]
        />
    }
}

/// lucide "calendar-clock"。
#[component]
fn IconCalendarClock() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M16 14v2.2l1.6 1",
                "M16 2v4",
                "M21 7.5V6a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h3.5",
                "M3 10h5",
                "M8 2v4",
            ]
            circles=vec![("16", "16", "3")]
        />
    }
}

/// lucide "blocks"。
#[component]
fn IconBlocks() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M10 22V7a1 1 0 0 0-1-1H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-5a1 1 0 0 0-1-1H2",
                "M14 2h6a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1h-6a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Z",
            ]
            circles=vec![]
        />
    }
}

/// lucide "shapes"。
#[component]
fn IconShapes() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M8.3 10a.7.7 0 0 1-.626-1.079L11.4 3a.7.7 0 0 1 1.198-.043L16.3 8.9a.7.7 0 0 1-.572 1.1Z",
                "M14.5 21a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1Z",
                "M3.5 21a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1Z",
            ]
            circles=vec![]
        />
    }
}

/// 侧栏面板（#sidebar）。
/// 结构 1:1 对齐 WorkspaceSidebar.tsx 1756-1940 行：
/// 顶部拖拽区 → 操作区（新任务/命令中心/自动化/插件商店/资产库）→ 任务区（工具条 + 列表）。
#[component]
fn SidebarPanel() -> impl IntoView {
    view! {
        <div
            id="sidebar"
            class="w-[240px] max-w-[50%] flex-none overflow-hidden duration-200 ease-out transition-[width,opacity]"
        >
            <aside class="flex h-full flex-col overflow-hidden bg-panel">
                // 顶部窗口拖拽区（desktop 壳；Tauri drag-region 在窗口框迁移时接入）。
                <div class="h-6"></div>
                <div class="relative min-h-0 flex-1 overflow-hidden">
                    <div class="absolute inset-0 flex min-h-0 flex-col">
                        <div class="flex flex-col gap-1 px-2 py-3">
                            <NewTaskButton />
                            {let show = expect_context::<RwSignal<bool>>();
                            view! {
                                <SidebarGhostButton
                                    icon=view! { <IconSearch /> }
                                    label="命令中心"
                                    shortcut="⌘K"
                                    on_click=Callback::new(move |_| show.set(true))
                                />
                            }}
                            <SidebarGhostButton icon=view! { <IconCalendarClock /> } label="自动化" />
                            <SidebarGhostButton icon=view! { <IconBlocks /> } label="插件商店" />
                            <SidebarGhostButton icon=view! { <IconShapes /> } label="资产库" />
                        </div>
                        <div class="relative flex min-h-0 flex-1 flex-col">
                            <div class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto">
                                <TaskViewToolbar />
                                // 各视图整块替换任务区（真源 WorkspaceSidebar.tsx:1901-1940
                                // 的 archived / grouped / timeline 分支结构）：
                                // 归档→归档区、分组→分组视图、其余→置顶区+会话列表。
                                {move || {
                                    let store = expect_context::<SessionsStore>();
                                    let view = store.view.get();
                                    if view == VIEW_ARCHIVED {
                                        view! { <crate::archived::ArchivedTasksSection /> }.into_any()
                                    } else if view == VIEW_GROUPED {
                                        view! { <crate::grouped::GroupedTasksView /> }.into_any()
                                    } else {
                                        view! {
                                            <crate::pinned::PinnedTasksSection />
                                            <SidebarSessionList />
                                        }
                                        .into_any()
                                    }
                                }}
                            </div>
                        </div>
                    </div>
                </div>
                <AgentPanel />
            </aside>
        </div>
    }
}

/// ghost 大按钮（对齐 Button variant=ghost size=lg 的类名组合）。
#[component]
fn SidebarGhostButton(
    icon: impl IntoView + 'static,
    label: &'static str,
    #[prop(optional)] shortcut: Option<&'static str>,
    #[prop(optional)] on_click: Option<Callback<()>>,
) -> impl IntoView {
    let handle = move |_| {
        if let Some(cb) = on_click {
            cb.run(());
        }
    };
    view! {
        <button
            class="flex w-full items-center justify-start gap-2 rounded-lg px-3 py-2 text-sm text-foreground hover:bg-surface-hover"
            on:click=handle
        >
            {icon}
            <span class="min-w-0 flex-1 truncate text-left">{label}</span>
            {shortcut.map(|s| {
                view! { <span class="ml-auto flex-none text-xs font-normal text-foreground-subtlest">{s}</span> }
            })}
        </button>
    }
}

/// 任务视图切换工具条（对齐 workspaceTaskToolbar 的视图切换机制；
/// 真实产品的自定义分组（员工/工作流组）语义待后续对齐）。
#[component]
fn TaskViewToolbar() -> impl IntoView {
    let store = expect_context::<SessionsStore>();
    view! {
        <div class="flex flex-none flex-wrap items-center gap-1 px-3 pt-1">
            {TASK_VIEWS
                .iter()
                .map(|v| {
                    let view_label = *v;
                    let is_active = move || store.view.get() == view_label;
                    let class = move || {
                        if is_active() {
                            "rounded bg-selected px-1.5 py-0.5 text-xs font-medium text-foreground"
                        } else {
                            "rounded px-1.5 py-0.5 text-xs text-muted hover:bg-surface-hover"
                        }
                    };
                    view! {
                        <button
                            class=class
                            on:click=move |_| {
                                store.view.set(view_label);
                                store.load();
                            }
                        >
                            {view_label}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn SidebarSeparator() -> impl IntoView {
    view! {
        <div
            role="separator"
            aria-controls="sidebar"
            aria-orientation="vertical"
            data-testid="resizable-handle"
            class="group/handle relative z-10 flex h-full w-1 shrink-0 touch-none cursor-ew-resize items-center justify-center bg-transparent outline-none"
        />
    }
}

/// 侧栏任务列表：session/list 真实数据（全局 store 共享）。
#[component]
fn SidebarSessionList() -> impl IntoView {
    let store = expect_context::<SessionsStore>();
    let triggers = expect_context::<DataRefreshTriggers>();

    // Agent 启动（SessionRefreshTrigger）与 agent 通知（state.updated）都触发刷新。
    Effect::new(move |_| {
        let started = expect_context::<SessionRefreshTrigger>().0.get();
        let notified = triggers.sessions.get();
        if started > 0 || notified > 0 {
            store.load();
        }
    });

    view! {
        <div class="flex min-h-0 flex-1 flex-col">
            <div class="flex items-center justify-between px-3 pb-1">
                <span class="text-xs font-medium text-muted">"任务"</span>
                <button
                    class="rounded px-1.5 py-0.5 text-xs text-muted hover:bg-selected"
                    on:click=move |_| store.load()
                    disabled=move || store.loading.get()
                    title="从 Agent 拉取 session/list"
                >
                    {move || if store.loading.get() { "加载中…" } else { "刷新" }}
                </button>
            </div>
            <div class="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
                {move || {
                    if !store.error.get().is_empty() {
                        return view! {
                            <p class="px-2 py-1 text-xs text-[#dc2626]">{move || store.error.get()}</p>
                        }.into_any();
                    }
                    let list = store.visible();
                    if list.is_empty() {
                        return view! {
                            <p class="px-2 py-1 text-xs text-muted">"启动 Agent 后拉取任务，或点「＋ 新任务」"</p>
                        }.into_any();
                    }
                    list.iter()
                        .map(|s| {
                            // 三处闭包各持一份：active 判断、点击选中、行内展示。
                            let session_id_for_click = s.session_id.clone();
                            let session_id_for_active = s.session_id.clone();
                            let title = s.title.clone();
                            let status = s.status.clone();
                            let time = relative_time(s.updated_at);
                            let dot = status_dot_class(&status).to_string();
                            let status_label = status.clone();
                            let selected_session = expect_context::<RwSignal<Option<String>>>();
                            let active = move || {
                                selected_session.get().as_deref()
                                    == Some(session_id_for_active.as_str())
                            };
                            let item_class = move || {
                                // WorkspaceSidebarItem 行样式同构：h-8/pl-2.5/pr-1 + hover surface-hover，
                                // 选中态 bg-selected + shadow-xl（拖拽选中的视觉）。
                                if active() {
                                    "flex h-8 w-full min-w-0 items-center gap-2 rounded-lg bg-selected pl-2.5 pr-1 text-left text-foreground shadow-xl"
                                } else {
                                    "flex h-8 w-full min-w-0 items-center gap-2 rounded-lg pl-2.5 pr-1 text-left text-foreground hover:bg-surface-hover hover:text-foreground"
                                }
                            };
                            view! {
                                <button
                                    class=item_class
                                    on:click=move |_| selected_session.set(Some(session_id_for_click.clone()))
                                >
                                    <span class=format!("size-1.5 flex-none rounded-full {dot}") title={status_label}></span>
                                    <span class="min-w-0 flex-1 truncate text-sm">{title}</span>
                                    <span class="flex-none text-[11px] text-muted">{time}</span>
                                </button>
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </div>
        </div>
    }
}

/// 新任务：session/create + WorkspaceRef（legacy-types 45-52 行）。
/// v1 workspace 固定用户主目录；真实工作区选择器按视图迁移节奏接入。
#[component]
fn NewTaskButton() -> impl IntoView {
    let (creating, set_creating) = signal(false);
    let (error, set_error) = signal(String::new());
    // 可选工作区路径：留空用主目录（真实工作区选择器后续接目录对话框）。
    let (workspace_input, set_workspace_input) = signal(String::new());

    let create = move |_| {
        if creating.get_untracked() {
            return;
        }
        set_creating.set(true);
        set_error.set(String::new());
        let trigger = expect_context::<SessionRefreshTrigger>();
        let selected = expect_context::<RwSignal<Option<String>>>();
        let custom_workspace = workspace_input.get_untracked();
        spawn_local(async move {
            // 显式输入优先，其次主进程取真实主目录。
            let home = if custom_workspace.trim().is_empty() {
                match invoke_json("get_home", serde_json::json!({})).await {
                    Ok(v) => v["home"].as_str().unwrap_or(".").to_string(),
                    Err(_) => ".".to_string(),
                }
            } else {
                custom_workspace.trim().to_string()
            };
            // v4 createSession 主路径（v4 会话表登记，sendText 外键依赖此）。
            let result = invoke_json(
                "agent_create_session_v4",
                serde_json::json!({ "workspace": home }),
            )
            .await;
            match result {
                Ok(v) => {
                    let session_id = v["sessionId"].as_str().map(|s| s.to_string());
                    match session_id {
                        Some(id) => {
                            // 刷新列表并直接选中新会话。
                            trigger.0.update(|n| *n += 1);
                            selected.set(Some(id));
                        }
                        None => set_error.set("v4 createSession 未返回 sessionId".into()),
                    }
                }
                Err(e) => set_error.set(format!("创建失败: {e}")),
            }
            set_creating.set(false);
        });
    };

    view! {
        <div class="w-full">
            <input
                class="mb-1 w-full rounded-lg border border-border bg-panel px-2 py-1.5 text-xs outline-none focus:border-border-hover"
                placeholder="工作区路径（留空用主目录）"
                prop:value=move || workspace_input.get()
                on:input=move |e| set_workspace_input.set(event_target_value(&e))
            />
            <button
                class="group flex w-full h-8 items-center justify-start gap-2 rounded-lg pl-2.5 pr-2.5 hover:bg-surface-hover disabled:cursor-not-allowed disabled:opacity-60"
                on:click=create
                disabled=move || creating.get()
            >
                {move || if creating.get() { "创建中…" } else { "＋ 新任务" }}
            </button>
            {move || {
                (!error.get().is_empty()).then(|| {
                    view! { <p class="px-1 pt-1 text-xs text-[#dc2626]">{move || error.get()}</p> }
                })
            }}
        </div>
    }
}

/// Agent 连接面板：启动/状态。
#[component]
fn AgentPanel() -> impl IntoView {
    let (agent_status, set_agent_status) = signal(String::from("未启动"));
    let (busy, set_busy) = signal(false);

    let start = move |_| {
        if busy.get_untracked() {
            return;
        }
        set_busy.set(true);
        let trigger = expect_context::<SessionRefreshTrigger>();
        spawn_local(async move {
            match invoke_json("agent_start", serde_json::json!({})).await {
                Ok(v) => {
                    let pid = v["pid"]
                        .as_u64()
                        .map(|p| p.to_string())
                        .unwrap_or("?".into());
                    set_agent_status.set(format!("已连接（pid {pid}）"));
                    // 启动成功后通知任务列表自动刷新。
                    trigger.0.update(|n| *n += 1);
                }
                Err(e) => set_agent_status.set(format!("启动失败：{e}")),
            }
            set_busy.set(false);
        });
    };

    // 连接状态轮询：agent 进程退出/崩溃时自动反映（重连按钮化体验）。
    {
        let set_agent_status = set_agent_status.clone();
        spawn_local(async move {
            loop {
                browser_wait(5000).await;
                if let Ok(v) = invoke_json("agent_status", serde_json::json!({})).await {
                    let running = v["running"].as_bool().unwrap_or(false);
                    let connected = v["connected"].as_bool().unwrap_or(false);
                    if running && connected {
                        let pid = v["pid"]
                            .as_u64()
                            .map(|p| p.to_string())
                            .unwrap_or("?".into());
                        set_agent_status.set(format!("已连接（pid {pid}）"));
                    } else if running {
                        let reason = v["closedReason"].as_str().unwrap_or("未知原因").to_string();
                        set_agent_status.set(format!("连接断开：{reason}，自动重连…"));
                        // 自动重连：agent_start 幂等（旧实例断开后重新 spawn 并握手）。
                        let _ = invoke_json("agent_start", serde_json::json!({})).await;
                    } else {
                        set_agent_status.set("未启动".into());
                    }
                }
            }
        });
    }

    view! {
        <div class="border-t border-border p-3 text-xs">
            <div class="mb-2 flex items-center gap-1.5 text-muted">
                <span class="dot inline-block h-2 w-2 rounded-full bg-[#16a34a]"></span>
                <span>Agent 桥</span>
            </div>
            <div class="flex flex-col gap-1.5">
                <button class="rounded-lg bg-primary px-2 py-1.5 text-xs font-semibold text-primary-foreground disabled:opacity-60" on:click=start disabled=move || busy.get()>
                    {move || if busy.get() { "启动中…" } else { "启动 Agent" }}
                </button>
                <p class="break-all leading-relaxed text-muted">{move || agent_status.get()}</p>
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// 主工作区（#content）
// ---------------------------------------------------------------------------

/// 右侧主工作区：上方 header，下面主视图区（对话 + 可选文件树侧板）。
#[component]
fn ContentPanel() -> impl IntoView {
    let show_file_tree = expect_context::<RwSignal<bool>>();
    view! {
        <div id="content" class="flex min-w-[320px] flex-1 flex-col">
            <WorkspaceHeaderBar />
            <div class="min-h-0 flex-1 overflow-hidden">
                <main class="flex h-full min-h-0 flex-1 bg-background">
                    <section class="relative flex h-full min-h-0 flex-1 flex-col overflow-hidden rounded-l-[12px] border border-border">
                        {move || match expect_context::<RwSignal<Option<String>>>().get() {
                            Some(id) => view! { <ChatView session_id=id /> }.into_any(),
                            None => view! { <HomeView /> }.into_any(),
                        }}
                    </section>
                    {move || {
                        show_file_tree.get().then(|| view! { <FileTreePanel /> }.into_any())
                    }}
                </main>
            </div>
        </div>
    }
}

/// 顶栏（WorkspaceHeader.tsx 138-228 行同构）：
/// h-12 + [app-region:drag] 容器；task 变体 border-border/50，draft 变体 border-transparent；
/// 左侧标题区 + 右侧动作区（文件树开关；终端/侧面板切换按视图迁移节奏补齐）。
#[component]
fn WorkspaceHeaderBar() -> impl IntoView {
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let store = expect_context::<SessionsStore>();
    let show_file_tree = expect_context::<RwSignal<bool>>();
    let title = move || {
        selected_session
            .get()
            .and_then(|id| store.title_of(&id))
            .unwrap_or_else(|| "新对话".into())
    };
    let status = move || {
        selected_session
            .get()
            .and_then(|id| store.status_of(&id))
            .unwrap_or_default()
    };
    // variant：有选中会话 = task，无 = draft（draft 无左边框）。
    let variant_is_task = move || selected_session.get().is_some();
    view! {
        <header class="relative flex w-full flex-none border-b h-12"
            class:border-border=variant_is_task
            class:border-transparent=move || !variant_is_task()
        >
            <div class="flex h-12 min-w-0 flex-1 items-center justify-between gap-2 overflow-hidden p-2">
                // 左侧标题区（task 变体）。
                {move || {
                    if variant_is_task() {
                        view! {
                            <div class="flex min-w-0 items-center gap-2">
                                <span class="min-w-0 truncate text-sm font-semibold text-foreground">{title}</span>
                                <span class="flex-none text-ui-xs text-muted">{move || {
                                    let s = status();
                                    if s.is_empty() { String::new() } else { s }
                                }}</span>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div class="min-w-0 flex-1" aria-hidden="true"></div> }.into_any()
                    }
                }}
                // 右侧动作区。
                <div class="flex flex-none items-center gap-1">
                    <button
                        class="rounded px-2 py-1 text-xs text-muted hover:bg-surface-hover hover:text-foreground"
                        on:click=move |_| show_file_tree.update(|v| *v = !*v)
                        title="文件树（当前会话工作区）"
                    >
                        "文件"
                    </button>
                </div>
            </div>
        </header>
    }
}

#[component]
fn HomeView() -> impl IntoView {
    view! {
        <div class="flex h-full flex-col items-center justify-center gap-2 p-6 text-center">
            <h1 class="text-lg font-semibold">"Rust 重构工作台"</h1>
            <p class="max-w-md text-sm text-muted">
                "布局骨架 1:1 对齐 WorkspaceShellLayout；启动 Agent 后从侧栏选择任务，或新建会话开始对话。"
            </p>
        </div>
    }
}

// ---------------------------------------------------------------------------
// 对话视图
// ---------------------------------------------------------------------------

#[component]
fn ChatView(session_id: String) -> impl IntoView {
    // 三处闭包各持一份：加载、发送、生命周期跟踪。
    let sid_load = session_id.clone();
    let sid_send = session_id.clone();
    // 消息刷新守卫专用副本（闭包捕获 prop 会move）。
    let sid_stream = session_id.clone();
    let (messages, set_messages) = signal(Vec::<ConversationRowView>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(String::new());
    let (draft, set_draft) = signal(String::new());
    let (sending, set_sending) = signal(false);
    // 模型选择（自由输入版；真实 provider 清单接入后换下拉）。
    // localStorage key: zcode-model-provider / zcode-model-id。
    let (model_provider, set_model_provider) = signal(read_ls("zcode-model-provider"));
    let (model_id, set_model_id) = signal(read_ls("zcode-model-id"));
    let (show_model, set_show_model) = signal(false);

    let scroll_box = NodeRef::<html::Div>::new();

    let load_messages = Callback::new(move |_: ()| {
        set_loading.set(true);
        let sid = sid_load.clone();
        let set_messages = set_messages.clone();
        let set_loading = set_loading.clone();
        let set_error = set_error.clone();
        spawn_local(async move {
            match invoke_json(
                "agent_rows_range",
                serde_json::json!({ "sessionId": sid, "limit": 200 }),
            )
            .await
            {
                Ok(v) => match serde_json::from_value::<RowsRangeResult>(v) {
                    Ok(result) => {
                        set_messages.set(result.rows);
                        set_error.set(String::new());
                    }
                    Err(e) => set_error.set(format!("解析失败: {e}")),
                },
                Err(e) => set_error.set(e),
            }
            set_loading.set(false);
        });
    });

    // 选中即加载 + 订阅会话事件流（state.updated/frame 通知由此驱动准流式刷新）。
    let triggers = expect_context::<DataRefreshTriggers>();
    Effect::new(move |_| {
        let _ = &session_id;
        load_messages.run(());
        let sid = session_id.clone();
        spawn_local(async move {
            let _ = invoke_json(
                "agent_request",
                serde_json::json!({
                    "method": "session/subscribe",
                    "params": {
                        "sessionId": sid,
                        "deliveryKind": "desktop-continuous",
                        "includeSnapshot": false
                    }
                }),
            )
            .await;
        });
    });
    // 消息刷新：**纯文本增量不重拉**（否则逐 token 会被整页替换抹掉）。
    // 通知桥已把纯文本增量标记为「累积即可」，这里只在结构变化时重拉 rowsRange。
    let stream = expect_context::<crate::stream::StreamStore>();
    Effect::new(move |_| {
        triggers.messages.track();
        if triggers.messages.get() == 0 {
            return;
        }
        // 该会话正在流式 → 文本由累积器逐 token 渲染，不做整页重拉。
        if stream.has_streaming(&sid_stream) {
            return;
        }
        load_messages.run(());
    });
    // 发送：session/send → 轮询 session/messages 直至回复稳定（v1 轮询，事件流后续接入）。
    let send = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        if sending.get_untracked() {
            return;
        }
        let content = draft.get_untracked();
        if content.trim().is_empty() {
            return;
        }
        set_sending.set(true);
        set_draft.set(String::new());
        let sid = sid_send.clone();
        let set_messages = set_messages.clone();
        let set_sending = set_sending.clone();
        let set_error = set_error.clone();
        spawn_local(async move {
            // v4 sendText 主路径（CommandEnvelope 由主进程构造经 v4/command 提交）。
            // 显式模型选择：填了就带（会话无默认模型时 admission 必需）。
            let provider = model_provider.get_untracked();
            let mid = model_id.get_untracked();
            let model_selection = (!provider.trim().is_empty() && !mid.trim().is_empty()).then(
                || serde_json::json!({ "providerId": provider.trim(), "modelId": mid.trim() }),
            );
            let send_result = invoke_json(
                "agent_send_text",
                serde_json::json!({ "sessionId": sid, "text": content, "modelSelection": model_selection }),
            )
            .await;

            if let Err(e) = send_result {
                set_error.set(format!("发送失败: {e}"));
                set_sending.set(false);
                return;
            }

            // 轮询拉取回复：assistant 文本连续两次一致即认为本轮稳定。
            // state.updated 通知触发的自动刷新与本轮询并行生效。
            let mut last_text = String::new();
            let mut stable = 0;
            for _ in 0..20 {
                browser_wait(1200).await;
                let fetch = invoke_json(
                    "agent_rows_range",
                    serde_json::json!({ "sessionId": sid, "limit": 200 }),
                )
                .await;
                if let Ok(v) = fetch {
                    if let Ok(result) = serde_json::from_value::<RowsRangeResult>(v) {
                        let view: Vec<ConversationRowView> = result.rows;
                        let tail_text = view
                            .iter()
                            .rev()
                            .find(|m| m.kind == "assistantText")
                            .map(|m| m.text.clone())
                            .unwrap_or_default();
                        if tail_text == last_text {
                            stable += 1;
                        } else {
                            stable = 0;
                        }
                        last_text = tail_text;
                        set_messages.set(view);
                        if stable >= 2 {
                            break;
                        }
                    }
                }
            }
            // 轮询终态后再拉一次，确保与 agent 最新状态一致。
            // 复用外层 sid（再 clone），避免二次 move 把 send 闭包降级成 FnOnce。
            let sid = sid.clone();
            spawn_local(async move {
                let _ = invoke_json(
                    "agent_rows_range",
                    serde_json::json!({ "sessionId": sid, "limit": 200 }),
                )
                .await;
            });
            set_sending.set(false);
        });
    };

    // 消息变化后滚到底部并做代码高亮（window.hljsLib 由 vendor/hljs.js 提供）。
    Effect::new(move |_| {
        messages.track();
        if let Some(el) = scroll_box.get() {
            el.set_scroll_top(el.scroll_height());
            // hljs 后处理：只处理未高亮过的 code 块（data-highlighted 防重复）。
            let nodes = js_sys::Reflect::get(el.as_ref(), &"querySelectorAll".into())
                .and_then(|f| f.dyn_into::<js_sys::Function>())
                .and_then(|f| f.call1(el.as_ref(), &"pre code:not([data-highlighted])".into()));
            if let Ok(nodes) = nodes {
                let length = js_sys::Reflect::get(&nodes, &"length".into())
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u32;
                let highlight = js_sys::Reflect::get(&window_hljs(), &"highlightElement".into())
                    .and_then(|f| f.dyn_into::<js_sys::Function>());
                for i in 0..length {
                    if let Ok(node) = js_sys::Reflect::get(&nodes, &i.into()) {
                        if let Ok(elem) = node.dyn_into::<web_sys::Element>() {
                            elem.set_attribute("data-highlighted", "yes").ok();
                            if let Ok(f) = &highlight {
                                let _ = f.call1(&JsValue::NULL, &elem);
                            }
                        }
                    }
                }
            }
        }
    });

    view! {
        <div class="flex h-full min-h-0 flex-col">
            <div node_ref=scroll_box class="min-h-0 flex-1 overflow-y-auto p-4">
                {move || {
                    if loading.get() && messages.get().is_empty() {
                        return view! { <p class="text-sm text-muted">"加载消息…"</p> }.into_any();
                    }
                    if !error.get().is_empty() {
                        return view! { <p class="text-sm text-[#dc2626]">{move || error.get()}</p> }.into_any();
                    }
                    let list = messages.get();
                    if list.is_empty() {
                        return view! { <p class="text-sm text-muted">"还没有消息，发送第一条开始对话。"</p> }.into_any();
                    }
                    // 工作行分组（真源 buildAssistantWorkRenderItems）：
                    // 连续终端命令折叠成 executeGroup 卡；Agent↔subagent 配对；
                    // 其余逐行。
                    let grouping_rows: Vec<crate::conversationWorkItems::GroupingRow> = list
                        .iter()
                        .map(|row| crate::conversationWorkItems::GroupingRow {
                            row_id: row.row_id,
                            kind: row.kind.clone(),
                            tool_name: row.tool_name.clone(),
                            tool_call_id: row.tool_call_id.clone(),
                            status: row.status.clone().unwrap_or_default(),
                            input: row.input.clone().unwrap_or(Value::Null),
                            input_text: row.input_text.clone().unwrap_or_default(),
                            started_at: row.started_at.as_ref().and_then(|v| v.as_f64()),
                            turn_id: row.turn_id.clone(),
                            parent_tool_call_id: row.parent_tool_call_id.clone(),
                        })
                        .collect();
                    // 阶段尾部是否运行中：最后一行仍在跑时，尾部分组的父状态显「执行中」
                    // （真源 stageTailIsRunning 由 turn 结构算出；v1 以末行状态近似）。
                    let tail_running = list.last().is_some_and(|row| {
                        matches!(
                            row.status.as_deref(),
                            Some("running") | Some("inputStreaming") | Some("pendingApproval")
                        )
                    });
                    let items = crate::conversationWorkItems::build_assistant_work_render_items(
                        &grouping_rows,
                        &crate::conversationWorkItems::WorkRenderOptions {
                            stage_tail_is_running: tail_running,
                            ..Default::default()
                        },
                    );
                    items
                        .into_iter()
                        .map(|item| match item {
                            crate::conversationWorkItems::WorkRenderItem::Row {
                                row_index, ..
                            } => view! { <ConversationRow row=list[row_index].clone() /> }.into_any(),
                            crate::conversationWorkItems::WorkRenderItem::ExecuteGroup {
                                node, ..
                            } => view! {
                                <div class="py-0" data-tool-call-id=node.tool_call.tool_id.clone()>
                                    <div data-conversation-selectable="true">
                                        <crate::ToolCallBlocks::ToolCallBlock::ToolCallBlock
                                            node=node.clone()
                                            context=crate::ToolCallBlocks::ToolCallBlock::ToolCallBlockContext::default()
                                        />
                                    </div>
                                </div>
                            }
                            .into_any(),
                            crate::conversationWorkItems::WorkRenderItem::ExploreGroup {
                                node, ..
                            } => view! {
                                <div class="py-0" data-tool-call-id=node.tool_call.tool_id.clone()>
                                    <div data-conversation-selectable="true">
                                        <crate::ToolCallBlocks::ToolCallBlock::ToolCallBlock
                                            node=node.clone()
                                            context=crate::ToolCallBlocks::ToolCallBlock::ToolCallBlockContext::default()
                                        />
                                    </div>
                                </div>
                            }
                            .into_any(),
                            // Changes 分组开关关闭时不产生；防御路径：逐行渲染。
                            crate::conversationWorkItems::WorkRenderItem::ChangesGroup {
                                row_indices, ..
                            } => row_indices
                                .into_iter()
                                .map(|i| view! { <ConversationRow row=list[i].clone() /> }.into_any())
                                .collect_view()
                                .into_any(),
                            // Agent 卡（agent.tsx 已迁）：从配对的 subagent 行取
                            // authoritativeAgentType（真源 ConversationAgentToolCallRow:93）。
                            crate::conversationWorkItems::WorkRenderItem::AgentToolCall {
                                row_index,
                                subagent_row_index,
                                ..
                            } => {
                                let authoritative = list
                                    .get(subagent_row_index)
                                    .and_then(|sub| sub.subagent_type.clone());
                                view! {
                                    <div class="py-0" data-row-id=list[row_index].row_id>
                                        <div data-conversation-selectable="true">
                                            <crate::ToolCallBlocks::ToolCallBlock::ToolCallBlock
                                                node=crate::ToolCallBlocks::toolCallRowAdapter::tool_call_row_to_legacy_node(
                                                    &row_to_legacy_json(&list[row_index]),
                                                )
                                                context=crate::ToolCallBlocks::ToolCallBlock::ToolCallBlockContext {
                                                    authoritative_agent_type: authoritative,
                                                    ..Default::default()
                                                }
                                            />
                                        </div>
                                    </div>
                                }
                                .into_any()
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </div>
            <div class="flex-none border-t border-border px-3 pt-2">
                <button
                    class="text-xs text-muted hover:text-foreground"
                    on:click=move |_| set_show_model.update(|v| *v = !*v)
                >
                    {move || if show_model.get() { "▾ 模型" } else { "▸ 模型" }}
                </button>
                {show_model.get().then(|| {
                    view! {
                        <div class="mt-1 flex items-center gap-2 pb-1">
                            <input
                                class="w-40 rounded-lg border border-border bg-panel px-2 py-1 text-xs outline-none focus:border-border-hover"
                                placeholder="providerId（如 zcode）"
                                prop:value=move || model_provider.get()
                                on:input=move |e| {
                                    let v = event_target_value(&e);
                                    set_ls("zcode-model-provider", &v);
                                    set_model_provider.set(v);
                                }
                            />
                            <input
                                class="w-52 rounded-lg border border-border bg-panel px-2 py-1 text-xs outline-none focus:border-border-hover"
                                placeholder="modelId"
                                prop:value=move || model_id.get()
                                on:input=move |e| {
                                    let v = event_target_value(&e);
                                    set_ls("zcode-model-id", &v);
                                    set_model_id.set(v);
                                }
                            />
                        </div>
                    }
                })}
            </div>
            <form
                class="flex flex-none items-end gap-2 border-t border-border p-3"
                on:submit=send
            >
                <textarea
                    class="min-h-[40px] flex-1 resize-none rounded-lg border border-border bg-panel px-3 py-2 text-sm outline-none focus:border-border-hover"
                    placeholder="输入消息，Enter 发送（Shift+Enter 换行）"
                    rows="2"
                    prop:value=move || draft.get()
                    on:input=move |e| set_draft.set(event_target_value(&e))
                    on:keydown=move |e| {
                        // Enter 发送 / Shift+Enter 换行（对齐主输入行为）。
                        if e.key() == "Enter" && !e.shift_key() {
                            e.prevent_default();
                            let form = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlFormElement>().ok());
                            if let Some(form) = form {
                                let _ = form.request_submit();
                            }
                        }
                    }
                ></textarea>
                <button
                    type="submit"
                    class="rounded-lg bg-primary px-3 py-2 text-sm font-semibold text-primary-foreground disabled:opacity-60"
                    disabled=move || sending.get()
                >
                    {move || if sending.get() { "回复中…" } else { "发送" }}
                </button>
            </form>
        </div>
    }
}

#[component]
fn ConversationRow(row: ConversationRowView) -> impl IntoView {
    match row.kind.as_str() {
        "userInput" => view! { <UserInputRowView row=row /> }.into_any(),
        "assistantText" => view! { <AssistantTextRowView row=row /> }.into_any(),
        "reasoning" => view! { <ReasoningRowView row=row /> }.into_any(),
        "toolCall" => view! { <ToolCallRowView row=row /> }.into_any(),
        "turnHeader" => view! { <TurnHeaderRowView row=row /> }.into_any(),
        "artifact" => view! { <ArtifactRowView row=row /> }.into_any(),
        // timelineMarker/subagent/hookInvocation 等按视图迁移节奏补齐。
        _ => ().into_any(),
    }
}

/// userInput 行（ConversationRowView 851 行起 UserInputRowView 的静态子集）：
/// 右对齐 + bg-surface 卡片（rounded-xl rounded-tr-xs + border），时间戳右下。
#[component]
fn UserInputRowView(row: ConversationRowView) -> impl IntoView {
    // 引擎尾注折叠：正文只到 epilogueStart（splitUserInputEpilogue 同构），
    // 之后是引擎附加文本（dwf ask 尾注/nudge），折进气泡底部披露。
    // epilogueStart 按 UTF-16 code unit 计（TS 侧 string 下标）；Rust 侧按
    // chars 计数换算到字节边界，避免多字节字符上 panic。
    let (body, epilogue) = match row.epilogue_start {
        Some(start) if start > 0 => {
            let byte_idx = row
                .text
                .char_indices()
                .nth(start)
                .map(|(b, _)| b)
                .unwrap_or(row.text.len());
            (
                row.text[..byte_idx].trim_end().to_string(),
                Some(row.text[byte_idx..].trim_start().to_string()),
            )
        }
        _ => (row.text.clone(), None),
    };
    view! {
        <div class="group/user-row flex flex-col items-end" data-row-id=row.row_id>
            <div class="flex max-w-xl max-w-full flex-col gap-2 rounded-xl rounded-tr-xs border border-border bg-surface px-4 py-3 text-ui-base text-foreground">
                {body}
                {epilogue.map(|ep| {
                    view! {
                        <details class="text-xs text-muted">
                            <summary class="cursor-pointer">"附加说明"</summary>
                            <div class="mt-1 whitespace-pre-wrap">{ep}</div>
                        </details>
                    }
                })}
            </div>
            <div class="mt-1 text-right text-ui-sm text-foreground-subtlest">"已发送"</div>
        </div>
    }
}

/// assistantText 行（AssistantTextRowView 1477 行同构）：
/// 正文 w-full text-ui-base（markdown 渲染等价 streamdown），
/// 完成态 actions hover 显现（opacity-0 group-hover/assistant-row:opacity-100）。
#[component]
fn AssistantTextRowView(row: ConversationRowView) -> impl IntoView {
    let stream = expect_context::<crate::stream::StreamStore>();
    // sessionId 从 row 拿不到，用 selected_session（渲染行时父级已选定会话）。
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let session_id = selected_session.get().unwrap_or_default();
    let row_id = row.row_id;
    let streaming = row.state == "streaming";
    // ★逐 token：流式累积文本优先于 rowsRange 的静态文本。
    // 收到 row.upserted 后累积被清空，自动回退到权威文本。
    let static_text = row.text.clone();
    let text = move || {
        stream
            .text_of(&session_id, row_id)
            .unwrap_or_else(|| static_text.clone())
    };
    let rendered = move || render_markdown(&text());
    view! {
        <div class="group/assistant-row" data-row-id=row.row_id>
            <div data-conversation-selectable="true" class="w-full text-ui-base">
                <div class="prose prose-sm max-w-none dark:prose-invert prose-pre:bg-[#f0f1f3] prose-pre:text-foreground prose-code:before:content-[''] prose-code:after:content-['']"
                    inner_html=rendered></div>
            </div>
            {(!streaming).then(|| {
                view! {
                    <div class="mt-1 flex gap-1 opacity-0 transition-opacity group-hover/assistant-row:opacity-100 focus-within:opacity-100">
                        <span class="text-ui-xs text-foreground-subtlest">{move || row.model.clone().unwrap_or_default()}</span>
                    </div>
                }
            })}
        </div>
    }
}

/// reasoning 行（ReasoningRowView 1592 行同构）：
/// 折叠披露，streaming/complete 都默认收起；streaming 且 text 空不渲染。
#[component]
fn ReasoningRowView(row: ConversationRowView) -> impl IntoView {
    let stream = expect_context::<crate::stream::StreamStore>();
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let session_id = selected_session.get().unwrap_or_default();
    let row_id = row.row_id;
    let streaming = row.state == "streaming";
    let duration_seconds = row
        .duration_ms
        .map(|ms| (ms / 1000).max(1))
        .map(|s| format!("{s}s"));
    // reasoning 也是流式字段（path = "reasoning"），同样走累积文本优先。
    let static_text = row.text.clone();
    let live_text = move || {
        stream
            .text_of(&session_id, row_id)
            .unwrap_or_else(|| static_text.clone())
    };
    // streaming 且无任何文本（含累积）时不渲染——真源同此判断。
    if streaming && live_text().is_empty() {
        return ().into_any();
    }
    view! {
        <div data-row-id=row.row_id>
            <details class="w-full">
                <summary class="cursor-pointer text-xs text-muted">
                    {move || {
                        if streaming {
                            "思考中…".to_string()
                        } else {
                            match &duration_seconds {
                                Some(d) => format!("思考过程（{d}）"),
                                None => "思考过程".to_string(),
                            }
                        }
                    }}
                </summary>
                <div data-conversation-selectable="true">
                    <pre class="mt-1 max-h-48 overflow-auto rounded-lg bg-surface p-2 text-xs whitespace-pre-wrap text-muted">{move || live_text()}</pre>
                </div>
            </details>
        </div>
    }
    .into_any()
}

/// 把 `ConversationRowView` 重组成 adapter 需要的行 JSON
/// （toolCallRowAdapter.rs 按宽容协议逐字段取）。
fn row_to_legacy_json(row: &ConversationRowView) -> Value {
    serde_json::json!({
        "kind": row.kind,
        "rowId": row.row_id,
        "toolCallId": row.tool_call_id.clone().unwrap_or_default(),
        "toolName": row.tool_name.clone(),
        "status": row.status.clone().unwrap_or_else(|| "inputStreaming".into()),
        "inputText": row.input_text.clone().unwrap_or_default(),
        "input": row.input.clone(),
        "output": row.output.clone(),
        "display": row.display.clone(),
        "cuaApp": row.cua_app.clone(),
        "startedAt": row.started_at.clone(),
        "error": row.error.clone(),
    })
}

/// toolCall 行（真源 ConversationRowView.tsx:1894-2010 的接线形态）：
/// 行 → toolCallRowAdapter → ToolCallBlock（按工具身份分流到专属卡/fallback）。
/// 工具行去掉纵向内边距（py-0），间距由组容器统一给（同真源注释）。
#[component]
fn ToolCallRowView(row: ConversationRowView) -> impl IntoView {
    let row_json = row_to_legacy_json(&row);
    let node = crate::ToolCallBlocks::toolCallRowAdapter::tool_call_row_to_legacy_node(&row_json);
    view! {
        <div class="py-0" data-row-id=row.row_id>
            <div data-conversation-selectable="true">
                <crate::ToolCallBlocks::ToolCallBlock::ToolCallBlock
                    node=node
                    context=crate::ToolCallBlocks::ToolCallBlock::ToolCallBlockContext::default()
                />
            </div>
        </div>
    }
}

/// turnHeader 行（TurnHeaderRowView 1627 行同构）：subtle 分隔文本。
#[component]
fn TurnHeaderRowView(row: ConversationRowView) -> impl IntoView {
    let origin = row.origin.clone().unwrap_or_default();
    let state = row.state.clone();
    view! {
        <div class="border-b border-border py-1 text-ui-sm text-foreground-subtle" data-row-id=row.row_id>
            {format!("turn · {origin} · {state}")}
        </div>
    }
}

/// artifact 行（ArtifactRowView 同构）：文件卡（图标 + 名称 + 类型·大小）。
#[component]
fn ArtifactRowView(row: ConversationRowView) -> impl IntoView {
    let display_name = row.display_name.clone().unwrap_or_default();
    let meta = format!(
        "{} · {} bytes",
        row.artifact_type.clone().unwrap_or_default().to_uppercase(),
        row.size_bytes.unwrap_or(0)
    );
    view! {
        <div class="px-4 py-1" data-row-id=row.row_id>
            <div class="flex items-center gap-2 rounded-lg border border-card-border bg-card px-3 py-2">
                <span class="size-4 flex-none text-foreground-subtle">"▤"</span>
                <div class="min-w-0 flex-1">
                    <p class="truncate text-ui-base text-foreground">{display_name}</p>
                    <p class="text-ui-sm text-foreground-subtle">{meta}</p>
                </div>
            </div>
        </div>
    }
}

/// localStorage 读写（模型选择等前端偏好）。
fn read_ls(key: &str) -> String {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(key).ok())
        .flatten()
        .unwrap_or_default()
}

fn set_ls(key: &str, value: &str) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = ls.set_item(key, value);
    }
}

/// 取 vendor/hljs.js 暴露的全局 hljsLib（不存在时返回 NULL）。
fn window_hljs() -> JsValue {
    web_sys::window()
        .map(|w| js_sys::Reflect::get(&w, &"hljsLib".into()).unwrap_or(JsValue::NULL))
        .unwrap_or(JsValue::NULL)
}

/// 浏览器环境的轻量等待（不引入 tokio runtime 依赖 wasm 侧）。
async fn browser_wait(ms: u64) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let callback = Closure::<dyn FnMut()>::new(move || {
            resolve.call0(&wasm_bindgen::JsValue::NULL).ok();
        });
        web_sys::window().map(|w| {
            w.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                ms as i32,
            )
            .ok();
        });
        callback.forget();
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}
