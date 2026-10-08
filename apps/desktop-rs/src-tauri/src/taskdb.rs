//! 本地任务索引库（`~/.zcode/v2/tasks-index.sqlite`）。
//!
//! 真源链路（全部 Rust 重写，不再依赖 node:sqlite）：
//! - 库路径 `getTasksIndexDatabasePath()` → `paths.ts:186` = `~/.zcode/v2/tasks-index.sqlite`
//! - 表结构 `tasksDatabase/schema-v1.ts:1-33`（pinned/archived/deleted 三列 + 复合主键）
//! - 分区查询 `taskIndexRepo.ts:1934-1940`：`pinned` 分区 = `pinned=1 AND archived=0`
//!
//! **workspace_key 存的是原始 workspacePath/identity，不是 hash。**
//! 这一点是实测真实库（65 行、9 个 workspace）确认的：distinct workspace_key
//! 与 distinct workspace_path 数量相等且值逐条相同，identity 列全为 null。
//! 库里另有 `getWorkspaceHash`（SHA-256 前 12 位，paths.ts:196），但它只用于
//! git checkpoint 目录名和 claude-native session import，与 tasks 主键无关——
//! 别被函数名误导。
//!
//! 这里只做数据存取，不含 UI 决策（如展示哪些）。排序/分区语义照抄真源 SQL。

use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;

/// workspace 身份键：远程优先 workspaceIdentity，本地回退 workspacePath
/// （真源 `paths.ts:188` getWorkspaceKey）。
///
/// 真源该函数不在本仓库内（在 node 侧服务实现中），语义由 `tasks` 表实测数据反推确认。
pub fn workspace_key(workspace_path: &str, workspace_identity: Option<&str>) -> String {
    workspace_identity
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(workspace_path)
        .to_string()
}

/// 任务库文件路径（`~/.zcode/v2/tasks-index.sqlite`）。
///
/// 真源通过 `getZCodeDataRootDir()` 解析，支持 `ZCODE_*` 环境变量覆盖
/// （`paths.ts:57readEnvValue`）。Rust 侧先只支持 `~/.zcode`，覆盖规则等用到再补。
pub fn database_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| "无法定位用户主目录".to_string())?;
    let base = PathBuf::from(home).join(".zcode").join("v2");
    std::fs::create_dir_all(&base).map_err(|e| format!("创建数据目录失败: {e}"))?;
    Ok(base.join("tasks-index.sqlite"))
}

/// 打开任务库（只读）。
///
/// 库不存在时返回 None 而非报错——全新安装还没有库是正常状态，
/// 上层应展示"暂无置顶"而不是弹错误。
pub fn open_readonly() -> Result<Option<Connection>, String> {
    let path = database_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let conn = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("打开任务库失败: {e}"))?;
    Ok(Some(conn))
}

/// 打开任务库（读写），不存在则按真源 schema 建表。
pub fn open_readwrite() -> Result<Connection, String> {
    let path = database_path()?;
    let conn = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("打开任务库失败: {e}"))?;
    // 真源 startup.ts:47 设busy_timeout=25（毫秒），这里放宽些给桌面端更从容的重试。
    conn.busy_timeout(std::time::Duration::from_millis(2000))
        .map_err(|e| format!("设置 busy_timeout 失败: {e}"))?;
    ensure_schema(&conn)?;
    Ok(conn)
}

/// 建表（照抄真源 schema-v1.ts 的 tasks 表，其他表按需再加）。
fn ensure_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
          workspace_key TEXT NOT NULL,
          workspace_path TEXT NOT NULL,
          workspace_identity TEXT,
          task_id TEXT NOT NULL,
          title TEXT NOT NULL DEFAULT '',
          task_status TEXT,
          provider TEXT,
          mode TEXT NOT NULL DEFAULT 'build',
          model TEXT,
          migration_source TEXT,
          forked_from_task_id TEXT,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL,
          unread_at INTEGER,
          last_unread_at INTEGER NOT NULL DEFAULT 0,
          pinned INTEGER NOT NULL DEFAULT 0,
          archived INTEGER NOT NULL DEFAULT 0,
          deleted INTEGER NOT NULL DEFAULT 0,
          title_overridden INTEGER NOT NULL DEFAULT 0,
          meta_json TEXT NOT NULL DEFAULT '{}',
          PRIMARY KEY (workspace_key, task_id)
        );
        CREATE INDEX IF NOT EXISTS idx_tasks_workspace_pinned_updated
          ON tasks (workspace_key, pinned, updated_at DESC)
          WHERE deleted = 0;
        "#,
    )
    .map_err(|e| format!("建表失败: {e}"))
}

/// 任务行（tasks 表投影，与真源 ZCodeTaskMeta 的公共字段对齐）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRow {
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

const TASK_COLUMNS: &str = "task_id, workspace_path, workspace_identity, title, \
                           task_status, created_at, updated_at, pinned, archived";

fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRow> {
    Ok(TaskRow {
        task_id: row.get(0)?,
        workspace_path: row.get(1)?,
        workspace_identity: row.get(2)?,
        title: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        pinned: row.get::<_, i64>(7)? != 0,
        archived: row.get::<_, i64>(8)? != 0,
    })
}

/// 任务分区（真源 `zcodeTaskListTypes.ts:3` ZCodeTaskListKind）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    /// 置顶区：pinned=1 AND archived=0
    Pinned,
    /// 归档：archived=1
    Archived,
    /// 常规活动区：pinned=0 AND archived=0（真源 else 分支，置顶项**不会**重复出现）
    Active,
}

impl TaskKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pinned" => Some(Self::Pinned),
            "archived" => Some(Self::Archived),
            "active" => Some(Self::Active),
            _ => None,
        }
    }
}

/// 按分区分组查询任务。
///
/// SQL 语义照抄 `taskIndexRepo.ts:1934-1940`：
/// ```text
/// if (kind === "pinned")       where.push("pinned = 1", "archived = 0");
/// else if (kind === "archived") where.push("archived = 1");
/// else                         where.push("pinned = 0", "archived = 0");
/// ```
/// 排序（:1961-1964）默认按 updated_at DESC。
pub fn list_tasks(
    conn: &Connection,
    ws_path: &str,
    ws_identity: Option<&str>,
    kind: TaskKind,
) -> Result<Vec<TaskRow>, String> {
    let ws_key = workspace_key(ws_path, ws_identity);
    let (filter, order) = match kind {
        TaskKind::Pinned => ("pinned = 1 AND archived = 0", "updated_at DESC, created_at DESC"),
        TaskKind::Archived => ("archived = 1", "updated_at DESC, created_at DESC"),
        TaskKind::Active => (
            "pinned = 0 AND archived = 0",
            "updated_at DESC, created_at DESC",
        ),
    };
    let sql = format!(
        "SELECT {TASK_COLUMNS} FROM tasks \
         WHERE workspace_key = ?1 AND deleted = 0 AND {filter} \
         ORDER BY {order}"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("准备查询失败: {e}"))?;
    let rows = stmt
        .query_map([ws_key], row_to_task)
        .map_err(|e| format!("执行查询失败: {e}"))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| format!("读取结果失败: {e}"))
}

/// 设置置顶状态（真源 `zcodeTaskServiceAdapter.ts:3060-3067` setTaskPinned）。
///
/// 真源是三段式（overlay → SQLite → 事件）；Rust 侧只有持久层，
/// 事件那一段由前端在调用成功后自行刷新列表承担。
///
/// 任务不存在时返回 Ok(false)——不擅自插入半截行，避免污染真源的数据。
pub fn set_pinned(
    conn: &Connection,
    ws_path: &str,
    ws_identity: Option<&str>,
    task_id: &str,
    pinned: bool,
) -> Result<bool, String> {
    let ws_key = workspace_key(ws_path, ws_identity);
    let changed = conn
        .execute(
            "UPDATE tasks SET pinned = ?2 WHERE workspace_key = ?1 AND task_id = ?3",
            rusqlite::params![ws_key, pinned as i64, task_id],
        )
        .map_err(|e| format!("更新置顶状态失败: {e}"))?;
    Ok(changed > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        conn
    }

    fn seed(conn: &Connection, ws: &str, task_id: &str, pinned: i64, archived: i64, updated: i64) {
        conn.execute(
            "INSERT INTO tasks (workspace_key, workspace_path, task_id, title, \
             task_status, created_at, updated_at, pinned, archived) \
             VALUES (?1, ?2, ?3, ?4, 'idle', 0, ?5, ?6, ?7)",
            rusqlite::params![ws, "C:/ws", task_id, format!("任务 {task_id}"), updated, pinned, archived],
        )
        .unwrap();
    }

    #[test]
    fn workspace_key_prefers_identity() {
        assert_eq!(workspace_key("C:/ws", Some("  ")), "C:/ws", "空白 identity 回退路径");
        assert_eq!(workspace_key("C:/ws", None), "C:/ws");
        assert_eq!(workspace_key("C:/ws", Some("remote-1")), "remote-1");
    }

    #[test]
    fn workspace_key_is_raw_path_not_hash() {
        // 实测真实库：workspace_key 就是原始 Windows 路径（反斜杠保留），
        // 不是 SHA-256 摘要。之前按函数名 getWorkspaceHash 推断成 hash 是错的，
        // 真源那个 hash 只用于 git checkpoint 目录名，不进 tasks 主键。
        let raw = "C:\\Users\\liuxinlong\\Desktop\\问题处理";
        assert_eq!(workspace_key(raw, None), raw);
    }

    #[test]
    fn workspace_key_uses_identity_when_present() {
        // 远端 workspace 的 workspace_key 存远端 identity（taskIndexRepo.ts:206）。
        assert_eq!(
            workspace_key("C:/ws", Some("remote-1")),
            "remote-1",
            "有 identity 时用 identity"
        );
        assert_ne!(
            workspace_key("C:/ws", None),
            workspace_key("C:/ws", Some("remote")),
            "identity 存在与否必须影响 key，否则同路径不同远端会串行"
        );
    }

    #[test]
    fn kind_parse_roundtrip() {
        assert_eq!(TaskKind::parse("pinned"), Some(TaskKind::Pinned));
        assert_eq!(TaskKind::parse("archived"), Some(TaskKind::Archived));
        assert_eq!(TaskKind::parse("active"), Some(TaskKind::Active));
        assert_eq!(TaskKind::parse("nope"), None);
    }

    #[test]
    fn pinned_partition_excludes_archived_and_active() {
        let conn = memory_db();
        let ws = workspace_key("C:/ws", None);
        seed(&conn, &ws, "p1", 1, 0, 100);
        seed(&conn, &ws, "p2", 1, 1, 200); // 置顶+归档 → 两个分区都不该出
        seed(&conn, &ws, "a1", 0, 0, 300);

        let pinned = list_tasks(&conn, "C:/ws", None, TaskKind::Pinned).unwrap();
        assert_eq!(pinned.len(), 1);
        assert_eq!(pinned[0].task_id, "p1");

        // 互斥：active 查询条件是 pinned=0，置顶项不重复出现（真源 SQL 强制）。
        let active = list_tasks(&conn, "C:/ws", None, TaskKind::Active).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].task_id, "a1");

        let archived = list_tasks(&conn, "C:/ws", None, TaskKind::Archived).unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].task_id, "p2");
    }

    #[test]
    fn list_sorted_by_updated_desc() {
        let conn = memory_db();
        let ws = workspace_key("C:/ws", None);
        seed(&conn, &ws, "old", 1, 0, 100);
        seed(&conn, &ws, "new", 1, 0, 300);
        seed(&conn, &ws, "mid", 1, 0, 200);

        let rows = list_tasks(&conn, "C:/ws", None, TaskKind::Pinned).unwrap();
        let ids: Vec<&str> = rows.iter().map(|r| r.task_id.as_str()).collect();
        assert_eq!(ids, vec!["new", "mid", "old"]);
    }

    #[test]
    fn deleted_rows_are_hidden() {
        let conn = memory_db();
        let ws = workspace_key("C:/ws", None);
        seed(&conn, &ws, "gone", 1, 0, 100);
        conn.execute("UPDATE tasks SET deleted = 1 WHERE task_id = 'gone'", [])
            .unwrap();

        assert!(list_tasks(&conn, "C:/ws", None, TaskKind::Pinned).unwrap().is_empty());
    }

    #[test]
    fn workspaces_are_isolated() {
        let conn = memory_db();
        let ws_a = workspace_key("C:/a", None);
        let ws_b = workspace_key("C:/b", None);
        seed(&conn, &ws_a, "task-a", 1, 0, 100);
        seed(&conn, &ws_b, "task-b", 1, 0, 100);

        let rows = list_tasks(&conn, "C:/a", None, TaskKind::Pinned).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].task_id, "task-a");
    }

    #[test]
    fn set_pinned_updates_and_reports_missing() {
        let conn = memory_db();
        let ws = workspace_key("C:/ws", None);
        seed(&conn, &ws, "t1", 0, 0, 100);

        assert!(set_pinned(&conn, "C:/ws", None, "t1", true).unwrap());
        let rows = list_tasks(&conn, "C:/ws", None, TaskKind::Pinned).unwrap();
        assert_eq!(rows.len(), 1, "置顶后应进入pinned 分区");

        // 取消置顶 → 从pinned 分区消失，回到 active。
        assert!(set_pinned(&conn, "C:/ws", None, "t1", false).unwrap());
        assert!(list_tasks(&conn, "C:/ws", None, TaskKind::Pinned).unwrap().is_empty());
        assert_eq!(list_tasks(&conn, "C:/ws", None, TaskKind::Active).unwrap().len(), 1);

        // 不存在的任务不插入、返回 false。
        assert!(!set_pinned(&conn, "C:/ws", None, "nope", true).unwrap());
    }
}