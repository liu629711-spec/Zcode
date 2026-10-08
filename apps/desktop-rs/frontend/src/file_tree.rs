//! 树形文件树面板（1:1 翻译 `packages/ui/src/workspace-file-tree/`）。
//!
//! 对照真源：
//! - 面板外壳 `WorkspaceFileTree.tsx:630-765`
//! - 行渲染 `WorkspaceFileTreeRowView.tsx:213-304`
//! - 数据层 `useWorkspaceFileTreeData.ts:80-92`（expanded/loaded/loading/error 六个集合）
//! - 平铺 `model.ts:435-461`（DFS：树 → 扁平 row 列表，再渲染）
//!
//! 差异（已知取舍，逐条标注）：
//! - **不做虚拟滚动**：真源用 @tanstack/react-virtual，行高固定 28px（constants.ts:1）。
//!   Rust 侧先用普通滚动；目录懒加载后行数量可控，overscan 优化留后续。
//! - **不做 compact folder 自动压平**：真源把 `a/b/c` 单子目录链合并成一行
//!   （model.ts:394-433）。本轮先出正确层级，压平另做。
//! - **不做 git 状态 / watcher / 拖拽 / 右键菜单**：依赖 gitService 与 fileWatcherService，
//!   Rust 侧主进程未接，等协议侧补齐再迁。
//! - **搜索**：真源走全局文件索引 IPC（useWorkspaceFileSearchIndex），本轮只过滤已加载的树节点。

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::{HashMap, HashSet};

use crate::app::Icon;
use crate::file_icons::{icon_src, resolve_icon_name};

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
    dir[ws.len() + 1..].split('/').filter(|s| !s.is_empty()).count()
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

    let result =
        crate::app::invoke_json("fs_list", serde_json::json!({ "path": dir_path, "includeHidden": true }))
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
            });
            if expanded {
                visit(&child.path, data, rows);
            }
        }
    }
    visit(root, data, &mut rows);
    rows
}

/// DFS 平铺 + compact folder 压平（model.ts:390-461 全量语义）。
///
/// 与 `flatten` 的差异：沿单子目录链合并节点，展示名变`a/b/c`，深度停在最外层，
/// 收起时整组 compacted_paths 都要退出 expanded 集合（否则下次展开只剩内层生效）。
fn flatten_compact(root: &str, data: &TreeData) -> Vec<TreeRow> {
    let mut rows: Vec<TreeRow> = Vec::new();

    // 沿链压平：返回(展示节点, 该链覆盖的全部路径, 子内容深度偏移)。
    fn compact<'a>(
        node: &'a TreeNode,
        data: &'a TreeData,
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
            let Some(children) = data.children_by_dir.get(&current.path) else {
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

    fn visit(dir_path: &str, depth_offset: usize, data: &TreeData, rows: &mut Vec<TreeRow>) {
        let Some(children) = data.children_by_dir.get(dir_path) else {
            return;
        };
        for child in children {
            let (node, compacted_paths, offset) = compact(child, data);
            let expanded = node.is_dir
                && compacted_paths.iter().any(|p| data.expanded.contains(p));
            rows.push(TreeRow {
                path: node.path.clone(),
                name: node.name.clone(),
                is_dir: node.is_dir,
                depth: node.depth.saturating_sub(depth_offset),
                expanded,
                loading: data.loading.contains(&node.path),
                error: data.error_dirs.contains(&node.path),
                compacted_paths,
            });
            if expanded {
                visit(&node.path, depth_offset + offset, data, rows);
            }
        }
    }

    visit(root, 0, data, &mut rows);
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
                } else {
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
                <span class="min-w-0 flex-1 truncate">{row.name.clone()}</span>
            </span>
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

    // 展开集合变更与懒加载分离（WorkspaceFileTree.tsx:495-513）。
    // compact folder 传整组 compactedPaths：收起时逐个删除，展开时整组写入并加载末端目录。
    let toggle_dir = Callback::new(move |paths: Vec<String>| {
        let Some(leaf_path) = paths.last().cloned() else { return };
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
        spawn_local(load_directory(data, ws.clone(), ws));
    });

    let rows = Memo::new(move |_| {
        let Some(root) = root.get() else {
            return Vec::new();
        };
        let all = flatten_compact(&root, &data.get());
        let q = query.get().trim().to_lowercase();
        if q.is_empty() {
            return all;
        }
        // 搜索：真源走全局文件索引 IPC（useWorkspaceFileSearchIndex），本轮只过滤已加载节点。
        let keep: HashSet<String> = all
            .iter()
            .filter(|r| r.name.to_lowercase().contains(&q))
            .map(|r| r.path.clone())
            .collect();
        if keep.is_empty() {
            return Vec::new();
        }
        all.into_iter()
            .filter(|r| {
                if keep.contains(&r.path) {
                    return true;
                }
                // 命中节点的祖先目录要保留，否则层级断裂。
                let prefix = format!("{}/", r.path.replace('\\', "/"));
                r.is_dir && keep.iter().any(|k| k.replace('\\', "/").starts_with(&prefix))
            })
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
            // 标题行（WorkspaceFileTree.tsx:686）。
            <div class="flex items-center justify-between px-2">
                <h3 class="min-w-0 truncate py-1 pl-2.5 pr-0.5 text-ui-base font-medium text-foreground-subtlest">
                    {move || workspace_path()}
                </h3>
            </div>
            // 滚动容器（WorkspaceFileTree.tsx:761-765）。真源内层 role="tree" + 虚拟定位，
            // 本轮不虚拟化，行高固定 h-7 保证滚动节奏一致。
            <div class="h-full min-h-0 flex-1 overflow-auto px-2 px-1" style="min-height: 0">
                <div role="tree" class="relative w-full" style=format!("min-height: {ROW_HEIGHT_PX}px")>
                    {move || {
                        let list = rows.get();
                        if list.is_empty() {
                            return view! {
                                <p class="px-2 py-1 text-xs text-muted">"没有匹配的文件"</p>
                            }
                            .into_any();
                        }
let selected_now = selected_path.get_untracked();
                            list.into_iter()
                                .map(|row| {
                                    let path = row.path.clone();
                                    let is_selected = selected_now.as_deref() == Some(path.as_str());
                                    view! {
                                    <FileTreeRow
                                        row=row
                                        workspace=workspace_path()
                                        selected=is_selected
                                        on_select=Callback::new(move |p: String| selected_path.set(Some(p)))
                                        on_toggle=toggle_dir
                                        on_preview=Callback::new(move |p: String| preview_file.run(p))
                                    />
                                }
                                })
                            .collect_view()
                            .into_any()
                    }}
                </div>
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
            vec![node("root\\a", "a", true, 0), node("root\\f.txt", "f.txt", false, 0)],
        );
        data.children_by_dir
            .insert("root\\a".into(), vec![node("root\\a\\x.rs", "x.rs", false, 1)]);
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
        data.children_by_dir
            .insert("root\\a".into(), vec![node("root\\a\\x.rs", "x.rs", false, 1)]);
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

        let rows = flatten_compact("root", &data);
        assert_eq!(rows.len(), 1, "单子目录链应压成一行");
        assert_eq!(rows[0].name, "a/b");
        assert_eq!(rows[0].depth, 0, "视觉深度停在最外层");
        assert_eq!(rows[0].path, "root\\a\\b", "行 path 指向链末端（展开/加载用真实路径）");
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
        data.children_by_dir
            .insert("root\\a\\b".into(), vec![node("root\\a\\b\\x.rs", "x.rs", false, 2)]);
        data.loaded.insert("root\\a".into());
        data.loaded.insert("root\\a\\b".into());
        data.expanded.insert("root\\a".into());
        data.expanded.insert("root\\a\\b".into());

        let rows = flatten_compact("root", &data);
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
        let rows = flatten_compact("root", &data);
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
        data.children_by_dir
            .insert("root\\link".into(), vec![node("root\\link\\x", "x", true, 1)]);
        data.loaded.insert("root\\link".into());

        let rows = flatten_compact("root", &data);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "link", "软链接目录保持单行，不拼成 link/x");
    }
}