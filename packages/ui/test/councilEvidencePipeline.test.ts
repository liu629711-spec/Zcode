// ============================================================
// 圆桌会独立页数据管道的可运行检查（node:test + node:assert/strict，无假件）：
//  1. council/evidence 证据抽取（bootstrap 纯规则）：councilId 对号的唤醒轮成
//     单元、assistant 发言随轮收口、副屏继承消息不混入、其他会议的轮只当边界；
//  2. 证据 → 会议模型（UI selectCouncilMeetings）：服务器证据单元无损驱动画布
//     模型——席位入座、裁定复算、主席轮决议正文与权威终局；
//  3. 独立页目录选中对账（resolveSelectedCouncilEntry）：命中两小节、目录刷新
//     后会议消失即回空态。
//
// 跨包相对导入：UI 测试直引 bootstrap 的零依赖纯模块（councilEvidence.ts 不
// import contracts/shared，可独立编译）。本文件只依赖相对路径模块（无 @/ 别名、
// 无 React、无 zod）。
//
// 运行（仓库根，tsc 直编 + node:test）：
//   T=.zcode/workflow-drafts/gate-out && node node_modules/typescript/bin/tsc \
//     packages/ui/test/councilEvidencePipeline.test.ts \
//     --outDir "$T" --rootDir . --module nodenext --moduleResolution nodenext \
//     --target es2023 --skipLibCheck --strict --types node --jsx react-jsx && \
//   node --test "$T/packages/ui/test/councilEvidencePipeline.test.js"
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  extractCouncilEvidenceUnits,
  type CouncilEvidenceSourceMessage,
} from "../../../apps/zcode-cli/packages/bootstrap/src/zcode-protocol/councilEvidence.js";
import { selectCouncilMeetings } from "../src/v4/councilMeeting.js";
import {
  buildCouncilMeetingDirectory,
  resolveSelectedCouncilEntry,
} from "../src/v4/councilMeetingDirectory.js";

let seq = 0;

function msg(spec: {
  role: string;
  councilId?: string;
  backgroundSource?: string;
  phase?: "deliberation" | "moderation";
  round?: 1 | 2;
  texts?: string[];
  ignoredText?: string;
  providerContextOnly?: boolean;
}): CouncilEvidenceSourceMessage {
  seq += 1;
  const parts: { type: string; text: string; ignored?: boolean }[] = [];
  for (const text of spec.texts ?? []) parts.push({ type: "text", text });
  if (spec.ignoredText !== undefined) {
    parts.push({ type: "text", text: spec.ignoredText, ignored: true });
  }
  return {
    id: `m-${seq}`,
    role: spec.role,
    ...(spec.councilId
      ? {
          originMeta: {
            backgroundSource: spec.backgroundSource ?? "agent_work_order_receipt",
            workId: `wo-${seq}`,
            title: "员工甲 交活",
            councilId: spec.councilId,
            councilRound: spec.round ?? 1,
            councilPhase: spec.phase ?? "deliberation",
            ...(spec.phase === "moderation"
              ? { councilOutcome: "approved" as const }
              : { councilSeat: { index: 0, lens: "impact" as const } }),
          },
        }
      : {}),
    parts,
    ...(spec.providerContextOnly ? { providerContextOnly: true } : {}),
  };
}

// ── 1. 证据抽取 ─────────────────────────────────────────────────────

test("证据抽取：carrier 开单元、后续 assistant 发言随轮收口", () => {
  const units = extractCouncilEvidenceUnits(
    [
      msg({ role: "user", councilId: "c1" }),
      msg({ role: "assistant", texts: ["第一段发言"] }),
      msg({ role: "assistant", texts: ["第二段发言"] }),
      msg({ role: "user", councilId: "c1", round: 2 }),
      msg({ role: "assistant", texts: ["质询发言"] }),
    ],
    "c1",
  );
  assert.equal(units.length, 2);
  const [round1, round2] = units;
  assert.equal(round1?.key, "m-1");
  assert.equal(round1?.header.origin, "backgroundResult");
  assert.equal(round1?.assistantTextRows?.length, 2);
  assert.equal(round1?.latestAssistantTextRow?.text, "第二段发言");
  assert.equal(round2?.latestAssistantTextRow?.text, "质询发言");
});

test("证据抽取：providerContextOnly assistant 与 ignored part 不进发言", () => {
  const units = extractCouncilEvidenceUnits(
    [
      msg({ role: "user", councilId: "c1" }),
      msg({ role: "assistant", providerContextOnly: true, texts: ["继承的副屏历史"] }),
      msg({ role: "assistant", texts: ["真发言"], ignoredText: "被忽略的流尾" }),
    ],
    "c1",
  );
  assert.equal(units.length, 1);
  assert.equal(units[0]?.assistantTextRows?.length, 1);
  assert.equal(units[0]?.latestAssistantTextRow?.text, "真发言");
});

test("证据抽取：别的会议轮与普通用户输入只当边界，不是证据", () => {
  const units = extractCouncilEvidenceUnits(
    [
      msg({ role: "user", councilId: "c1" }),
      msg({ role: "assistant", texts: ["甲的发言"] }),
      msg({ role: "user", councilId: "c2" }),
      msg({ role: "assistant", texts: ["别的会议的发言"] }),
      msg({ role: "user" }),
      msg({ role: "assistant", texts: ["普通回复"] }),
    ],
    "c1",
  );
  assert.equal(units.length, 1);
  assert.equal(units[0]?.latestAssistantTextRow?.text, "甲的发言");
});

test("证据抽取：无 carrier 的会话如实给空，畸形 originMeta 不猜", () => {
  assert.deepEqual(
    extractCouncilEvidenceUnits(
      [msg({ role: "assistant", texts: ["孤儿发言"] })],
      "c1",
    ),
    [],
  );
  const bad = extractCouncilEvidenceUnits(
    [{ id: "m-bad", role: "user", originMeta: "corrupted", parts: [] }],
    "c1",
  );
  assert.deepEqual(bad, []);
});

// ── 2. 证据 → 会议模型（服务器载荷形状无损驱动画布）───────────────

test("证据管道：抽取输出直接喂 selectCouncilMeetings 得到完整会议模型", () => {
  let moderationCarrier: CouncilEvidenceSourceMessage;
  const messages: CouncilEvidenceSourceMessage[] = [
    // 第一轮：员工甲席位回执（通过票）。
    msg({ role: "user", councilId: "c9" }),
    msg({
      role: "assistant",
      texts: ["影响面看过了，方案闭环。\n裁定：通过|信心：高|否决：否"],
    }),
    // 主席合议轮：权威终局 + 决议正文。
    (moderationCarrier = msg({
      role: "user",
      councilId: "c9",
      backgroundSource: "agent_work_order_batch_qc",
      phase: "moderation",
      round: 2,
    })),
    msg({ role: "assistant", texts: ["全场合议通过，进入执行。"] }),
  ];
  const units = extractCouncilEvidenceUnits(messages, "c9");
  const models = selectCouncilMeetings(units);
  assert.equal(models.length, 1);
  const meeting = models[0];
  assert.ok(meeting, "模型缺失");
  assert.equal(meeting.councilId, "c9");
  assert.equal(meeting.round, 2);
  // 终态以主席轮权威决议为准（缺席弃权收口的会议复算永远等不齐票）。
  assert.equal(meeting.status, "approved");
  assert.equal(meeting.seats.length, 1);
  assert.equal(meeting.seats[0]?.agentName, "员工甲");
  assert.equal(meeting.seats[0]?.verdict?.stance, "approve");
  // 计票只认最新轮一致的席位：席位在第 1 轮投票、会议已推进到第 2 轮，
  // 最新轮票面如实为 0（该席在最新轮还没发言，进 pending）。
  assert.equal(meeting.tally.approve, 0);
  assert.equal(meeting.votesComplete, false);
  assert.deepEqual(
    meeting.pending.map((seat) => seat.reason),
    ["awaitingRound"],
  );
  assert.equal(meeting.moderatorSummary?.text, "全场合议通过，进入执行。");
  // 挂载轮 = 最后一个证据轮（主席合议轮）。
  assert.equal(meeting.hostUnitKey, moderationCarrier.id);
});

// ── 3. 目录选中对账 ────────────────────────────────────────────────

function summaryOf(spec: {
  councilId: string;
  status: "running" | "approved" | "cancelled";
  timeUpdated: number;
}) {
  return {
    councilId: spec.councilId,
    sessionId: `session-${spec.councilId}`,
    kind: "plan" as const,
    status: spec.status,
    round: 1 as const,
    motion: `议题 ${spec.councilId}`,
    timeUpdated: spec.timeUpdated,
  };
}

test("目录选中：进行中与已收口都能命中；目录刷新后会议消失即回空态", () => {
  const directory = buildCouncilMeetingDirectory([
    summaryOf({ councilId: "c-run", status: "running", timeUpdated: 20 }),
    summaryOf({ councilId: "c-done", status: "approved", timeUpdated: 10 }),
  ]);
  assert.equal(resolveSelectedCouncilEntry(directory, "c-run")?.councilId, "c-run");
  assert.equal(resolveSelectedCouncilEntry(directory, "c-done")?.councilId, "c-done");
  assert.equal(resolveSelectedCouncilEntry(directory, null), undefined);
  assert.equal(resolveSelectedCouncilEntry(directory, "c-gone"), undefined);
  const empty = buildCouncilMeetingDirectory([]);
  assert.equal(resolveSelectedCouncilEntry(empty, "c-run"), undefined);
});
