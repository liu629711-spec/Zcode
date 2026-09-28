import assert from "node:assert/strict";
import test from "node:test";
import {
  resetPersonaChatBadgeStoreForTest,
  selectPersonaChatBadgesForWorkspace,
  usePersonaChatBadgeStore,
} from "../src/store/personaChatBadgeStore.js";

// ============================================================
// 驻场智能体徽章登记 store：注册 / 改名换牌 / 删除摘牌
// ============================================================
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/personaChatBadgeStore.test.ts
// node 环境无 window：落盘通道自动跳过，只验状态机本身。

const KEY_A = "D:/repo";
const KEY_B = "D:/other";

function badgeMapFor(workspaceKey: string): ReadonlyMap<string, { name: string; color?: string }> {
  return selectPersonaChatBadgesForWorkspace(usePersonaChatBadgeStore.getState(), workspaceKey);
}

test("register：登记按 workspaceKey 分桶，同 taskId 同 badge 引用不触发重复写", () => {
  resetPersonaChatBadgeStoreForTest();
  const badge = { name: "ui-pro", color: "purple" as const };
  const store = usePersonaChatBadgeStore.getState();
  store.registerPersonaChatBadge({ workspacePath: KEY_A, taskId: "s1", badge });
  assert.deepEqual(badgeMapFor(KEY_A).get("s1"), badge);
  assert.equal(badgeMapFor(KEY_B).size, 0);

  // 同引用重复登记：状态原样（map 引用不变），不同引用同内容：内容等价。
  const before = badgeMapFor(KEY_A);
  store.registerPersonaChatBadge({ workspacePath: KEY_A, taskId: "s1", badge });
  assert.equal(badgeMapFor(KEY_A), before);
  store.registerPersonaChatBadge({
    workspacePath: KEY_A,
    taskId: "s1",
    badge: { ...badge },
  });
  assert.deepEqual(badgeMapFor(KEY_A).get("s1"), badge);
});

test("relabel：该工作区 fromName 的登记整体换 toBadge，别的工作区与别名不受牵连", () => {
  resetPersonaChatBadgeStoreForTest();
  const store = usePersonaChatBadgeStore.getState();
  store.registerPersonaChatBadge({
    workspacePath: KEY_A,
    taskId: "s1",
    badge: { name: "ui-pro" },
  });
  store.registerPersonaChatBadge({
    workspacePath: KEY_A,
    taskId: "s2",
    badge: { name: "planner" },
  });
  store.registerPersonaChatBadge({
    workspacePath: KEY_B,
    taskId: "s3",
    badge: { name: "ui-pro" },
  });

  store.relabelPersonaChatBadges({
    workspacePath: KEY_A,
    fromName: "ui-pro",
    toBadge: { name: "ui-test", color: "purple" as const },
  });
  assert.deepEqual(badgeMapFor(KEY_A).get("s1"), { name: "ui-test", color: "purple" });
  assert.deepEqual(badgeMapFor(KEY_A).get("s2"), { name: "planner" }, "别名单不受牵连");
  assert.deepEqual(
    badgeMapFor(KEY_B).get("s3"),
    { name: "ui-pro" },
    "他工作区同名档案是独立实体，不跟改",
  );
});

test("remove：按名单摘牌，只动命中名字的登记", () => {
  resetPersonaChatBadgeStoreForTest();
  const store = usePersonaChatBadgeStore.getState();
  store.registerPersonaChatBadge({
    workspacePath: KEY_A,
    taskId: "s1",
    badge: { name: "ui-pro" },
  });
  store.registerPersonaChatBadge({
    workspacePath: KEY_A,
    taskId: "s2",
    badge: { name: "planner" },
  });

  store.removePersonaChatBadgesByNames({ workspacePath: KEY_A, names: ["ui-pro"] });
  assert.equal(badgeMapFor(KEY_A).has("s1"), false);
  assert.equal(badgeMapFor(KEY_A).has("s2"), true);
});
