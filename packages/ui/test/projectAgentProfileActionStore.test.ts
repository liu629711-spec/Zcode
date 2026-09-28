import assert from "node:assert/strict";
import test from "node:test";
import {
  requestProjectAgentProfileAction,
  requestProjectAgentRenameRelink,
  useProjectAgentProfileActionStore,
  type ProjectAgentProfileActionHandlers,
} from "../src/store/projectAgentProfileActionStore.js";

// ============================================================
// 档案操作请求通道（D4/D27）：侧栏是唯一执行者，别人只发请求。
// 处理器缺席（侧栏没挂载）必须返回 false，让调用方降级而不是静默失败。
//
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/projectAgentProfileActionStore.test.ts
// ============================================================

function captureHandlers(): { calls: unknown[]; handlers: ProjectAgentProfileActionHandlers } {
  const calls: unknown[] = [];
  return {
    calls,
    handlers: {
      edit: (task) => calls.push(["edit", task.taskId]),
      remove: (task) => calls.push(["remove", task.taskId]),
      renameRelink: (request) => calls.push(["renameRelink", request]),
    },
  };
}

test("无处理器：两类请求都返回 false（调用方据此不假装成功）", () => {
  useProjectAgentProfileActionStore.getState().setHandlers(null);
  assert.equal(
    requestProjectAgentProfileAction("edit", { taskId: "t1" } as never),
    false,
    "侧栏没挂载时 Header 菜单项不渲染",
  );
  assert.equal(
    requestProjectAgentRenameRelink({ workspacePath: "D:/repo", oldName: "a", newName: "b" }),
    false,
    "侧栏没挂载时设置页改名跳过补链",
  );
});

test("改名补链请求：原样交给自己登记的侧栏处理器", () => {
  const captured = captureHandlers();
  useProjectAgentProfileActionStore.getState().setHandlers(captured.handlers);
  const request = {
    workspacePath: "D:/repo",
    workspaceIdentity: "wsid-1",
    agentId: "0f6a2c1e-77db-4a1b-9c3d-5e2f8b1a4c9d",
    oldName: "ui-test",
    newName: "ui-pro",
    color: "cyan" as const,
  };
  assert.equal(requestProjectAgentRenameRelink(request), true);
  assert.deepEqual(captured.calls, [["renameRelink", request]]);

  // 卸载即摘处理器：之后的请求一律落回 false，不留指向已销毁侧栏的死引用。
  useProjectAgentProfileActionStore.getState().setHandlers(null);
  assert.equal(requestProjectAgentRenameRelink(request), false);
});
