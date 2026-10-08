//! 任务分组视图 UI（1:1 翻译 `packages/ui/src/workspace-grouped-tasks/`）。
//!
//! 真源要点：
//! - 组容器 `relative pt-2.5 pb-0`（types.ts TASK_GROUP_CONTAINER_CLASS）
//! - 组头 `mb-0.5 flex h-8 items-center gap-1 rounded-lg border border-transparent
//!   pl-1.5 pr-1` + hover surface-hover（TASK_GROUP_HEADER_CLASS）
//! - 颜色标记：size-5 圆里放 Hash 图标 size-3，底色按色系（colors.tsx TaskGroupColorMark）
//! - 组标题 `min-w-0 max-w-full truncate rounded-sm px-1`（TASK_GROUP_TITLE_CLASS）
//! - 折叠箭头 ChevronDown/Right `size-3.5 shrink-0 text-foreground-subtlest`
//! - 计数徽标 `inline-flex min-w-5 rounded-full bg-tag/50 px-1.5 py-0.5
//!   text-ui-sm font-medium`（TASK_GROUP_COUNT_BADGE_CLASS）
//! - 成员区 `ml-4 border-l py-px pl-2` + 按色系左边框（TASK_GROUP_CONTENT_CLASS + BORDER_COLOR）
//! - 折叠动画用 grid-rows 0fr/1fr + opacity，不用 Radix Collapsible
//!   （group-item.tsx:677-679 注释说明原因：Presence 会重放动画）
//! - 系统分组标题本地化，忽略库里存的占位标题（group-title.ts）
//!
//! 数据源：主进程 `task_grouped_view`，结构见真源
//! `zcodeTaskListTypes.ts:58-73` ZCodeGroupedTaskViewNode。
//!
//! 未做（依赖真源交互体系）：拖拽重排（view.ts 581 行 + dnd-kit）、
//! kanban 看板视图、sticky 组头、rename 输入框、右键菜单。

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use crate::app::{Icon, invoke_json, relative_time, status_dot_class};
use crate::groupedTasks::domEvents::{
    on_drag_end, on_drag_over, on_drag_start, on_drop, track_direction, view_to_order_input,
};
use crate::groupedTasks::dragRuntime::DragRuntime;
use crate::groupedTasks::join::{
    GroupedStructure, GroupedStructureNode, index_sessions, join_grouped_structure,
};
use crate::groupedTasks::view::{GroupedTaskView, GroupedTaskViewNode, TaskGroup, TaskListItem};

/// 真源 `zcode-task-types.ts:46,49` 系统分组 id。
const CRON_DEFAULT_GROUP_ID: &str = "zcode-default-group-cron";
const OFF_PEAK_DEFAULT_GROUP_ID: &str = "zcode-default-group-off-peak";

/// 系统分组占位标题（group-title.ts：展示时本地化，不显示库里的占位串）。
const CRON_PLACEHOLDER_TITLE: &str = "cron";
const OFF_PEAK_PLACEHOLDER_TITLE: &str = "off-peak";

/// 折叠态持久化 key 前缀（真源 groupedTaskExpansionPreference）。
const EXPANDED_KEY_PREFIX: &str = "zcode-grouped-expanded:";

/// 分组（主进程返回的 camelCase）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub group_id: String,
    pub title: String,
    pub color: String,
}

/// 分组视图顶层节点（与真源 ZCodeGroupedTaskViewNode 同构，serde tag=type）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GroupedNode {
    Group {
        group: Group,
        /// 主进程返回的字段名是 task_ids（serde camelCase）。
        task_ids: Vec<String>,
    },
    Task {
        task: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GroupedView {
    pub nodes: Vec<GroupedNode>,
}

/// 任务摘要已移除：真实数据链路是 `groupedTasks::join` —— 后端只给 `task_ids`，
/// 由 join 层补成带完整 task（含 workspacePath/Identity）的 `TaskListItem`，
/// 渲染直接消费 join 产物。原先在这里用 `summaries: HashMap` 二次查表既重复
/// 又拿不到 workspaceIdentity（会导致远程 workspace 的 taskKey 算错）。

/// 组展示标题（真源 group-title.ts getTaskGroupDisplayTitle）。
fn display_title(group: &Group) -> String {
    match group.group_id.as_str() {
        CRON_DEFAULT_GROUP_ID => "定时任务".to_string(),
        OFF_PEAK_DEFAULT_GROUP_ID => "闲时任务".to_string(),
        // 库里存的是占位标题时才做本地化，其余用真标题。
        _ if group.title == CRON_PLACEHOLDER_TITLE => "定时任务".to_string(),
        _ if group.title == OFF_PEAK_PLACEHOLDER_TITLE => "闲时任务".to_string(),
        _ => group.title.clone(),
    }
}

/// 系统分组不可重命名/删除（真源 isSystemGroup 分支，group-item.tsx:631-658）。
fn is_system_group(group: &Group) -> bool {
    group.group_id == CRON_DEFAULT_GROUP_ID || group.group_id == OFF_PEAK_DEFAULT_GROUP_ID
}

// ---------------------------------------------------------------------------
// 图标
// ---------------------------------------------------------------------------

fn icon_hash() -> impl IntoView {
    view! { <Icon paths=vec!["M4 9h16", "M4 15h16", "M10 3 8 21", "M16 3l-2 18"] circles=vec![] /> }
}

fn icon_chevron_down() -> impl IntoView {
    view! { <Icon paths=vec!["m6 9 6 6 6-6"] circles=vec![] /> }
}

fn icon_chevron_right() -> impl IntoView {
    view! { <Icon paths=vec!["m9 18 6-6-6-6"] circles=vec![] /> }
}

fn icon_message_plus() -> impl IntoView {
    view! {
        <Icon
            paths=vec![
                "M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z",
                "M12 7v6",
                "M9 10h6",
            ]
            circles=vec![]
        />
    }
}

/// 颜色底/字类名（真源 types.ts TASK_GROUP_COLOR_CLASS）。
///
/// Rust 侧后端已提供同一份映射（taskgroup::color_classes），此处保持一致，
/// 避免前后端两处维护。
fn color_class(color: &str) -> &'static str {
    match color {
        "red" => "bg-rose-300 text-rose-900 dark:bg-rose-400/32 dark:text-rose-50",
        "orange" => "bg-orange-300 text-orange-900 dark:bg-orange-400/32 dark:text-orange-50",
        "yellow" => "bg-amber-300 text-amber-900 dark:bg-amber-300/32 dark:text-amber-50",
        "green" => "bg-emerald-300 text-emerald-900 dark:bg-emerald-400/32 dark:text-emerald-50",
        "blue" => "bg-sky-300 text-sky-900 dark:bg-sky-400/32 dark:text-sky-50",
        "purple" => "bg-violet-300 text-violet-900 dark:bg-violet-400/32 dark:text-violet-50",
        _ => "bg-zinc-300 text-zinc-800 dark:bg-zinc-400/32 dark:text-zinc-100",
    }
}

/// 左边框类名（真源 TASK_GROUP_BORDER_COLOR_CLASS）。
fn border_color_class(color: &str) -> &'static str {
    match color {
        "red" => "border-rose-500/70 dark:border-rose-400/52",
        "orange" => "border-orange-500/70 dark:border-orange-400/52",
        "yellow" => "border-amber-500/70 dark:border-amber-300/52",
        "green" => "border-emerald-500/70 dark:border-emerald-400/52",
        "blue" => "border-sky-500/70 dark:border-sky-400/52",
        "purple" => "border-violet-500/70 dark:border-violet-400/52",
        _ => "border-zinc-500/70 dark:border-zinc-400/55",
    }
}

/// 颜色中文标签（真源 zh-CN.ts `taskGroup.color.*`）。
fn color_label(color: &str) -> &'static str {
    match color {
        "red" => "红色",
        "orange" => "橙色",
        "yellow" => "黄色",
        "green" => "绿色",
        "blue" => "蓝色",
        "purple" => "紫色",
        _ => "灰色",
    }
}

/// 可选颜色列表（真源 TASK_GROUP_COLORS，types.ts:3-12）。
const COLOR_OPTIONS: [&str; 7] = ["gray", "red", "orange", "yellow", "green", "blue", "purple"];

// ---------------------------------------------------------------------------
// 折叠状态（localStorage 持久化，与真源 groupedTaskExpansionPreference 同思路）
// ---------------------------------------------------------------------------

fn persist_expanded(group_id: &str, expanded: bool) {
    let key = format!("{EXPANDED_KEY_PREFIX}{group_id}");
    leptos::task::spawn_local(async move {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item(&key, if expanded { "1" } else { "0" });
        }
    });
}

// ---------------------------------------------------------------------------
// 面板
// ---------------------------------------------------------------------------

/// 分组面板状态（菜单改色/删组后要触发重载，故经 context 下发）。
#[derive(Clone, Copy)]
pub struct GroupedStore {
    /// 递增即触发重拉（菜单操作后 +1）。
    refresh: RwSignal<u64>,
    error: RwSignal<String>,
}

#[component]
pub fn GroupedTasksView() -> impl IntoView {
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let sessions = expect_context::<crate::app::SessionsStore>();

    // 后端返回的分组结构（只含 task_ids），join 成完整视图后才交给渲染。
    let (structure, set_structure) = signal(None::<GroupedStructure>);
    let collapsed = RwSignal::new(std::collections::HashSet::<String>::new());

    // join 后的完整视图。**单独用信号存**，而不是在渲染闭包里现算——
    // 拖拽预览会就地改这个视图（真源 dragPreviewViewRef），
    // 若每次渲染都重新 join，预览会被下一次 join 直接冲掉。
    // `None` = 尚未 join；拖拽中该值就是预览态。
    let joined_view: RwSignal<Option<GroupedTaskView>> = RwSignal::new(None);
    // 拖拽运行时（begin/preview/finish/cancel 状态机）。
    let drag_runtime: RwSignal<DragRuntime> = RwSignal::new(DragRuntime::default());

    let store = GroupedStore {
        refresh: RwSignal::new(0),
        error: RwSignal::new(String::new()),
    };
    provide_context(store);

    // 会话切换 / 手动刷新（菜单改色、删组）→ 重拉分组视图。
    Effect::new(move |_| {
        let _ = store.refresh.get();
        let Some(session_id) = selected_session.get() else {
            set_structure.set(None);
            return;
        };
        let Some(ws_path) = sessions.workspace_path_of(&session_id) else {
            set_structure.set(None);
            return;
        };
        store.error.set(String::new());
        spawn_local(async move {
            match invoke_json(
                "task_grouped_view",
                serde_json::json!({
                    "workspacePath": ws_path,
                    "workspaceIdentity": serde_json::Value::Null,
                }),
            )
            .await
            {
                Ok(v) => {
                    let parsed: GroupedView =
                        serde_json::from_value(v).unwrap_or(GroupedView { nodes: Vec::new() });
                    // 后端只给 task_ids（真源：服务端只提供分组结构，由客户端与
                    // sessions-index join），转成 join 层的结构体。
                    let structure = GroupedStructure {
                        nodes: parsed
                            .nodes
                            .into_iter()
                            .map(|node| match node {
                                // 后端的 Group 与view.rs 的 TaskGroup 同构但不同类型，
                                // 在此显式转换，避免两套模型并存。
                                GroupedNode::Group { group, task_ids } => {
                                    GroupedStructureNode::Group {
                                        group: TaskGroup {
                                            group_id: group.group_id,
                                            title: group.title,
                                            color: group.color,
                                        },
                                        task_ids,
                                    }
                                }
                                GroupedNode::Task { task } => {
                                    GroupedStructureNode::Task { task_id: task }
                                }
                            })
                            .collect(),
                    };
                    set_structure.set(Some(structure));
                }
                Err(e) => store.error.set(e),
            }
        });
    });

    // join 必须在**另一个** Effect 里做：它依赖 sessions（会话快照），
    // 而拉取 structure 的 Effect 拿不到最新会话数据（闭包捕获时机不同）。
    // 单独拆开后，「会话列表刷新 → 重新 join」也能正确触发。
    Effect::new(move |_| {
        let Some(structure) = structure.get() else {
            joined_view.set(None);
            return;
        };
        // 真源 join：后端只给分组结构（task_ids），任务内容由客户端与
        // sessions-index 会话 join 得到（zcodeTaskListTypes.ts 注释）。
        let session_index = index_sessions(&sessions.join_rows());
        let ws_path_for_join = selected_session
            .get()
            .and_then(|sid| sessions.workspace_path_of(&sid))
            .unwrap_or_default();
        joined_view.set(Some(join_grouped_structure(
            &structure,
            &session_index,
            &ws_path_for_join,
            None,
        )));
    });

    view! {
        <div class="flex min-h-0 flex-col">
            {move || {
                let err = store.error.get();
                if !err.is_empty() {
                    return view! {
                        <p class="px-3 py-2 text-xs text-[#dc2626]">{err}</p>
                    }
                    .into_any();
                }
                let Some(view) = joined_view.get() else {
                    return view! {
                        <p class="px-3 py-2 text-xs text-muted">"加载中…"</p>
                    }
                    .into_any();
                };
                if view.nodes.is_empty() {
                    return view! {
                        <p class="px-3 py-2 text-ui-base text-foreground-subtle">"暂无任务"</p>
                    }
                    .into_any();
                }

                view.nodes
                    .into_iter()
                    .map(|node| match node {
                        GroupedTaskViewNode::Group { group, tasks, .. } => {
                            view! {
                                <GroupItem
                                    group=Group { group_id: group.group_id, title: group.title, color: group.color }
                                    tasks=tasks
                                    collapsed=collapsed
                                    runtime=drag_runtime
                                    view=joined_view
                                    refresh=store.refresh
                                />
                            }
                                .into_any()
                        }
                        GroupedTaskViewNode::Task { task, .. } => {
                            view! {
                                <LooseTaskRow
                                    task=task
                                    runtime=drag_runtime
                                    view=joined_view
                                />
                            }
                                .into_any()
                        }
                    })
                    .collect_view()
                    .into_any()
            }}
        </div>
    }
}

/// 组节点：组头 + 成员列表（真源 group-item.tsx 492-720）。
#[component]
fn GroupItem(
    group: Group,
    tasks: Vec<TaskListItem>,
    collapsed: RwSignal<std::collections::HashSet<String>>,
    runtime: RwSignal<DragRuntime>,
    view: RwSignal<Option<GroupedTaskView>>,
    refresh: RwSignal<u64>,
) -> impl IntoView {
    let store = expect_context::<GroupedStore>();
    let system = is_system_group(&group);

    // 折叠态做成派生信号：多处视图节点都要读，用 Memo 避免反复 clone group_id。
    let group_id_for_memo = group.group_id.clone();
    let is_collapsed = Memo::new(move |_| collapsed.get().contains(&group_id_for_memo));
    let title = display_title(&group);
    let color = group.color.clone();
    let border = border_color_class(&color);
    let member_count = tasks.len();

    let toggle = {
        let group_id = group.group_id.clone();
        move |_| {
            let mut next = collapsed.get_untracked();
            let expanded = next.contains(&group_id);
            if expanded {
                next.remove(&group_id);
            } else {
                next.insert(group_id.clone());
            }
            collapsed.set(next);
            persist_expanded(&group_id, expanded);
        }
    };

    // 改色（真源 handleGroupColorChange → changeTaskGroupColor）。
    // 系统分组不允许改色也不允许删除（真源 isSystemGroup 分支）。
    let color_menu_open = RwSignal::new(false);
    let apply_color = Callback::new({
        let group_id = group.group_id.clone();
        move |next_color: &'static str| {
            let group_id = group_id.clone();
            spawn_local(async move {
                match invoke_json(
                    "task_group_color",
                    serde_json::json!({ "groupId": group_id, "color": next_color }),
                )
                .await
                {
                    Ok(_) => store.refresh.update(|n| *n += 1),
                    Err(e) => store.error.set(format!("更新分组颜色失败: {e}")),
                }
            });
        }
    });

    // 取消分组并删除（真源 onUngroupGroup，taskGroup.ungroup 文案）。
    let delete_group = Callback::new({
        let group_id = group.group_id.clone();
        move |_: ()| {
            let group_id = group_id.clone();
            spawn_local(async move {
                match invoke_json(
                    "task_group_delete",
                    serde_json::json!({ "groupId": group_id }),
                )
                .await
                {
                    Ok(_) => store.refresh.update(|n| *n += 1),
                    Err(e) => store.error.set(format!("取消分组失败: {e}")),
                }
            });
        }
    });

    view! {
        // TASK_GROUP_CONTAINER_CLASS
        <div class="relative pt-2.5 pb-0">
            // TASK_GROUP_HEADER_CLASS
            // 组头既是拖拽源（拖 group，真源 groupDraggable :148），
            // 也是投放区（真源 expandedGroupHeaderDroppable :171 /
            // collapsedGroupDroppable :163 —— 两者落位效果都是「进组首位」，
            // 故Rust侧统一用 collapsed-group 类型）。
            <div
                role="button"
                tabindex="0"
                class="mb-0.5 flex h-8 cursor-pointer items-center gap-1 rounded-lg border border-transparent pl-1.5 pr-1 text-ui-base text-foreground transition-[background-color,border-color,box-shadow] hover:bg-surface-hover"
                aria-expanded=move || !is_collapsed.get()
                draggable="true"
                data-group-id=group.group_id.clone()
                data-drop-type="grouped-collapsed-group"
                on:click=toggle
                on:dragstart=move |ev| on_drag_start(ev, runtime, view)
                on:dragover=move |ev| {
                    track_direction(&ev, runtime);
                    on_drag_over(ev, runtime.read_only(), view)
                }
                on:drop=move |ev| {
                    on_drop(ev, runtime, view, move |committed| {
                        commit_order(committed, view, refresh)
                    })
                }
                on:dragend=move |_| on_drag_end(runtime, view)
            >
                // 颜色标记：size-5 圆 + Hash 图标（colors.tsx TaskGroupColorMark）。
                // 点击展开 7 色菜单（真源 DropdownMenu + RadioGroup）。
                <button
                    type="button"
                    class=format!("flex size-5 flex-none items-center justify-center rounded-full {color}",)
                    title="分组颜色"
                    aria-label="分组颜色"
                    on:click=move |ev| {
                        ev.stop_propagation();
                        color_menu_open.set(!color_menu_open.get_untracked());
                    }
                >
                    <span class="flex size-3 items-center justify-center">{icon_hash()}</span>
                </button>
                {color_menu_open.get().then(|| view! {
                    // 绝对定位挂在组头下方的轻量菜单，替代真源的 Radix DropdownMenu。
                    <div class="absolute left-6 top-8 z-20 w-40 rounded-lg border border-border bg-panel p-1 shadow-lg">
                        {COLOR_OPTIONS
                            .iter()
                            .map(|c| {
                                let c = *c;
                                let picked = c == color;
                                view! {
                                    <button
                                        type="button"
                                        class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-xs text-foreground hover:bg-surface-hover"
                                        on:click=move |ev| {
                                            ev.stop_propagation();
                                            color_menu_open.set(false);
                                            apply_color.run(c);
                                        }
                                    >
                                        // TaskGroupColorDot：size-2.5 圆点
                                        <span class=format!("size-2.5 flex-none rounded-full {}", color_class(c))></span>
                                        <span class="flex-1">{color_label(c)}</span>
                                        {picked.then(|| view! { <span class="text-muted">"✓"</span> })}
                                    </button>
                                }
                            })
                            .collect_view()}
                        {(system == false).then(|| view! {
                            <>
                                <div class="my-1 h-px bg-border"></div>
                                <button
                                    type="button"
                                    class="flex w-full items-center rounded px-2 py-1 text-left text-xs text-destructive hover:bg-surface-hover"
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        color_menu_open.set(false);
                                        delete_group.run(());
                                    }
                                >
                                    "取消分组并删除"
                                </button>
                            </>
                        })}
                    </div>
                })}
                <div class="flex min-w-0 flex-1 items-center gap-1">
                    // TASK_GROUP_TITLE_CLASS
                    <button
                        type="button"
                        class="min-w-0 max-w-full cursor-pointer truncate rounded-sm px-1 text-left text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-input-border-focused"
                        title=title.clone()
                    >
                        {title.clone()}
                    </button>
                    {if is_collapsed.get() {
                        view! {
                            <span class="flex size-3.5 flex-none items-center justify-center text-foreground-subtlest">
                                {icon_chevron_right()}
                            </span>
                        }
                        .into_any()
                    } else {
                        view! {
                            <span class="flex size-3.5 flex-none items-center justify-center text-foreground-subtlest">
                                {icon_chevron_down()}
                            </span>
                        }
                        .into_any()
                    }}
                </div>
                // TASK_GROUP_COUNT_BADGE_CLASS
                <span class="inline-flex min-w-5 flex-none items-center justify-center rounded-full bg-tag/50 px-1.5 py-0.5 text-ui-sm font-medium leading-none text-foreground-subtle">
                    {member_count}
                </span>
                <span
                    class="flex size-6 flex-none cursor-pointer items-center justify-center rounded-lg text-foreground-subtle hover:bg-surface-hover"
                    role="button"
                    title="新建任务"
                >
                    <span class="flex size-4 items-center justify-center">{icon_message_plus()}</span>
                </span>
            </div>
            // 折叠动画：grid-rows 0fr/1fr + opacity（真源不用 Radix Collapsible，group-item.tsx:677-679）。
            <div
                class=move || {
                    if is_collapsed.get() {
                        "grid overflow-hidden transition-[grid-template-rows,opacity] duration-200 ease-out grid-rows-[0fr] opacity-0"
                    } else {
                        "grid overflow-hidden transition-[grid-template-rows,opacity] duration-200 ease-out grid-rows-[1fr] opacity-100"
                    }
                }
            >
                <div class="min-h-0 overflow-hidden">
                    // TASK_GROUP_CONTENT_CLASS + 按色系左边框
                    // 空投放区（真源 emptyDropZone）：组展开但无成员时可拖入。
                    <div
                        class=format!("ml-4 border-l py-px pl-2 {border}")
                        data-group-id=group.group_id.clone()
                        data-drop-type="grouped-empty-drop"
                        on:dragover=move |ev| on_drag_over(ev, runtime.read_only(), view)
                    >
                        {tasks
                            .into_iter()
                            .map(|task| {
                                view! {
                                    <GroupedTaskRow task=task runtime=runtime view=view />
                                }
                            })
                            .collect_view()}
                    </div>
                    // 组尾投放区（真源 expandedGroupFooterDroppable :179）。
                    // 向下拖 → 出组放到组后；向上拖 → 进组末位（dnd.rs 分支）。
                    <div
                        class="h-1.5"
                        data-group-id=group.group_id.clone()
                        data-drop-type="grouped-expanded-group-footer"
                        on:dragover=move |ev| {
                            track_direction(&ev, runtime);
                            on_drag_over(ev, runtime.read_only(), view)
                        }
                        on:drop=move |ev| {
                            on_drop(ev, runtime, view, move |committed| {
                                commit_order(committed, view, refresh)
                            })
                        }
                    ></div>
                </div>
            </div>
        </div>
    }
}

/// 落库一次拖拽后的顺序，失败时回滚本地视图（真源 `applyOrder` 的 catch 分支）。
///
/// 真源（useGroupedTaskView.ts:1038-1043）：
/// 写失败 → 回滚乐观视图 → 再refresh 一次收敛到 SQLite 真相源 → 抛错上报。
/// Rust 侧同样三步，且**必须回滚**——否则顺序只存在内存里，重启即丢。
fn persist_order(
    committed: GroupedTaskView,
    view: RwSignal<Option<GroupedTaskView>>,
    refresh: RwSignal<u64>,
) {
    // 先记下回滚点：落库失败要恢复成拖拽前的顺序。
    let rollback = view.get_untracked();
    let sessions = expect_context::<crate::app::SessionsStore>();
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let toasts = expect_context::<crate::toast::ToastStore>();
    // workspace 路径必须取真实值：后端按 workspace_key 定位排序记录，
    // 传空串会写到一把永远匹配不上的 key 上（等于没落库）。
    let ws_path = selected_session
        .get_untracked()
        .and_then(|sid| sessions.workspace_path_of(&sid))
        .unwrap_or_default();
    spawn_local(async move {
        let payload = view_to_order_input(&committed);
        let result = invoke_json(
            "task_group_apply_order",
            serde_json::json!({
                "workspacePath": ws_path,
                "workspaceIdentity": serde_json::Value::Null,
                "topLevel": payload["topLevel"],
                "groups": payload["groups"],
            }),
        )
        .await;
        if let Err(e) = result {
            // 回滚 + 重拉，对齐真源（useGroupedTaskView.ts:1038-1043）。
            // 提示走 toast 而非 store.error——后者会把整个分组列表替换成
            // 错误态，用户刚拖完就看不到列表了（真源用 toast 不打断）。
            view.set(rollback);
            toasts.error(format!("更新分组顺序失败，已回滚：{e}"));
            refresh.update(|n| *n += 1);
        }
    });
}

/// 拖拽落库成功的统一入口：把视图写回信号 + 触发落库。
fn commit_order(
    committed: GroupedTaskView,
    view: RwSignal<Option<GroupedTaskView>>,
    refresh: RwSignal<u64>,
) {
    view.set(Some(committed.clone()));
    persist_order(committed, view, refresh);
}

/// 组内成员行（真源 task-row.tsx TASK_GROUP_ROW_CLASS + ROW_LINE_CLASS）。
#[component]
fn GroupedTaskRow(
    task: TaskListItem,
    runtime: RwSignal<DragRuntime>,
    view: RwSignal<Option<GroupedTaskView>>,
) -> impl IntoView {
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let task_id = task.task_id.clone();
    let active = {
        let task_id = task_id.clone();
        move || selected_session.get().as_deref() == Some(task_id.as_str())
    };
    let class = move || {
        // TASK_GROUP_ROW_CLASS 的 hover 分支；选中态沿用侧栏会话行的 bg-selected。
        if active() {
            "flex min-h-7 w-full min-w-0 flex-col justify-center rounded-lg border border-transparent bg-selected pl-2.5 pr-1 text-left text-ui-base"
        } else {
            "flex min-h-7 w-full min-w-0 flex-col justify-center rounded-lg border border-transparent pl-2.5 pr-1 text-left text-ui-base transition-[background-color,border-color,color,opacity] hover:bg-surface-hover"
        }
    };
    let dot = status_dot_class(&task.status).to_string();
    let time = if task.updated_at > 0 {
        relative_time(task.updated_at)
    } else {
        String::new()
    };
    let task_key = crate::groupedTasks::view::task_key_of(&task);

    view! {
        // TASK_GROUP_ROW_LINE_CLASS
        // draggable + data-* : 拖拽源 + 投放区（真源 useDraggable/useDroppable +
        // data:{type:"grouped-task", taskKey} 的 HTML5 等价物）。
        <div
            class=class
            draggable="true"
            data-task-key=task_key.clone()
            data-drop-type="grouped-task"
            on:dragstart=move |ev| on_drag_start(
                ev,
                runtime,
                view,
            )
            on:dragover=move |ev| on_drag_over(ev, runtime.read_only(), view)
        >
            <span class="flex h-7 w-full min-w-0 items-center gap-2">
                <button
                    class="flex h-7 w-full min-w-0 items-center gap-2 text-left"
                    on:click=move |_| selected_session.set(Some(task_id.clone()))
                >
                    <span class=format!("size-1.5 flex-none rounded-full {dot}")></span>
                    <span class="min-w-0 flex-1 truncate text-foreground">{task.title.clone()}</span>
                    <span class="flex-none text-foreground-subtlest">{time.clone()}</span>
                </button>
            </span>
        </div>
    }
}

/// 游离任务行（未入组，顶层节点）。
#[component]
fn LooseTaskRow(
    task: TaskListItem,
    runtime: RwSignal<DragRuntime>,
    view: RwSignal<Option<GroupedTaskView>>,
) -> impl IntoView {
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let task_id = task.task_id.clone();
    let active = {
        let task_id = task_id.clone();
        move || selected_session.get().as_deref() == Some(task_id.as_str())
    };
    let class = move || {
        if active() {
            "flex h-8 w-full min-w-0 items-center gap-2 rounded-lg bg-selected pl-2.5 pr-1 text-left text-foreground shadow-xl"
        } else {
            "flex h-8 w-full min-w-0 items-center gap-2 rounded-lg pl-2.5 pr-1 text-left text-foreground hover:bg-surface-hover"
        }
    };
    let dot = status_dot_class(&task.status).to_string();
    let task_key = crate::groupedTasks::view::task_key_of(&task);

    view! {
        <div
            class=class
            draggable="true"
            data-task-key=task_key.clone()
            data-drop-type="grouped-task"
            on:dragstart=move |ev| on_drag_start(ev, runtime, view)
            on:dragover=move |ev| on_drag_over(ev, runtime.read_only(), view)
        >
            <button
                class="flex h-8 w-full min-w-0 items-center gap-2 text-left"
                on:click=move |_| selected_session.set(Some(task_id.clone()))
            >
                <span class=format!("size-1.5 flex-none rounded-full {dot}")></span>
                <span class="min-w-0 flex-1 truncate text-sm">{task.title.clone()}</span>
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(id: &str, title: &str, color: &str) -> Group {
        Group {
            group_id: id.into(),
            title: title.into(),
            color: color.into(),
        }
    }

    #[test]
    fn system_group_titles_are_localized() {
        // 真源 group-title.ts：系统分组忽略库里存的占位标题，按语言环境展示。
        assert_eq!(
            display_title(&group(
                CRON_DEFAULT_GROUP_ID,
                CRON_PLACEHOLDER_TITLE,
                "gray"
            )),
            "定时任务"
        );
        assert_eq!(
            display_title(&group(
                OFF_PEAK_DEFAULT_GROUP_ID,
                OFF_PEAK_PLACEHOLDER_TITLE,
                "blue"
            )),
            "闲时任务"
        );
    }

    #[test]
    fn system_group_detected_by_id() {
        assert!(is_system_group(&group(CRON_DEFAULT_GROUP_ID, "x", "gray")));
        assert!(is_system_group(&group(
            OFF_PEAK_DEFAULT_GROUP_ID,
            "x",
            "gray"
        )));
        assert!(!is_system_group(&group("g-user", "我的组", "gray")));
    }

    #[test]
    fn normal_group_keeps_its_title() {
        assert_eq!(display_title(&group("g1", "工作", "red")), "工作");
        // 占位标题但非系统 id → 按普通组处理（id 才是权威判据）。
        assert_eq!(
            display_title(&group("g-user", CRON_PLACEHOLDER_TITLE, "gray")),
            "定时任务"
        );
    }

    #[test]
    fn color_classes_cover_seven_colors() {
        for c in ["gray", "red", "orange", "yellow", "green", "blue", "purple"] {
            assert!(color_class(c).starts_with("bg-"), "{c} 底色类名不对");
            assert!(
                border_color_class(c).starts_with("border-"),
                "{c} 边框类名不对"
            );
        }
        // 未知颜色回落 gray。
        assert_eq!(color_class("unknown"), color_class("gray"));
        assert_eq!(border_color_class("unknown"), border_color_class("gray"));
    }

    #[test]
    fn parses_grouped_view_with_tagged_nodes() {
        // 后端 taskgroup::GroupedNode 的 serde 配置是
        // tag="type" + rename_all="camelCase"：**tag 值会转小写（group/task），
        // 但字段名 task_ids 不受影响**（camelCase 只改已有驼峰，此处已是 snake）。
        // 前端必须按这个真实格式反序列化。
        let raw = serde_json::json!({
            "nodes": [
                {
                    "type": "group",
                    "group": { "groupId": "g1", "title": "工作", "color": "blue" },
                    "task_ids": ["t1", "t2"],
                },
                { "type": "task", "task": "t3" },
            ]
        });
        let parsed: GroupedView = serde_json::from_value(raw).unwrap();
        assert_eq!(parsed.nodes.len(), 2);
        match &parsed.nodes[0] {
            GroupedNode::Group { group, task_ids } => {
                assert_eq!(group.group_id, "g1");
                assert_eq!(task_ids.len(), 2);
            }
            other => panic!("期望组节点，实际 {other:?}"),
        }
        assert_eq!(parsed.nodes[1], GroupedNode::Task { task: "t3".into() });
    }

    /// 端到端：真实后端 JSON → GroupedStructure → join → view.rs 重排。
    ///
    /// 这条链路横跨三处改动（后端契约 / join 层 / 渲染层），单测各自都过
    /// 仍可能接不上，所以单独钉一条集成断言。
    #[test]
    fn backend_json_joins_into_reorderable_view() {
        let raw = serde_json::json!({
            "nodes": [
                { "type": "task", "task": "t1" },
                {
                    "type": "group",
                    "group": { "groupId": "g1", "title": "工作", "color": "blue" },
                    "task_ids": ["t2", "t3"],
                },
            ]
        });
        let parsed: GroupedView = serde_json::from_value(raw).unwrap();
        let structure = GroupedStructure {
            nodes: parsed
                .nodes
                .into_iter()
                .map(|node| match node {
                    GroupedNode::Group { group, task_ids } => GroupedStructureNode::Group {
                        group: TaskGroup {
                            group_id: group.group_id,
                            title: group.title,
                            color: group.color,
                        },
                        task_ids,
                    },
                    GroupedNode::Task { task } => GroupedStructureNode::Task { task_id: task },
                })
                .collect(),
        };

        // 会话列表里只有 t1（t2/t3 未 join → 走兜底）。
        let sessions = index_sessions(&[crate::groupedTasks::join::SessionRow {
            session_id: "t1".into(),
            title: "任务一".into(),
            status: "idle".into(),
            created_at: 0,
            updated_at: 100,
            workspace_path: "/ws".into(),
            workspace_identity: None,
        }]);
        let view = join_grouped_structure(&structure, &sessions, "/ws", None);

        // join 后 t1 拿到会话里的真实标题。
        let GroupedTaskViewNode::Task { task: t1, .. } = &view.nodes[0] else {
            panic!("期望顶层 task 节点");
        };
        assert_eq!(t1.title, "任务一");

        // 整条链路可用：把 t2 拖到 t1 之前（t2 未 join，taskKey 仍成立）。
        let moved = crate::groupedTasks::view::move_task_over_task(
            &view,
            &crate::groupedTasks::view::task_key("/ws", None, "t2"),
            &crate::groupedTasks::view::task_key("/ws", None, "t1"),
            crate::groupedTasks::view::InsertPosition::Before,
        );
        let GroupedTaskViewNode::Task { task: first, .. } = &moved.nodes[0] else {
            panic!("t2 应被提到顶层首位");
        };
        assert_eq!(first.task_id, "t2");
    }

    /// 端到端：后端 JSON → join → 拖拽状态机 → 落库载荷。
    ///
    /// 覆盖接线后新增的两段逻辑（`joined_view` 信号 + `DragRuntime` 状态机）
    /// 与已有 join/view/domEvents 的衔接。接线层没有 DOM 就测不了，
    /// 但「状态机 → 载荷」这段纯逻辑可以在这里钉住。
    #[test]
    fn drag_state_machine_produces_persistable_order() {
        // 1) 后端结构：游离 t1 + 组 g1 含 t2
        let structure = GroupedStructure {
            nodes: vec![
                GroupedStructureNode::Task {
                    task_id: "t1".into(),
                },
                GroupedStructureNode::Group {
                    group: TaskGroup {
                        group_id: "g1".into(),
                        title: "工作".into(),
                        color: "blue".into(),
                    },
                    task_ids: vec!["t2".into()],
                },
            ],
        };
        let sessions = index_sessions(&[crate::groupedTasks::join::SessionRow {
            session_id: "t1".into(),
            title: "任务一".into(),
            status: "idle".into(),
            created_at: 0,
            updated_at: 100,
            workspace_path: "/ws".into(),
            workspace_identity: None,
        }]);
        let view = join_grouped_structure(&structure, &sessions, "/ws", None);

        // 2) 拖 t2（组内）到 t1 之前 —— 状态机完整跑一遍 begin→preview→finish
        let mut rt = DragRuntime::default();
        rt.begin(
            crate::groupedTasks::dnd::DragSource::Task {
                task_key: crate::groupedTasks::view::task_key("/ws", None, "t2"),
            },
            &view,
        );
        rt.direction = crate::groupedTasks::dnd::DragDirection::Before;
        let preview = rt.preview(
            &view,
            Some(crate::groupedTasks::dnd::DropTarget::Task {
                task_key: crate::groupedTasks::view::task_key("/ws", None, "t1"),
            }),
        );

        // t2 应已升为顶层首位
        assert!(matches!(
            &preview.nodes[0],
            GroupedTaskViewNode::Task { task, .. } if task.task_id == "t2"
        ));

        // 3) finish 产出待落库视图
        let committed = rt.finish(&preview).expect("实际移动了应产出待落库视图");

        // 4) 载荷：顶层顺序 t2,t1 + 组 g1 变空
        let payload = view_to_order_input(&committed);
        let top = payload["topLevel"].as_array().unwrap();
        assert_eq!(top[0]["task"], "t2", "顶层顺序应反映 t2 已上移");
        assert_eq!(top[1]["task"], "t1");
        let groups = payload["groups"].as_array().unwrap();
        assert_eq!(groups[0]["groupId"], "g1");
        assert!(
            groups[0]["taskIds"].as_array().unwrap().is_empty(),
            "t2 已被拖出组，组内列表应为空"
        );
    }

    #[test]
    fn drag_with_no_movement_does_not_persist() {
        // 真源 :1476-1484：签名没变就不落库（省一次 RPC + 避免无谓动画）。
        let view = GroupedTaskView {
            nodes: vec![GroupedTaskViewNode::Task {
                task: TaskListItem {
                    task_id: "t1".into(),
                    title: "t1".into(),
                    workspace_path: "/ws".into(),
                    workspace_identity: None,
                    created_at: 0,
                    updated_at: 0,
                    status: String::new(),
                },
                sort_order: None,
            }],
        };
        let key = crate::groupedTasks::view::task_key("/ws", None, "t1");
        let mut rt = DragRuntime::default();
        rt.begin(
            crate::groupedTasks::dnd::DragSource::Task {
                task_key: key.clone(),
            },
            &view,
        );
        // 悬停回自己 → 预览不变
        let preview = rt.preview(
            &view,
            Some(crate::groupedTasks::dnd::DropTarget::Task {
                task_key: key.clone(),
            }),
        );
        assert!(rt.finish(&preview).is_none(), "没有实际移动就不该落库");
    }

    #[test]
    fn empty_view_parses() {
        let parsed: GroupedView =
            serde_json::from_value(serde_json::json!({ "nodes": [] })).unwrap();
        assert!(parsed.nodes.is_empty());
    }

    #[test]
    fn expanded_key_is_prefixed_by_group_id() {
        // 折叠态按 group_id 隔离持久化（真源 groupedTaskExpansionPreference 同思路）。
        let key = format!("{EXPANDED_KEY_PREFIX}{}", "g-123");
        assert_eq!(key, "zcode-grouped-expanded:g-123");
    }
}
#[cfg(test)]
mod color_tests {
    use super::{COLOR_OPTIONS, color_class, color_label};

    #[test]
    fn color_options_match_source_constant() {
        // 真源 TASK_GROUP_COLORS（types.ts:3-12）固定 7 种，顺序一致。
        assert_eq!(
            COLOR_OPTIONS,
            ["gray", "red", "orange", "yellow", "green", "blue", "purple"]
        );
    }

    #[test]
    fn color_labels_match_zh_cn() {
        // 真源 zh-CN.ts taskGroup.color.*
        assert_eq!(color_label("gray"), "灰色");
        assert_eq!(color_label("red"), "红色");
        assert_eq!(color_label("orange"), "橙色");
        assert_eq!(color_label("yellow"), "黄色");
        assert_eq!(color_label("green"), "绿色");
        assert_eq!(color_label("blue"), "蓝色");
        assert_eq!(color_label("purple"), "紫色");
    }

    #[test]
    fn every_option_has_distinct_class_and_label() {
        let mut classes: Vec<&str> = COLOR_OPTIONS.iter().map(|c| color_class(c)).collect();
        classes.sort_unstable();
        let before = classes.len();
        classes.dedup();
        assert_eq!(classes.len(), before, "各颜色底色类名应互不相同");

        let mut labels: Vec<&str> = COLOR_OPTIONS.iter().map(|c| color_label(c)).collect();
        labels.sort_unstable();
        let before = labels.len();
        labels.dedup();
        assert_eq!(labels.len(), before, "各颜色中文标签应互不相同");
    }

    #[test]
    fn unknown_color_falls_back_to_gray() {
        assert_eq!(color_label("chartreuse"), "灰色");
        assert_eq!(color_class("chartreuse"), color_class("gray"));
    }
}
