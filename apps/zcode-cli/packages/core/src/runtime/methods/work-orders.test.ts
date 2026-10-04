// ============================================================
// 工单首轮上下文清场（2026-10-02 真机破案）的可运行检查：
// fresh 会话 contextInitialized=false 时，先 addUser 后 init 会被
// messageHistory.init 整体清场——工单从模型请求里消失，员工对着空气
// 报到"在的，老板"（三个本地模型一致复现"要派两次"）。修复 =
// runWorkOrderCommand 先 ensureContextInitialized（幂等，同
// background-notifications / goal-state-reminder 先例）。
// 本测试用最小假件仿真 ensure 的清场语义 + 真实 runWorkOrderCommand，
// 断言轮启动时刻的历史快照必须包含工单正文（修复前必红）。
// 运行：npx tsx --test apps/zcode-cli/packages/core/src/runtime/methods/work-orders.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import type { WorkOrderRuntimeCommand } from "../command-queue.js";
import { runWorkOrderCommand } from "./work-orders.js";

const CONTEXT_ENTRY =
  "<system-reminder>skills 列表与 AGENTS.md 附件（初始化注入的两条 meta-user）</system-reminder>";

/**
 * fresh runtime 的最小仿真：ensure 的清场语义照抄真实链
 * （context.ts:92-93 initializeMessageHistoryFromContext +
 * message-history.ts:144 init 整体替换 entries）。
 */
function makeFreshRuntime() {
  const history: string[] = [];
  const ops: string[] = [];
  const runtime = {
    sessionId: "sess-worker-fresh",
    branchGeneration: 1,
    contextInitialized: false,
    activeForegroundExecution: undefined as unknown,
    turnSnapshot: "",
    // 复盘轮（学习沉淀 v1）读 persona/记忆配置决定是否开轮：空 config = 非驻场，
    // 复盘静默跳过，时序不变量只关心工单本体的 init→addUser。
    config: {},
    async ensureContextInitialized() {
      if (runtime.contextInitialized) return;
      ops.push("init");
      history.length = 0;
      history.push(CONTEXT_ENTRY);
      runtime.contextInitialized = true;
    },
    messageHistory: {
      addUser(text: string) {
        ops.push("addUser");
        history.push(text);
      },
    },
    async persistSyntheticUserNoticeForSession() {},
    async executeTurnCommand() {
      // 真实 turn 准入（turn.ts:229）的第一步：先 ensure，再取请求消息。
      await runtime.ensureContextInitialized();
      runtime.turnSnapshot = history.join("\n");
    },
  };
  return { runtime, ops };
}

function workOrderCommand(task: string): WorkOrderRuntimeCommand {
  return {
    id: "cmd-1",
    mode: "work-order",
    branchGeneration: 1,
    source: "agent_work_order",
    workOrderId: "wo-1",
    inputId: "workorder-wo-1",
    envelope: {
      workOrderId: "wo-1",
      fromAgentName: "老板",
      fromSessionId: "sess-boss",
      task,
    },
    text: `<work-order id="wo-1" from-agent="老板" from-session="sess-boss">\n${task}\n</work-order>`,
  } as unknown as WorkOrderRuntimeCommand;
}

test("工单首轮：初始化不得清掉刚入史的信封——轮启动快照必须含任务正文", async () => {
  const { runtime } = makeFreshRuntime();
  await runWorkOrderCommand.call(
    runtime as never,
    workOrderCommand("在工作区创建 hello3.txt，内容一行：验收第4步"),
  );
  assert.match(runtime.turnSnapshot, /验收第4步/);
});

test("工单首轮时序不变量：ensure（init）必须发生在 addUser 之前", async () => {
  const { runtime, ops } = makeFreshRuntime();
  await runWorkOrderCommand.call(runtime as never, workOrderCommand("任务"));
  assert.deepEqual(ops, ["init", "addUser"]);
});
