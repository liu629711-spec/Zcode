// ============================================================
// 圆桌画布（2026-10-03 刀2）的纯规则：席位环绕布局、状态灯映射、桌面气泡
// 选取、裁决石结论行。只依赖 councilMeeting.ts 的类型与数据形状（@/ 别名
// 不进编译面），配 node:test（packages/ui/test/councilStageModel.test.ts）。
//
// 刀2 是静态/半静态画布：状态灯是三态静态色（呼吸动画归刀3），映射必须
// 如实——数据里席位只有 completed/failed/cancelled 与「最新轮还没轮到」，
// 没有「正在说话」信号；思考灯在静态数据下=仍在会中待言（曾发声但最近
// 一次回执失败/中断），真正的实时思考态等刀3接证据流。
// ============================================================

import type {
  CouncilFlowEntry,
  CouncilMeetingModel,
  CouncilMeetingSeatModel,
} from "./councilMeeting.js";

/** 席位状态灯三态（刀2 静态）：思考中=琥珀待呼吸 / 发言完成=常亮 / 缺席=灰。 */
export type CouncilSeatLampState = "thinking" | "spoken" | "absent";

/**
 * 状态灯映射（如实口径）：
 * - completed → spoken：最近一次回执是有效交活，「发言完成」；
 * - 从未有效发声（!everSpoke）→ absent：攻角分了座但全场没听到声音（盲区席位）；
 * - 其余（曾发声、最近一次回执失败/被中断）→ thinking：人还在会中、话音断了，
 *   刀3 接实时证据后这里升级成真「正在思考」。
 */
export function councilSeatLampState(
  seat: Pick<CouncilMeetingSeatModel, "status" | "everSpoke">,
): CouncilSeatLampState {
  if (seat.status === "completed") return "spoken";
  if (!seat.everSpoke) return "absent";
  return "thinking";
}

/** 画布坐标系（百分比）：圆桌椭圆中心。组件 CSS 与此同源，手工同步。 */
export const COUNCIL_TABLE_CENTER = { xPct: 50, yPct: 56 } as const;

/** 席位坐在桌沿的椭圆半径（百分比；桌身 72×58，席位略外圈）。 */
const SEAT_ORBIT_RX = 42;
const SEAT_ORBIT_RY = 34;

export interface CouncilSeatPosition {
  xPct: number;
  yPct: number;
}

function round2(value: number): number {
  return Math.round(value * 100) / 100;
}

/**
 * 席位环绕椭圆桌的绝对定位（DOM 细线不引库，位置就是百分比）。
 * 起始角错开半个步长：主席席独占正上方槽位，任何席都不压在主席头顶。
 * count=4 → 左上/左下/右下/右上对称四角；count=5（五攻角全到）同样均匀环绕。
 */
export function councilSeatPositions(count: number): CouncilSeatPosition[] {
  if (count <= 0) return [];
  const positions: CouncilSeatPosition[] = [];
  for (let index = 0; index < count; index += 1) {
    const angleDeg = 90 + ((index + 0.5) * 360) / count;
    const angle = (angleDeg * Math.PI) / 180;
    positions.push({
      xPct: round2(COUNCIL_TABLE_CENTER.xPct + SEAT_ORBIT_RX * Math.cos(angle)),
      yPct: round2(COUNCIL_TABLE_CENTER.yPct - SEAT_ORBIT_RY * Math.sin(angle)),
    });
  }
  return positions;
}

/** 桌面气泡：只留当前+上一条发言（历史沉卷宗）；散文/失败回执不算发言。 */
export interface CouncilStageBubble {
  /** 卷宗过程流条目 key（点气泡滚动定位用）。 */
  entryKey: string;
  agentName: string;
  text: string;
  round: 1 | 2;
  type: CouncilFlowEntry["type"];
}

export interface CouncilStageBubbles {
  current: CouncilStageBubble | null;
  previous: CouncilStageBubble | null;
}

function toBubble(entry: CouncilFlowEntry): CouncilStageBubble {
  return {
    entryKey: entry.key,
    agentName: entry.agentName,
    text: entry.text,
    round: entry.round,
    type: entry.type,
  };
}

/** 最近两条有效发言：当前=最后一条，上一条=倒数第二条；不足如实给 null。 */
export function councilStageBubbles(
  meeting: Pick<CouncilMeetingModel, "flow">,
): CouncilStageBubbles {
  const spoken = meeting.flow.filter(
    (entry) => entry.status === "completed" && entry.text.trim().length > 0,
  );
  const current = spoken.at(-1);
  const previous = spoken.at(-2);
  return {
    ...(current ? { current: toBubble(current) } : { current: null }),
    ...(previous ? { previous: toBubble(previous) } : { previous: null }),
  };
}

/** 裁决石结论行字符上界（全文在卷宗主席结论区，石上只留一行）。 */
const COUNCIL_STONE_LINE_MAX_CHARS = 120;

/**
 * 裁决石的一行结论：主席决议正文的首个非空行（有界）；主席轮还没到/没有
 * 正文时返回 null，组件退状态词，绝不从散文编结论。
 */
export function councilStoneConclusionLine(
  meeting: Pick<CouncilMeetingModel, "moderatorSummary" | "status">,
): string | null {
  const text = meeting.moderatorSummary?.text;
  if (!text) return null;
  const line = text
    .split("\n")
    .map((candidate) => candidate.trim())
    .find(Boolean);
  if (!line) return null;
  return line.length > COUNCIL_STONE_LINE_MAX_CHARS
    ? `${line.slice(0, COUNCIL_STONE_LINE_MAX_CHARS - 1)}…`
    : line;
}

/** 分歧弦（刀4）：最近一次裁定立场相异的席位对——只有 通过↔打回 拉弦，弃权不拉。 */
export interface CouncilStageClashPair {
  seatIndexA: number;
  seatIndexB: number;
}

export function councilSeatClashPairs(
  seats: ReadonlyArray<Pick<CouncilMeetingSeatModel, "index" | "verdict">>,
): CouncilStageClashPair[] {
  const voiced: Array<{ index: number; stance: "approve" | "reject" }> = [];
  for (const seat of seats) {
    if (seat.verdict?.stance === "approve" || seat.verdict?.stance === "reject") {
      voiced.push({ index: seat.index, stance: seat.verdict.stance });
    }
  }
  const pairs: CouncilStageClashPair[] = [];
  for (let left = 0; left < voiced.length; left += 1) {
    const first = voiced[left];
    if (!first) continue;
    for (let right = left + 1; right < voiced.length; right += 1) {
      const second = voiced[right];
      if (!second) continue;
      if (first.stance !== second.stance) {
        pairs.push({ seatIndexA: first.index, seatIndexB: second.index });
      }
    }
  }
  return pairs;
}

/** 会议是否已收口（终局四态去 running）：收口=桌心裁决石+只读回放。 */
export function isCouncilMeetingClosed(
  meeting: Pick<CouncilMeetingModel, "status">,
): boolean {
  return meeting.status !== "running";
}
