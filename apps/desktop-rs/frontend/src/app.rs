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

#[wasm_bindgen]
extern "C" {
    // Tauri 2 在 WebView 中注入 __TAURI_INTERNALS__，wasm 侧经此调用 Rust command。
    #[wasm_bindgen(js_namespace = ["window", "__TAURI_INTERNALS__"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

async fn invoke_json(cmd: &str, args: Value) -> Result<Value, String> {
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

/// 消息部件（zcodeMessagePartSchema discriminated union 的宽松视图）。
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessagePartView {
    #[serde(rename = "type")]
    part_type: String,
    #[serde(default)]
    text: Option<String>,
}

/// 消息（zcodeMessageWithPartsSchema）：info.role 区分 user/assistant。
#[derive(Debug, Clone, serde::Deserialize)]
struct MessageWithPartsView {
    info: Value,
    parts: Vec<MessagePartView>,
}

impl MessageWithPartsView {
    fn role(&self) -> &str {
        self.info["role"].as_str().unwrap_or("unknown")
    }

    fn text_content(&self) -> String {
        self.parts
            .iter()
            .filter(|p| p.part_type == "text")
            .filter_map(|p| p.text.as_deref())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn reasoning_content(&self) -> Option<String> {
        let texts: Vec<&str> = self
            .parts
            .iter()
            .filter(|p| p.part_type == "reasoning")
            .filter_map(|p| p.text.as_deref())
            .collect();
        if texts.is_empty() {
            None
        } else {
            Some(texts.join("\n"))
        }
    }

    /// 其他部件类型的展示摘要（tool/file 等后续按类型迁移专属视图）。
    fn other_part_kinds(&self) -> Vec<String> {
        self.parts
            .iter()
            .filter(|p| p.part_type != "text" && p.part_type != "reasoning")
            .map(|p| p.part_type.clone())
            .collect()
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessagesResult {
    messages: Vec<MessageWithPartsView>,
}

/// 单条消息的展示视图（role + 文本 + reasoning 折叠 + 其他部件摘要）。
#[derive(Debug, Clone)]
struct ChatMessage {
    role: String,
    text: String,
    reasoning: Option<String>,
    other_parts: Vec<String>,
}

impl ChatMessage {
    fn from_wire(m: MessageWithPartsView) -> Self {
        Self {
            role: m.role().to_string(),
            text: m.text_content(),
            reasoning: m.reasoning_content(),
            other_parts: m.other_part_kinds(),
        }
    }
}

/// 会话状态徽标配色（zcodeSessionStatusSchema 六态）。
fn status_dot_class(status: &str) -> &'static str {
    match status {
        "running" => "bg-[#2563eb]",
        "waiting" => "bg-[#d97706]",
        "paused" => "bg-[#7c3aed]",
        "completed" => "bg-[#16a34a]",
        "error" => "bg-[#dc2626]",
        _ => "bg-[#9ca3af]",
    }
}

fn relative_time(ms: i64) -> String {
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
fn render_markdown(src: &str) -> String {
    use pulldown_cmark::{Event, Options, Parser};
    let options = Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS;
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
struct SessionsStore {
    sessions: RwSignal<Vec<SessionInfo>>,
    loading: RwSignal<bool>,
    error: RwSignal<String>,
    /// 当前视图（对齐 taskViewMode 概念的轻量版）。
    view: RwSignal<&'static str>,
}

/// 任务视图定义。归档视图走 includeArchived 通道单独拉取。
const TASK_VIEWS: [&str; 5] = ["全部", "进行中", "已完成", "出错", "归档"];

impl SessionsStore {
    fn load(&self) {
        let include_archived = self.view.get_untracked() == "归档";
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
        let view = self.view.get();
        self.sessions
            .get()
            .into_iter()
            .filter(|s| match view {
                "进行中" => matches!(s.status.as_str(), "running" | "waiting" | "paused"),
                "已完成" => matches!(s.status.as_str(), "completed" | "idle"),
                "出错" => s.status == "error",
                _ => true,
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

    fn workspace_path_of(&self, session_id: &str) -> Option<String> {
        self.sessions
            .get_untracked()
            .iter()
            .find(|s| s.session_id == session_id)
            .and_then(|s| s.workspace.as_ref())
            .and_then(|w| w["workspacePath"].as_str())
            .map(|p| p.to_string())
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
        let handler = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            if (e.meta_key() || e.ctrl_key()) && e.key() == "k" {
                e.prevent_default();
                show.update(|v| *v = !*v);
            }
            if e.key() == "Escape" {
                show.set(false);
            }
        });
        web_sys::window().map(|w| {
            w.add_event_listener_with_callback("keydown", handler.as_ref().unchecked_ref::<js_sys::Function>())
                .ok();
        });
        handler.forget();
    }

    // 文件树滑出面板开关（对齐真实侧栏行内的 onOpenFileTree）。
    let show_file_tree: RwSignal<bool> = RwSignal::new(false);
    provide_context(show_file_tree);

    // agent 通知桥（主进程 emit `agent://notification`）：
    // state.updated → 刷新会话列表与当前会话消息。
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
                "state.updated" => {
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
            .filter(|s| {
                q.is_empty() || s.title.to_lowercase().contains(&q)
            })
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
                                                "flex w-full items-center gap-2 rounded-lg bg-accent-weak px-3 py-2 text-left text-sm"
                                            } else {
                                                "flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm hover:bg-surface-hover"
                                            }
                                        };
                                        view! {
                                            <button class=row_class on:click=move |_| open.run(sid.clone())>
                                                <span class=format!("h-2 w-2 flex-none rounded-full {dot}")></span>
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

/// lucide 图标内联渲染（path 数据取自 node_modules/lucide-react/dist/esm/icons，
/// 与 React 版完全同源；stroke 规则统一 lucide 默认 2px round）。
#[component]
fn Icon(paths: Vec<&'static str>, circles: Vec<(&'static str, &'static str, &'static str)>) -> impl IntoView {
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
                                <SidebarSessionList />
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
                            "rounded bg-accent-weak px-1.5 py-0.5 text-xs font-medium text-accent"
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
                    class="rounded px-1.5 py-0.5 text-xs text-muted hover:bg-accent-weak"
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
                                if active() {
                                    "flex w-full items-center gap-2 rounded-lg bg-accent-weak px-2 py-1.5 text-left"
                                } else {
                                    "flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left hover:bg-accent-weak"
                                }
                            };
                            view! {
                                <button
                                    class=item_class
                                    on:click=move |_| selected_session.set(Some(session_id_for_click.clone()))
                                >
                                    <span class=format!("h-2 w-2 flex-none rounded-full {dot}") title={status_label}></span>
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
                class="mb-1 w-full rounded-lg border border-border bg-panel px-2 py-1.5 text-xs outline-none focus:border-accent"
                placeholder="工作区路径（留空用主目录）"
                prop:value=move || workspace_input.get()
                on:input=move |e| set_workspace_input.set(event_target_value(&e))
            />
            <button
                class="flex w-full items-center justify-center gap-1.5 rounded-lg bg-accent px-3 py-2 text-sm font-semibold text-accent-foreground disabled:opacity-60"
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
                    let pid = v["pid"].as_u64().map(|p| p.to_string()).unwrap_or("?".into());
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
                        let pid = v["pid"].as_u64().map(|p| p.to_string()).unwrap_or("?".into());
                        set_agent_status.set(format!("已连接（pid {pid}）"));
                    } else if running {
                        let reason = v["closedReason"]
                            .as_str()
                            .unwrap_or("未知原因")
                            .to_string();
                        set_agent_status.set(format!("连接断开：{reason}"));
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
                <button class="rounded-lg bg-accent px-2 py-1.5 text-xs font-semibold text-accent-foreground disabled:opacity-60" on:click=start disabled=move || busy.get()>
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

/// 文件树侧板（导航式浏览 v1：目录逐级进入 + 文件预览）。
/// 真实产品是 WorkspaceFileTree 树形展开 + code viewer，交互面后续对齐。
#[component]
fn FileTreePanel() -> impl IntoView {
    let show_file_tree = expect_context::<RwSignal<bool>>();
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let store = expect_context::<SessionsStore>();

    let workspace = move || {
        selected_session
            .get()
            .and_then(|id| store.workspace_path_of(&id))
    };

    // 当前目录与列表（workspace 切换时重置）。
    let cwd = RwSignal::new(None::<String>);
    let (entries, set_entries) = signal(Vec::<FsEntry>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(String::new());
    let (preview, set_preview) = signal(None::<FilePreview>);

    let load_dir = Callback::new(move |dir: Option<String>| {
        let Some(dir) = dir else { return };
        cwd.set(Some(dir.clone()));
        set_loading.set(true);
        set_error.set(String::new());
        let set_entries = set_entries.clone();
        let set_loading = set_loading.clone();
        let set_error = set_error.clone();
        spawn_local(async move {
            match invoke_json("fs_list", serde_json::json!({ "path": dir })).await {
                Ok(v) => {
                    let list: Vec<FsEntry> = v["entries"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .map(|e| FsEntry {
                                    name: e["name"].as_str().unwrap_or("").to_string(),
                                    is_dir: e["isDir"].as_bool().unwrap_or(false),
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    set_entries.set(list);
                    set_error.set(String::new());
                }
                Err(e) => set_error.set(e),
            }
            set_loading.set(false);
        });
    });

    // 会话/面板打开时初始化根目录。
    Effect::new(move |_| {
        let open = show_file_tree.get();
        if !open {
            return;
        }
        let root = workspace();
        match root {
            Some(root) => {
                if cwd.get_untracked().is_none() {
                    load_dir.run(Some(root));
                }
            }
            None => set_error.set("当前会话没有工作区路径".into()),
        }
    });

    let open_file = Callback::new(move |path: String| {
        let set_preview = set_preview.clone();
        spawn_local(async move {
            match invoke_json("fs_read", serde_json::json!({ "path": path })).await {
                Ok(v) => {
                    set_preview.set(Some(FilePreview {
                        content: v["content"].as_str().unwrap_or("").to_string(),
                        truncated: v["truncated"].as_bool().unwrap_or(false),
                    }));
                }
                Err(e) => set_preview.set(Some(FilePreview {
                    content: format!("读取失败: {e}"),
                    truncated: false,
                })),
            }
        });
    });

    view! {
        <aside class="flex h-full w-[38%] min-w-[260px] max-w-[480px] flex-col overflow-hidden rounded-r-[12px] border border-l-0 border-border bg-panel">
            <div class="flex flex-none items-center justify-between border-b border-border px-3 py-2">
                <span class="min-w-0 flex-1 truncate text-xs text-muted">
                    {move || cwd.get().unwrap_or_default()}
                </span>
                <button
                    class="ml-2 flex-none rounded px-1.5 py-0.5 text-xs text-muted hover:bg-surface-hover"
                    on:click=move |_| show_file_tree.set(false)
                    title="关闭文件树"
                >
                    "✕"
                </button>
            </div>
            <div class="flex flex-none items-center gap-2 px-3 py-1.5">
                <button
                    class="rounded px-1.5 py-0.5 text-xs text-muted hover:bg-surface-hover disabled:opacity-40"
                    disabled=move || {
                        let Some(c) = cwd.get() else { return true };
                        let Some(w) = workspace() else { return true };
                        !c.starts_with(&w) || c == w
                    }
                    on:click=move |_| {
                        let Some(c) = cwd.get_untracked() else { return };
                        let Some(w) = workspace() else { return };
                        // 回上级但不越过 workspace 根。
                        let parent = std::path::Path::new(&c)
                            .parent()
                            .map(|p| p.to_string_lossy().into_owned());
                        if let Some(p) = parent {
                            if p.starts_with(&w) {
                                load_dir.run(Some(p));
                            } else {
                                load_dir.run(Some(w));
                            }
                        }
                    }
                >
                    "↑ 上级"
                </button>
            </div>
            <div class="min-h-0 flex-1 overflow-y-auto px-1">
                {move || {
                    if loading.get() {
                        return view! { <p class="px-3 py-1 text-xs text-muted">"加载中…"</p> }.into_any();
                    }
                    if !error.get().is_empty() {
                        return view! { <p class="px-3 py-1 text-xs text-[#dc2626]">{move || error.get()}</p> }.into_any();
                    }
                    let list = entries.get();
                    if list.is_empty() {
                        return view! { <p class="px-3 py-1 text-xs text-muted">"空目录"</p> }.into_any();
                    }
                    list.into_iter()
                        .map(|e| {
                            // 显示文本与 full 闭包各持一份 name。
                            let entry_name = e.name.clone();
                            let entry_name_for_full = e.name.clone();
                            let is_dir = e.is_dir;
                            let full = move || {
                                format!(
                                    "{}{}",
                                    cwd.get_untracked().unwrap_or_default().trim_end_matches(['/', '\\']),
                                    format!("/{}", entry_name_for_full)
                                )
                            };
                            if is_dir {
                                let enter = Callback::new(move |_: ()| load_dir.run(Some(full())));
                                view! {
                                    <button
                                        class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-sm hover:bg-surface-hover"
                                        on:click=move |_| enter.run(())
                                    >
                                        <span class="text-muted">"▸"</span>
                                        <span class="min-w-0 flex-1 truncate">{entry_name}</span>
                                    </button>
                                }.into_any()
                            } else {
                                let open = open_file;
                                view! {
                                    <button
                                        class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-sm hover:bg-surface-hover"
                                        on:click=move |_| open.run(full())
                                    >
                                        <span class="text-foreground-subtlest">"·"</span>
                                        <span class="min-w-0 flex-1 truncate">{entry_name}</span>
                                    </button>
                                }.into_any()
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </div>
            {move || preview.get().map(|p| {
                view! {
                    <div class="flex max-h-[45%] flex-none flex-col border-t border-border">
                        <div class="flex items-center justify-between px-3 py-1 text-xs text-muted">
                            <span>"预览"</span>
                            {p.truncated.then(|| view! { <span>"（超长截断）"</span> })}
                        </div>
                        <pre class="min-h-0 flex-1 overflow-auto bg-[#f0f1f3] p-2 text-xs leading-relaxed whitespace-pre-wrap">{p.content}</pre>
                    </div>
                }
            })}
        </aside>
    }
}

#[derive(Debug, Clone)]
struct FsEntry {
    name: String,
    is_dir: bool,
}

#[derive(Debug, Clone)]
struct FilePreview {
    content: String,
    truncated: bool,
}

/// 顶栏：显示当前会话标题与状态（全局 store 共享）。
#[component]
fn WorkspaceHeaderBar() -> impl IntoView {
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let store = expect_context::<SessionsStore>();
    let show_file_tree = expect_context::<RwSignal<bool>>();
    let title = move || {
        selected_session
            .get()
            .and_then(|id| store.title_of(&id))
            .unwrap_or_else(|| "未选择任务".into())
    };
    let status = move || {
        selected_session
            .get()
            .and_then(|id| store.status_of(&id))
            .unwrap_or_default()
    };
    view! {
        <header class="flex h-11 flex-none items-center gap-2 border-b border-border bg-panel px-3">
            <span class="text-sm font-semibold">"ZCode 桌面端（Rust）"</span>
            <span class="mx-1 text-border">"|"</span>
            <span class=format!("h-2 w-2 flex-none rounded-full {}", "bg-[#9ca3af]")></span>
            <span class="min-w-0 flex-1 truncate text-sm">{title}</span>
            <button
                class="flex-none rounded px-2 py-1 text-xs text-muted hover:bg-surface-hover"
                on:click=move |_| show_file_tree.update(|v| *v = !*v)
                title="文件树（当前会话工作区）"
            >
                "文件"
            </button>
            <span class="flex-none text-xs text-muted">{move || {
                let s = status();
                if s.is_empty() { String::new() } else { s }
            }}</span>
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
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(String::new());
    let (draft, set_draft) = signal(String::new());
    let (sending, set_sending) = signal(false);

    let scroll_box = NodeRef::<html::Div>::new();

    let load_messages = Callback::new(move |_: ()| {
        set_loading.set(true);
        let sid = sid_load.clone();
        let set_messages = set_messages.clone();
        let set_loading = set_loading.clone();
        let set_error = set_error.clone();
        spawn_local(async move {
            match invoke_json(
                "agent_request",
                serde_json::json!({
                    "method": "session/messages",
                    "params": { "sessionId": sid, "limit": 200 }
                }),
            )
            .await
            {
                Ok(v) => match serde_json::from_value::<MessagesResult>(v) {
                    Ok(result) => {
                        set_messages
                            .set(result.messages.into_iter().map(ChatMessage::from_wire).collect());
                        set_error.set(String::new());
                    }
                    Err(e) => set_error.set(format!("解析失败: {e}")),
                },
                Err(e) => set_error.set(e),
            }
            set_loading.set(false);
        });
    });

    // 选中即加载；agent state.updated 通知驱动自动刷新。
    let triggers = expect_context::<DataRefreshTriggers>();
    Effect::new(move |_| {
        let _ = &session_id;
        load_messages.run(());
    });
    Effect::new(move |_| {
        triggers.messages.track();
        if triggers.messages.get() > 0 {
            load_messages.run(());
        }
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
            let send_result = invoke_json(
                "agent_send_text",
                serde_json::json!({ "sessionId": sid, "text": content }),
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
                    "agent_request",
                    serde_json::json!({
                        "method": "session/messages",
                        "params": { "sessionId": sid, "limit": 200 }
                    }),
                )
                .await;
                if let Ok(v) = fetch {
                    if let Ok(result) = serde_json::from_value::<MessagesResult>(v) {
                        let view: Vec<ChatMessage> =
                            result.messages.into_iter().map(ChatMessage::from_wire).collect();
                        let tail_text = view
                            .iter()
                            .rev()
                            .find(|m| m.role == "assistant")
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
                    list.iter()
                        .map(|m| view! { <MessageBubble msg=m.clone() /> })
                        .collect_view()
                        .into_any()
                }}
            </div>
            <form
                class="flex flex-none items-end gap-2 border-t border-border p-3"
                on:submit=send
            >
                <textarea
                    class="min-h-[40px] flex-1 resize-none rounded-lg border border-border bg-panel px-3 py-2 text-sm outline-none focus:border-accent"
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
                    class="rounded-lg bg-accent px-3 py-2 text-sm font-semibold text-accent-foreground disabled:opacity-60"
                    disabled=move || sending.get()
                >
                    {move || if sending.get() { "回复中…" } else { "发送" }}
                </button>
            </form>
        </div>
    }
}

#[component]
fn MessageBubble(msg: ChatMessage) -> impl IntoView {
    let is_user = msg.role == "user";
    let bubble_class = if is_user {
        "ml-auto max-w-[80%] rounded-2xl rounded-br-sm bg-accent px-3.5 py-2 text-sm text-accent-foreground"
    } else {
        "mr-auto max-w-[85%] rounded-2xl rounded-bl-sm border border-border bg-panel px-3.5 py-2 text-sm"
    };
    let other = msg.other_parts.clone();
    view! {
        <div class="mb-3 flex flex-col">
            {match msg.reasoning.clone() {
                Some(reasoning) => view! {
                    <details class=if is_user { "ml-auto mb-1 max-w-[80%]" } else { "mr-auto mb-1 max-w-[85%]" }>
                        <summary class="cursor-pointer text-xs text-muted">"思考过程"</summary>
                        <pre class="mt-1 max-h-48 overflow-auto rounded-lg bg-[#f0f1f3] p-2 text-xs whitespace-pre-wrap text-muted">{reasoning}</pre>
                    </details>
                }.into_any(),
                None => ().into_any(),
            }}
            <div class=bubble_class>
                {if msg.text.is_empty() && !other.is_empty() {
                    view! { <span>{format!("[{}]", other.join(", "))}</span> }.into_any()
                } else if is_user {
                    // 用户输入保持纯文本（所见即所输）。
                    view! { <span>{msg.text.clone()}</span> }.into_any()
                } else {
                    // 助手回复走 markdown 结构化渲染。
                    let rendered = render_markdown(&msg.text);
                    view! {
                        <div
                            class="prose prose-sm max-w-none dark:prose-invert prose-pre:bg-[#f0f1f3] prose-pre:text-foreground prose-code:before:content-[''] prose-code:after:content-['']"
                            inner_html=rendered
                        ></div>
                    }
                    .into_any()
                }}
            </div>
            {(!other.is_empty() && !msg.text.is_empty()).then(|| {
                let other = other.clone();
                view! { <div class="mt-1 text-[11px] text-muted">{format!("包含部件: {}", other.join(", "))}</div> }
            })}
        </div>
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
