// ============================================================
// 圆桌会卡片纯规则的可运行检查（node:test + node:assert/strict，无假件）：
//  1. 裁定行解析：标准行/全角分隔符/最后一个匹配为准；变体与散文不认；
//     返回的正文已剥裁定行；
//  2. 聚合：席位按座次入席、后到回执覆盖、名字在解不出时不被抹掉；
//  3. 决议（镜像内核）：首轮全票早退；分裂进二轮；二轮过半制+一票否决；
//     票没交齐绝不假报终局（running）；
//  4. 裁决卡分区：共识/分歧按最新一轮票面；没交票席位进 pending（区分
//     失败/中断/等新一轮/无裁定行）；盲区=分了座但从未有效发言（判据
//     everSpoke——二轮回执失败但首轮说过话的不算盲区，第一声保留）；
//  5. 过程流：同人同轮短发言合并（裁定取最后），长发言/跨轮不合并；
//  6. 权威决议：主席合议轮带 councilOutcome 即收口（缺裁定行/缺席弃权收口的
//     会议不再卡「进行中」），票面复算如实保留。
//
// 本文件只依赖相对路径的纯函数模块（无 @/ 别名、无 React）。
//
// 运行（本机 tsx 挂死，走 tsc 直编 + node:test）：
//   T=$(mktemp -d) && node node_modules/typescript/bin/tsc \
//     packages/ui/test/councilMeeting.test.ts \
//     --outDir "$T" --rootDir . --module nodenext --moduleResolution nodenext \
//     --target es2023 --skipLibCheck --strict --types node && \
//   node --test "$T/packages/ui/test/councilMeeting.test.js"
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  extractCouncilVerdict,
  selectCouncilMeetingRenderInfo,
  selectCouncilMeetings,
  type CouncilEvidenceUnit,
} from "../src/v4/councilMeeting.js";

let unitSeq = 0;

interface ReceiptSpec {
  councilId: string;
  seatIndex: number;
  lens: "impact" | "requirement" | "edge" | "cost" | "minimal";
  title: string;
  text?: string;
  round?: 1 | 2;
  kind?: "plan" | "acceptance";
}

function seatReceiptUnit(spec: ReceiptSpec): CouncilEvidenceUnit {
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
        ...(spec.kind ? { councilKind: spec.kind } : {}),
        councilRound: spec.round ?? 1,
        councilPhase: "deliberation",
        councilSeat: { index: spec.seatIndex, lens: spec.lens },
      },
    },
    latestAssistantTextRow: spec.text ? { text: spec.text } : undefined,
  };
}

/** 主席合议轮轮头（CLI 权威下发决议的载体）。 */
function moderationUnit(
  councilId: string,
  round: 1 | 2,
  outcome?: "approved" | "rejected" | "deadlocked",
): CouncilEvidenceUnit {
  unitSeq += 1;
  return {
    key: `moderation-${unitSeq}`,
    header: {
      origin: "backgroundResult",
      state: "running",
      originMeta: {
        backgroundSource: "agent_work_order_batch_qc",
        workId: councilId,
        title: "合议 · 圆桌会",
        councilId,
        councilRound: round,
        councilPhase: "moderation",
        ...(outcome ? { councilOutcome: outcome } : {}),
      },
    },
  };
}

const VERDICT_APPROVE = "裁定：通过｜信心：高｜否决：否";
const VERDICT_REJECT = "裁定：打回｜信心：中｜否决：否";
const VERDICT_VETO = "裁定：打回｜信心：高｜否决：是";
const VERDICT_ABSTAIN = "裁定：弃权｜信心：低｜否决：否";

test("裁定行解析：标准行、全角分隔符、取最后一个匹配；变体不认；正文剥裁定行", () => {
  const approve = extractCouncilVerdict(`影响面看过了。\n${VERDICT_APPROVE}`);
  assert.deepEqual(approve?.verdict, {
    stance: "approve",
    confidence: "high",
    veto: false,
  });
  assert.equal(approve?.body, "影响面看过了。");

  // 全角冒号+全角竖线、多余空格都容忍。
  const full = extractCouncilVerdict(
    "看法如下。\n裁定： 打回 ｜ 信心： 中 ｜ 否决： 否",
  );
  assert.deepEqual(full?.verdict, {
    stance: "reject",
    confidence: "medium",
    veto: false,
  });

  // 中途改主意：最后一个匹配为准。
  const changed = extractCouncilVerdict(
    `裁定：打回｜信心：中｜否决：否\n再想想\n${VERDICT_APPROVE}`,
  );
  assert.equal(changed?.verdict.stance, "approve");
  assert.equal(changed?.body, "再想想");

  // 散文里的「我同意」不是票；槽位越界（"较大"）不认。
  assert.equal(extractCouncilVerdict("我同意这个方案，信心较大。"), undefined);
  assert.equal(
    extractCouncilVerdict("裁定：同意｜信心：高｜否决：否"),
    undefined,
  );
  assert.equal(extractCouncilVerdict(""), undefined);
});

test("聚合：席位按座次入席排序，后到回执覆盖先到的，名字解不出不抹掉", () => {
  const meeting = selectCouncilMeetings([
    seatReceiptUnit({
      councilId: "c1",
      seatIndex: 1,
      lens: "requirement",
      title: "小 B 交活",
      text: `需求没偏。\n${VERDICT_APPROVE}`,
    }),
    seatReceiptUnit({
      councilId: "c1",
      seatIndex: 0,
      lens: "impact",
      title: "小 A 交活",
      text: `影响面点名了三处。\n${VERDICT_REJECT}`,
    }),
    // 小 A 改主意（重试/重发）：后到回执覆盖。
    seatReceiptUnit({
      councilId: "c1",
      seatIndex: 0,
      lens: "impact",
      title: "小 A 交活",
      text: `重新看了，没问题。\n${VERDICT_APPROVE}`,
    }),
    // 小 B 的失败回执（标题解不出名）：已有名字保留，状态换 failed。
    seatReceiptUnit({
      councilId: "c1",
      seatIndex: 1,
      lens: "requirement",
      title: "随便一句话",
    }),
  ]);
  assert.equal(meeting.length, 1);
  const seats = meeting[0]?.seats ?? [];
  assert.deepEqual(
    seats.map((seat) => [seat.index, seat.agentName, seat.status]),
    [
      [0, "小 A", "completed"],
      [1, "小 B", "failed"],
    ],
  );
  // 席位 0 覆盖为改主意的通过；席位 1 失败回执没有正文与裁定。
  assert.equal(seats[0]?.verdict?.stance, "approve");
  assert.equal(seats[1]?.verdict, undefined);
  assert.equal(seats[1]?.agentName, "小 B");
});

test("决议：首轮全票早退；首轮分裂=进行中；二轮过半制；票没交齐绝不假报终局", () => {
  const id = "c2";
  // 首轮全票通过 → approved。
  const unanimous = selectCouncilMeetings([
    seatReceiptUnit({
      councilId: id,
      seatIndex: 0,
      lens: "impact",
      title: "A 交活",
      text: `行\n${VERDICT_APPROVE}`,
    }),
    seatReceiptUnit({
      councilId: id,
      seatIndex: 1,
      lens: "requirement",
      title: "B 交活",
      text: `行\n${VERDICT_APPROVE}`,
    }),
  ])[0];
  assert.equal(unanimous?.status, "approved");
  assert.equal(unanimous?.votesComplete, true);
  assert.equal(unanimous?.tally.approve, 2);

  // 首轮分裂 → running（等二轮），弃权也压死通过。
  const split = selectCouncilMeetings([
    seatReceiptUnit({
      councilId: "c3",
      seatIndex: 0,
      lens: "impact",
      title: "A 交活",
      text: `行\n${VERDICT_APPROVE}`,
    }),
    seatReceiptUnit({
      councilId: "c3",
      seatIndex: 1,
      lens: "requirement",
      title: "B 交活",
      text: `不行\n${VERDICT_REJECT}`,
    }),
  ])[0];
  assert.equal(split?.status, "running");
  assert.equal(split?.round, 1);

  // 二轮过半通过；一票否决即否决；平票=分歧待裁决。
  const round2 = (verdicts: readonly string[]) =>
    selectCouncilMeetings([
      seatReceiptUnit({
        councilId: "c4",
        seatIndex: 0,
        lens: "impact",
        title: "A 交活",
        text: `一\n${verdicts[0] ?? ""}`,
        round: 2,
      }),
      seatReceiptUnit({
        councilId: "c4",
        seatIndex: 1,
        lens: "requirement",
        title: "B 交活",
        text: `二\n${verdicts[1] ?? ""}`,
        round: 2,
      }),
      seatReceiptUnit({
        councilId: "c4",
        seatIndex: 2,
        lens: "edge",
        title: "C 交活",
        text: `三\n${verdicts[2] ?? ""}`,
        round: 2,
      }),
    ])[0];
  assert.equal(
    round2([VERDICT_APPROVE, VERDICT_APPROVE, VERDICT_REJECT])?.status,
    "approved",
  );
  assert.equal(
    round2([VERDICT_APPROVE, VERDICT_REJECT, VERDICT_REJECT])?.status,
    "rejected",
  );
  assert.equal(
    round2([VERDICT_APPROVE, VERDICT_VETO, VERDICT_APPROVE])?.status,
    "rejected",
  );
  // 二轮弃权不阻断：过半同意仍通过；同意不过半（平票/弃权拖垮）= 分歧待裁决。
  assert.equal(
    round2([VERDICT_APPROVE, VERDICT_APPROVE, VERDICT_ABSTAIN])?.status,
    "approved",
  );
  assert.equal(
    round2([VERDICT_APPROVE, VERDICT_ABSTAIN, VERDICT_REJECT])?.status,
    "deadlocked",
  );
  assert.equal(
    round2([VERDICT_APPROVE, VERDICT_ABSTAIN, VERDICT_ABSTAIN])?.status,
    "deadlocked",
  );

  // 二轮有一席没交票（首轮说过话也不算数）→ running。
  const partial = selectCouncilMeetings([
    seatReceiptUnit({
      councilId: "c5",
      seatIndex: 0,
      lens: "impact",
      title: "A 交活",
      text: `一\n${VERDICT_APPROVE}`,
    }),
    seatReceiptUnit({
      councilId: "c5",
      seatIndex: 1,
      lens: "requirement",
      title: "B 交活",
      text: `二\n${VERDICT_APPROVE}`,
    }),
    seatReceiptUnit({
      councilId: "c5",
      seatIndex: 0,
      lens: "impact",
      title: "A 交活",
      text: `一\n${VERDICT_APPROVE}`,
      round: 2,
    }),
    seatReceiptUnit({
      councilId: "c5",
      seatIndex: 1,
      lens: "requirement",
      title: "B 交活",
      text: `二\n${VERDICT_APPROVE}`,
      round: 2,
    }),
    seatReceiptUnit({
      councilId: "c5",
      seatIndex: 2,
      lens: "edge",
      title: "C 交活",
      text: `三\n${VERDICT_APPROVE}`,
    }),
  ])[0];
  assert.equal(partial?.round, 2);
  assert.equal(partial?.status, "running");
  assert.equal(partial?.tally.approve, 2);
  assert.deepEqual(
    partial?.pending.map((seat) => seat.reason),
    ["awaitingRound"],
  );
});

test("裁决卡分区：共识/分歧按最新一轮票面；盲区=分了座但从未有效发言", () => {
  const meeting = selectCouncilMeetings([
    seatReceiptUnit({
      councilId: "c6",
      seatIndex: 0,
      lens: "impact",
      title: "A 交活",
      text: `同意行。\n${VERDICT_APPROVE}`,
    }),
    seatReceiptUnit({
      councilId: "c6",
      seatIndex: 1,
      lens: "cost",
      title: "B 交活",
      text: `太贵了。\n${VERDICT_REJECT}`,
    }),
    seatReceiptUnit({
      councilId: "c6",
      seatIndex: 2,
      lens: "edge",
      title: "C 的工单被中断",
    }),
  ])[0];
  assert.equal(meeting?.status, "running");
  assert.deepEqual(
    meeting?.consensus.map((entry) => entry.agentName),
    ["A"],
  );
  assert.deepEqual(
    meeting?.disagreements.map((entry) => [entry.agentName, entry.stance]),
    [["B", "reject"]],
  );
  // 摘要取正文首行；原话全文可回。
  assert.equal(meeting?.consensus[0]?.snippet, "同意行。");
  assert.equal(meeting?.consensus[0]?.statement, "同意行。");
  assert.deepEqual(
    meeting?.pending.map((seat) => [seat.agentName, seat.reason]),
    [["C", "receiptCancelled"]],
  );
  // 边界席被中断没发过言 → 盲区；成本席发言了 → 不是盲区。
  assert.deepEqual(
    meeting?.blindSpots.map((spot) => spot.lens),
    ["edge"],
  );

  // 长正文的首行摘要截到上界。
  const longLine = "很".repeat(120);
  const bounded = selectCouncilMeetings([
    seatReceiptUnit({
      councilId: "c7",
      seatIndex: 0,
      lens: "impact",
      title: "A 交活",
      text: `${longLine}\n${VERDICT_APPROVE}`,
    }),
  ])[0];
  assert.equal(bounded?.consensus[0]?.snippet, `${"很".repeat(79)}…`);
});

test("盲区与第一声：首轮交过活的席位二轮失败不算盲区，首轮发言保留不消声", () => {
  const meeting = selectCouncilMeetings([
    seatReceiptUnit({
      councilId: "c12",
      seatIndex: 0,
      lens: "impact",
      title: "小甲 交活",
      text: `影响面点名三处。\n${VERDICT_APPROVE}`,
    }),
    // 第二轮：小甲回执被中断（内核按缺席弃权计，首轮发言仍是听过的证据）。
    seatReceiptUnit({
      councilId: "c12",
      seatIndex: 0,
      lens: "impact",
      title: "小甲的工单被中断",
      round: 2,
    }),
    // 对照组：边界席从未交过活 → 仍是盲区。
    seatReceiptUnit({
      councilId: "c12",
      seatIndex: 1,
      lens: "edge",
      title: "小乙 的工单被中断",
      round: 2,
    }),
  ])[0];
  const seat0 = meeting?.seats.find((seat) => seat.index === 0);
  assert.equal(seat0?.everSpoke, true);
  assert.equal(seat0?.status, "cancelled");
  assert.equal(seat0?.statement, undefined);
  assert.equal(seat0?.firstStatement, "影响面点名三处。");
  assert.ok(seat0?.firstUnitKey);
  // 盲区只有从未发声的边界席；影响面席虽然二轮失败但首轮说过话，不在列。
  assert.deepEqual(
    meeting?.blindSpots.map((spot) => [spot.lens, spot.agentNames]),
    [["edge", ["小乙"]]],
  );
});

test("权威决议：主席合议轮带 councilOutcome 即收口，票面不齐不再卡「进行中」", () => {
  // 一席交活失败 + 主席轮已到：内核按缺席弃权收口为 deadlocked——
  // UI 终态让位于权威字段，复算票面如实（只有一席有裁定行）。
  const closedModeration = moderationUnit("c13", 1, "deadlocked");
  const closed = selectCouncilMeetings([
    seatReceiptUnit({
      councilId: "c13",
      seatIndex: 0,
      lens: "impact",
      title: "A 交活",
      text: `行\n${VERDICT_APPROVE}`,
    }),
    seatReceiptUnit({
      councilId: "c13",
      seatIndex: 1,
      lens: "requirement",
      title: "B 的工单被中断",
    }),
    closedModeration,
  ])[0];
  assert.equal(closed?.status, "deadlocked");
  assert.equal(closed?.votesComplete, false);
  assert.equal(closed?.hostUnitKey, closedModeration.key);

  // 对照：同样的证据没有权威决议时仍是 running（复算口径不越权）。
  const open = selectCouncilMeetings([
    seatReceiptUnit({
      councilId: "c14",
      seatIndex: 0,
      lens: "impact",
      title: "A 交活",
      text: `行\n${VERDICT_APPROVE}`,
    }),
    seatReceiptUnit({
      councilId: "c14",
      seatIndex: 1,
      lens: "requirement",
      title: "B 的工单被中断",
    }),
    moderationUnit("c14", 1),
  ])[0];
  assert.equal(open?.status, "running");
});

test("过程流：同人同轮短发言合并（裁定取最后），长发言/跨轮不合并；轮次换类型徽章", () => {
  const longLine = "长".repeat(200);
  const meeting = selectCouncilMeetings([
    seatReceiptUnit({
      councilId: "c8",
      seatIndex: 0,
      lens: "impact",
      title: "A 交活",
      text: `先说一句。\n${VERDICT_REJECT}`,
    }),
    seatReceiptUnit({
      councilId: "c8",
      seatIndex: 0,
      lens: "impact",
      title: "A 交活",
      text: `补一句。\n${VERDICT_APPROVE}`,
    }),
    seatReceiptUnit({
      councilId: "c8",
      seatIndex: 1,
      lens: "cost",
      title: "B 交活",
      text: `${longLine}\n${VERDICT_REJECT}`,
    }),
    seatReceiptUnit({
      councilId: "c8",
      seatIndex: 0,
      lens: "impact",
      title: "A 交活",
      text: `二轮质询。\n${VERDICT_APPROVE}`,
      round: 2,
    }),
  ])[0];
  const flow = meeting?.flow ?? [];
  assert.deepEqual(
    flow.map((entry) => [entry.agentName, entry.type, entry.text]),
    [
      ["A", "statement", "先说一句。\n\n补一句。"],
      ["B", "statement", longLine], // 长发言不合并，各自成条
      ["A", "cross", "二轮质询。"],
    ],
  );
  assert.equal(flow[0]?.verdict?.stance, "approve");
});

test("非圆桌轮与无席位回执只注册会议存在，不进席位；host 跟最新证据轮", () => {
  const units: CouncilEvidenceUnit[] = [
    {
      key: "plain",
      header: {
        origin: "backgroundResult",
        state: "completedSuccess",
        originMeta: {
          backgroundSource: "agent_work_order_receipt",
          workId: "wo-x",
          title: "D 交活",
          batchId: "batch-1",
        },
      },
    },
    seatReceiptUnit({
      councilId: "c9",
      seatIndex: 0,
      lens: "impact",
      title: "A 交活",
      text: `行\n${VERDICT_APPROVE}`,
    }),
    {
      key: "moderation",
      header: {
        origin: "backgroundResult",
        state: "running",
        originMeta: {
          backgroundSource: "agent_work_order_batch_qc",
          workId: "c9",
          title: "合议 · 圆桌会",
          councilId: "c9",
          councilRound: 1,
          councilPhase: "moderation",
        },
      },
    },
  ];
  const meetings = selectCouncilMeetings(units);
  assert.equal(meetings.length, 1);
  const meeting = meetings[0];
  assert.equal(meeting?.seats.length, 1);
  assert.equal(meeting?.hostUnitKey, "moderation");
  const renderInfo = selectCouncilMeetingRenderInfo(units);
  assert.equal(renderInfo.size, 1);
  assert.equal(renderInfo.get("moderation")?.councilId, "c9");
});
