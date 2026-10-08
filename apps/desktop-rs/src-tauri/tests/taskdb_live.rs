//! 针对**真实用户任务库**的集成测试（与 agent_live.rs 同类：跑真实环境）。
//!
//! 单测用 in-memory 库验证 SQL 语义；本测试验证真实库文件
//! （~/.zcode/v2/tasks-index.sqlite）的 schema 与我们的查询兼容。
//! **全程只读**，不写用户数据。

use zcode_desktop_rs_lib::taskdb::{self, TaskKind};

#[test]
fn live_reads_real_tasks_database() {
    let Some(conn) = taskdb::open_readonly().expect("打开真实任务库") else {
        eprintln!("[live] 跳过：任务库尚未创建（全新安装）");
        return;
    };

    // 真实库 tasks 表必须存在且列名与查询一致（schema-v1.ts 的列）。
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
        .expect("统计真实库任务数");
    println!("[live] 真实任务库可读，tasks 共 {total} 行");

    // 取一个真实存在的 workspace，验证三个分区查询都不崩且口径互斥。
    let sample: Option<(String, String, Option<String>)> = conn
        .query_row(
            "SELECT workspace_key, workspace_path, workspace_identity FROM tasks LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok();
    let Some((ws_key, ws_path, ws_identity)) = sample else {
        println!("[live] 库为空，跳过分区查询验证");
        return;
    };
    println!("[live] 样例 workspace_key={ws_key}");

    let pinned = taskdb::list_tasks(&conn, &ws_path, ws_identity.as_deref(), TaskKind::Pinned)
        .expect("查询 pinned 分区");
    let active = taskdb::list_tasks(&conn, &ws_path, ws_identity.as_deref(), TaskKind::Active)
        .expect("查询 active 分区");
    let archived =
        taskdb::list_tasks(&conn, &ws_path, ws_identity.as_deref(), TaskKind::Archived)
            .expect("查询 archived 分区");
    println!(
        "[live] 该 workspace 分区计数 pinned={} active={} archived={}",
        pinned.len(),
        active.len(),
        archived.len()
    );

    // 关键口径校验：pinned 与 active 互斥（真源 SQL 强制 pinned=0 AND archived=0）。
    let pinned_ids: std::collections::HashSet<&str> =
        pinned.iter().map(|t| t.task_id.as_str()).collect();
    for task in &active {
        assert!(
            !pinned_ids.contains(task.task_id.as_str()),
            "任务 {} 同时出现在 pinned 和 active，置顶区与主列表互斥被破坏",
            task.task_id
        );
    }
    assert!(
        pinned.len() + active.len() + archived.len() <= total as usize,
        "分区计数不应超过总行数"
    );
}