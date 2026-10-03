// ============================================================
// 圆桌画布纯规则的可运行检查（node:test + node:assert/strict，无假件）：
//  1. 席位环绕布局：数量还原、椭圆轨道落点、主席席正上方槽位不被任何席
//     位占用、偶数席左右对称、落点两两不同；
//  2. 状态灯映射（如实口径）：completed=发言完成；从未有效发声=缺席；
//     曾发声但最近回执失败/中断=思考中（刀3 接实时证据升级真思考态）；
//  3. 桌面气泡：只留最近两条有效发言（散文/失败回执不算），不足如实给空；
//  4. 裁决石结论行：主席决议首行、有界截断、无主席轮/空白正文如实给空；
//  5. 收口判定：终局三态收口、running 不收口。
//
// 本文件只依赖相对路径的纯函数模块（无 @/ 别名、无 React）。
//
// 运行（本机 tsx 挂死，走 tsc 直编 + node:test）：
//   T=.zcode/workflow-drafts/gate-out && node node_modules/typescript/bin/tsc \
//     packages/ui/test/councilStageModel.test.ts \
//     --outDir "$T" --rootDir . --module nodenext --moduleResolution nodenext \
//     --target es2023 --skipLibCheck --strict --types node && \
//   node --test "$T/packages/ui/test/councilStageModel.test.js"
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";

import type { CouncilEvidenceUnit } from "../src/v4/councilMeeting.js";
import { selectCouncilMeetings } from "../src/v4/councilMeeting.js";
import {
  councilSeatClashPairs,
  councilSeatLampState,
  councilSeatPositions,
  councilStageBubbles,
  councilStoneConclusionLine,
  COUNCIL_TABLE_CENTER,
  isCouncilMeetingClosed,
} from "../src/v4/councilStageModel.js";

let unitSeq = 0;

function seatReceiptUnit(spec: {
  councilId: string;
  seatIndex: number;
  lens: "impact" | "requirement" | "edge" | "cost" | "minimal";
  title: string;
  text?: string;
  round?: 1 | 2;
}): CouncilEvidenceUnit {
  unitSeq += 1;
  return {
    key: `unit-${unitSeq}`,
    header: {
      origin: "backgroundResult",
      state: "completedSuccess",
      originMeta: {
        backgroundSource: "agent_work_order_receipt",
        workId: `wo-${unitSeq}`,
        title: spec.title,
        councilId: spec.councilId,
        councilRound: spec.round ?? 1,
        councilPhase: "deliberation",
        councilSeat: { index: spec.seatIndex, lens: spec.lens },
      },
    },
    latestAssistantTextRow: spec.text ? { text: spec.text } : undefined,
  };
}

function meetingUnit(spec: {
  councilId: string;
  outcome: "approved" | "rejected" | "deadlocked";
  text?: string;
}): CouncilEvidenceUnit {
  unitSeq += 1;
  return {
    key: `moderation-${unitSeq}`,
    header: {
      origin: "backgroundResult",
      state: "completedSuccess",
      originMeta: {
        backgroundSource: "agent_work_order_receipt",
        workId: `wo-${unitSeq}`,
        title: "合议 · 圆桌会",
        councilId: spec.councilId,
        councilRound: 2,
        councilPhase: "moderation",
        councilOutcome: spec.outcome,
      },
    },
    latestAssistantTextRow: spec.text ? { text: spec.text } : undefined,
  };
}

// ── 1. 席位环绕布局 ─────────────────────────────────────────────────

test("席位布局：0 席空表，4 席还原四点且左右对称", () => {
  assert.deepEqual(councilSeatPositions(0), []);
  const four = councilSeatPositions(4);
  assert.equal(four.length, 4);
  const xs = four.map((position) => position.xPct).sort((a, b) => a - b);
  // 偶数席左右对称：x 关于桌心 50 镜像成对。
  assert.ok(Math.abs((xs[0] ?? 0) + (xs[3] ?? 0) - 100) < 0.01);
  assert.ok(Math.abs((xs[1] ?? 0) + (xs[2] ?? 0) - 100) < 0.01);
});

test("席位布局：主席席正上方槽位不被占用，落点两两不同且在画布内", () => {
  for (const count of [1, 2, 3, 4, 5, 6]) {
    const positions = councilSeatPositions(count);
    assert.equal(positions.length, count);
    const seen = new Set<string>();
    for (const position of positions) {
      // 桌心正上方的主席槽位（同 x、高于桌心）永不让给席位。
      if (Math.abs(position.xPct - COUNCIL_TABLE_CENTER.xPct) < 1) {
        assert.ok(
          position.yPct > COUNCIL_TABLE_CENTER.yPct,
          `count=${count} 有席位压在主席正上方`,
        );
      }
      assert.ok(position.xPct >= 0 && position.xPct <= 100);
      assert.ok(position.yPct >= 0 && position.yPct <= 100);
      seen.add(`${position.xPct},${position.yPct}`);
    }
    assert.equal(seen.size, count, `count=${count} 落点重复`);
  }
});

// ── 2. 状态灯映射（如实口径） ───────────────────────────────────────

test("状态灯：completed=发言完成；从未有效发声=缺席", () => {
  assert.equal(
    councilSeatLampState({ status: "completed", everSpoke: true }),
    "spoken",
  );
  assert.equal(
    councilSeatLampState({ status: "failed", everSpoke: false }),
    "absent",
  );
  assert.equal(
    councilSeatLampState({ status: "cancelled", everSpoke: false }),
    "absent",
  );
});

test("状态灯：曾发声但最近回执失败/中断=思考中（人还在会中、话音断了）", () => {
  assert.equal(
    councilSeatLampState({ status: "failed", everSpoke: true }),
    "thinking",
  );
  assert.equal(
    councilSeatLampState({ status: "cancelled", everSpoke: true }),
    "thinking",
  );
});

// ── 3. 桌面气泡 ─────────────────────────────────────────────────────

test("桌面气泡：只留最近两条有效发言，失败回执与空散文不算", () => {
  const councilId = "bubbles-1";
  const meetings = (() => {
    // 直接用聚合纯函数造真模型（同 councilMeeting.test.ts 的造法）。
    const units: CouncilEvidenceUnit[] = [
      seatReceiptUnit({
        councilId,
        seatIndex: 0,
        lens: "impact",
        title: "小 A 交活",
        text: "第一声",
      }),
      seatReceiptUnit({
        councilId,
        seatIndex: 1,
        lens: "edge",
        title: "小 B 交活",
        text: "第二声",
      }),
      seatReceiptUnit({
        councilId,
        seatIndex: 2,
        lens: "cost",
        title: "小 C 的工单未完成",
      }),
      seatReceiptUnit({
        councilId,
        seatIndex: 3,
        lens: "requirement",
        title: "小 D 交活",
        text: "   ",
      }),
    ];
    return selectCouncilMeetingsForTest(units);
  })();
  const meeting = meetings[0];
  assert.ok(meeting);
  const bubbles = councilStageBubbles(meeting);
  assert.equal(bubbles.current?.text, "第二声");
  assert.equal(bubbles.current?.agentName, "小 B");
  assert.equal(bubbles.previous?.text, "第一声");
  // entryKey 对得上过程流条目（气泡点按滚动定位的靶子）。
  const flowKeys = new Set(meeting.flow.map((entry) => entry.key));
  assert.ok(bubbles.current ? flowKeys.has(bubbles.current.entryKey) : false);
});

test("桌面气泡：一条发言都还没有时如实给空", () => {
  const councilId = "bubbles-empty";
  const meeting = selectCouncilMeetingsForTest([
    seatReceiptUnit({
      councilId,
      seatIndex: 0,
      lens: "impact",
      title: "小 A 的工单被中断",
    }),
  ])[0];
  assert.ok(meeting);
  const bubbles = councilStageBubbles(meeting);
  assert.equal(bubbles.current, null);
  assert.equal(bubbles.previous, null);
});

// ── 4. 裁决石结论行 ─────────────────────────────────────────────────

test("裁决石结论行：主席决议首行入选，超长有界截断", () => {
  const councilId = "stone-1";
  const meeting = selectCouncilMeetingsForTest([
    seatReceiptUnit({
      councilId,
      seatIndex: 0,
      lens: "impact",
      title: "小 A 交活",
      text: "裁定：通过|信心：高|否决：否\n正文",
    }),
    meetingUnit({
      councilId,
      outcome: "approved",
      text: "第一行结论\n第二行细节",
    }),
  ])[0];
  assert.ok(meeting);
  assert.equal(councilStoneConclusionLine(meeting), "第一行结论");

  const longLine = "长".repeat(200);
  const truncated = selectCouncilMeetingsForTest([
    meetingUnit({ councilId: "stone-2", outcome: "approved", text: `${longLine}\n尾行` }),
  ])[0];
  assert.ok(truncated);
  const line = councilStoneConclusionLine(truncated);
  assert.ok(line);
  assert.ok(line.endsWith("…"));
  assert.ok(line.length < 200);
});

test("裁决石结论行：无主席轮/空白正文如实给空，绝不从散文编结论", () => {
  const councilId = "stone-empty";
  const meeting = selectCouncilMeetingsForTest([
    seatReceiptUnit({
      councilId,
      seatIndex: 0,
      lens: "impact",
      title: "小 A 交活",
      text: "裁定：通过|信心：高|否决：否\n正文",
    }),
  ])[0];
  assert.ok(meeting);
  assert.equal(councilStoneConclusionLine(meeting), null);
  const blank = selectCouncilMeetingsForTest([
    meetingUnit({ councilId: "stone-blank", outcome: "approved", text: "  \n  " }),
  ])[0];
  assert.ok(blank);
  assert.equal(councilStoneConclusionLine(blank), null);
});

// ── 5. 收口判定 ─────────────────────────────────────────────────────

test("收口判定：终局三态收口，running 不收口", () => {
  assert.equal(isCouncilMeetingClosed({ status: "running" }), false);
  assert.equal(isCouncilMeetingClosed({ status: "approved" }), true);
  assert.equal(isCouncilMeetingClosed({ status: "rejected" }), true);
  assert.equal(isCouncilMeetingClosed({ status: "deadlocked" }), true);
});

test("分歧弦席位对：立场相异拉弦，弃权与缺席不拉", () => {
  const seats = [
    { index: 0, verdict: { stance: "approve" as const, confidence: "high" as const, veto: false } },
    { index: 1, verdict: { stance: "reject" as const, confidence: "medium" as const, veto: false } },
    { index: 2, verdict: { stance: "approve" as const, confidence: "low" as const, veto: true } },
    { index: 3, verdict: undefined },
  ];
  assert.deepEqual(councilSeatClashPairs(seats), [
    { seatIndexA: 0, seatIndexB: 1 },
    { seatIndexA: 1, seatIndexB: 2 },
  ]);
  const unanimous = [seats[0], seats[0], seats[0]];
  assert.deepEqual(councilSeatClashPairs(unanimous), []);
});

// ── 测试脚手架 ──────────────────────────────────────────────────────

function selectCouncilMeetingsForTest(units: CouncilEvidenceUnit[]) {
  return selectCouncilMeetings(units);
}
