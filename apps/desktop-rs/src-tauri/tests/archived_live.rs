//! 归档区端到端验证：**在真实库的副本上**跑，绝不动用户数据。
//!
//! 为什么要副本：unarchive/delete 是写操作，直接对~/.zcode/v2/tasks-index.sqlite
//! 跑会真实修改用户的归档状态。单测用 in-memory 库覆盖不到"真实 schema +
//! 真实数据形态"的组合，所以这里复制真实库到临时目录再验。

use std::path::PathBuf;

use zcode_desktop_rs_lib::taskdb::{self, TaskKind};

/// 复制真实库到临时文件；真实库不存在时返回 None。
fn copy_real_db() -> Option<(PathBuf, i64)> {
    let src = taskdb::database_path().ok()?;
    if !src.exists() {
        eprintln!("[live] 跳过：真实任务库不存在");
        return None;
    }
    let total: i64 = {
        // open_readonly 返回 Option（库不存在时为 None），这里已确认文件存在故unwrap。
        let conn = taskdb::open_readonly()
            .expect("打开真实库")
            .expect("真实库文件已确认存在，不应为 None");
        conn.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .expect("统计真实库")
    };
    let tmp = std::env::temp_dir().join(format!("zcode-taskdb-test-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    std::fs::copy(&src, &tmp).expect("复制真实库到临时文件");
    Some((tmp, total))
}

#[test]
fn live_unarchive_and_delete_on_real_db_copy() {
    let Some((tmp_path, total)) = copy_real_db() else {
        return;
    };
    println!("[live] 已复制真实库（{total} 行）到 {}", tmp_path.display());

    let conn = rusqlite::Connection::open(&tmp_path).expect("打开副本");

    // 挑一个真实归档任务（真源里 archived 分区就是取消归档的来源）。
    let sample: Option<(String, String, Option<String>)> = conn
        .query_row(
            "SELECT workspace_key, workspace_path, workspace_identity FROM tasks \
             WHERE archived = 1 AND deleted = 0 LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok();

    let Some((_ws_key, ws_path, ws_identity)) = sample else {
        println!("[live] 真实库里没有归档任务，跳过写操作验证");
        let _ = std::fs::remove_file(&tmp_path);
        return;
    };
    println!("[live] 选中归档任务所在 workspace={ws_path}");

    let archived_before =
        taskdb::list_tasks(&conn, &ws_path, ws_identity.as_deref(), TaskKind::Archived)
            .expect("查询归档分区");
    println!("[live] 取消归档前 archived 数量={}", archived_before.len());
    assert!(!archived_before.is_empty(), "样本任务必须在 archived 分区里");

    let target = archived_before[0].task_id.clone();

    // --- 取消归档 ---
    let changed = taskdb::set_archived(&conn, &ws_path, ws_identity.as_deref(), &target, false)
        .expect("执行取消归档");
    assert!(changed, "取消归档应命中一行");

    let archived_after =
        taskdb::list_tasks(&conn, &ws_path, ws_identity.as_deref(), TaskKind::Archived)
            .expect("重新查询归档分区");
    assert_eq!(
        archived_after.len(),
        archived_before.len() - 1,
        "取消归档后 archived 分区应少一条"
    );
    assert!(
        !archived_after.iter().any(|t| t.task_id == target),
        "目标任务不应还在归档列表"
    );
    // 取消归档后应回到 active 分区。
    let active = taskdb::list_tasks(&conn, &ws_path, ws_identity.as_deref(), TaskKind::Active)
        .expect("查询活动分区");
    assert!(
        active.iter().any(|t| t.task_id == target),
        "取消归档后任务应回到 active 分区"
    );
    println!("[live] 取消归档验证通过");

    // --- 软删除 ---
    let deleted =
        taskdb::delete_task(&conn, &ws_path, ws_identity.as_deref(), &target).expect("执行软删除");
    assert!(deleted, "软删除应命中一行");

    // 行必须还在（tombstone join 需要）。
    let row_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE task_id = ?1",
            [&target],
            |r| r.get(0),
        )
        .expect("统计目标行");
    assert_eq!(row_count, 1, "软删除不能真的删行");

    // 所有分区都看不到它。
    for kind in [TaskKind::Pinned, TaskKind::Active, TaskKind::Archived] {
        let rows = taskdb::list_tasks(&conn, &ws_path, ws_identity.as_deref(), kind)
            .expect("查询分区");
        assert!(
            !rows.iter().any(|t| t.task_id == target),
            "{kind:?} 分区不应暴露已删除任务"
        );
    }
    // 总行数不变（软删）。
    let total_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
        .expect("统计总行数");
    assert_eq!(total_after, total, "软删除后总行数应不变");
    println!("[live] 软删除验证通过（行保留、不可见、总行数不变）");

    let _ = std::fs::remove_file(&tmp_path);
}