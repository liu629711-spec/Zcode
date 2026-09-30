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
