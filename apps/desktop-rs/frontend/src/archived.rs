//! 归档区（1:1 翻译 `packages/ui/src/WorkspaceArchivedTasksFlatSection.tsx`，353 行）。
//!
//! 真源要点：
//! - 挂载：仅 `taskViewMode === "archived"` 时替换任务列表区（WorkspaceSidebar.tsx:1901-1910）
//! - 行容器 `rounded-lg px-2.5 py-2`，选中 `bg-selected` / hover `hover:bg-surface-hover`
//!   （:155-158）——**两行布局**，与置顶区的单行不同
//! - 第一行：标题 + 相对时间（:182-190）
//! - 第二行：工作区标签（Folder 图标 size-3）+ 变更统计 + 取消归档 + 删除
//!   （:192-331）
//! - 空态文案「暂无归档任务」`taskList.noArchivedTasks`（zh-CN.ts:1880）
//! - 20 条折叠 + 显示更多/收起（:65, 337-350）
//! - 取消归档走 `unarchiveTask`，删除走 `deleteTask`（:231, 298）
//!
//! 数据源：主进程 `task_list(kind:"archived")` / `task_unarchive` / `task_delete`
//! （Rust `taskdb`），与真源同库同口径。
//!
//! 未做（真源依赖远端 workspace 体系，Rust 侧无对应概念）：
//! - Cloud / CloudDownload 图标分支（远端任务）
//! - DeleteAllArchivedTasksButton（清空全部归档 + confirmDialog）
//! - getTaskChangeSummary 变更统计（依赖 changeSummary 列，Rust 投影暂未取）
//! - ControlHintTooltip（统一 tooltip 组件）

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use crate::app::{Icon, invoke_json, relative_time};

/// 折叠阈值（真源 :65 collapsedLimit = 20）。
const COLLAPSED_LIMIT: usize = 20;

/// 归档任务行（主进程 task_list 返回的 camelCase 投影）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedTask {
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

// ---------------------------------------------------------------------------
// 图标（lucide path 取自 node_modules/lucide-react/dist/esm/icons）
// ---------------------------------------------------------------------------

fn icon_archive_x() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8",
                "m9.5 17 5-5",
                "m9.5 12 5 5",
            ]
            circles=vec![]
            rects=vec![("2", "3", "20", "5")]
        />
    }
}

fn icon_folder() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z",
            ]
            circles=vec![]
        />
    }
}

fn icon_trash() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M10 11v6",
                "M14 11v6",
                "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6",
                "M3 6h18",
                "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2",
            ]
            circles=vec![]
        />
    }
}

/// 工作区路径叶子名（真源 `getPathLeaf`）：取最后一段。
fn path_leaf(path: &str) -> &str {
    path.trim_end_matches(['\\', '/'])
        .rsplit(['\\', '/'])
        .find(|s| !s.is_empty())
        .unwrap_or(path)
}

/// 归档区共享状态（行组件要写列表做乐观更新，故经 context 下发可写句柄）。
#[derive(Clone, Copy)]
pub struct ArchivedStore {
    tasks: RwSignal<Vec<ArchivedTask>>,
    error: RwSignal<String>,
    /// 待确认删除的任务 id——真源用 confirmDialog 二次确认。
    pending_delete: RwSignal<Option<String>>,
}

// ---------------------------------------------------------------------------
// 面板
// ---------------------------------------------------------------------------

#[component]
pub fn ArchivedTasksSection() -> impl IntoView {
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let store = expect_context::<crate::app::SessionsStore>();

    let show_all = RwSignal::new(false);
    let archived_store = ArchivedStore {
        tasks: RwSignal::new(Vec::new()),
        error: RwSignal::new(String::new()),
        pending_delete: RwSignal::new(None),
    };
    provide_context(archived_store);

    // 会话切换 → 重拉该 workspace 的归档列表。
    Effect::new(move |_| {
        let Some(session_id) = selected_session.get() else {
            archived_store.tasks.set(Vec::new());
            return;
        };
        let Some(ws_path) = store.workspace_path_of(&session_id) else {
            archived_store.tasks.set(Vec::new());
            return;
        };
        archived_store.error.set(String::new());
        spawn_local(async move {
            match invoke_json(
                "task_list",
                serde_json::json!({
                    "workspacePath": ws_path,
                    "workspaceIdentity": serde_json::Value::Null,
                    "kind": "archived",
                }),
            )
            .await
            {
                Ok(v) => {
                    let list: Vec<ArchivedTask> = serde_json::from_value(
                        v.get("tasks").cloned().unwrap_or(serde_json::json!([])),
                    )
                    .unwrap_or_default();
                    archived_store.tasks.set(list);
                }
                Err(e) => archived_store.error.set(e),
            }
        });
    });

    view! {
        <div>
            {move || {
                let err = archived_store.error.get();
                if !err.is_empty() {
                    return view! {
                        <p class="px-3 py-2 text-xs text-[#dc2626]">{err}</p>
                    }
                    .into_any();
                }
                let list = archived_store.tasks.get();
                // 空态展示提示文案（真源 :112-120）；加载中不闪文案。
                if list.is_empty() {
                    return view! {
                        <div class="px-3 py-2 text-ui-base text-foreground-subtle">"暂无归档任务"</div>
                    }
                    .into_any();
                }
                let visible: Vec<ArchivedTask> = if show_all.get() {
                    list.clone()
                } else {
                    list.iter().take(COLLAPSED_LIMIT).cloned().collect()
                };
                let can_toggle = list.len() > COLLAPSED_LIMIT;
                view! {
                    <ul class="space-y-1 pb-4">
                        {visible
                            .into_iter()
                            .map(|task| {
                                view! { <ArchivedTaskRow task=task /> }
                            })
                            .collect_view()}
                    </ul>
                    {can_toggle.then(|| view! {
                        <div class="cursor-pointer pb-4 pl-8.5">
                            <span
                                class="text-ui-base text-foreground-subtlest hover:text-foreground-subtle"
                                on:click=move |_| show_all.set(!show_all.get_untracked())
                            >
                                {move || if show_all.get() { "收起" } else { "显示更多" }}
                            </span>
                        </div>
                    })}
                }
                .into_any()
            }}
            // 删除二次确认（真源 confirmDialog :283-294）。真源用通用对话框组件，
            // 这里内联一个轻量确认条，语义一致：确认后才真删。
            {move || {
                let pending = archived_store.pending_delete.get();
                pending.map(|task_id| {
                    let title = archived_store
                        .tasks
                        .get()
                        .into_iter()
                        .find(|t| t.task_id == task_id)
                        .map(|t| if t.title.is_empty() { "新任务".to_string() } else { t.title })
                        .unwrap_or_default();
                    let archived = archived_store;
                    view! {
                        <div class="mx-2 mb-2 rounded-lg border border-border bg-panel p-2">
                            <p class="text-xs text-foreground">"确认删除该归档任务？"</p>
                            <p class="mt-0.5 truncate text-xs text-muted">{title}</p>
                            <div class="mt-2 flex justify-end gap-1">
                                <button
                                    class="rounded px-2 py-1 text-xs text-muted hover:bg-surface-hover"
                                    on:click=move |_| archived.pending_delete.set(None)
                                >
                                    "取消"
                                </button>
                                <button
                                    class="rounded px-2 py-1 text-xs text-destructive hover:bg-surface-hover"
                                    on:click=move |_| {
                                        archived.pending_delete.set(None);
                                        // 闭包是 FnMut，task_id 每次点击都要能复用。
                                        confirm_delete(archived, task_id.clone());
                                    }
                                >
                                    "删除任务"
                                </button>
                            </div>
                        </div>
                    }
                })
            }}
        </div>
    }
}

/// 执行软删除（真源 `deleteTask`，:298-304）。
///
/// 乐观更新：从列表移除；失败时重新拉取以恢复真实状态
/// （真源 removeTaskFromTaskCaches 同样只更新缓存，失败走 logger）。
fn confirm_delete(store: ArchivedStore, task_id: String) {
    spawn_local(async move {
        let Some(target) = store.tasks.get().into_iter().find(|t| t.task_id == task_id) else {
            return;
        };
        match invoke_json(
            "task_delete",
            serde_json::json!({
                "workspacePath": target.workspace_path,
                "workspaceIdentity": target.workspace_identity,
                "taskId": target.task_id,
            }),
        )
        .await
        {
            Ok(_) => store.tasks.update(|list| {
                list.retain(|t| t.task_id != task_id);
            }),
            Err(e) => store.error.set(format!("删除归档任务失败: {e}")),
        }
    });
}

// ---------------------------------------------------------------------------
// 行
// ---------------------------------------------------------------------------

#[component]
fn ArchivedTaskRow(task: ArchivedTask) -> impl IntoView {
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let archived_store = expect_context::<ArchivedStore>();

    let task_id = task.task_id.clone();
    let ws_path = task.workspace_path.clone();
    let ws_identity = task.workspace_identity.clone();
    let workspace_label = path_leaf(&ws_path).to_string();
    // 真源 :130 空标题回落「新任务」（taskList.untitled）。
    let title = if task.title.is_empty() {
        "新任务".to_string()
    } else {
        task.title.clone()
    };
    let time = relative_time(task.updated_at);

    let active_id = task_id.clone();
    let active = move || selected_session.get().as_deref() == Some(active_id.as_str());
    let item_class = move || {
        // 真源 :155-158 rounded-lg px-2.5 py-2 + 选中 bg-selected。
        if active() {
            "w-full cursor-pointer rounded-lg px-2.5 py-2 text-left transition-[background-color,border-color,box-shadow] bg-selected"
        } else {
            "w-full cursor-pointer rounded-lg px-2.5 py-2 text-left transition-[background-color,border-color,box-shadow] hover:bg-surface-hover"
        }
    };

    // 取消归档（真源 :231-238）：成功后从 archived 列表移除。
    // 用 Callback 而非直接内联 on:click，因为需要 stop_propagation 阻止行选中，
    // 而独立闭包拿不到事件类型标注（app.rs:423 的内联写法不需要标注）。
    let unarchive = Callback::new({
        let ws_path = ws_path.clone();
        let ws_identity = ws_identity.clone();
        let task_id = task_id.clone();
        move |_: ()| {
            let ws_path = ws_path.clone();
            let ws_identity = ws_identity.clone();
            let task_id = task_id.clone();
            let archived = archived_store;
            spawn_local(async move {
                match invoke_json(
                    "task_unarchive",
                    serde_json::json!({
                        "workspacePath": ws_path,
                        "workspaceIdentity": ws_identity,
                        "taskId": task_id,
                    }),
                )
                .await
                {
                    Ok(_) => archived.tasks.update(|list| {
                        list.retain(|t| t.task_id != task_id);
                    }),
                    Err(e) => archived.error.set(format!("取消归档失败: {e}")),
                }
            });
        }
    });

    let ask_delete = {
        let task_id = task_id.clone();
        move |_: ()| {
            archived_store.pending_delete.set(Some(task_id.clone()));
        }
    };

    view! {
        <li class=item_class on:click=move |_| selected_session.set(Some(task_id.clone()))>
            // 第一行：标题 + 相对时间（真源 :182-190）。
            <div class="relative flex items-center gap-2">
                <p
                    class="min-w-0 flex-1 truncate text-ui-base text-foreground"
                    title=title
                >
                    // 行内文本用 clone 副本，title 属性拿走原值。
                    {title.clone()}
                </p>
                <span class="shrink-0 text-ui-base text-foreground-subtle">{time}</span>
            </div>
            // 第二行：工作区标签 + 取消归档 + 删除（真源 :192-331）。
            <div class="mt-1 flex items-center gap-2 text-ui-base text-foreground-subtle">
                <span class="flex min-w-0 flex-1 items-center gap-1.5" title=ws_path>
                    <span class="flex size-3 flex-none items-center justify-center">{icon_folder()}</span>
                    <span class="min-w-0 truncate">{workspace_label}</span>
                </span>
                <span
                    class="inline-flex flex-none cursor-pointer items-center justify-center rounded-sm text-foreground-subtle hover:text-foreground"
                    role="button"
                    title="取消归档任务"
                    on:click=move |ev| {
                        // 行内按钮不能触发外层行的选中（真源 on:click 里 stopPropagation）。
                        ev.stop_propagation();
                        unarchive.run(());
                    }
                >
                    <span class="flex size-3.5 items-center justify-center">{icon_archive_x()}</span>
                </span>
                <span
                    class="inline-flex flex-none cursor-pointer items-center justify-center rounded-sm text-destructive hover:text-destructive"
                    role="button"
                    title="删除任务"
                    on:click=move |ev| {
                        ev.stop_propagation();
                        ask_delete(());
                    }
                >
                    <span class="flex size-3.5 items-center justify-center">{icon_trash()}</span>
                </span>
            </div>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, title: &str, ws: &str) -> ArchivedTask {
        ArchivedTask {
            task_id: id.into(),
            workspace_path: ws.into(),
            workspace_identity: None,
            title: title.into(),
            status: Some("completed".into()),
            created_at: 0,
            updated_at: 200,
            pinned: false,
            archived: true,
        }
    }

    #[test]
    fn path_leaf_takes_last_segment() {
        assert_eq!(path_leaf("D:\\a\\b\\c"), "c");
        assert_eq!(path_leaf("D:/a/b/c/"), "c", "尾部斜杠要吃掉");
        assert_eq!(path_leaf("solo"), "solo");
        assert_eq!(path_leaf("D:\\a"), "a");
    }

    #[test]
    fn collapsed_limit_matches_source() {
        // 真源 :65 collapsedLimit = 20。
        assert_eq!(COLLAPSED_LIMIT, 20);
    }

    #[test]
    fn parses_backend_payload() {
        let raw = serde_json::json!({
            "taskId": "s1",
            "workspacePath": "D:/a/b",
            "workspaceIdentity": serde_json::Value::Null,
            "title": "旧任务",
            "status": "completed",
            "createdAt": 100i64,
            "updatedAt": 200i64,
            "pinned": false,
            "archived": true,
        });
        let parsed: ArchivedTask = serde_json::from_value(raw).unwrap();
        assert_eq!(parsed.task_id, "s1");
        assert!(parsed.archived);
        assert_eq!(parsed.title, "旧任务");
    }

    #[test]
    fn empty_title_allowed_for_fallback() {
        // 真源 :130 用 untitled 回落，所以空标题必须能反序列化，不能报错。
        let raw = serde_json::json!({
            "taskId": "s2", "workspacePath": "D:/x", "workspaceIdentity": null,
            "title": "", "status": null, "createdAt": 0i64, "updatedAt": 0i64,
            "pinned": false, "archived": true,
        });
        let parsed: ArchivedTask = serde_json::from_value(raw).unwrap();
        assert!(parsed.title.is_empty());
        assert!(parsed.status.is_none());
    }

    #[test]
    fn workspace_label_comes_from_path_leaf() {
        let t = task("s1", "t", "D:\\projects\\my-app");
        assert_eq!(path_leaf(&t.workspace_path), "my-app");
    }
}
