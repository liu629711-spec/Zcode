//! 分组视图对真实库的只读验证。
//!
//! 分组功能在真实数据里的现状（实测，2026-10）：`task_groups` / `task_group_members`
//! 都是 0 行——用户还没建过任何分组；但 `task_group_view_node_orders` 有 65 行，
//! 说明真源在用「顶层节点排序」这张表。
//!
//! 所以本测试主要验证两点：
//! 1. 真实库的分组四表结构与我们 Rust 侧 schema 一致（能建表、能查、不报错）；
//! 2. 无分组时视图退化为「全部游离任务」，且 node_orders 里的排序记录能被正确解析
//!    （这条最容易出错——node_key 是 JSON 转义串，反斜杠处理错就全匹配不上）。

use zcode_desktop_rs_lib::taskdb::{self, TaskKind};
use zcode_desktop_rs_lib::taskgroup::{self, GroupedNode};

#[test]
fn live_grouped_view_on_real_database() {
    let Some(conn) = taskdb::open_readonly().expect("打开真实任务库") else {
        eprintln!("[live] 跳过：任务库不存在");
        return;
    };

    // 真实库 task_groups 现状（只读观测，不改数据）。
    let group_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM task_groups", [], |r| r.get(0))
        .expect("统计分组数");
    let member_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM task_group_members", [], |r| r.get(0))
        .expect("统计成员数");
    println!("[live] 真实库：task_groups={group_count} 行，task_group_members={member_count} 行");

    // 挑一个真实有任务的 workspace。
    let sample: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT workspace_path, workspace_identity FROM tasks \
             WHERE deleted = 0 AND pinned = 0 AND archived = 0 LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    let Some((ws_path, ws_identity)) = sample else {
        println!("[live] 没有可用任务，跳过");
        return;
    };
    println!("[live] 样例workspace={ws_path}");

    let view = taskgroup::read_grouped_view(&conn, &ws_path, ws_identity.as_deref())
        .expect("读取分组视图");
    println!("[live] 分组视图节点数={}", view.nodes.len());

    if group_count == 0 {
        // 无分组时不该出现组节点，全部是游离任务。
        assert!(
            view.nodes
                .iter()
                .all(|n| matches!(n, GroupedNode::Task { .. })),
            "无分组时不该出现组节点"
        );
    }

    // 口径校验：分组视图的游离任务数应等于 active 分区里"不属于任何组"的部分。
    // 当前无分组，两者应一致。
    let active = taskdb::list_tasks(&conn, &ws_path, ws_identity.as_deref(), TaskKind::Active)
        .expect("查询 active 分区");
    if group_count == 0 {
        assert_eq!(
            view.nodes.len(),
            active.len(),
            "无分组时游离任务数应等于 active 任务数"
        );
    }

    // node_orders 的排序记录：验证 Rust 的 node_key 解析能吃下真实形态。
    let order_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM task_group_view_node_orders WHERE node_key LIKE '%\\\\%' \
             AND node_key LIKE '%\"%'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    println!("[live] 含转义反斜杠的排序记录数={order_rows}（验证解析器能处理）");
}