// 探测真实库里任务分组相关表的数据情况。
// 目的：确认分组功能在真实数据里的形态，决定 Rust 侧实现范围。
const { DatabaseSync } = require("node:sqlite");
const os = require("node:os");
const path = require("node:path");

const db = new DatabaseSync(path.join(os.homedir(), ".zcode", "v2", "tasks-index.sqlite"), {
  readOnly: true,
});

const tables = db
  .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '%group%' ORDER BY name")
  .all();
console.log("分组相关表:", tables.map((t) => t.name).join(", "));

for (const t of tables) {
  const n = db.prepare(`SELECT COUNT(*) n FROM ${t.name}`).get().n;
  console.log(`\n--- ${t.name}: ${n} 行---`);
  if (n > 0) {
    const sample = db.prepare(`SELECT * FROM ${t.name} LIMIT 3`).all();
    for (const row of sample) console.log("   ", JSON.stringify(row));
  }
}

// group_color 取值范围（UI 要映射成 Tailwind 类）。
const colors = db.prepare("SELECT DISTINCT color FROM task_groups").all();
console.log("\n实际使用的分组颜色:", JSON.stringify(colors.map((c) => c.color)));