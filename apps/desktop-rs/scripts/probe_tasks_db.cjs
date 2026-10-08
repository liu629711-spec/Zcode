// 探测真实任务库里 workspace_key 的实际存储格式。
// 目的：确认它是原始 workspacePath 还是 hash——这决定Rust 侧查询口径。
const { DatabaseSync } = require("node:sqlite");
const os = require("node:os");
const path = require("node:path");

const dbPath = path.join(os.homedir(), ".zcode", "v2", "tasks-index.sqlite");
const db = new DatabaseSync(dbPath, { readOnly: true });

const rows = db
  .prepare(
    "SELECT DISTINCT workspace_key, workspace_path, workspace_identity FROM tasks LIMIT 6",
  )
  .all();
for (const r of rows) {
  console.log("key  :", JSON.stringify(r.workspace_key));
  console.log("path :", JSON.stringify(r.workspace_path));
  console.log("ident:", JSON.stringify(r.workspace_identity));
  console.log("---");
}

const c = db
  .prepare(
    "SELECT COUNT(DISTINCT workspace_key) k, COUNT(DISTINCT workspace_path) p FROM tasks",
  )
  .get();
console.log("distinct keys:", c.k, " distinct paths:", c.p);
console.log(
  "pinned/active/archived:",
  JSON.stringify(
    db
      .prepare(
        "SELECT SUM(pinned=1 AND archived=0 AND deleted=0) p, \
         SUM(pinned=0 AND archived=0 AND deleted=0) a, \
         SUM(archived=1 AND deleted=0) ar FROM tasks",
      )
      .get(),
  ),
);
console.log("\n建表 SQL（真库实际 schema）:");
console.log(
  db
    .prepare("SELECT sql FROM sqlite_master WHERE name='tasks'")
    .get()?.sql,
);