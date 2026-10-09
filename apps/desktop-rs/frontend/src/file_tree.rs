//! 树形文件树面板（1:1 翻译 `packages/ui/src/workspace-file-tree/`）。
//!
//! 对照真源：
//! - 面板外壳 `WorkspaceFileTree.tsx:630-765`
//! - 行渲染 `WorkspaceFileTreeRowView.tsx:213-304`
//! - 数据层 `useWorkspaceFileTreeData.ts:80-92`（expanded/loaded/loading/error 六个集合）
//! - 平铺 `model.ts:435-461`（DFS：树 → 扁平 row 列表，再渲染）
//! - git 状态 / watcher / 过滤：`workspace_file_tree/` 子模块（model.ts 纯层、
//!   gitStatus.ts、useWorkspaceFileTreeWatchers.ts、statusStyles.ts）
//!
//! 差异（已知取舍，逐条标注）：
//! - **虚拟滚动**：真源用 @tanstack/react-virtual（28px 定高、overscan 12）。
//!   Rust 侧按同参数手写窗口化：scrollTop/clientHeight 信号 + 总高占位 + 可见切片。
//! - **compact folder 自动压平**：已实现（flatten_compact）。
//! - **搜索**：真源走全局文件索引 IPC（useWorkspaceFileSearchIndex），本轮只过滤已加载的树节点。
//! - **拖拽 / 右键菜单**：真源 Radix ContextMenu + dnd，未迁。

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::{HashMap, HashSet};
use wasm_bindgen::JsCast;

use crate::app::Icon;
use crate::file_icons::{icon_src, resolve_icon_name};
use crate::workspace_file_tree::gitStatus::{
    load_workspace_file_tree_git_status, WorkspaceFileTreeGitStatusState,
};
use crate::workspace_file_tree::model::{
    add_deleted_git_status_rows_to_tree, get_workspace_directory_git_statuses,
    get_workspace_file_git_status, is_workspace_file_tree_deleted_file, GitDeletedNode,
    WorkspaceFileGitStatus,
};
use crate::workspace_file_tree::statusStyles::{
    get_workspace_file_git_status_dot_class_name, get_workspace_file_git_status_indicator,
    get_workspace_file_git_status_indicator_class_name, get_workspace_file_git_status_text_class_name,
    get_workspace_file_tree_row_display_git_status,
};
use crate::workspace_file_tree::useWorkspaceFileTreeWatchers::{WatcherCommand, WatcherRegistry};

/// 真源 constants.ts:1-2 —— 行高 28px，gap 0（虚拟估高与 h-7 双向绑定）。
const ROW_HEIGHT_PX: usize = 28;
/// 层级步进 0.75rem = 12px（RowView.tsx:223 `pl-[calc(var(--depth)*0.75rem+0.5rem)]`）。
const DEPTH_STEP_REM: f64 = 0.75;

// ---------------------------------------------------------------------------
// 行模型（model.ts:6-20）
// ---------------------------------------------------------------------------

/// 树节点（真源 WorkspaceFileTreeNode）。
#[derive(Debug, Clone)]
struct TreeNode {
    path: String,
    name: String,
    is_dir: bool,
    is_symlink: bool,
    depth: usize,
}

/// 平铺后的行（真源 WorkspaceFileTreeRow）：`expanded` 等是flatten 阶段派生的，不是存储字段。
#[derive(Debug, Clone, PartialEq)]
pub struct TreeRow {
    pub path: String,
    pub name: String,
    is_dir: bool,
    depth: usize,
    expanded: bool,
    loading: bool,
    error: bool,
    /// compact folder 覆盖的全部路径（真源 compactedPaths）：单元素=普通目录，
    /// 多元素=压平链 `a/b/c`。展开/收起按整组处理。
    compacted_paths: Vec<String>,
    /// 自身 git 状态（真源 `gitStatus` prop；文件有、目录无）。
    git_status: Option<WorkspaceFileGitStatus>,
    /// 目录聚合的 descendant 状态表（真源 `directoryGitStatuses`，已按 decoration 优先级排序）。
    dir_statuses: Vec<WorkspaceFileGitStatus>,
    /// deleted 文件（不在文件系统，点开预览被禁）。
    deleted: bool,
}

/// 目录懒加载数据（useWorkspaceFileTreeData.ts:80-92 的 Rust 对应）。
#[derive(Default, Clone)]
struct TreeData {
    children_by_dir: HashMap<String, Vec<TreeNode>>,
    expanded: HashSet<String>,
    loaded: HashSet<String>,
    loading: HashSet<String>,
    error_dirs: HashSet<String>,
}

/// 参与空目录链自动压平的目录（model.ts:124-130）：软链接目录不参与，
/// 否则 self/parent symlink 会让路径去重失效、无限加载 `linked-dir/linked-dir/...`。
fn is_auto_flattenable(node: &TreeNode) -> bool {
    node.is_dir && !node.is_symlink
}

// ---------------------------------------------------------------------------
// 路径工具（model.ts:274-286 getWorkspaceFileDirectoryChildDepth）
// ---------------------------------------------------------------------------

fn normalize_for_compare(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

/// 物理深度：相对 workspace 的层级数。写入 children 时用，compact 压平后再扣回视觉深度。
fn child_depth(workspace: &str, dir_path: &str) -> usize {
    let ws = normalize_for_compare(workspace);
    let dir = normalize_for_compare(dir_path);
    if dir == ws || !dir.starts_with(&ws) {
        return 0;
    }
    dir[ws.len() + 1..]
        .split('/')
        .filter(|s| !s.is_empty())
        .count()
}

/// 相对 workspace 的展示路径（行 title / 预览头用）。
fn relative_path(workspace: &str, path: &str) -> String {
    let ws = normalize_for_compare(workspace);
    let dir = normalize_for_compare(path);
    if dir.starts_with(&ws) {
        dir[ws.len() + 1..].to_string()
    } else {
        path.to_string()
    }
}

// ---------------------------------------------------------------------------
// 树数据操作
// ---------------------------------------------------------------------------

impl TreeData {
    /// 是否还需要加载（真源 loadDirectory 的去重，useWorkspaceFileTreeData.ts:195-201）。
    fn needs_load(&self, dir_path: &str, force: bool) -> bool {
        force || (!self.loaded.contains(dir_path) && !self.loading.contains(dir_path))
    }

    /// 标记进入 loading（同步部分）。异步读取在 `load_directory` 里做。
    fn mark_loading(&mut self, dir_path: &str) {
        self.loading.insert(dir_path.to_string());
        self.error_dirs.remove(dir_path);
    }

    /// 写入加载结果。
    fn apply_result(&mut self, dir_path: &str, entries: Vec<TreeNode>) {
        self.children_by_dir.insert(dir_path.to_string(), entries);
        self.loaded.insert(dir_path.to_string());
        self.loading.remove(dir_path);
    }

    /// 标记加载失败（行内显示 AlertCircle，model.ts 的 error 字段）。
    fn apply_error(&mut self, dir_path: &str) {
        self.loading.remove(dir_path);
        self.error_dirs.insert(dir_path.to_string());
    }
}

/// 异步加载目录子项。真源走 `fileService.readdir({ includeHidden: true })`
/// （useWorkspaceFileTreeData.ts:222-225）。
///
/// 竞态防护：真源用 workspaceGeneration / requestVersion / directoryRequestVersion 三层版本号
/// （useWorkspaceFileTreeData.ts:71-74）。Rust 侧目录以 path 为唯一 key、同 key 后写覆盖先写，
/// 且首版不接 watcher（无并发刷新源），暂不引版本号——接 watcher 时必须补上。
async fn load_directory(data: RwSignal<TreeData>, workspace: String, dir_path: String) {
    // 递归 async fn 需装箱，否则 future 类型无限大。
    Box::pin(load_directory_inner(data, workspace, dir_path)).await
}

async fn load_directory_inner(data: RwSignal<TreeData>, workspace: String, dir_path: String) {
    let mut current = data.get_untracked();
    current.mark_loading(&dir_path);
    data.set(current);

    let result = crate::app::invoke_json(
        "fs_list",
        serde_json::json!({ "path": dir_path, "includeHidden": true }),
    )
    .await;
    let mut current = data.get_untracked();
    let mut preload: Option<TreeNode> = None;
    match result {
        Ok(v) => {
            let entries = parse_entries(&v, child_depth(&workspace, &dir_path));
            // compact folder 预加载：若目录里只有一个可压平子目录，顺链继续读
            // （真源 useWorkspaceFileTreeData.ts:278-284，静默预加载不打扰用户）。
            if entries.len() == 1 && is_auto_flattenable(&entries[0]) {
                preload = Some(entries[0].clone());
            }
            current.apply_result(&dir_path, entries);
        }
        Err(_) => current.apply_error(&dir_path),
    }
    data.set(current);

    if let Some(next) = preload {
        // 已加载过则停住，避免软链接链无限下钻。
        if !data.get_untracked().loaded.contains(&next.path) {
            load_directory(data, workspace, next.path).await;
        }
    }
}

fn parse_entries(v: &serde_json::Value, depth: usize) -> Vec<TreeNode> {
    let mut nodes: Vec<TreeNode> = v["entries"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|e| {
                    let path = e["path"].as_str()?.to_string();
                    Some(TreeNode {
                        name: e["name"].as_str().unwrap_or("").to_string(),
                        is_dir: e["isDir"].as_bool().unwrap_or(false),
                        is_symlink: e["isSymbolicLink"].as_bool().unwrap_or(false),
                        depth,
                        path,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    // 目录在前、同类按名称排序（后端已排，这里兜底防排序规则漂移）。
    nodes.sort_by(|a, b| {
        (!a.is_dir)
            .cmp(&!b.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    nodes
}

/// DFS 平铺的**非压平**版本（model.ts:435-461）。
///
/// 实际渲染走 `flatten_compact`（压平单子目录链）。此版本保留为基础形态：
/// 层级逻辑本身更直观，单测用它验证"只在expanded 目录下降"这条规则。
#[cfg(test)]
fn flatten(root: &str, data: &TreeData) -> Vec<TreeRow> {
    let mut rows: Vec<TreeRow> = Vec::new();
    fn visit(dir_path: &str, data: &TreeData, rows: &mut Vec<TreeRow>) {
        let Some(children) = data.children_by_dir.get(dir_path) else {
            return;
        };
        for child in children {
            let expanded = child.is_dir && data.expanded.contains(&child.path);
            rows.push(TreeRow {
                path: child.path.clone(),
                name: child.name.clone(),
                is_dir: child.is_dir,
                depth: child.depth,
                expanded,
                loading: data.loading.contains(&child.path),
                error: data.error_dirs.contains(&child.path),
                compacted_paths: vec![child.path.clone()],
                git_status: None,
                dir_statuses: Vec::new(),
                deleted: false,
            });
            if expanded {
                visit(&child.path, data, rows);
            }
        }
    }
    visit(root, data, &mut rows);
    rows
}

/// DFS 平铺 + compact folder 压平（model.ts:390-461 全量语义）+ git 状态装饰。
///
/// 与 `flatten` 的差异：沿单子目录链合并节点，展示名变`a/b/c`，深度停在最外层，
/// 收起时整组 compacted_paths 都要退出 expanded 集合（否则下次展开只剩内层生效）。
/// git 状态：deleted 文件经 `add_deleted_git_status_rows_to_tree` 注入
/// （model.ts:158-208，文件系统里已经没有这些行）；其余状态在行级挂
/// 自身状态与目录聚合表（真源 useWorkspaceFileTreeRows.ts:26-41 的两步）。
fn flatten_compact(
    root: &str,
    data: &TreeData,
    status_by_path: &HashMap<String, WorkspaceFileGitStatus>,
) -> Vec<TreeRow> {
    // deleted 注入：TreeData 的 children 先转成 model 层的 pairs 形态，
    // 走 model.rs 的 1:1 实现（含目录在前、同名去重），再转回 TreeNode。
    let children_as_pairs: HashMap<String, Vec<(GitDeletedNode, bool)>> = data
        .children_by_dir
        .iter()
        .map(|(dir, nodes)| {
            (
                dir.clone(),
                nodes
                    .iter()
                    .map(|n| {
                        (
                            GitDeletedNode {
                                path: n.path.clone(),
                                name: n.name.clone(),
                                depth: n.depth,
                                is_symlink: n.is_symlink,
                            },
                            n.is_dir,
                        )
                    })
                    .collect(),
            )
        })
        .collect();
    let injected = add_deleted_git_status_rows_to_tree(root, &children_as_pairs, status_by_path);
    let children_by_dir: HashMap<String, Vec<TreeNode>> = injected
        .into_iter()
        .map(|(dir, pairs)| {
            (
                dir,
                pairs
                    .into_iter()
                    .map(|(node, is_dir)| TreeNode {
                        path: node.path,
                        name: node.name,
                        is_dir,
                        is_symlink: node.is_symlink,
                        depth: node.depth,
                    })
                    .collect(),
            )
        })
        .collect();

    let mut rows: Vec<TreeRow> = Vec::new();

    // 沿链压平：返回(展示节点, 该链覆盖的全部路径, 子内容深度偏移)。
    fn compact(
        node: &TreeNode,
        children_by_dir: &HashMap<String, Vec<TreeNode>>,
        data: &TreeData,
    ) -> (TreeNode, Vec<String>, usize) {
        if !is_auto_flattenable(node) {
            return (node.clone(), vec![node.path.clone()], 0);
        }
        let mut chain = vec![node.clone()];
        let mut current = node.clone();
        while data.loaded.contains(&current.path)
            && !data.loading.contains(&current.path)
            && !data.error_dirs.contains(&current.path)
        {
            let Some(children) = children_by_dir.get(&current.path) else {
                break;
            };
            if children.len() != 1 || !is_auto_flattenable(&children[0]) {
                break;
            }
            current = children[0].clone();
            chain.push(current.clone());
        }
        if chain.len() == 1 {
            return (node.clone(), vec![node.path.clone()], 0);
        }
        let paths: Vec<String> = chain.iter().map(|n| n.path.clone()).collect();
        let name = chain
            .iter()
            .map(|n| n.name.as_str())
            .collect::<Vec<_>>()
            .join("/");
        // 展示节点沿用最内层的加载/错误态与展开判定依据（行内 spinner/圆点要反映真实末端目录）。
        let mut merged = current.clone();
        merged.name = name;
        merged.depth = node.depth;
        (merged, paths, current.depth - node.depth)
    }

    fn visit(
        dir_path: &str,
        depth_offset: usize,
        children_by_dir: &HashMap<String, Vec<TreeNode>>,
        data: &TreeData,
        status_by_path: &HashMap<String, WorkspaceFileGitStatus>,
        rows: &mut Vec<TreeRow>,
    ) {
        let Some(children) = children_by_dir.get(dir_path) else {
            return;
        };
        for child in children {
            let (node, compacted_paths, offset) = compact(child, children_by_dir, data);
            let expanded = node.is_dir && compacted_paths.iter().any(|p| data.expanded.contains(p));
            let own_status = get_workspace_file_git_status(status_by_path, &node.path);
            // git 状态三件套：文件自身状态、目录聚合表、deleted 判定
            // （真源 RowView props 由 useWorkspaceFileTreeRows 阶段派生，此处同步计算）。
            let git_status = if node.is_dir { None } else { own_status };
            let dir_statuses = if node.is_dir {
                get_workspace_directory_git_statuses(status_by_path, &node.path)
            } else {
                Vec::new()
            };
            let deleted =
                is_workspace_file_tree_deleted_file(node.is_dir, own_status);
            rows.push(TreeRow {
                path: node.path.clone(),
                name: node.name.clone(),
                is_dir: node.is_dir,
                depth: node.depth.saturating_sub(depth_offset),
                expanded,
                loading: data.loading.contains(&node.path),
                error: data.error_dirs.contains(&node.path),
                compacted_paths,
                git_status,
                dir_statuses,
                deleted,
            });
            if expanded {
                visit(
                    &node.path,
                    depth_offset + offset,
                    children_by_dir,
                    data,
                    status_by_path,
                    rows,
                );
            }
        }
    }

    visit(root, 0, &children_by_dir, data, status_by_path, &mut rows);
    rows
}

// ---------------------------------------------------------------------------
// 图标（lucide 数据取自 node_modules/lucide-react/dist/esm/icons，与 React 版同源）
// ---------------------------------------------------------------------------

fn chevron_right() -> impl IntoView {
    view! { <Icon paths=vec!["m9 18 6-6-6-6"] circles=vec![] /> }
}

fn loader_circle() -> impl IntoView {
    view! { <Icon paths=vec!["M21 12a9 9 0 1 1-6.219-8.56"] circles=vec![] /> }
}

fn circle_alert() -> impl IntoView {
    view! {
        <Icon
            paths=vec![]
            circles=vec![("12", "12", "10")]
        />
    }
}

fn arrow_left() -> impl IntoView {
    view! { <Icon paths=vec!["m12 19-7-7 7-7", "M19 12H5"] circles=vec![] /> }
}

fn icon_search() -> impl IntoView {
    view! { <Icon paths=vec!["m21 21-4.34-4.34"] circles=vec![("11", "11", "8")] /> }
}

fn icon_x() -> impl IntoView {
    view! { <Icon paths=vec!["M18 6 6 18", "m6 6 12 12"] circles=vec![] /> }
}

// ---------------------------------------------------------------------------
// 行组件（WorkspaceFileTreeRowView.tsx:213-304）
// ---------------------------------------------------------------------------

/// 层级引导线竖条（hierarchyGuides.ts:5-17）：
/// `repeating-linear-gradient(to right, var(--color-border) 0 1px, transparent 1px 0.75rem)`。
fn hierarchy_guide(depth: usize) -> impl IntoView {
    let width = format!("calc({depth} * {DEPTH_STEP_REM}rem)");
    let background = format!(
        "repeating-linear-gradient(to right, var(--color-border) 0 1px, transparent 1px {DEPTH_STEP_REM}rem)"
    );
    view! {
        <span
            aria-hidden="true"
            class="pointer-events-none absolute -inset-y-px left-2.5"
            style:background-image=background
            style:width=width
        ></span>
    }
}

/// git 状态的 tooltip 文案（真源 `gitStatusLabelByStatus`，WorkspaceFileTree.tsx:240-250）。
fn git_status_label(status: WorkspaceFileGitStatus) -> String {
    use crate::ToolCallBlocks::i18n;
    i18n::text(match status {
        WorkspaceFileGitStatus::Modified => "git.kind.modified",
        WorkspaceFileGitStatus::Added => "git.kind.added",
        WorkspaceFileGitStatus::Deleted => "git.kind.deleted",
        WorkspaceFileGitStatus::Renamed => "git.kind.renamed",
        WorkspaceFileGitStatus::Untracked => "git.section.untracked",
        WorkspaceFileGitStatus::Ignored => "workspaceFileTree.gitStatus.ignored",
    })
}

#[component]
fn FileTreeRow(
    row: TreeRow,
    workspace: String,
    selected: bool,
    on_select: Callback<String>,
    on_toggle: Callback<Vec<String>>,
    on_preview: Callback<String>,
) -> impl IntoView {
    let is_dir = row.is_dir;
    let depth = row.depth;
    let compacted_paths = row.compacted_paths.clone();
    let file_icon = if is_dir {
        None
    } else {
        // 目录刻意不挂图标（RowView.tsx:116 fileIconSrc 对目录强制 null），只有箭头。
        Some(icon_src(&resolve_icon_name(&row.path)))
    };

    // git 状态装饰（RowView.tsx:117-129）：
    // 文件行 = 状态字母；目录行 = descendant 聚合圆点（loading 期间隐藏避免与 spinner 混义）。
    let display_status =
        get_workspace_file_tree_row_display_git_status(row.git_status, &row.dir_statuses);
    let name_text_class =
        get_workspace_file_git_status_text_class_name(display_status).unwrap_or_default();
    let indicator = row
        .git_status
        .and_then(get_workspace_file_git_status_indicator);
    let indicator_class = row
        .git_status
        .and_then(get_workspace_file_git_status_indicator_class_name)
        .unwrap_or_default();
    let indicator_label = row.git_status.map(git_status_label);
    let show_dir_dot = !row.loading && !row.dir_statuses.is_empty();
    let dot_class = get_workspace_file_git_status_dot_class_name(row.dir_statuses.first().copied());
    let dot_label = row
        .dir_statuses
        .iter()
        .map(|status| git_status_label(*status))
        .collect::<Vec<_>>()
        .join(", ");
    // deleted 文件不在磁盘上：行主动作被禁（RowView.tsx:357 canOpenPrimary=false）。
    let can_open = !row.deleted;

    // 横向连接线：从父级竖线延伸到本行图标（RowView.tsx:275-284）。
    // depth=0 时整段不渲染（真源 depth>0 才画），故用 Option<String> 交由条件分支控制。
    let branch_style: Option<String> = (depth > 0).then(|| {
        format!(
            "left: calc(0.625rem + {}rem); width: calc({DEPTH_STEP_REM}rem - 1px); background-color: var(--color-border);",
            (depth - 1) as f64 * DEPTH_STEP_REM
        )
    });

    // CSS 变量 --workspace-file-tree-depth 供 padding-left 的calc() 使用
    // （RowView.tsx:223 + 111-114）。写成整段 style 字符串，避免自定义属性绑定差异。
    let row_style = format!("--workspace-file-tree-depth: {depth}");

    view! {
        <div
            role="treeitem"
            tabindex="0"
            class="group/file-tree-row relative flex h-7 w-full min-w-0 cursor-pointer items-center gap-1.5 rounded-lg border py-1 pl-[calc(var(--workspace-file-tree-depth)*0.75rem+0.5rem)] pr-2 text-left text-ui-base text-foreground outline-none transition-[background-color,border-color,box-shadow] hover:bg-surface-hover focus-visible:border-border-hover"
            // 选中态只改边框颜色、背景保持透明（RowView.tsx:224-226）——不是 bg-selected。
            class:border-input-border-focused=selected
            class:border-transparent=move || !selected
            style=row_style
            title=relative_path(&workspace, &row.path)
            on:click=move |_| {
                on_select.run(row.path.clone());
                // 单击即执行主要动作：目录展开 / 文件预览（RowView.tsx:139-156）。
                if is_dir {
                    on_toggle.run(compacted_paths.clone());
                } else if can_open {
                    on_preview.run(row.path.clone());
                }
            }
        >
            {branch_style.map(|s| view! {
                <span aria-hidden="true" class="pointer-events-none absolute top-1/2 h-px" style=s></span>
            })}
            {(depth > 0).then(|| hierarchy_guide(depth))}
            // 只有目录有箭头，文件行无占位（真源既有行为，不"修正"）。
            {is_dir.then(|| view! {
                <span class="flex size-4 shrink-0 items-center justify-center text-foreground-subtle">
                    <span
                        class="flex size-3 text-foreground-subtlest transition-transform"
                        class:rotate-90=row.expanded
                    >
                        {chevron_right()}
                    </span>
                </span>
            })}
            <span class="flex min-w-0 flex-1 items-center gap-1.5">
                {file_icon
                    .map(|src| {
                        view! {
                            <img src=src class="shrink-0 size-4" alt="" />
                        }
                    })}
                <span
                    class=format!("min-w-0 flex-1 truncate {name_text_class}")
                >
                    {row.name.clone()}
                </span>
            </span>
            // 文件行的状态字母（RowView.tsx:305-323）：tooltip 挂状态词，ml-auto 顶到行尾。
            {(indicator.is_some() && indicator_label.is_some()).then(|| {
                let indicator = indicator.clone().unwrap_or_default();
                let indicator_label = indicator_label.clone().unwrap_or_default();
                // 全部在 view! 之外构造（多行宏在属性里的解析器坑 + 数据准备一次到位）。
                let indicator_span_class = format!(
                    "ml-auto inline-flex shrink-0 justify-center font-mono text-ui-base font-bold leading-none {indicator_class}",
                );
                let indicator_children = {
                    // indicator 是 &'static str（Copy），闭包直接按值捕获。
                    let indicator_label_for_closure = indicator_label.clone();
                    std::sync::Arc::new(move || {
                        let indicator_label = indicator_label_for_closure.clone();
                        view! {
                            <span aria-label=indicator_label class=indicator_span_class.clone()>
                                {indicator}
                            </span>
                        }
                        .into_any()
                    })
                        as std::sync::Arc<dyn Fn() -> AnyView + Send + Sync + 'static>
                };
                let indicator_data = crate::ControlHintTooltip::ControlHintTooltipData {
                    title: indicator_label.clone(),
                    side: Some(crate::ControlHintTooltip::TooltipSide::Right),
                    ..Default::default()
                };
                view! {
                    <crate::ControlHintTooltip::ControlHintTooltip
                        children=indicator_children
                        data=indicator_data
                    />
                }
            })}
            // 目录行的聚合圆点（RowView.tsx:324-341）。
            {show_dir_dot.then(|| {
                let dot_label_for_data = dot_label.clone();
                let dot_children = std::sync::Arc::new(move || {
                    view! {
                        <span aria-label="●" class="flex shrink-0 items-center">
                            <span class=format!("size-1.5 rounded-full {dot_class}")></span>
                        </span>
                    }
                    .into_any()
                })
                    as std::sync::Arc<dyn Fn() -> AnyView + Send + Sync + 'static>;
                let dot_data = crate::ControlHintTooltip::ControlHintTooltipData {
                    title: dot_label_for_data,
                    side: Some(crate::ControlHintTooltip::TooltipSide::Right),
                    ..Default::default()
                };
                view! {
                    <crate::ControlHintTooltip::ControlHintTooltip
                        children=dot_children
                        data=dot_data
                    />
                }
            })}
            {row.loading.then(|| view! {
                <span class="ml-2 flex size-3 shrink-0 animate-spin text-foreground-subtlest">
                    {loader_circle()}
                </span>
            })}
            {row.error.then(|| view! {
                <span class="ml-2 flex size-3 shrink-0 text-warning">
                    {circle_alert()}
                </span>
            })}
        </div>
    }
}

// ---------------------------------------------------------------------------
// 面板（WorkspaceFileTree.tsx:630-765）
// ---------------------------------------------------------------------------

#[component]
pub fn FileTreePanel() -> impl IntoView {
    let show_file_tree = expect_context::<RwSignal<bool>>();
    let selected_session = expect_context::<RwSignal<Option<String>>>();
    let store = expect_context::<crate::app::SessionsStore>();

    let workspace = move || {
        selected_session
            .get()
            .and_then(|id| store.workspace_path_of(&id))
    };

    let root = RwSignal::new(None::<String>);
    let data = RwSignal::new(TreeData::default());
    let selected_path = RwSignal::new(None::<String>);
    let query = RwSignal::new(String::new());
    let (preview, set_preview) = signal(None::<FilePreview>);
    // git 状态（WorkspaceFileTree.tsx:240-250 + useWorkspaceFileTreeData 的 loadGitStatus）。
    let git_status = RwSignal::new(WorkspaceFileTreeGitStatusState::default());
    // 只看变更开关（WorkspaceFileTree.tsx:109；git 不可用时强制回退）。
    let show_changed_only = RwSignal::new(false);
    // 目录监听登记表（useWorkspaceFileTreeWatchers.ts 的 Rust 对应物）。
    let watcher_registry = RwSignal::new(WatcherRegistry::default());
    // 变更刷新合并拍：多次 watcher 事件合并成一次重载（真源 enqueueWatchRefresh 的去抖）。
    let refresh_epoch = RwSignal::new(0u64);
    // 虚拟滚动状态（真源 useVirtualizer：28px 定高 + overscan 12）。
    let scroll_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let scroll_top = RwSignal::new(0.0f64);
    let viewport_height = RwSignal::new(0.0f64);

    // 展开集合变更与懒加载分离（WorkspaceFileTree.tsx:495-513）。
    // compact folder 传整组 compactedPaths：收起时逐个删除，展开时整组写入并加载末端目录。
    let toggle_dir = Callback::new(move |paths: Vec<String>| {
        let Some(leaf_path) = paths.last().cloned() else {
            return;
        };
        let mut current = data.get_untracked();
        let is_expanded = paths.iter().any(|p| current.expanded.contains(p));
        let needs_load = !is_expanded && current.needs_load(&leaf_path, false);
        if is_expanded {
            for p in &paths {
                current.expanded.remove(p);
            }
        } else {
            for p in &paths {
                current.expanded.insert(p.clone());
            }
        }
        data.set(current);
        if needs_load {
            let Some(ws) = workspace() else { return };
            spawn_local(load_directory(data, ws, leaf_path));
        }
    });

    let preview_file = Callback::new(move |path: String| {
        let set_preview = set_preview.clone();
        spawn_local(async move {
            match crate::app::invoke_json("fs_read", serde_json::json!({ "path": path })).await {
                Ok(v) => set_preview.set(Some(FilePreview {
                    content: v["content"].as_str().unwrap_or("").to_string(),
                    truncated: v["truncated"].as_bool().unwrap_or(false),
                })),
                Err(e) => set_preview.set(Some(FilePreview {
                    content: format!("读取失败: {e}"),
                    truncated: false,
                })),
            }
        });
    });

    // workspace 变化时重置树并强制加载根目录（真源挂载只读根目录，force=true）。
    Effect::new(move |_| {
        if !show_file_tree.get() {
            return;
        }
        let Some(ws) = workspace() else { return };
        if root.get_untracked().as_deref() == Some(ws.as_str()) {
            return;
        }
        root.set(Some(ws.clone()));
        data.set(TreeData::default());
        selected_path.set(None);
        set_preview.set(None);
        git_status.set(WorkspaceFileTreeGitStatusState::default());
        show_changed_only.set(false);
        // git 不可用的 workspace 切换后强制收起只看变更（真源 :212-216）。
        spawn_local(load_directory(data, ws.clone(), ws));
    });

    // git 状态装载（真源 loadGitStatus：workspace 变化 + watcher 变更时刷新）。
    Effect::new(move |_| {
        let Some(ws) = root.get() else { return };
        spawn_local(async move {
            if let Ok(state) = load_workspace_file_tree_git_status(&ws).await {
                git_status.set(state);
            }
        });
    });

    // 目录监听（useWorkspaceFileTreeData.ts:623-636）：watched = {workspace} ∪ expanded。
    // 每次集合变化对齐登记表，命令经 invoke 执行（真源直接调 fileWatcherService）。
    Effect::new(move |_| {
        let Some(ws) = root.get() else { return };
        let mut watched: HashSet<String> = data.get().expanded.iter().cloned().collect();
        watched.insert(ws);
        // RwSignal::update 借用改写（WatcherRegistry 不可 Clone，不走 get/set 往返）。
        let mut commands: Vec<WatcherCommand> = Vec::new();
        watcher_registry.update(|registry| {
            commands = registry.sync(&watched);
        });
        for command in commands {
            match command {
                WatcherCommand::Watch { dir_path } => {
                    spawn_local(async move {
                        match crate::app::invoke_json(
                            "fs_watch",
                            serde_json::json!({ "path": dir_path }),
                        )
                        .await
                        {
                            Ok(v) => {
                                let id = v["id"].as_str().unwrap_or_default().to_string();
                                // 迟到注册：拿到 id 时集合可能已变（真源 :70-75）。
                                let current = data.get_untracked();
                                let mut latest: HashSet<String> =
                                    current.expanded.iter().cloned().collect();
                                if let Some(ws) = root.get_untracked() {
                                    latest.insert(ws);
                                }
                                watcher_registry.update(|registry| {
                                    registry.on_watch_ok(&dir_path, id, &latest);
                                });
                            }
                            Err(_) => {
                                watcher_registry.update(|registry| registry.on_watch_failed(&dir_path));
                            }
                        }
                    });
                }
                WatcherCommand::Unwatch { watcher_id, .. } => {
                    spawn_local(async move {
                        let _ = crate::app::invoke_json(
                            "fs_unwatch",
                            serde_json::json!({ "id": watcher_id }),
                        )
                        .await;
                    });
                }
            }
        }
    });

    // 变更事件订阅（一次性；真源 onDynamicChange → enqueueWatchRefresh）。
    // 收到变更后推进合并拍，Effect 按拍去抖刷新已加载目录与 git 状态。
    {
        let refresh_epoch = refresh_epoch;
        Effect::new(move |_| {
            spawn_local(async move {
                let handler = wasm_bindgen::closure::Closure::<dyn Fn(wasm_bindgen::JsValue)>::new(
                    move |event: wasm_bindgen::JsValue| {
                        let payload = js_sys::Reflect::get(&event, &"payload".into()).ok();
                        let text = payload.and_then(|p| p.as_string());
                        let dir_path = text
                            .as_deref()
                            .and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok())
                            .and_then(|v| {
                                v.get("dirPath").and_then(|d| d.as_str().map(str::to_string))
                            });
                        if dir_path.is_some() {
                            refresh_epoch.update(|n| *n += 1);
                        }
                    },
                );
                let handler_fn: &js_sys::Function = handler.as_ref().unchecked_ref();
                let subscribed = crate::app::listen_tauri_event("fs-watcher-change", handler_fn).await;
                if subscribed.is_err() {
                    web_sys::console::warn_1(&"文件树 watcher 事件订阅失败，退化为手动刷新".into());
                }
                handler.forget();
            });
        });
    }

    // 合并拍 → 去抖刷新：重读 workspace ∪ loaded ∪ expanded（真源
    // getWorkspaceFileTreeRefreshDirectoryPaths），再重载 git 状态。
    Effect::new(move |_| {
        let epoch = refresh_epoch.get();
        if epoch == 0 {
            return;
        }
        let Some(ws) = root.get() else { return };
        let data = data;
        let git_status = git_status;
        set_timeout(
            move || {
                // 刷新路径按真源顺序：根 → loaded → expanded（同拍去重，且都必须在 workspace 内）。
                let current = data.get_untracked();
                let mut refresh_paths: Vec<String> = Vec::new();
                let mut seen: HashSet<String> = HashSet::new();
                let mut add_refresh = |path: String| {
                    if crate::workspace_file_tree::model::is_workspace_file_path_inside(&ws, &path)
                        && seen.insert(path.clone())
                    {
                        refresh_paths.push(path);
                    }
                };
                add_refresh(ws.clone());
                for path in current.loaded.clone() {
                    add_refresh(path);
                }
                for path in current.expanded.clone() {
                    add_refresh(path);
                }
                for path in refresh_paths {
                    spawn_local(load_directory(data, ws.clone(), path));
                }
                // git 状态随后重载（真源 loadGitStatus 与目录刷新同拍）。
                let ws_for_git = ws.clone();
                spawn_local(async move {
                    if let Ok(state) =
                        load_workspace_file_tree_git_status(&ws_for_git).await
                    {
                        git_status.set(state);
                    }
                });
            },
            std::time::Duration::from_millis(300),
        );
    });

    // 虚拟滚动：挂载与面板打开时量一次视口高度（真源 useVirtualizer 内部有
    // ResizeObserver；Rust 侧以挂载测量 + 滚动事件随测近似，面板宽度不频繁变）。
    Effect::new(move |_| {
        if !show_file_tree.get() {
            return;
        }
        let Some(el) = scroll_ref.get() else { return };
        let element = el.unchecked_ref::<web_sys::Element>();
        viewport_height.set(element.client_height() as f64);
        scroll_top.set(element.scroll_top() as f64);
    });

    let rows = Memo::new(move |_| {
        let Some(root) = root.get() else {
            return Vec::new();
        };
        let status_state = git_status.get();
        let all = flatten_compact(&root, &data.get(), &status_state.status_by_path);
        // 只看变更 + 搜索过滤（真源 :179-198 走 filterWorkspaceFileTreeRows；
        // 搜索语义按已加载节点近似——真源全局索引 IPC 未迁）。
        let filter_input: Vec<crate::workspace_file_tree::model::FilterRow> = all
            .iter()
            .map(|row| crate::workspace_file_tree::model::FilterRow {
                path: row.path.clone(),
                name: row.name.clone(),
                is_dir: row.is_dir,
            })
            .collect();
        let kept = crate::workspace_file_tree::model::filter_rows(
            &filter_input,
            &query.get(),
            show_changed_only.get(),
            &status_state.status_by_path,
        );
        let kept_set: HashSet<String> = kept.into_iter().map(|row| row.path).collect();
        all.into_iter()
            .filter(|row| kept_set.contains(&row.path))
            .collect()
    });

    let workspace_path = move || root.get().unwrap_or_default();

    view! {
            <section
                data-testid="workspace-file-tree-panel"
                class="flex h-full min-h-0 flex-col text-foreground"
            >
                // 返回按钮（WorkspaceFileTree.tsx:635-648）。
                <div class="px-2 pb-3 pt-3">
                    <button
                        type="button"
                        class="flex w-full items-center justify-start gap-2 rounded-xl px-2.5 text-foreground-subtle hover:bg-surface-hover hover:text-foreground"
                        on:click=move |_| show_file_tree.set(false)
                    >
                        <span class="flex size-4 flex-none items-center justify-center">{arrow_left()}</span>
                        "工作区文件"
                    </button>
                </div>
                // 搜索框（WorkspaceFileTree.tsx:649-664）。
                <div class="flex shrink-0 items-center px-2 pb-2">
                    <div class="relative min-w-0 flex-1">
                        <span class="pointer-events-none absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-foreground-subtlest">
                            {icon_search()}
                        </span>
                        <input
                            type="text"
                            class="h-7 w-full rounded-lg border border-border bg-transparent pl-7 pr-7 text-ui-base text-foreground outline-none placeholder:text-foreground-subtlest focus-visible:border-border-hover focus-visible:bg-input-focused"
                            placeholder="搜索文件"
                            prop:value=move || query.get()
                            on:input=move |ev| {
                                query.set(event_target_value(&ev));
                            }
                        />
                        {(!query.get().is_empty()).then(|| view! {
                            <button
                                type="button"
                                class="absolute right-2 top-1/2 -translate-y-1/2 text-foreground-subtlest hover:text-foreground"
                                on:click=move |_| query.set(String::new())
                            >
                                <span class="flex size-3 items-center justify-center">{icon_x()}</span>
                            </button>
                        })}
                    </div>
                </div>
                // 标题行（WorkspaceFileTree.tsx:686）+ 只看变更开关（:717-744）。
                <div class="flex items-center justify-between px-2">
                    <h3 class="min-w-0 truncate py-1 pl-2.5 pr-0.5 text-ui-base font-medium text-foreground-subtlest">
                        {move || workspace_path()}
                    </h3>
                    {move || {
                        // git 不可用时开关整个缺席（真源 :717）。
                        if !git_status.get().available {
                            return ().into_any();
                        }
                        let label = if show_changed_only.get() {
                            crate::ToolCallBlocks::i18n::text("workspaceFileTree.showAllFiles")
                        } else {
                            crate::ToolCallBlocks::i18n::text("workspaceFileTree.showChangedFiles")
                        };
                        let pressed = show_changed_only.get();
                        view! {
                            <button
                                aria-label=label.clone()
                                aria-pressed=pressed
                                class=format!(
                                    "flex size-6 shrink-0 items-center justify-center rounded-lg text-foreground-subtle hover:bg-surface-hover hover:text-foreground {}",
                                    if pressed { "bg-selected text-foreground" } else { "" },
                                )
                                title=label
                                type="button"
                                on:click=move |_| show_changed_only.update(|v| *v = !*v)
                            >
                                <span class="flex size-3.5 items-center justify-center">
                                    <Icon paths=vec!["M12 3v6", "M12 15v6"] circles=vec![("12", "12", "3")] />
                                </span>
                            </button>
                        }
                            .into_any()
                    }}
                </div>
                // 滚动容器 + 虚拟滚动（WorkspaceFileTree.tsx:761-765 + useVirtualizer :199-204）。
                // 28px 定高 + overscan 12；总高占位撑出真实滚动条，窗口内绝对定位渲染。
                <div
                    class="h-full min-h-0 flex-1 overflow-auto px-1 px-2"
                    node_ref=scroll_ref
                    style="min-height: 0"
                    on:scroll=move |ev| {
                        let el = event_target::<web_sys::Element>(&ev);
                        scroll_top.set(el.scroll_top() as f64);
                        viewport_height.set(el.client_height() as f64);
                    }
                >
                    {move || {
                        let list = rows.get();
                        if list.is_empty() {
                            return view! {
                                <p class="px-2 py-1 text-xs text-muted">"没有匹配的文件"</p>
                            }
                            .into_any();
                        }
                        let selected_now = selected_path.get_untracked();
                        let total = list.len();
                        let total_height = total * ROW_HEIGHT_PX;
                        // 可见窗口：scrollTop 上取整行号，向下补满视口，上下各扩 12 行（真源 overscan=12）。
                        const OVERSCAN: usize = 12;
                        let start = ((scroll_top.get() / ROW_HEIGHT_PX as f64) as usize)
                            .saturating_sub(OVERSCAN);
                        let visible_end = ((scroll_top.get() + viewport_height.get())
                            / ROW_HEIGHT_PX as f64)
                            as usize
                            + 1
                            + OVERSCAN;
                        let end = total.min(visible_end);
                        view! {
                            <div
                                role="tree"
                                class="relative w-full"
                                style=format!("height: {total_height}px")
                            >
                                {list
                                    .into_iter()
                                    .enumerate()
                                    .skip(start)
                                    .take(end.saturating_sub(start))
                                    .map(|(index, row)| {
                                        let path = row.path.clone();
                                        let is_selected = selected_now.as_deref() == Some(path.as_str());
                                        let offset = index * ROW_HEIGHT_PX;
                                        view! {
                                            <div
                                                class="absolute left-0 top-0 w-full px-1"
                                                style=format!("transform: translateY({offset}px); height: {ROW_HEIGHT_PX}px")
                                            >
                                                <FileTreeRow
                                                    row=row
                                                    workspace=workspace_path()
                                                    selected=is_selected
                                                    on_select=Callback::new(move |p: String| selected_path.set(Some(p)))
                                                    on_toggle=toggle_dir
                                                    on_preview=Callback::new(move |p: String| preview_file.run(p))
                                                />
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
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
            </section>
        }
}

#[derive(Debug, Clone)]
pub struct FilePreview {
    pub content: String,
    pub truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(path: &str, name: &str, is_dir: bool, depth: usize) -> TreeNode {
        TreeNode {
            path: path.into(),
            name: name.into(),
            is_dir,
            is_symlink: false,
            depth,
        }
    }

    #[test]
    fn flatten_only_descends_into_expanded_dirs() {
        let mut data = TreeData::default();
        data.children_by_dir.insert(
            "root".into(),
            vec![
                node("root\\a", "a", true, 0),
                node("root\\f.txt", "f.txt", false, 0),
            ],
        );
        data.children_by_dir.insert(
            "root\\a".into(),
            vec![node("root\\a\\x.rs", "x.rs", false, 1)],
        );
        data.expanded.insert("root\\a".into());

        let rows = flatten("root", &data);
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["a", "x.rs", "f.txt"]);
        assert!(rows[0].expanded);
    }

    #[test]
    fn collapsed_dir_hides_children() {
        let mut data = TreeData::default();
        data.children_by_dir
            .insert("root".into(), vec![node("root\\a", "a", true, 0)]);
        data.children_by_dir.insert(
            "root\\a".into(),
            vec![node("root\\a\\x.rs", "x.rs", false, 1)],
        );
        // expanded 为空 → 子节点不进平铺列表。
        let rows = flatten("root", &data);
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].expanded);
    }

    #[test]
    fn child_depth_counts_segments_relative_to_workspace() {
        assert_eq!(child_depth("D:/ws", "D:/ws"), 0);
        assert_eq!(child_depth("D:/ws", "D:/ws/a"), 1);
        assert_eq!(child_depth("D:/ws", "D:/ws/a/b/c"), 3);
        // 大小写与分隔符差异（Windows 路径）不应影响深度。
        assert_eq!(child_depth("D:\\ws", "d:/ws/a/b"), 2);
        // 越界路径回退 0。
        assert_eq!(child_depth("D:/ws", "D:/other"), 0);
    }

    #[test]
    fn auto_flattenable_excludes_symlink_dirs() {
        let mut symlinked = node("l", "l", true, 0);
        symlinked.is_symlink = true;
        assert!(!is_auto_flattenable(&symlinked));
        assert!(!is_auto_flattenable(&node("f.txt", "f.txt", false, 0)));
        assert!(is_auto_flattenable(&node("d", "d", true, 0)));
    }

    #[test]
    fn relative_path_strips_workspace_prefix() {
        assert_eq!(relative_path("D:/ws", "D:/ws/a/b.rs"), "a/b.rs");
        assert_eq!(relative_path("D:/ws", "D:/other"), "D:/other");
    }

    /// compact folder：root 下只有 a，a 下只有 b，b 下有两个文件 → 压平为一行 `a/b`。
    #[test]
    fn compact_flattens_single_child_directory_chain() {
        let mut data = TreeData::default();
        data.children_by_dir
            .insert("root".into(), vec![node("root\\a", "a", true, 0)]);
        data.children_by_dir
            .insert("root\\a".into(), vec![node("root\\a\\b", "b", true, 1)]);
        data.children_by_dir.insert(
            "root\\a\\b".into(),
            vec![
                node("root\\a\\b\\x.rs", "x.rs", false, 2),
                node("root\\a\\b\\y.rs", "y.rs", false, 2),
            ],
        );
        data.loaded.insert("root\\a".into());
        data.loaded.insert("root\\a\\b".into());

        let rows = flatten_compact("root", &data, &HashMap::new());
        assert_eq!(rows.len(), 1, "单子目录链应压成一行");
        assert_eq!(rows[0].name, "a/b");
        assert_eq!(rows[0].depth, 0, "视觉深度停在最外层");
        assert_eq!(
            rows[0].path, "root\\a\\b",
            "行 path 指向链末端（展开/加载用真实路径）"
        );
        assert_eq!(
            rows[0].compacted_paths,
            vec!["root\\a".to_string(), "root\\a\\b".to_string()],
            "compacted_paths 覆盖整条链"
        );
    }

    /// 压平后展开：子内容深度要减去 offset，不能与父目录同级（model.ts:507-512 的坑）。
    #[test]
    fn compact_depth_offset_applies_to_children() {
        let mut data = TreeData::default();
        data.children_by_dir
            .insert("root".into(), vec![node("root\\a", "a", true, 0)]);
        data.children_by_dir
            .insert("root\\a".into(), vec![node("root\\a\\b", "b", true, 1)]);
        data.children_by_dir.insert(
            "root\\a\\b".into(),
            vec![node("root\\a\\b\\x.rs", "x.rs", false, 2)],
        );
        data.loaded.insert("root\\a".into());
        data.loaded.insert("root\\a\\b".into());
        data.expanded.insert("root\\a".into());
        data.expanded.insert("root\\a\\b".into());

        let rows = flatten_compact("root", &data, &HashMap::new());
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["a/b", "x.rs"]);
        assert_eq!(rows[1].depth, 1, "子行深度 = 物理深度2 - 偏移1");
    }

    /// 未加载的目录不压平（拿不到子项就无法判断链）。
    #[test]
    fn compact_skips_unloaded_directories() {
        let mut data = TreeData::default();
        data.children_by_dir
            .insert("root".into(), vec![node("root\\a", "a", true, 0)]);
        // root\\a 未 loaded → 只知道它有个子项名，但没读过，压平条件不成立。
        let rows = flatten_compact("root", &data, &HashMap::new());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "a");
    }

    /// 软链接目录不参与压平（否则路径去重失效、无限加载）。
    #[test]
    fn compact_does_not_flatten_symlink_chain() {
        let mut data = TreeData::default();
        let mut link = node("root\\link", "link", true, 0);
        link.is_symlink = true;
        data.children_by_dir.insert("root".into(), vec![link]);
        data.children_by_dir.insert(
            "root\\link".into(),
            vec![node("root\\link\\x", "x", true, 1)],
        );
        data.loaded.insert("root\\link".into());

        let rows = flatten_compact("root", &data, &HashMap::new());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "link", "软链接目录保持单行，不拼成 link/x");
    }
}
