import assert from "node:assert/strict";
import test from "node:test";
import type { BrowserStorageLike } from "../src/lib/browserEnvironment.js";
import {
  persistSidebarTaskPreferences,
  readSidebarTaskPreferences,
} from "../src/lib/sidebarTaskPreferences.js";

// ============================================================
// 侧栏任务偏好的可运行检查（内存 storage 直跑）
// ============================================================
// 钉的是 round-trip 与非法值/遗留键容错：boardMode 已随侧栏看板退场删除，
// localStorage 里的遗留 boardMode 键必须被宽容忽略而不是破坏解析。
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/sidebarTaskPreferences.test.ts

function memoryStorage(initial?: Record<string, string>): BrowserStorageLike & { data: Map<string, string> } {
  const data = new Map<string, string>(Object.entries(initial ?? {}));
  return {
    data,
    getItem: (key) => data.get(key) ?? null,
    setItem: (key, value) => {
      data.set(key, value);
    },
  };
}

test("round-trip：persist 后 read 原样取回", () => {
  const storage = memoryStorage();
  persistSidebarTaskPreferences(
    { organizeBy: "grouped", sortBy: "created" },
    storage,
  );
  assert.deepEqual(readSidebarTaskPreferences(storage), {
    organizeBy: "grouped",
    sortBy: "created",
  });

  persistSidebarTaskPreferences(
    { organizeBy: "project", sortBy: "updated" },
    storage,
  );
  assert.deepEqual(readSidebarTaskPreferences(storage), {
    organizeBy: "project",
    sortBy: "updated",
  });
});

test("非法值回落：organizeBy/sortBy 非法值回落默认，未知遗留键不破坏解析", () => {
  const storage = memoryStorage({
    "zcode-sidebar-task-preferences": JSON.stringify({
      organizeBy: "bogus",
      sortBy: 42,
      boardMode: "yes",
    }),
  });
  assert.deepEqual(readSidebarTaskPreferences(storage), {
    organizeBy: "project",
    sortBy: "updated",
  });
});

test("遗留键失效：旧版本 payload 带 boardMode 键时被忽略，既有键正常读出", () => {
  const storage = memoryStorage({
    "zcode-sidebar-task-preferences": JSON.stringify({
      organizeBy: "chronological",
      sortBy: "created",
      boardMode: true,
    }),
  });
  assert.deepEqual(readSidebarTaskPreferences(storage), {
    organizeBy: "chronological",
    sortBy: "created",
  });
});

test("损坏 JSON / 空 storage：整体回落默认值", () => {
  assert.deepEqual(
    readSidebarTaskPreferences(memoryStorage({ "zcode-sidebar-task-preferences": "{not json" })),
    { organizeBy: "project", sortBy: "updated" },
  );
  assert.deepEqual(readSidebarTaskPreferences(memoryStorage()), {
    organizeBy: "project",
    sortBy: "updated",
  });
});
