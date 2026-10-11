import assert from "node:assert/strict";
import test from "node:test";
import type { SessionSummary } from "@zcode/shared/zcode-protocol-v4";
import {
  collectSessionInteractionNotificationPayloads,
  collectTerminalTaskNotificationPayloads,
} from "../src/lib/taskNotificationOrchestrator.js";

// 通知中枢（2026-10-12）编排器的可运行检查：
// 摘要版跨会话提问/待批准 payload（正文用会话标题，ExitPlanMode 换计划批准词表）
// + 终态边沿（此前只盖活跃 workspace，本次升级为多工作区的同一编排器）。

// 假 intl：词表原样回显 id，values 以 JSON 附在尾部（taskTitleBody 靠它带标题，
// 断言查标题原文就等于查"values 确实传进来了"）。
const formatMessage = ((descriptor: { id: string }, values?: Record<string, string | number>) =>
  `<${descriptor.id}:${JSON.stringify(values ?? null)}>`
) as unknown as Parameters<typeof collectSessionInteractionNotificationPayloads>[0]["formatMessage"];

function summary(overrides: Partial<SessionSummary> = {}): SessionSummary {
  return {
    sessionId: "s1",
    workspaceId: "w1",
    title: "修复登录页",
    phase: "running",
    sessionEnded: false,
    hasBackgroundWork: false,
    ...overrides,
  } as SessionSummary;
}

test("摘要版提问通知：permission/userInput/ExitPlanMode 三种身份，seen 不重发", () => {
  const summaries = [
    summary({
      sessionId: "s-perm",
      pendingInteraction: { interactionId: "i-1", kind: "permission", toolName: "Bash" },
    }),
    summary({
      sessionId: "s-ask",
      pendingInteraction: { interactionId: "i-2", kind: "userInput", toolName: "AskUserQuestion" },
    }),
    summary({
      sessionId: "s-plan",
      pendingInteraction: { interactionId: "i-3", kind: "userInput", toolName: "ExitPlanMode" },
    }),
    summary({ sessionId: "s-quiet" }),
  ];

  const first = collectSessionInteractionNotificationPayloads({
    summaries,
    seenInteractionIds: new Set(),
    formatMessage,
  });
  assert.deepEqual(
    first.map((payload) => [payload.taskId, payload.status, payload.requestId]),
    [
      ["s-perm", "permission_request", "i-1"],
      ["s-ask", "elicitation_request", "i-2"],
      ["s-plan", "elicitation_request", "i-3"],
    ],
  );
  // 正文带会话标题（taskTitleBody）；计划批准用专属词表且不带标题。
  assert.match(first[0]!.body, /修复登录页/);
  assert.match(first[2]!.title, /planApprovalRequired/);

  const second = collectSessionInteractionNotificationPayloads({
    summaries,
    seenInteractionIds: new Set(["i-1", "i-2", "i-3"]),
    formatMessage,
  });
  assert.deepEqual(second, [], "已见过的 interactionId 不重发");
});

test("终态边沿：running→error 发 failed；成功→完成；同态/新会话不发声", () => {
  const previous = new Map<string, SessionSummary>([
    ["s-err", summary({ sessionId: "s-err", phase: "running" })],
    ["s-done", summary({ sessionId: "s-done", phase: "running" })],
    ["s-same", summary({ sessionId: "s-same", phase: "error" })],
  ]);
  const payloads = collectTerminalTaskNotificationPayloads({
    previousBySessionId: previous,
    sessions: [
      summary({ sessionId: "s-err", phase: "error" }),
      summary({ sessionId: "s-done", phase: "completedSuccess" }),
      summary({ sessionId: "s-same", phase: "error" }),
      // 新会话（上一帧没有）不发声：首帧是基线。
      summary({ sessionId: "s-new", phase: "error" }),
    ],
    formatMessage,
  });
  assert.deepEqual(
    payloads.map((payload) => [payload.taskId, payload.status]),
    [
      ["s-err", "failed"],
      ["s-done", "completed"],
    ],
  );
});
