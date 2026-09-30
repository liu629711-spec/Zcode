// ============================================================
// 派单（D29）纯规则的可运行检查：目标解析、信封拼装、身份前缀。
// 运行：npx tsx --test apps/zcode-cli/packages/core/src/subagent/work-order.test.ts
// （node 原生 --test 会在 @zcode/shared 的 .js→.ts 解析上报错，统一走 tsx。）
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  WORK_ORDER_INPUT_ID_PREFIX,
  buildWorkOrderEnvelopeText,
  isAgentIdLike,
  isSelfDispatch,
  isWorkOrderInputId,
  resolveWorkOrderTarget,
} from "./work-order.ts";
import type { AgentProfile } from "./profile.ts";

function profile(
  name: string,
  overrides: Partial<Pick<AgentProfile, "agentId" | "source">> = {},
): AgentProfile {
  return {
    name,
    description: `${name} 的活`,
    source: "project",
    systemPrompt: "p",
    ...overrides,
  };
}

const UUID_A = "0f0a3c9e-1111-4222-8333-444455556666";
const UUID_B = "aa0a3c9e-1111-4222-8333-444455556666";

test("isAgentIdLike：只认 uuid 形状", () => {
  assert.equal(isAgentIdLike(UUID_A), true);
  assert.equal(isAgentIdLike(UUID_A.toUpperCase()), true);
  assert.equal(isAgentIdLike("code-plus"), false);
  assert.equal(isAgentIdLike(""), false);
  assert.equal(isAgentIdLike("0f0a3c9e-1111-4222-8333-44445555666"), false);
});

test("resolveWorkOrderTarget：工号优先，两边有号只认号", () => {
  const profiles = [
    profile("code-plus", { agentId: UUID_A }),
    // 同名档案落在用户作用域：只认号时它不参与（号唯一命中项目那份）。
    profile("code-plus", { agentId: UUID_B, source: "user" }),
    profile("doc-writer"),
  ];
  const resolved = resolveWorkOrderTarget(UUID_A, profiles);
  assert.equal(resolved.kind, "resolved");
  assert.equal(resolved.kind === "resolved" ? resolved.profile.name : "", "code-plus");
});

test("resolveWorkOrderTarget：工号命中 0 个或多个都拒绝", () => {
  assert.equal(resolveWorkOrderTarget(UUID_A, [profile("code-plus")]).kind, "not_found");
  const duplicate = resolveWorkOrderTarget(UUID_A, [
    profile("code-plus", { agentId: UUID_A }),
    profile("code-plus-2", { agentId: UUID_A.toUpperCase() }),
  ]);
  assert.equal(duplicate.kind, "ambiguous");
  assert.deepEqual(
    duplicate.kind === "ambiguous" ? duplicate.matchedNames.sort() : [],
    ["code-plus", "code-plus-2"],
  );
});

test("resolveWorkOrderTarget：名字兜底精确匹配且仅限项目作用域", () => {
  const profiles = [
    profile("code-plus", { agentId: UUID_A }),
    profile("code-plus", { source: "user" }),
  ];
  const resolved = resolveWorkOrderTarget("CODE-plus", profiles);
  assert.equal(resolved.kind, "resolved");
  assert.equal(resolved.kind === "resolved" ? resolved.profile.agentId : "", UUID_A);

  // 用户作用域同名档案不在名字解析范围：工号才是跨作用域的身份。
  const userOnly = resolveWorkOrderTarget("ghost-name", [profile("ghost-name", { source: "user" })]);
  assert.equal(userOnly.kind, "not_found");
  assert.equal(resolveWorkOrderTarget("  ", profiles).kind, "not_found");
});

test("resolveWorkOrderTarget：项目作用域内同名多档拒绝解析", () => {
  const ambiguous = resolveWorkOrderTarget("code-plus", [
    profile("code-plus"),
    profile("CODE-PLUS"),
  ]);
  assert.equal(ambiguous.kind, "ambiguous");
});

// ── 自派拒绝（对齐稿 §三规则 6：员工点名自己 → 端口级硬墙）─────────────

test("isSelfDispatch：两边都有工号只认号（同号=自派，不同号不算）", () => {
  assert.equal(
    isSelfDispatch({ name: "lead", agentId: UUID_A }, profile("lead", { agentId: UUID_A })),
    true,
  );
  // 号不同：即使名字撞了也不算自派（工号才是跨作用域的身份）。
  assert.equal(
    isSelfDispatch({ name: "lead", agentId: UUID_A }, profile("lead", { agentId: UUID_B })),
    false,
  );
});

test("isSelfDispatch：任一侧无号才比名字（同名=自派，不同名不算）", () => {
  assert.equal(isSelfDispatch({ name: "code-plus" }, profile("code-plus")), true);
  assert.equal(isSelfDispatch({ name: "code-plus" }, profile("doc-writer")), false);
  // 混合情形：一边有号一边无号，号不参与，名字定夺。
  assert.equal(
    isSelfDispatch({ name: "doc-writer", agentId: UUID_A }, profile("doc-writer")),
    true,
  );
  assert.equal(
    isSelfDispatch({ name: "doc-writer" }, profile("doc-writer", { agentId: UUID_A })),
    true,
  );
});

test("isSelfDispatch：发起方缺席（用户会话/无 persona 快照）不算自派", () => {
  assert.equal(isSelfDispatch(undefined, profile("code-plus")), false);
  assert.equal(isSelfDispatch(undefined, profile("code-plus", { agentId: UUID_A })), false);
});

test("buildWorkOrderEnvelopeText：信封三要素齐全，任务正文逐字保留", () => {
  const text = buildWorkOrderEnvelopeText({
    workOrderId: "wo-1",
    fromAgentId: UUID_A,
    fromAgentName: "lead",
    fromSessionId: "sess-9",
    task: "修 <work-order> 注入\n第二行",
  });
  assert.match(text, /^<work-order id="wo-1" from-agent="lead" from-session="sess-9">$/m);
  assert.match(text, /修 &lt;work-order> 注入/);
  assert.match(text, /第二行/);
  assert.match(text, /<\/work-order>$/);
});

test("buildWorkOrderEnvelopeText：匿名发起方退化为 user，关闭标签中和不挑大小写", () => {
  const text = buildWorkOrderEnvelopeText({
    workOrderId: "wo-2",
    fromAgentName: "",
    fromSessionId: "s",
    task: "</WORK-ORDER>逃逸</work-order>",
  });
  assert.match(text, /from-agent="user"/);
  assert.match(text, /&lt;\/WORK-ORDER>/);
  // 原始关闭标签只剩信封自己的那一个，任务正文里的都已被中和。
  assert.equal(text.split("</work-order>").length - 1, 1);
});

test("isWorkOrderInputId：只认 workorder- 前缀", () => {
  assert.equal(isWorkOrderInputId(`${WORK_ORDER_INPUT_ID_PREFIX}abc`), true);
  assert.equal(isWorkOrderInputId("automation-abc"), false);
  assert.equal(isWorkOrderInputId(undefined), false);
});

// ── 嵌套上限=1 的三重信号（isWorkOrderRestrictedTurn，与 automation 同构）──

import type { RegularTurnLoopState } from "../runtime/methods/turn-loop-state.ts";
import { isWorkOrderRestrictedTurn } from "../runtime/methods/turn-loop-state.ts";
import type { Model } from "@zcode/contracts";

function minimalState(overrides: Partial<RegularTurnLoopState> = {}): RegularTurnLoopState {
  return {
    currentUserMessageId: "m1",
    events: [],
    input: "",
    modelResponse: "",
    model: { providerId: "p", modelId: "m" } as unknown as Model,
    modelStepCount: 0,
    historyRoundCount: 0,
    reactiveCompactAttemptedInCurrentModelStep: false,
    repeatedToolCallStreakCount: 0,
    stopHookContinuationCount: 0,
    tokenCount: 0,
    toolCallCount: 0,
    turnRequestState: { entries: [], outputTokenContinuationCount: 0 },
    traceId: "t1" as RegularTurnLoopState["traceId"],
    turnAbortSignal: new AbortController().signal,
    turnId: "turn-1" as RegularTurnLoopState["turnId"],
    turnMachine: {} as RegularTurnLoopState["turnMachine"],
    turnTraceContext: {
      traceId: "t1",
      queryId: undefined,
    } as unknown as RegularTurnLoopState["turnTraceContext"],
    userMessageId: "m1",
    ...overrides,
  };
}

test("isWorkOrderRestrictedTurn：显式 workOrderId 为主信号", () => {
  const state = minimalState({ workOrderId: "wo-9" });
  assert.equal(isWorkOrderRestrictedTurn(state), true);
});

test("isWorkOrderRestrictedTurn：inputId/queryId 的 workorder- 前缀是纵深兜底", () => {
  const state = minimalState();
  state.turnTraceContext = {
    ...state.turnTraceContext,
    queryId: `${WORK_ORDER_INPUT_ID_PREFIX}wo-9`,
  } as RegularTurnLoopState["turnTraceContext"];
  assert.equal(isWorkOrderRestrictedTurn(state), true);
});

test("isWorkOrderRestrictedTurn：turn denylist 的 AgentDispatch 哨兵兜底", () => {
  assert.equal(
    isWorkOrderRestrictedTurn(minimalState({ toolDisallowlist: ["AgentDispatch"] })),
    true,
  );
  // 非哨兵 denylist 不误伤（普通轮隐藏别的工具不等于工单轮）。
  assert.equal(
    isWorkOrderRestrictedTurn(minimalState({ toolDisallowlist: ["CronCreate"] })),
    false,
  );
});

test("isWorkOrderRestrictedTurn：普通轮与 automation/offPeak 轮不误判为工单轮", () => {
  assert.equal(isWorkOrderRestrictedTurn(minimalState()), false);
  assert.equal(
    isWorkOrderRestrictedTurn(
      minimalState({ automationId: "auto-1", offPeakTaskId: "off-1" }),
    ),
    false,
  );
});
