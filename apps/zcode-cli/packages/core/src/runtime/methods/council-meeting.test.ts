// ============================================================
// 圆桌会编排状态机的可运行检查：驱动**真实** maybeAdvanceCouncilRound /
// conveneCouncilMeeting / setCouncilMeetingPaused / addCouncilInterjection /
// runCouncilModerationCommand（互调用真实实现，只有 sessionStore/派单端口/
// enqueueRuntimeCommand 是假件；council.ts 纯函数另测不重测）。台账行用真实
// payload 形状（派单行信封带 council* 五参、回执行 outcome 在 payload 顶层、
// saveSessionInput 同 id upsert）。验证：
//  1. 召集：会议行先落账再逐席派单，席位单带 council* 五参；个别席位失败标注
//     缺席照常开会；全军覆没取消并抛逐席原因；
//  2. 首轮全票早退：直接终局 + 主席轮一次 + 闸门幂等（二触发不二开）；
//  3. 首轮分裂进第 2 轮：匿名质询材料（姓名不外泄）+ 抗从众条款 + 插话点名；
//     终轮收票出决议；
//  4. 缺裁定行重询一次：补交行在册后不再重询；仍缺按弃权如实标注；
//  5. 交活失败按缺席弃权；全席派单失败不空转第 2 轮，直接未决收口；
//  6. 暂停冻结推进（在答回执也不收票），恢复即补推进；
//  7. 插话进下一轮信封（点名只进被点席位）；
//  8. 决议落库：轮闸门行带票数明细与票源，会议行落终态 + moderation 阶段；
//  9. 主席轮禁派单（toolDisallowlist 是机制不是提示词）。
// 运行：node --test（编译跑法见仓库铁律）
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import type {
  AgentDispatchRequest,
  AgentWorkOrderEnvelope,
} from "@zcode/contracts";
import type { CouncilModerationRuntimeCommand } from "../command-queue.js";
import {
  addCouncilInterjection,
  conveneCouncilMeeting,
  maybeAdvanceCouncilRound,
  runCouncilModerationCommand,
  setCouncilMeetingPaused,
} from "./council-meeting.js";
import {
  councilMeetingLedgerId,
  councilRoundLedgerId,
  councilSeatRequeryLedgerId,
} from "../../subagent/council-meeting.js";

const SESSION = "sess-boss";
const COUNCIL = "council-1";
const VERDICT_APPROVE = "我认为可行。\n裁定：通过｜信心：高｜否决：否";
const VERDICT_REJECT = "有边界问题。\n裁定：打回｜信心：中｜否决：否";

interface LedgerRow {
  id: string;
  sessionID: string;
  kind: string;
  delivery: string;
  payload: { text: string; [key: string]: unknown };
  admittedSequence: number;
  status: "admitted" | "discarded";
  time: { created: number; updated: number };
}

function seatEnvelope(
  workOrderId: string,
  seatIndex: number,
  round: 1 | 2,
): AgentWorkOrderEnvelope {
  return {
    workOrderId,
    fromAgentName: "老板",
    fromSessionId: SESSION,
    task: `议题正文（席位${seatIndex}）`,
    councilId: COUNCIL,
    councilKind: "plan",
    councilRound: round,
    councilSeatIndex: seatIndex,
    councilSeatLens: seatIndex === 0 ? "impact" : "requirement",
  };
}

function dispatchRow(
  workOrderId: string,
  seatIndex: number,
  round: 1 | 2,
  status: "admitted" | "discarded",
): LedgerRow {
  return {
    id: `agentWorkOrderDispatch:${workOrderId}`,
    sessionID: SESSION,
    kind: "agentWorkOrderDispatch",
    delivery: "queue",
    payload: {
      text: `议题正文（席位${seatIndex}）`,
      workOrderId,
      agentName: seatIndex === 0 ? "小验" : "小验2",
      targetSessionId: "sess-worker",
      envelope: seatEnvelope(workOrderId, seatIndex, round),
    },
    admittedSequence: 1,
    status,
    time: { created: 0, updated: 0 },
  };
}

function receiptRow(
  workOrderId: string,
  seatIndex: number,
  round: 1 | 2,
  outcome:
    | { status: "completed"; response: string }
    | { status: "failed"; reason: string }
    | { status: "cancelled" },
  admittedSequence = 1,
): LedgerRow {
  return {
    id: `runtime_command_receipt-${workOrderId}`,
    sessionID: SESSION,
    kind: "agentWorkOrderReceipt",
    delivery: "queue",
    payload: {
      text: "receipt",
      workOrderId,
      outcome,
      envelope: seatEnvelope(workOrderId, seatIndex, round),
    },
    admittedSequence,
    status: "discarded",
    time: { created: 0, updated: 0 },
  };
}

function meetingRow(extra?: Record<string, unknown>): LedgerRow {
  return {
    id: councilMeetingLedgerId(SESSION, COUNCIL),
    sessionID: SESSION,
    kind: "councilMeeting",
    delivery: "queue",
    payload: {
      text: "登录页改成双栏布局",
      council: {
        councilId: COUNCIL,
        kind: "plan",
        status: "running",
        round: 1,
        phase: "seating",
        seats: [
          { index: 0, agentName: "小验", lens: "impact" },
          { index: 1, agentName: "小验2", lens: "requirement" },
        ],
        motion: "登录页改成双栏布局",
        convenedBySessionId: SESSION,
      },
      title: "登录页方案",
      ...extra,
    },
    admittedSequence: 0,
    status: "discarded",
    time: { created: 0, updated: 0 },
  };
}

function makeStore(rows: LedgerRow[], events?: string[]) {
  let sequence = 100;
  const trace = events ?? [];
  return {
    rows,
    events: trace,
    async listSessionInputs() {
      return this.rows;
    },
    // 与真实 saveSessionInput 同语义：同 id 重入更新（upsert），不是追加。
    async saveSessionInput(input: {
      id: string;
      sessionID: string;
      kind: string;
      delivery: string;
      payload: { text: string; [key: string]: unknown };
    }) {
      trace.push(`save:${input.id}`);
      sequence += 1;
      const existing = this.rows.findIndex((row) => row.id === input.id);
      const row: LedgerRow = {
        ...(input as unknown as LedgerRow),
        admittedSequence: sequence,
        status: "admitted",
        time: { created: 0, updated: 0 },
      };
      if (existing >= 0) this.rows[existing] = row;
      else this.rows.push(row);
      return { id: input.id };
    },
  };
}

function makePort(failNames: Record<string, string> = {}, events?: string[]) {
  const calls: AgentDispatchRequest[] = [];
  let sequence = 0;
  return {
    calls,
    port: {
      async dispatch(input: AgentDispatchRequest) {
        calls.push(input);
        events?.push(`dispatch:${input.agent ?? ""}`);
        const failure = failNames[input.agent ?? ""];
        if (failure) throw new Error(failure);
        sequence += 1;
        return {
          targetSessionId: "sess-worker",
          agentName: input.agent ?? "",
          delivery: "started" as const,
          createdSession: false,
          workOrderId: `wo-minted-${sequence}`,
        };
      },
    },
  };
}

function makeRuntime(rows: LedgerRow[], port?: ReturnType<typeof makePort>["port"]) {
  const store = makeStore(rows);
  const moderationCommands: CouncilModerationRuntimeCommand[] = [];
  const runtime = {
    sessionId: SESSION,
    shuttingDown: false,
    branchGeneration: 0,
    rootTraceContext: { traceId: "trace" },
    logger: undefined,
    sessionStore: store,
    agentDispatchPort: port,
    trackResidencyBlockingWork: async () => ({}),
    enqueueRuntimeCommand(command: CouncilModerationRuntimeCommand) {
      moderationCommands.push(command);
    },
    moderationCommands,
    store,
  };
  return runtime as typeof runtime & Record<string, unknown>;
}

const advance = (
  runtime: never,
  input: { round?: 1 | 2; workOrderId?: string; outcome?: unknown } = {},
) =>
  maybeAdvanceCouncilRound.call(runtime, {
    councilId: COUNCIL,
    ...input,
  } as never);

function meetingCouncil(runtime: ReturnType<typeof makeRuntime>): Record<string, unknown> {
  const row = runtime.store.rows.find(
    (candidate: LedgerRow) => candidate.id === councilMeetingLedgerId(SESSION, COUNCIL),
  ) as LedgerRow | undefined;
  return row!.payload.council as Record<string, unknown>;
}

test("召集：会议行先落账再逐席派单，席位单带 council* 五参", async () => {
  const events: string[] = [];
  const { port, calls } = makePort({}, events);
  const store = makeStore([], events);
  const deps = {
    sessionId: SESSION,
    sessionStore: store,
    agentDispatchPort: port,
  };
  const result = await conveneCouncilMeeting(
    deps as never,
    {
      councilId: COUNCIL,
      kind: "plan",
      topic: "登录页改成双栏布局",
      seats: ["小验", "小验2"],
      materials: "见 src/login.tsx",
    } as never,
  );
  assert.equal(result.status, "running");
  assert.equal(result.dispatchedSeats, 2);
  assert.deepEqual(result.failedSeats, []);
  // 会议行先落账（先于任何派单）：快的席位回执到了也有对账锚点。
  assert.equal(events[0], `save:${councilMeetingLedgerId(SESSION, COUNCIL)}`);
  assert.equal(calls.length, 2);
  assert.deepEqual(
    calls.map((call) => call.councilSeatIndex),
    [0, 1],
  );
  assert.ok(calls.every((call) => call.councilId === COUNCIL && call.councilRound === 1));
  assert.ok(calls.every((call) => typeof call.councilSeatLens === "string"));
  // 席位任务带议题与材料；来源会话是召集方。
  assert.match(calls[0]!.task, /登录页改成双栏布局/);
  assert.match(calls[0]!.task, /见 src\/login\.tsx/);
  assert.ok(calls.every((call) => call.sourceSessionId === SESSION));
});

test("召集：个别席位派单失败标注缺席照常开会；全军覆没取消并抛逐席原因", async () => {
  const partialPort = makePort({ 小验2: "No agent profile matches 小验2" });
  const partialStore = makeStore([]);
  const partialResult = await conveneCouncilMeeting(
    {
      sessionId: SESSION,
      sessionStore: partialStore,
      agentDispatchPort: partialPort.port,
    } as never,
    { councilId: COUNCIL, kind: "plan", topic: "议题", seats: ["小验", "小验2"] } as never,
  );
  assert.equal(partialResult.status, "running");
  assert.equal(partialResult.dispatchedSeats, 1);
  assert.deepEqual(partialResult.failedSeats, [
    { agentName: "小验2", reason: "No agent profile matches 小验2" },
  ]);
  const partialCouncil = partialStore.rows.find(
    (row: LedgerRow) => row.id === councilMeetingLedgerId(SESSION, COUNCIL),
  )!.payload as { failedDispatches?: { seatIndex: number; round: number }[] };
  assert.deepEqual(partialCouncil.failedDispatches, [
    { round: 1, seatIndex: 1, error: "No agent profile matches 小验2" },
  ]);

  const allFailPort = makePort({ 小验: "boom", 小验2: "boom" });
  const allFailStore = makeStore([]);
  // 全军覆没：不抛散单错误，返回 cancelled（工具 handler 据此给模型逐席原因）。
  const cancelled = await conveneCouncilMeeting(
    { sessionId: SESSION, sessionStore: allFailStore, agentDispatchPort: allFailPort.port } as never,
    { councilId: COUNCIL, kind: "plan", topic: "议题", seats: ["小验", "小验2"] } as never,
  );
  assert.equal(cancelled.status, "cancelled");
  assert.equal(cancelled.dispatchedSeats, 0);
  const cancelledRow = allFailStore.rows.find(
    (row: LedgerRow) => row.id === councilMeetingLedgerId(SESSION, COUNCIL),
  )!.payload as unknown as { council: { status: string } };
  assert.equal(cancelledRow.council.status, "cancelled");
});

test("首轮全票通过：早退终局 + 主席轮一次 + 闸门幂等", async () => {
  const runtime = makeRuntime([
    meetingRow(),
    dispatchRow("wo-a", 0, 1, "discarded"),
    dispatchRow("wo-b", 1, 1, "discarded"),
    receiptRow("wo-a", 0, 1, { status: "completed", response: VERDICT_APPROVE }),
    receiptRow("wo-b", 1, 1, { status: "completed", response: VERDICT_APPROVE }),
  ]);
  await advance(runtime as never, {
    round: 1,
    workOrderId: "wo-b",
    outcome: { status: "completed", response: VERDICT_APPROVE },
  });
  assert.equal(runtime.moderationCommands.length, 1);
  const command = runtime.moderationCommands[0]!;
  assert.equal(command.mode, "council-moderation");
  assert.equal(command.originMeta.councilId, COUNCIL);
  assert.equal(command.originMeta.councilPhase, "moderation");
  assert.equal(command.originMeta.title, "圆桌会 · 登录页方案");
  // 决议落库：轮闸门行带票数明细；全票通过没有第 2 轮派单。
  const gate = runtime.store.rows.find(
    (row: LedgerRow) => row.id === councilRoundLedgerId(SESSION, COUNCIL, 1),
  ) as LedgerRow | undefined;
  assert.ok(gate);
  assert.equal(gate!.payload.outcome, "approved");
  assert.deepEqual(gate!.payload.tally, { total: 2, approve: 2, reject: 0, abstain: 0, vetoes: 0 });
  assert.equal(meetingCouncil(runtime).status, "approved");
  assert.equal((meetingCouncil(runtime) as { phase: string }).phase, "moderation");
  // 闸门幂等：再触发不二开主席轮。
  await advance(runtime as never, { round: 1 });
  assert.equal(runtime.moderationCommands.length, 1);
});

test("首轮分裂：进第 2 轮匿名质询（姓名不外泄）+ 抗从众条款 + 插话点名；终轮过半出决议", async () => {
  const { port, calls } = makePort();
  const runtime = makeRuntime(
    [
      meetingRow({
        interjections: [
          { id: "i1", text: "评审C请注意回滚方案" },
          { id: "i2", text: "大家都聚焦成本", targetSeatIndexes: [1] },
        ],
      }),
      dispatchRow("wo-a", 0, 1, "discarded"),
      dispatchRow("wo-b", 1, 1, "discarded"),
      receiptRow("wo-a", 0, 1, { status: "completed", response: VERDICT_APPROVE }),
      receiptRow("wo-b", 1, 1, { status: "completed", response: VERDICT_REJECT }),
    ],
    port,
  );
  await advance(runtime as never, {
    round: 1,
    workOrderId: "wo-b",
    outcome: { status: "completed", response: VERDICT_REJECT },
  });
  // 第 2 轮逐席直派（代码驱动，不是模型派单）。
  assert.equal(calls.length, 2);
  assert.ok(calls.every((call) => call.councilRound === 2 && call.councilId === COUNCIL));
  // 匿名质询材料：发言按座次字母编号，真实姓名不进信封；抗从众条款在场。
  const seat0Task = calls.find((call) => call.councilSeatIndex === 0)!.task;
  assert.match(seat0Task, /评审A：我认为可行。/);
  assert.match(seat0Task, /评审B：有边界问题。/);
  assert.ok(!seat0Task.includes("小验"), "真实姓名不得出现在跨席材料里");
  assert.match(seat0Task, /改立场必须点名你自己第 1 轮论证的具体缺陷/);
  // 插话：未点名的全体可见；点名 seat1 的只进 seat1 信封。
  assert.match(seat0Task, /主持人：评审C请注意回滚方案/);
  assert.ok(!seat0Task.includes("大家都聚焦成本"));
  const seat1Task = calls.find((call) => call.councilSeatIndex === 1)!.task;
  assert.match(seat1Task, /（点名你）主持人：大家都聚焦成本/);
  // 会议行推进到第 2 轮。
  const council = meetingCouncil(runtime) as { round: number; phase: string };
  assert.equal(council.round, 2);
  assert.equal(council.phase, "seating");

  // 终轮：makePort 铸的工单是 wo-minted-1/2（seat0/seat1），落台账行后收票。
  runtime.store.rows.push(
    dispatchRow("wo-minted-1", 0, 2, "discarded"),
    dispatchRow("wo-minted-2", 1, 2, "discarded"),
    receiptRow(
      "wo-minted-1",
      0,
      2,
      { status: "completed", response: "第1轮论证缺陷：忽略了移动端。\n裁定：通过｜信心：高｜否决：否" },
      2,
    ),
    receiptRow("wo-minted-2", 1, 2, { status: "completed", response: VERDICT_APPROVE }, 2),
  );
  await advance(runtime as never, {
    round: 2,
    workOrderId: "wo-minted-2",
    outcome: { status: "completed", response: VERDICT_APPROVE },
  });
  assert.equal(runtime.moderationCommands.length, 1);
  assert.equal((meetingCouncil(runtime) as { status: string }).status, "approved");
  const gate2 = runtime.store.rows.find(
    (row: LedgerRow) => row.id === councilRoundLedgerId(SESSION, COUNCIL, 2),
  ) as LedgerRow | undefined;
  assert.ok(gate2);
  assert.deepEqual(gate2!.payload.tally, { total: 2, approve: 2, reject: 0, abstain: 0, vetoes: 0 });
  // 主席信封带两轮票源如实标注（终轮发言来自第 2 轮回执）。
  assert.match(runtime.moderationCommands[0]!.text, /评审A：第1轮论证缺陷：忽略了移动端。/);
});

test("缺裁定行：重询一次（补交行在册不再重询）；仍缺按弃权如实标注", async () => {
  const { port, calls } = makePort();
  const runtime = makeRuntime(
    [
      meetingRow(),
      dispatchRow("wo-a", 0, 1, "discarded"),
      dispatchRow("wo-b", 1, 1, "discarded"),
      receiptRow("wo-a", 0, 1, { status: "completed", response: VERDICT_APPROVE }),
      receiptRow("wo-b", 1, 1, { status: "completed", response: "我觉得不行，但忘了写裁定行" }),
    ],
    port,
  );
  // 第一次推进：对缺行席位补交一次，本轮不收票。
  await advance(runtime as never, {
    round: 1,
    workOrderId: "wo-b",
    outcome: { status: "completed", response: "我觉得不行，但忘了写裁定行" },
  });
  assert.equal(calls.length, 1);
  assert.match(calls[0]!.task, /【裁定行补交】/);
  assert.ok(
    runtime.store.rows.some(
      (row: LedgerRow) => row.id === councilSeatRequeryLedgerId(SESSION, COUNCIL, 1, 1),
    ),
  );
  assert.equal(runtime.moderationCommands.length, 0);
  // 补交回执仍缺行：第二次推进按弃权收票（不再重询——补交行已在册）。
  runtime.store.rows.push(
    dispatchRow("wo-minted-1", 1, 1, "discarded"),
    receiptRow("wo-minted-1", 1, 1, { status: "completed", response: "还是没写" }, 2),
  );
  await advance(runtime as never, {
    round: 1,
    workOrderId: "wo-minted-1",
    outcome: { status: "completed", response: "还是没写" },
  });
  // 补交只此一次（第二轮的席位直派不算补交）。
  assert.equal(
    calls.filter((call) => call.task.includes("裁定行补交")).length,
    1,
    "补交只此一次",
  );
  // 1 通过 + 1 弃权 → 首轮非全票 → 进第 2 轮（还没有主席轮）。
  assert.equal(runtime.moderationCommands.length, 0);
  const gate = runtime.store.rows.find(
    (row: LedgerRow) => row.id === councilRoundLedgerId(SESSION, COUNCIL, 1),
  ) as LedgerRow | undefined;
  assert.ok(gate);
  const votes = gate!.payload.votes as { index: number; source: string; stance: string }[];
  assert.deepEqual(
    votes.map((vote) => [vote.index, vote.stance, vote.source]),
    [
      [0, "approve", "verdict_line"],
      [1, "abstain", "missing_verdict"],
    ],
  );
  // 1 通过 + 1 弃权：首轮非全票 → 进第 2 轮（弃权拖垮早退）。
  assert.equal((meetingCouncil(runtime) as { status: string }).status, "running");
});

test("交活失败按缺席弃权；全席派单失败不空转第 2 轮，直接未决收口", async () => {
  const runtime = makeRuntime([
    meetingRow(),
    dispatchRow("wo-a", 0, 1, "discarded"),
    dispatchRow("wo-b", 1, 1, "discarded"),
    receiptRow("wo-a", 0, 1, { status: "failed", reason: "provider rejected" }),
    receiptRow("wo-b", 1, 1, { status: "completed", response: VERDICT_APPROVE }),
  ]);
  await advance(runtime as never, {
    round: 1,
    workOrderId: "wo-b",
    outcome: { status: "completed", response: VERDICT_APPROVE },
  });
  const gate = runtime.store.rows.find(
    (row: LedgerRow) => row.id === councilRoundLedgerId(SESSION, COUNCIL, 1),
  ) as LedgerRow | undefined;
  const votes = gate!.payload.votes as { index: number; stance: string; source: string; workOrderId?: string }[];
  assert.deepEqual(votes[0], {
    index: 0,
    agentName: "小验",
    lens: "impact",
    stance: "abstain",
    veto: false,
    source: "no_response",
    workOrderId: "wo-a",
  });
  // 1 通过 + 1 缺席弃权：非全票 → 分裂进第 2 轮（缺席席也再给一次机会）。
  assert.equal((meetingCouncil(runtime) as { status: string }).status, "running");

  const allFail = makeRuntime(
    [meetingRow({ failedDispatches: [{ round: 1, seatIndex: 0 }, { round: 1, seatIndex: 1 }] })],
    makePort().port,
  );
  await advance(allFail as never, { round: 1 });
  assert.equal(allFail.moderationCommands.length, 1);
  assert.equal((meetingCouncil(allFail) as { status: string }).status, "deadlocked");
});

test("暂停冻结推进：在答回执也不收票；恢复立即补推进", async () => {
  const runtime = makeRuntime([
    meetingRow({ paused: true }),
    dispatchRow("wo-a", 0, 1, "discarded"),
    dispatchRow("wo-b", 1, 1, "discarded"),
    receiptRow("wo-a", 0, 1, { status: "completed", response: VERDICT_APPROVE }),
    receiptRow("wo-b", 1, 1, { status: "completed", response: VERDICT_APPROVE }),
  ]);
  await advance(runtime as never, { round: 1, workOrderId: "wo-b" });
  assert.equal(runtime.moderationCommands.length, 0, "暂停时不得收票");
  assert.equal((meetingCouncil(runtime) as { status: string }).status, "running");
  // 恢复：立即自查推进一次。
  await setCouncilMeetingPaused.call(runtime as never, {
    councilId: COUNCIL,
    paused: false,
  } as never);
  assert.equal(runtime.moderationCommands.length, 1);
  assert.equal((meetingCouncil(runtime) as { status: string }).status, "approved");
});

test("插话：进下一轮信封（点名只进被点席位）；终态会议忽略插话", async () => {
  const { port, calls } = makePort();
  const runtime = makeRuntime(
    [
      meetingRow(),
      dispatchRow("wo-a", 0, 1, "discarded"),
      dispatchRow("wo-b", 1, 1, "discarded"),
      receiptRow("wo-a", 0, 1, { status: "completed", response: VERDICT_APPROVE }),
      receiptRow("wo-b", 1, 1, { status: "completed", response: VERDICT_REJECT }),
    ],
    port,
  );
  await addCouncilInterjection.call(runtime as never, {
    councilId: COUNCIL,
    text: "请注意影响面",
    targetSeatIndexes: [1],
  } as never);
  await advance(runtime as never, {
    round: 1,
    workOrderId: "wo-b",
    outcome: { status: "completed", response: VERDICT_REJECT },
  });
  const seat0Task = calls.find((call) => call.councilSeatIndex === 0)!.task;
  const seat1Task = calls.find((call) => call.councilSeatIndex === 1)!.task;
  assert.ok(!seat0Task.includes("请注意影响面"), "点名插话只进被点席位");
  assert.match(seat1Task, /（点名你）主持人：请注意影响面/);
});

test("主席轮禁派单是机制不是空话：executeTurnCommand 带 AgentDispatch denylist", async () => {
  const captured: Record<string, unknown>[] = [];
  const runtime = {
    sessionId: SESSION,
    shuttingDown: false,
    branchGeneration: 0,
    rootTraceContext: { traceId: "trace" },
    activeForegroundExecution: undefined,
    messageHistory: { addUser: () => undefined },
    persistSyntheticUserNoticeForSession: async () => undefined,
    sessionStore: undefined,
    logger: undefined,
    executeTurnCommand: async (
      _text: string,
      _x: unknown,
      options: Record<string, unknown>,
    ) => {
      captured.push(options);
    },
  } as never;
  const command = {
    branchGeneration: 0,
    createdAt: new Date(),
    id: "cmd-1",
    mode: "council-moderation",
    source: "agent_work_order_batch_qc",
    councilId: COUNCIL,
    kind: "plan",
    round: 1,
    motion: "议题",
    originMeta: {
      backgroundSource: "agent_work_order_batch_qc",
      workId: COUNCIL,
      title: "圆桌会 · 议题",
      councilId: COUNCIL,
      councilPhase: "moderation",
    },
    priority: "next",
    text: "主席信封",
    traceContext: { traceId: "trace" },
  } as unknown as CouncilModerationRuntimeCommand;
  await runCouncilModerationCommand.call(runtime, command);
  assert.equal(captured.length, 1);
  assert.deepEqual(captured[0]!.toolDisallowlist, ["AgentDispatch"]);
  assert.equal(captured[0]!.inputPresentation, "agent_work_order_batch_qc");
});

test("会议记录缺失/畸形：推进空转不猜测", async () => {
  const noMeeting = makeRuntime([dispatchRow("wo-a", 0, 1, "discarded")]);
  await advance(noMeeting as never, { round: 1 });
  assert.equal(noMeeting.moderationCommands.length, 0);

  const broken = makeRuntime([meetingRow(), dispatchRow("wo-a", 0, 1, "discarded")]);
  (broken.store.rows[0]!.payload as Record<string, unknown>).council = { broken: true };
  await advance(broken as never, { round: 1 });
  assert.equal(broken.moderationCommands.length, 0);
});
