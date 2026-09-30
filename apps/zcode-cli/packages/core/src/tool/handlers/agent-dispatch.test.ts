// ============================================================
// AgentDispatch 未点名派生普通会话（对齐稿 §三 #2/#5）的可运行检查：
// 契约 refine（缺 agent 必须 newSession=true）+ handler 无 agent 透传端口。
// 运行：npx tsx --test apps/zcode-cli/packages/core/src/tool/handlers/agent-dispatch.test.ts
// （node 原生 --test 会在 @zcode/shared 的 .js→.ts 解析上报错，统一走 tsx。）
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  AgentDispatchInputSchema,
  AgentDispatchOutputSchema,
  type AgentDispatchPort,
  type AgentDispatchRequest,
  type AgentDispatchResult,
} from "@zcode/contracts";
import type { ToolExecutionContext } from "../types.ts";
import { agentDispatchToolEntry } from "./agent-dispatch.ts";

function recordingPort(result: AgentDispatchResult): {
  port: AgentDispatchPort;
  calls: AgentDispatchRequest[];
} {
  const calls: AgentDispatchRequest[] = [];
  return {
    calls,
    port: {
      dispatch: async (input) => {
        calls.push(input);
        return result;
      },
    },
  };
}

function contextWith(port: AgentDispatchPort): ToolExecutionContext {
  return {
    toolCallId: "call_1",
    traceId: "trace_1",
    abortSignal: new AbortController().signal,
    workingDirectory: "D:/tmp",
    workspaceRoot: "D:/tmp",
    sessionId: "sess_source",
    agentDispatchPort: port,
  } as unknown as ToolExecutionContext;
}

test("契约 refine：缺 agent 且不带 newSession=true 被拒", () => {
  const refused = AgentDispatchInputSchema.safeParse({ task: "发个你好" });
  assert.equal(refused.success, false);
  if (!refused.success) {
    assert.ok(
      refused.error.issues.some((issue) =>
        issue.message.includes("agent is required unless newSession is true"),
      ),
    );
  }
});

test("契约 refine：缺 agent + newSession=true 放行；点名 agent 的既有形状不受影响", () => {
  assert.equal(
    AgentDispatchInputSchema.safeParse({ task: "发个你好", newSession: true }).success,
    true,
  );
  assert.equal(AgentDispatchInputSchema.safeParse({ agent: "code-plus", task: "干活" }).success, true);
  assert.equal(
    AgentDispatchInputSchema.safeParse({ agent: "code-plus", task: "干活", newSession: true })
      .success,
    true,
  );
});

test("未点名 + newSession=true：handler 不解析目标直接透传端口，message 按普通会话语义", async () => {
  const { port, calls } = recordingPort({
    targetSessionId: "sess_target",
    agentName: "",
    delivery: "started",
    createdSession: true,
  });
  const output = (await agentDispatchToolEntry.handler(
    { task: "发个你好", newSession: true, title: "发个你好" },
    contextWith(port),
  )) as Record<string, unknown>;
  assert.equal(calls.length, 1);
  assert.equal(calls[0]!.agent, undefined);
  assert.equal(calls[0]!.newSession, true);
  assert.equal(calls[0]!.task, "发个你好");
  assert.equal(calls[0]!.sourceSessionId, "sess_source");
  assert.equal("agentName" in output, false);
  assert.ok(String(output.message).includes("Opened a new ordinary session"));
  assert.ok(String(output.message).includes("发个你好"));
  // 放宽后的 Output 契约收下「无 agentName」的输出（additive 放宽不打回）。
  assert.equal(AgentDispatchOutputSchema.safeParse(output).success, true);
});

test("点名员工路径不回归：agent 原样透传，message 报受理对象", async () => {
  const { port, calls } = recordingPort({
    targetSessionId: "sess_target",
    agentName: "code-plus",
    delivery: "queued",
    createdSession: false,
  });
  const output = (await agentDispatchToolEntry.handler(
    { agent: "code-plus", task: "干活" },
    contextWith(port),
  )) as Record<string, unknown>;
  assert.equal(calls.length, 1);
  assert.equal(calls[0]!.agent, "code-plus");
  assert.equal(calls[0]!.newSession, undefined);
  assert.equal(output.agentName, "code-plus");
  assert.ok(String(output.message).includes("Work order accepted by code-plus."));
});

test("说明书规则表覆盖 §三总表（2026-09-30 重写）：自派禁令、未点名普通会话、具名默认最近一段等", () => {
  const rules = agentDispatchToolEntry.metadata.modelInstructions.join("\n");
  // §三 #6：自派禁令（端口 guard.agentWorkOrderSelfTarget 的提示词先教）。
  assert.match(rules, /NEVER dispatch a work order to yourself/);
  // §三 #2/#5：未点名 + 要新会话 = 无 agent 的无工牌普通会话，回话说明开好了什么。
  assert.match(rules, /WITHOUT naming which agent/);
  assert.match(rules, /omit agent entirely and pass newSession=true/);
  assert.match(rules, /unbadged ordinary session/);
  assert.match(rules, /say in one sentence what was opened/);
  // §三 #3/D1：具名员工默认落最近一段，仅用户明说新的一段才 newSession=true。
  assert.match(rules, /For a NAMED agent, set newSession=true only when/);
  assert.match(rules, /latest persona session/);
  // ask-first：没点名也没要新会话的活，问清楚而不是猜。
  assert.match(rules, /the request has no doer/);
  // 既有正确条目不丢：点名必派单、task 独立成文、model 逐字传、回执语义、嵌套禁令。
  assert.match(rules, /names another agent of this workspace as the doer/);
  assert.match(rules, /complete standalone instructions/);
  assert.match(rules, /pass the user's words verbatim as model/);
  assert.match(rules, /arrives as a separate receipt/);
  assert.match(rules, /nesting is not allowed/);
});
