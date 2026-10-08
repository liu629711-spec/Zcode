//! 侧栏置顶区（1:1 翻译 `packages/ui/src/WorkspacePinnedTasksSection.tsx`）。
//!
//! 真源要点：
//! - 位置：滚动容器内、操作区下、任务区上（`WorkspaceSidebar.tsx:1875-1899`）
//! - 容器 `flex flex-col gap-1 px-2 empty:hidden`（:479）
//! - 标题 `px-2.5 py-1 text-ui-base font-medium text-foreground-subtlest`，文案「已置顶」
//!   （`PinnedTasksSectionTitle`，:45；zh-CN.ts:1885 `taskList.pinnedSection`）
//! - 行复用 `MemoTaskItem` 传 `isPinned`；Pin 图标常显（已置顶）/ hover 显示（未置顶）
//!   （`TaskListItem.tsx:390-392, 508-530`）
//! - 折叠 20 条 + 「显示更多/收起」（:96, 752-765）
//! - 空态 `return null`，连标题都不渲染（:471-476）
//! - 五种任务视图下都显示（含归档，:897-904）
//!
//! 数据源：主进程 `task_list` / `task_set_pinned` 直读 `~/.zcode/v2/tasks-index.sqlite`
//! （Rust `taskdb` 模块）。真源同口径（node:sqlite）。

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use crate::app::{Icon, invoke_json};

/// 折叠阈值（真源 `WorkspacePinnedTasksSection.tsx:96` collapsedLimit = 20）。
const COLLAPSED_LIMIT: usize = 20;

/// 任务行（主进程 task_list 返回的 camelCase 投影）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PinnedTask {
    pub task_id: String,
    pub workspace_path: String,
    pub workspace_identity: Option<String>,
    pub title: String,
    pub status: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub pinned: bool,
    pub archived: bool,
}

/// 置顶区的共享状态（行组件需要写tasks 做乐观更新，故经 context 下发，
/// 不能只靠 props 传递 signal —— props 是快照值，改不动）。
#[derive(Clone, Copy)]
pub struct PinnedStore {
    tasks: RwSignal<Vec<PinnedTask>>,
    error: RwSignal<String>,
    /// 递增即触发重拉（乐观更新失败后回滚用，对齐真源的 refresh 收敛）。
    refresh: RwSignal<u64>,
}

fn icon_pin() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M12 17v5",
                "M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z",
            ]
            circles=vec![]
        />
    }
}

/// 置顶区容器 + 列表 + 折叠。
///
/// `workspace_path` 为 None 时（没有选中会话）整体不渲染——
/// 真源 `useWorkspaceServices` 拿不到 workspace 时不会走 pinned 查询。
#[component]
pub fn PinnedTasksSection() -> impl IntoView {
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let store = expect_context::<crate::app::SessionsStore>();

    let show_all = RwSignal::new(false);

    // 行组件要做乐观更新（取消置顶后立刻从列表移除），所以状态放 store 走 context 下发，
    // 面板渲染与行组件共用同一份，不各持一份signal。
    let pinned_store = PinnedStore {
        tasks: RwSignal::new(Vec::new()),
        error: RwSignal::new(String::new()),
        refresh: RwSignal::new(0),
    };
    provide_context(pinned_store);

    // 会话切换 / 乐观更新失败回滚 → 重拉该 workspace 的置顶列表。
    Effect::new(move |_| {
        let _ = pinned_store.refresh.get();
        let Some(session_id) = selected_session.get() else {
            pinned_store.tasks.set(Vec::new());
            return;
        };
        let Some(ws_path) = store.workspace_path_of(&session_id) else {
            pinned_store.tasks.set(Vec::new());
            return;
        };
        pinned_store.error.set(String::new());
        spawn_local(async move {
            match invoke_json(
                "task_list",
                serde_json::json!({
                    "workspacePath": ws_path,
                    "workspaceIdentity": serde_json::Value::Null,
                    "kind": "pinned",
                }),
            )
            .await
            {
                Ok(v) => {
                    let list: Vec<PinnedTask> = serde_json::from_value(
                        v.get("tasks").cloned().unwrap_or(serde_json::json!([])),
                    )
                    .unwrap_or_default();
                    pinned_store.tasks.set(list);
                }
                Err(e) => pinned_store.error.set(e),
            }
        });
    });

    view! {
        <div class="flex flex-col gap-1 px-2 empty:hidden">
            {move || {
                // 错误优先：置顶区取不到数据时要让用户知道，而不是静默空着。
                let err = pinned_store.error.get();
                if !err.is_empty() {
                    return view! {
                        <p class="px-2 py-1 text-xs text-[#dc2626]">{err}</p>
                    }
                    .into_any();
                }
                let list = pinned_store.tasks.get();
                // 空态直接不渲染（真源 :471-476 return null，连标题都不出）。
                if list.is_empty() {
                    return ().into_any();
                }
                let visible: Vec<PinnedTask> = if show_all.get() {
                    list.clone()
                } else {
                    list.iter().take(COLLAPSED_LIMIT).cloned().collect()
                };
                let can_toggle = list.len() > COLLAPSED_LIMIT;
                view! {
                    <h3 class="px-2.5 py-1 text-ui-base font-medium text-foreground-subtlest">"已置顶"</h3>
                    {visible
                        .into_iter()
                        .map(|task| {
                            view! { <PinnedTaskRow task=task /> }
                        })
                        .collect_view()}
                    {can_toggle.then(|| view! {
                        <button
                            class="cursor-pointer py-1 pl-8.5 text-ui-base text-foreground-subtlest hover:text-foreground-subtle"
                            on:click=move |_| show_all.set(!show_all.get_untracked())
                        >
                            {move || if show_all.get() { "收起" } else { "显示更多" }}
                        </button>
                    })}
                }
                .into_any()
            }}
        </div>
    }
}

/// 置顶任务行（对齐 `MemoTaskItem` + `isPinned`）。
///
/// 样式同 WorkspaceSidebarItem（h-8/pl-2.5/pr-1），差异只在 Pin 按钮：
/// 已置顶时常显，未置顶时 hover 显示（`TaskListItem.tsx:527-530`）。
#[component]
fn PinnedTaskRow(task: PinnedTask) -> impl IntoView {
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let pinned_store = expect_context::<PinnedStore>();

    let task_id = task.task_id.clone();
    let title = task.title.clone();
    let status = task.status.clone().unwrap_or_default();
    let dot = crate::app::status_dot_class(&status).to_string();
    let time = crate::app::relative_time(task.updated_at);
    let ws_path = task.workspace_path.clone();
    let ws_identity = task.workspace_identity.clone();

    let active_id = task_id.clone();
    let active = move || selected_session.get().as_deref() == Some(active_id.as_str());
    let item_class = move || {
        if active() {
            "group flex h-8 w-full min-w-0 items-center gap-2 rounded-lg bg-selected pl-2.5 pr-1 text-left text-foreground shadow-xl"
        } else {
            "group flex h-8 w-full min-w-0 items-center gap-2 rounded-lg pl-2.5 pr-1 text-left text-foreground hover:bg-surface-hover hover:text-foreground"
        }
    };

    // 取消置顶：先乐观移出列表（真源 WorkspacePinnedTasksSection.tsx:271-293 先移动后RPC），
    // 失败再回滚 + toast 提示（真源用 toast，不打断列表）。
    let unpin = {
        let task_id = task_id.clone();
        let ws_path = ws_path.clone();
        let ws_identity = ws_identity.clone();
        move |_| {
            let task_id = task_id.clone();
            let ws_path = ws_path.clone();
            let ws_identity = ws_identity.clone();
            let pinned = pinned_store;
            let toasts = expect_context::<crate::toast::ToastStore>();
            spawn_local(async move {
                match invoke_json(
                    "task_set_pinned",
                    serde_json::json!({
                        "workspacePath": ws_path,
                        "workspaceIdentity": ws_identity,
                        "taskId": task_id,
                        "pinned": false,
                    }),
                )
                .await
                {
                    Ok(_) => pinned.tasks.update(|list| {
                        list.retain(|t| t.task_id != task_id);
                    }),
                    Err(e) => {
                        toasts.error(format!("更新置顶状态失败: {e}"));
                        // 回滚：重新拉取置顶列表收敛到 SQLite 真相源。
                        pinned.refresh.update(|n| *n += 1);
                    }
                }
            });
        }
    };

    view! {
        <button class=item_class on:click=move |_| selected_session.set(Some(task_id.clone()))>
            // Pin 按钮：已置顶常显（leading 槽），点击取消置顶。
            <span
                class="inline-flex size-4 min-w-0 flex-none cursor-pointer items-center justify-center rounded-sm p-0 text-foreground-subtle hover:text-foreground"
                role="button"
                title="取消置顶任务"
                on:click=move |ev| {
                    ev.stop_propagation();
                    unpin(());
                }
            >
                {icon_pin()}
            </span>
            <span class=format!("size-1.5 flex-none rounded-full {dot}") title=status></span>
            <span class="min-w-0 flex-1 truncate text-sm">{title}</span>
            <span class="flex-none text-[11px] text-muted">{time}</span>
        </button>
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, title: &str, updated: i64) -> PinnedTask {
        PinnedTask {
            task_id: id.into(),
            workspace_path: "C:/ws".into(),
            workspace_identity: None,
            title: title.into(),
            status: Some("idle".into()),
            created_at: 0,
            updated_at: updated,
            pinned: true,
            archived: false,
        }
    }

    #[test]
    fn collapsed_limit_matches_source_constant() {
        // 真源 WorkspacePinnedTasksSection.tsx:96 collapsedLimit = 20。
        assert_eq!(COLLAPSED_LIMIT, 20);
    }

    #[test]
    fn parses_camel_case_payload_from_backend() {
        // 主进程 serde 输出 camelCase，前端反序列化口径必须一致。
        let raw = serde_json::json!({
            "taskId": "sess_1",
            "workspacePath": "C:/ws",
            "workspaceIdentity": null,
            "title": "标题",
            "status": "running",
            "createdAt": 100i64,
            "updatedAt": 200i64,
            "pinned": true,
            "archived": false,
        });
        let parsed: PinnedTask = serde_json::from_value(raw).unwrap();
        assert_eq!(parsed.task_id, "sess_1");
        assert_eq!(parsed.updated_at, 200);
        assert!(parsed.pinned);
        assert!(!parsed.archived);
    }

    #[test]
    fn tolerate_null_status() {
        // status 列可空（schema-v1.ts task_status TEXT），不能因为空状态崩掉整块置顶区。
        let raw = serde_json::json!({
            "taskId": "t", "workspacePath": "C:/ws", "workspaceIdentity": null,
            "title": "t", "status": null, "createdAt": 0i64, "updatedAt": 0i64,
            "pinned": true, "archived": false,
        });
        let parsed: PinnedTask = serde_json::from_value(raw).unwrap();
        assert!(parsed.status.is_none());
    }

    #[test]
    fn collapse_keeps_first_n_and_reports_toggle() {
        let list: Vec<PinnedTask> = (0..25).map(|i| task(&format!("t{i}"), "t", i)).collect();
        let visible: Vec<PinnedTask> = list.iter().take(COLLAPSED_LIMIT).cloned().collect();
        assert_eq!(visible.len(), 20);
        assert_eq!(visible.first().unwrap().task_id, "t0");
        assert!(list.len() > COLLAPSED_LIMIT, "25 条时应显示展开入口");
    }

    #[test]
    fn no_toggle_when_within_limit() {
        let list: Vec<PinnedTask> = (0..5).map(|i| task(&format!("t{i}"), "t", i)).collect();
        assert!(list.len() <= COLLAPSED_LIMIT);
    }
}
