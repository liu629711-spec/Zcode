import assert from "node:assert/strict";
import test from "node:test";
import type { BrowserStorageLike } from "../src/lib/browserEnvironment.js";
import {
  persistSidebarTaskPreferences,
  readSidebarTaskPreferences,
} from "../src/lib/sidebarTaskPreferences.js";

// ============================================================
// 侧栏任务偏好 boardMode 开关的可运行检查（内存 storage 直跑）
// ============================================================
// 钉的是 round-trip 与非法值回落：新增独立布尔不得破坏既有 organizeBy/sortBy 容错。
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

test("round-trip：persist 后 read 原样取回（含 boardMode）", () => {
  const storage = memoryStorage();
  persistSidebarTaskPreferences(
    { organizeBy: "grouped", sortBy: "created", boardMode: true },
    storage,
  );
  assert.deepEqual(readSidebarTaskPreferences(storage), {
    organizeBy: "grouped",
    sortBy: "created",
    boardMode: true,
  });

  persistSidebarTaskPreferences(
    { organizeBy: "project", sortBy: "updated", boardMode: false },
    storage,
  );
  assert.deepEqual(readSidebarTaskPreferences(storage), {
    organizeBy: "project",
    sortBy: "updated",
    boardMode: false,
  });
});

test("非法值回落：boardMode 非布尔回落默认 false，既有字段容错不受影响", () => {
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
    boardMode: false,
  });
});

test("向后兼容：旧版本 payload 缺 boardMode 键时读出默认 false，不丢既有键", () => {
  const storage = memoryStorage({
    "zcode-sidebar-task-preferences": JSON.stringify({
      organizeBy: "chronological",
      sortBy: "created",
    }),
  });
  assert.deepEqual(readSidebarTaskPreferences(storage), {
    organizeBy: "chronological",
    sortBy: "created",
    boardMode: false,
  });
});

test("损坏 JSON / 空 storage：整体回落默认值", () => {
  assert.deepEqual(
    readSidebarTaskPreferences(memoryStorage({ "zcode-sidebar-task-preferences": "{not json" })),
    { organizeBy: "project", sortBy: "updated", boardMode: false },
  );
  assert.deepEqual(readSidebarTaskPreferences(memoryStorage()), {
    organizeBy: "project",
    sortBy: "updated",
    boardMode: false,
  });
});
