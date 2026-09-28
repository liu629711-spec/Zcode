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

test("relabel 按号（D26）：登记里名字陈旧也换成现役档案的新牌，无号登记靠名字并集", () => {
  resetPersonaChatBadgeStoreForTest();
  const store = usePersonaChatBadgeStore.getState();
  // s1 = 带号登记但名字还写着改名前的旧名；s2 = 号之前的老登记（只有旧名）。
  store.registerPersonaChatBadge({
    workspacePath: KEY_A,
    taskId: "s1",
    badge: { name: "ui-test", agentId: "id-1" },
  });
  store.registerPersonaChatBadge({ workspacePath: KEY_A, taskId: "s2", badge: { name: "ui-test" } });
  store.registerPersonaChatBadge({
    workspacePath: KEY_A,
    taskId: "s3",
    badge: { name: "ui-test", agentId: "id-2" },
  });

  store.relabelPersonaChatBadges({
    workspacePath: KEY_A,
    fromName: "ui-test",
    fromAgentId: "id-1",
    toBadge: { name: "ui-pro", agentId: "id-1" },
  });
  assert.deepEqual(badgeMapFor(KEY_A).get("s1"), { name: "ui-pro", agentId: "id-1" });
  assert.deepEqual(badgeMapFor(KEY_A).get("s2"), { name: "ui-pro", agentId: "id-1" });
  assert.deepEqual(
    badgeMapFor(KEY_A).get("s3"),
    { name: "ui-test", agentId: "id-2" },
    "别人的号不许被牵连",
  );
});

test("remove 按号（D26）：改名过又没补链的登记，光按名字会留下死入口", () => {
  resetPersonaChatBadgeStoreForTest();
  const store = usePersonaChatBadgeStore.getState();
  store.registerPersonaChatBadge({
    workspacePath: KEY_A,
    taskId: "s1",
    badge: { name: "ui-test", agentId: "id-1" },
  });
  store.registerPersonaChatBadge({ workspacePath: KEY_A, taskId: "s2", badge: { name: "planner" } });

  store.removePersonaChatBadgesByNames({
    workspacePath: KEY_A,
    names: ["ui-pro"],
    agentIds: ["id-1"],
  });
  assert.equal(badgeMapFor(KEY_A).has("s1"), false, "档案现名 ui-pro、登记里还写着旧名");
  assert.equal(badgeMapFor(KEY_A).has("s2"), true);
});
