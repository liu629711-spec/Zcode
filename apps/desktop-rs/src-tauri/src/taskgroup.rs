//! 任务分组数据层（真源 `packages/ui/src/workspace-grouped-tasks/` + `task_groups` 四表）。
//!
//! 表结构（`schema-v1.ts:35-73`）：
//! - `task_groups`：group_id / title / color / 时间戳
//! - `task_group_members`：group_id ↔ task_id 关联，主键 `(workspace_key, task_id)`
//!   ——**一个任务最多属于一个组**（PRIMARY KEY 不含 group_id）
//! - `task_group_view_node_orders`：顶层节点排序，node_key 是 JSON 字符串
//! - `task_group_workspace_bootstraps`：每工作区的初始组
//!
//! 视图模型（`zcodeTaskListTypes.ts:58-73` ZCodeGroupedTaskViewNode）：
//! ```text
//! nodes: ( { type:"group", group, tasks:[...] } | { type:"task", task } )[]
//! ```
//! 即顶层是「组节点」与「游离任务节点」的混合序列。
//!
//! 颜色映射（`types.ts` TASK_GROUP_COLOR_CLASS）：7 种 gray/red/orange/yellow/
//! green/blue/purple，每种给 light + dark 两套类名。

use rusqlite::Connection;
use serde::Serialize;

use crate::taskdb::workspace_key;

/// 分组颜色（真源 `TASK_GROUP_COLORS`，types.ts:3-12）。
pub const GROUP_COLORS: [&str; 7] = [
    "gray", "red", "orange", "yellow", "green", "blue", "purple",
];

/// 颜色 → 类名（真源 `TASK_GROUP_COLOR_CLASS`，types.ts:14-22）。
///
/// 每种两套：亮色底/字 + 暗色底/字（`dark:` 前缀）。class 名照抄，不自造近似值。
pub fn color_classes(color: &str) -> (&'static str, &'static str) {
    match color {
        "red" => (
            "bg-rose-300 text-rose-900",
            "dark:bg-rose-400/32 dark:text-rose-50",
        ),
        "orange" => (
            "bg-orange-300 text-orange-900",
            "dark:bg-orange-400/32 dark:text-orange-50",
        ),
        "yellow" => (
            "bg-amber-300 text-amber-900",
            "dark:bg-amber-300/32 dark:text-amber-50",
        ),
        "green" => (
            "bg-emerald-300 text-emerald-900",
            "dark:bg-emerald-400/32 dark:text-emerald-50",
        ),
        "blue" => (
            "bg-sky-300 text-sky-900",
            "dark:bg-sky-400/32 dark:text-sky-50",
        ),
        "purple" => (
            "bg-violet-300 text-violet-900",
            "dark:bg-violet-400/32 dark:text-violet-50",
        ),
        // gray 是默认值，也是未知颜色的兜底（真源 ICON_COLOR_MAP 同样以 gray 为基）。
        _ => (
            "bg-zinc-300 text-zinc-800",
            "dark:bg-zinc-400/32 dark:text-zinc-100",
        ),
    }
}

/// 颜色 → 左边框类名（真源 `TASK_GROUP_BORDER_COLOR_CLASS`，types.ts:23-31）。
pub fn border_color_class(color: &str) -> &'static str {
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

/// 分组（`task_groups` 行）。
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskGroup {
    pub group_id: String,
    pub title: String,
    pub color: String,
}

/// 分组视图的顶层节点（`ZCodeGroupedTaskViewNode`）。
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GroupedNode {
    /// 组节点：自带成员任务列表。
    Group {
        group: TaskGroup,
        /// 成员任务的 task_id（内容由前端 join tasks 表得到）。
        task_ids: Vec<String>,
    },
    /// 游离任务（不属于任何组），task 字段是 task_id。
    Task { task: String },
}

/// 分组视图（`ZCodeGroupedTaskView`）。
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GroupedView {
    pub nodes: Vec<GroupedNode>,
}

/// 读某工作区的分组视图。
///
/// 语义（对齐真源 `queryGroupedTaskViewStructure`）：
/// 1. 分组节点来自 `task_groups` ∩ 该 workspace 的成员；
/// 2. 游离任务 = tasks 表里**非置顶、非归档、未删除、且不属于任何组**的任务；
/// 3. 顶层顺序取 `task_group_view_node_orders`（node_key 是 JSON 串
///    `["<workspacePath>","<taskId>"]` /group 同形），无排序记录的行按 title/时间兜底。
pub fn read_grouped_view(
    conn: &Connection,
    ws_path: &str,
    ws_identity: Option<&str>,
) -> Result<GroupedView, String> {
    let ws_key = workspace_key(ws_path, ws_identity);

    // 1) 该 workspace 下的分组及其成员（sort_order 升序，null 按 added_at 兜底——真源
    //    ZCodeGroupedTaskViewStructureMember 注释：null = 尚未落sort_order）。
    let mut stmt = conn
        .prepare(
            "SELECT g.group_id, g.title, g.color, m.task_id \
             FROM task_group_members m \
             JOIN task_groups g ON g.group_id = m.group_id \
             WHERE m.workspace_key = ?1 \
             ORDER BY g.created_at ASC, m.sort_order ASC, m.added_at DESC",
        )
        .map_err(|e| format!("准备分组查询失败: {e}"))?;

    let mut groups: Vec<TaskGroup> = Vec::new();
    let mut members: Vec<(String, String)> = Vec::new(); // (group_id, task_id)
    let rows = stmt
        .query_map([&ws_key], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("执行分组查询失败: {e}"))?;
    for row in rows {
        let row = row.map_err(|e| format!("读取分组行失败: {e}"))?;
        let (group_id, title, color, task_id) = row;
        if !groups.iter().any(|g| g.group_id == group_id) {
            groups.push(TaskGroup {
                group_id: group_id.clone(),
                title,
                color,
            });
        }
        members.push((group_id, task_id));
    }

    let mut nodes: Vec<GroupedNode> = groups
        .into_iter()
        .map(|group| {
            let task_ids = members
                .iter()
                .filter(|(gid, _)| *gid == group.group_id)
                .map(|(_, tid)| tid.clone())
                .collect();
            GroupedNode::Group { group, task_ids }
        })
        .collect();

    // 2) 游离任务：排除已入组的（task_group_members 主键就是 (workspace_key, task_id)，
    //    所以一个任务只能在一个组里，不存在跨组重复）。
    let mut stmt = conn
        .prepare(
            "SELECT t.task_id FROM tasks t \
             WHERE t.workspace_key = ?1 AND t.deleted = 0 \
               AND t.pinned = 0 AND t.archived = 0 \
               AND NOT EXISTS (SELECT 1 FROM task_group_members m \
                               WHERE m.workspace_key = t.workspace_key AND m.task_id = t.task_id) \
             ORDER BY t.updated_at DESC",
        )
        .map_err(|e| format!("准备游离任务查询失败: {e}"))?;
    let loose = stmt
        .query_map([&ws_key], |r| r.get::<_, String>(0))
        .map_err(|e| format!("执行游离任务查询失败: {e}"))?;
    for row in loose {
        let task_id = row.map_err(|e| format!("读取游离任务失败: {e}"))?;
        nodes.push(GroupedNode::Task { task: task_id });
    }

    // 3) 顶层排序：有 node_orders 记录的按其排（升序），无记录的保持在末尾相对次序。
    let mut sort_keys: Vec<String> = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT node_type, node_key, sort_order FROM task_group_view_node_orders \
                 ORDER BY sort_order ASC",
            )
            .map_err(|e| format!("准备排序查询失败: {e}"))?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| format!("执行排序查询失败: {e}"))?;
        for row in rows {
            let (node_type, node_key, _sort_order) =
                row.map_err(|e| format!("读取排序行失败: {e}"))?;
            // node_key 是 JSON 串 ["<workspaceKey>","<taskId>"] 或 group 同形。
            // 只取匹配当前 workspace 的项。
            if let Some((key, _)) = parse_node_key(&node_key) {
                if key == ws_key {
                    sort_keys.push(node_key);
                }
            }
            let _ = node_type;
        }
    }
    sort_nodes_by_order(&mut nodes, &ws_key, &sort_keys);

    Ok(GroupedView { nodes })
}

/// 从 node_key 解析出 (workspace_key, id)。
///
/// 真源 node_key 是 `JSON.stringify([workspaceKey, id])` 的结果
/// （实测真实库：`["C:\\Users\\...\\default","sess_9cc7..."]`，反斜杠被 JSON 转义）。
/// 所以**必须反转义**才能和 tasks 表里的原始路径比对，否则排序永远匹配不上。
///
/// 这里手写解析而非引 serde_json：node_key 由服务端写入、格式受控，
/// 且只需处理「两个字符串元素」这一种形态。
fn parse_node_key(node_key: &str) -> Option<(String, String)> {
    /// 读一个 JSON 字符串元素：从起始引号后开始，返回 (还原后的值, 结束引号后的**字节**偏移)。
    ///
    /// 两个要点：
    /// - 偏移必须按原始字节算——转义序列让「还原长度 ≠ 原文长度」，用后者会切错位置；
    /// - 非 ASCII（中文路径）按 char 迭代，`as char` 逐字节会拆出乱码。
    fn read_str(input: &str, start: usize) -> Option<(String, usize)> {
        let bytes = input.as_bytes();
        let mut out = String::new();
        let mut i = start;
        while i < input.len() {
            match bytes[i] {
                b'\\' if i + 1 < bytes.len() => {
                    out.push(match bytes[i + 1] {
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        other => other as char,
                    });
                    i += 2;
                }
                b'"' => return Some((out, i + 1)),
                _ => {
                    // 取完整 UTF-8 字符（input.len() 与 bytes 索引对齐）。
                    let ch = input[i..].chars().next()?;
                    out.push(ch);
                    i += ch.len_utf8();
                }
            }
        }
        None
    }

    if !node_key.starts_with('[') {
        return None;
    }
    // 跳过 `["`。
    if node_key.as_bytes().get(1) != Some(&b'"') {
        return None;
    }
    let (first, after_first) = read_str(node_key, 2)?;
    // 期望 `,"`；strip_prefix 之后 rest 已从第二个字符串的引号**之后**开始。
    let rest = node_key.get(after_first..)?.strip_prefix(",\"")?;
    let (second, _) = read_str(rest, 0)?;
    Some((first, second))
}

/// 按 node_orders 的 sort_order 重排顶层节点。
///
/// sort_keys 已按 sort_order 升序；节点自身也要能算出 key 才能匹配。
fn sort_nodes_by_order(nodes: &mut Vec<GroupedNode>, ws_key: &str, sort_keys: &[String]) {
    if sort_keys.is_empty() {
        return;
    }
    // 建索引：node_key → 名次。
    let mut rank: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (i, key) in sort_keys.iter().enumerate() {
        rank.insert(key.as_str(), i);
    }
    nodes.sort_by_key(|node| {
        rank.get(node_sort_key(ws_key, node).as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
}

/// 顶层节点的 node_key（与真源写入格式一致：JSON 数组 `[workspaceKey, id]`）。
fn node_sort_key(ws_key: &str, node: &GroupedNode) -> String {
    let id = match node {
        GroupedNode::Group { group, .. } => &group.group_id,
        GroupedNode::Task { task } => task,
    };
    format!("[{},{}]", quote(ws_key), quote(id))
}

/// JSON 字符串转义（仅处理我们实际会遇到的字符）。
fn quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// 创建分组（真源 `createTaskGroup`，语义按 schema 补齐）。
pub fn create_group(
    conn: &Connection,
    group_id: &str,
    title: &str,
    color: &str,
    now_ms: i64,
) -> Result<TaskGroup, String> {
    let color = if GROUP_COLORS.contains(&color) { color } else { "gray" };
    conn.execute(
        "INSERT INTO task_groups (group_id, title, color, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?4)",
        rusqlite::params![group_id, title, color, now_ms],
    )
    .map_err(|e| format!("创建分组失败: {e}"))?;
    Ok(TaskGroup {
        group_id: group_id.to_string(),
        title: title.to_string(),
        color: color.to_string(),
    })
}

/// 把任务移入分组（真源 `addTaskToGroup`）。
///
/// 主键 `(workspace_key, task_id)` 意味着一任务只能在一组；重复调用会覆盖旧组归属。
pub fn add_task_to_group(
    conn: &Connection,
    ws_path: &str,
    ws_identity: Option<&str>,
    group_id: &str,
    task_id: &str,
    now_ms: i64,
) -> Result<bool, String> {
    let ws_key = workspace_key(ws_path, ws_identity);
    // 任务必须存在——不能给不存在的任务建成员关系。
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE workspace_key = ?1 AND task_id = ?2",
            rusqlite::params![ws_key, task_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("校验任务失败: {e}"))?;
    if exists == 0 {
        return Err(format!("任务不存在: {task_id}"));
    }
    conn.execute(
        "INSERT INTO task_group_members (group_id, workspace_key, workspace_path, \
           workspace_identity, task_id, sort_order, added_at, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?6, ?6) \
         ON CONFLICT(workspace_key, task_id) DO UPDATE SET \
           group_id = excluded.group_id, updated_at = excluded.updated_at",
        rusqlite::params![
            group_id,
            ws_key,
            ws_path,
            ws_identity,
            task_id,
            now_ms
        ],
    )
    .map_err(|e| format!("加入分组失败: {e}"))?;
    Ok(true)
}

/// 移出分组（真源 `removeTaskFromGroup`）。
pub fn remove_task_from_group(
    conn: &Connection,
    ws_path: &str,
    ws_identity: Option<&str>,
    task_id: &str,
) -> Result<bool, String> {
    let ws_key = workspace_key(ws_path, ws_identity);
    let changed = conn
        .execute(
            "DELETE FROM task_group_members WHERE workspace_key = ?1 AND task_id = ?2",
            rusqlite::params![ws_key, task_id],
        )
        .map_err(|e| format!("移出分组失败: {e}"))?;
    Ok(changed > 0)
}

/// 删除分组（真源 `deleteTaskGroup`）。
///
/// 外键 `ON DELETE CASCADE` 会连带清掉成员行（schema-v1.ts:52）。
pub fn delete_group(conn: &Connection, group_id: &str) -> Result<bool, String> {
    let changed = conn
        .execute("DELETE FROM task_groups WHERE group_id = ?1", [group_id])
        .map_err(|e| format!("删除分组失败: {e}"))?;
    if changed > 0 {
        // 级联删成员（部分 SQLite 构建默认不开 foreign_keys，显式兜底）。
        conn.execute("DELETE FROM task_group_members WHERE group_id = ?1", [group_id])
            .map_err(|e| format!("清理组成员失败: {e}"))?;
    }
    Ok(changed > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskdb::ensure_schema;

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        conn
    }

    fn seed_task(conn: &Connection, ws: &str, task_id: &str, updated: i64) {
        conn.execute(
            "INSERT INTO tasks (workspace_key, workspace_path, task_id, title, task_status, \
             created_at, updated_at) VALUES (?1, 'C:/ws', ?2, ?3, 'idle', 0, ?4)",
            rusqlite::params![ws, task_id, format!("任务 {task_id}"), updated],
        )
        .unwrap();
    }

    #[test]
    fn color_classes_cover_all_seven() {
        for c in GROUP_COLORS {
            let (light, dark) = color_classes(c);
            assert!(!light.is_empty() && !dark.is_empty(), "{c} 缺类名");
            assert!(light.contains("bg-"), "{c} 亮色类名应以 bg- 开头");
        }
    }

    #[test]
    fn unknown_color_falls_back_to_gray() {
        assert_eq!(color_classes("chartreuse"), color_classes("gray"));
        assert_eq!(border_color_class("nope"), border_color_class("gray"));
    }

    #[test]
    fn parse_node_key_unescapes_json_backslashes() {
        // 真实库形态（实测）：JSON.stringify 把 Windows 路径反斜杠转义成双反斜杠。
        // 解析后必须还原成单反斜杠，否则和 tasks.workspace_key 比对不上、排序全失效。
        let key = r#"["C:\\Users\\liuxinlong\\.zcode\\workspace\\default","sess_9cc7"]"#;
        let (ws, task) = parse_node_key(key).unwrap();
        assert_eq!(ws, r"C:\Users\liuxinlong\.zcode\workspace\default");
        assert_eq!(task, "sess_9cc7");
    }

    #[test]
    fn parse_node_key_handles_forward_slash_paths() {
        let key = r#"["D:/ws","sess_1"]"#;
        let (ws, task) = parse_node_key(key).unwrap();
        assert_eq!(ws, "D:/ws");
        assert_eq!(task, "sess_1");
    }

    #[test]
    fn node_sort_key_matches_stored_format() {
        // 我们生成的 key 必须和真源写入的格式一致，否则永远匹配不上排序记录。
        let node = GroupedNode::Task {
            task: "sess_1".into(),
        };
        let key = node_sort_key(r"C:\ws", &node);
        assert_eq!(key, r#"["C:\\ws","sess_1"]"#);
        // 往返一致。
        let (ws, id) = parse_node_key(&key).unwrap();
        assert_eq!(ws, r"C:\ws");
        assert_eq!(id, "sess_1");
    }

    #[test]
    fn parse_node_key_handles_non_ascii_path() {
        // 中文工作区名是真实场景（实测库里有 C:\Users\...\Desktop\问题处理）。
        // 解析按 UTF-8 字节收集，不能把多字节字符拆坏。
        let key = r#"["D:\\工作\\处理","sess_中文"]"#;
        let (ws, task) = parse_node_key(key).unwrap();
        assert_eq!(ws, r"D:\工作\处理");
        assert_eq!(task, "sess_中文");
    }

    #[test]
    fn parse_node_key_rejects_malformed() {
        assert!(parse_node_key("not-json").is_none());
        assert!(parse_node_key(r#"["only-one"]"#).is_none());
    }

    #[test]
    fn empty_workspace_yields_loose_tasks_only() {
        let conn = memory_db();
        let ws = workspace_key("C:/ws", None);
        seed_task(&conn, &ws, "t1", 100);
        seed_task(&conn, &ws, "t2", 200);

        let view = read_grouped_view(&conn, "C:/ws", None).unwrap();
        assert_eq!(view.nodes.len(), 2, "无分组时全是游离任务");
        assert!(
            view.nodes
                .iter()
                .all(|n| matches!(n, GroupedNode::Task { .. })),
            "不该出现组节点"
        );
    }

    #[test]
    fn grouped_task_moves_from_loose_into_group() {
        let conn = memory_db();
        let ws = workspace_key("C:/ws", None);
        seed_task(&conn, &ws, "t1", 100);
        seed_task(&conn, &ws, "t2", 200);
        create_group(&conn, "g1", "工作", "blue", 1000).unwrap();

        add_task_to_group(&conn, "C:/ws", None, "g1", "t1", 1000).unwrap();

        let view = read_grouped_view(&conn, "C:/ws", None).unwrap();
        let group_nodes: Vec<_> = view
            .nodes
            .iter()
            .filter_map(|n| match n {
                GroupedNode::Group { group, task_ids } => Some((group, task_ids)),
                _ => None,
            })
            .collect();
        assert_eq!(group_nodes.len(), 1);
        assert_eq!(group_nodes[0].0.title, "工作");
        assert_eq!(group_nodes[0].0.color, "blue");
        assert_eq!(group_nodes[0].1.as_slice(), ["t1".to_string()]);

        // t1 不应再作为游离任务出现。
        let loose: Vec<&String> = view
            .nodes
            .iter()
            .filter_map(|n| match n {
                GroupedNode::Task { task } => Some(task),
                _ => None,
            })
            .collect();
        assert_eq!(loose, vec!["t2"], "只有 t2 应留在游离列表");
    }

    #[test]
    fn one_task_belongs_to_single_group() {
        // task_group_members 主键是 (workspace_key, task_id)，不含 group_id。
        let conn = memory_db();
        let ws = workspace_key("C:/ws", None);
        seed_task(&conn, &ws, "t1", 100);
        create_group(&conn, "g1", "A", "red", 1000).unwrap();
        create_group(&conn, "g2", "B", "green", 2000).unwrap();

        add_task_to_group(&conn, "C:/ws", None, "g1", "t1", 1000).unwrap();
        add_task_to_group(&conn, "C:/ws", None, "g2", "t1", 2000).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_group_members WHERE workspace_key = ?1",
                [&ws],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "重复入组应覆盖而非新增行");
    }

    #[test]
    fn add_to_group_rejects_missing_task() {
        let conn = memory_db();
        create_group(&conn, "g1", "A", "red", 1000).unwrap();
        assert!(add_task_to_group(&conn, "C:/ws", None, "g1", "nope", 1000).is_err());
    }

    #[test]
    fn remove_task_returns_it_to_loose() {
        let conn = memory_db();
        let ws = workspace_key("C:/ws", None);
        seed_task(&conn, &ws, "t1", 100);
        create_group(&conn, "g1", "A", "red", 1000).unwrap();
        add_task_to_group(&conn, "C:/ws", None, "g1", "t1", 1000).unwrap();

        assert!(remove_task_from_group(&conn, "C:/ws", None, "t1").unwrap());

        let view = read_grouped_view(&conn, "C:/ws", None).unwrap();
        assert!(
            matches!(view.nodes.first(), Some(GroupedNode::Task { task }) if task == "t1"),
            "移出组后应回到游离任务"
        );
    }

    #[test]
    fn delete_group_cascades_members() {
        let conn = memory_db();
        let ws = workspace_key("C:/ws", None);
        seed_task(&conn, &ws, "t1", 100);
        create_group(&conn, "g1", "A", "red", 1000).unwrap();
        add_task_to_group(&conn, "C:/ws", None, "g1", "t1", 1000).unwrap();

        assert!(delete_group(&conn, "g1").unwrap());

        let members: i64 = conn
            .query_row("SELECT COUNT(*) FROM task_group_members", [], |r| r.get(0))
            .unwrap();
        assert_eq!(members, 0, "删组要连带清成员");

        // 任务本身不该被删，只是回到游离状态。
        let view = read_grouped_view(&conn, "C:/ws", None).unwrap();
        assert_eq!(view.nodes.len(), 1);
        assert!(matches!(view.nodes[0], GroupedNode::Task { .. }));
    }

    #[test]
    fn pinned_and_archived_tasks_excluded_from_grouped_view() {
        // 真源 buildGroupedTaskViewFromSessions 会过滤 pinned/archived
        // （pinnedIds/archivedIds 口径）。
        let conn = memory_db();
        let ws = workspace_key("C:/ws", None);
        seed_task(&conn, &ws, "loose", 100);
        seed_task(&conn, &ws, "pinned", 100);
        conn.execute("UPDATE tasks SET pinned = 1 WHERE task_id = 'pinned'", [])
            .unwrap();
        seed_task(&conn, &ws, "arch", 100);
        conn.execute("UPDATE tasks SET archived = 1 WHERE task_id = 'arch'", [])
            .unwrap();

        let view = read_grouped_view(&conn, "C:/ws", None).unwrap();
        let tasks: Vec<&String> = view
            .nodes
            .iter()
            .filter_map(|n| match n {
                GroupedNode::Task { task } => Some(task),
                _ => None,
            })
            .collect();
        assert_eq!(tasks, vec!["loose"], "置顶/归档任务不进分组视图");
    }

    #[test]
    fn create_group_normalizes_unknown_color() {
        let conn = memory_db();
        let g = create_group(&conn, "g1", "标题", "不存在", 1).unwrap();
        assert_eq!(g.color, "gray", "未知颜色应回落 gray");
    }

    #[test]
    fn groups_isolated_per_workspace() {
        let conn = memory_db();
        let ws_a = workspace_key("C:/a", None);
        // B 工作区不需seed key（下面用字面量插入）。
        seed_task(&conn, &ws_a, "ta", 100);
        conn.execute(
            "INSERT INTO tasks (workspace_key, workspace_path, task_id, title, task_status, \
             created_at, updated_at) VALUES ('C:/b', 'C:/b', 'tb', 'b', 'idle', 0, 100)",
            [],
        )
        .unwrap();
        create_group(&conn, "g1", "A组", "red", 1000).unwrap();
        add_task_to_group(&conn, "C:/a", None, "g1", "ta", 1000).unwrap();

        let view_a = read_grouped_view(&conn, "C:/a", None).unwrap();
        assert_eq!(view_a.nodes.len(), 1, "A 组只有 1 个组节点");
        match &view_a.nodes[0] {
            GroupedNode::Group { group, task_ids } => {
                assert_eq!(task_ids.as_slice(), ["ta".to_string()]);
                assert_eq!(group.group_id, "g1");
            }
            other => panic!("期望组节点，实际 {:?}", other),
        }

        let view_b = read_grouped_view(&conn, "C:/b", None).unwrap();
        assert!(
            matches!(view_b.nodes.first(), Some(GroupedNode::Task { task }) if task == "tb"),
            "B 工作区不该看到 A 的分组"
        );
    }
}