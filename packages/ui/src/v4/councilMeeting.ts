// ============================================================
// 圆桌会（真会议）卡片纯规则：把发起方会话里的席位回执轮按 councilId 聚成
// 一场会议的模型，供一行摘要卡与评审专区渲染（对照 batchId 的工地卡聚合）。
//
// 证据只有轮头 originMeta.council*（CLI 权威下发，不从文本反推会议身份）：
// 1. 席位回执轮：backgroundSource=agent_work_order_receipt + councilId +
//    councilRound/councilSeat —— 席位入座、发言正文（回执正文全文，裁定行钉在
//    结尾）与终态（标题同工地卡口径：交活/未完成/被中断）；
// 2. 主席合议轮：councilPhase=moderation —— 会议走到主席整理（证据只证明
//    会议存在与推进，轮内决议正文由合议轮自己渲染，这里不重复）。
//
// 票与决议按内核协议（core/src/subagent/council.ts 的
// parseCouncilVerdictLine / countCouncilVotes / resolveCouncilDecision）镜像复算：
// 散文不算票，终局=票（主席无权重、不许改票），所以 UI 复算与内核判定同真。
// ponytail: 内核不在 UI 可依赖的工作区（packages/ui 只认 @zcode/shared），
// 解析与判定在这里手工同步；若 originMeta 将来下发结构化决议字段，复算让位。
// ============================================================

import {
  parseFailedReceiptAgentName,
  parseReceiptDelivererName,
} from "./workOrderForward.js";

/** 席位攻角 id 闭集（与 contracts COUNCIL_SEAT_LENS_IDS / 内核 COUNCIL_SEAT_LENSES 手工同步）。 */
export const COUNCIL_SEAT_LENS_IDS = [
  "impact",
  "requirement",
  "edge",
  "cost",
  "minimal",
] as const;
export type CouncilSeatLensId = (typeof COUNCIL_SEAT_LENS_IDS)[number];

/** 裁定行三态结论（内核 CouncilStance 口径：只有打回阻断，弃权压死通过）。 */
export type CouncilStance = "approve" | "reject" | "abstain";
/** 信心度（内核 CouncilConfidence 口径）。 */
export type CouncilConfidence = "high" | "medium" | "low";

export interface CouncilVerdict {
  stance: CouncilStance;
  confidence: CouncilConfidence;
  /** 一票否决：终轮出现即否决整案。 */
  veto: boolean;
}

/** 一场会议在 UI 上的终局四态（proposed/cancelled 无轮头证据，不进卡片）。 */
export type CouncilMeetingViewStatus =
  "running" | "approved" | "rejected" | "deadlocked";

export type CouncilSeatReceiptStatus = "completed" | "failed" | "cancelled";

/** 一个席位：座次+员工+攻角是稳定的，发言/裁定按「最近一次回执」更新。 */
export interface CouncilMeetingSeatModel {
  index: number;
  /** 员工真名（回执标题权威解出；解不出为空串，UI 退灰占位）。 */
  agentName: string;
  lens: CouncilSeatLensId;
  /** 本席最近一次回执所在轮。 */
  round: 1 | 2;
  status: CouncilSeatReceiptStatus;
  /** 最近一次发言正文（裁定行已剥，原话全文）。 */
  statement?: string;
  /** 最近一次裁定行解析结果（散文不算票，缺行=没交票）。 */
  verdict?: CouncilVerdict;
  /** 最近一次回执所在轮的 unit key（回原话定位用）。 */
  unitKey: string;
}

/** 裁决卡里的一条立场（共识/分歧共用）：立场+信心+一句话，可回原话。 */
export interface CouncilStanceEntry {
  seatIndex: number;
  agentName: string;
  lens: CouncilSeatLensId;
  stance: CouncilStance;
  confidence: CouncilConfidence;
  veto: boolean;
  /** 一句话摘要（正文首个非空行，有界）。 */
  snippet: string;
  /** 原话全文（裁定行已剥）。 */
  statement: string;
  unitKey: string;
}

/** 这轮还没交票的席位（发言失败/被中断/新一轮还没轮到他/没有有效裁定行）。 */
export interface CouncilPendingSeat {
  seatIndex: number;
  agentName: string;
  lens: CouncilSeatLensId;
  reason:
    "receiptFailed" | "receiptCancelled" | "awaitingRound" | "noVerdictLine";
  unitKey?: string;
}

/** 盲区：攻角分到了席位但从未发出有效声音（回执失败/中断，散文不算发言）。 */
export interface CouncilBlindSpot {
  lens: CouncilSeatLensId;
  agentNames: string[];
}

export interface CouncilFlowEntry {
  key: string;
  agentName: string;
  round: 1 | 2;
  /** 陈述=首轮发言；质询=第二轮（内核封顶 2 轮，二轮即质询轮）。 */
  type: "statement" | "cross";
  status: CouncilSeatReceiptStatus;
  /** 发言正文（裁定行已剥；失败/中断为空串，UI 换大白话）。 */
  text: string;
  verdict?: CouncilVerdict;
  unitKey: string;
}

export interface CouncilMeetingTally {
  approve: number;
  reject: number;
  abstain: number;
  vetoes: number;
  /** 已解析出裁定行的席位数（散文不算票）。 */
  voted: number;
}

export interface CouncilMeetingModel {
  councilId: string;
  /** 会议类型（席位回执轮头带；缺席=旧数据，UI 退通用词）。 */
  kind?: "plan" | "acceptance";
  /** 最近一次证据所在轮（终局后保留最后一轮）。 */
  round: 1 | 2;
  status: CouncilMeetingViewStatus;
  seats: CouncilMeetingSeatModel[];
  /** 最新一轮票面（每席一票，同人后到回执覆盖先到的）。 */
  tally: CouncilMeetingTally;
  /** 最新一轮是否所有已知席位都交了票（没交齐=进行中，绝不假报终局）。 */
  votesComplete: boolean;
  /** 最新一轮的同意席（裁决卡共识区）。 */
  consensus: CouncilStanceEntry[];
  /** 最新一轮的打回/弃权席（裁决卡分歧区；永不折叠、不显示成已解决）。 */
  disagreements: CouncilStanceEntry[];
  /** 最新一轮没交票的席位。 */
  pending: CouncilPendingSeat[];
  /** 攻角分了座但从未有效发言（裁决卡盲区区）。 */
  blindSpots: CouncilBlindSpot[];
  /** 过程流：全部席位回执按到达顺序（同人同轮短发言已合并）。 */
  flow: CouncilFlowEntry[];
  /** 卡片挂载轮 = 最后一个 contributing 证据轮（活边，同工地卡纪律）。 */
  hostUnitKey: string;
}

/**
 * 本模块关注的轮证据面（结构兼容 ConversationTurnRenderUnit/TurnHeaderRow）：
 * 独立成窄形状是为了纯函数零别名依赖可单测（@/ 别名与 shared 类型不进编译面）。
 */
export interface CouncilEvidenceUnit {
  readonly key: string;
  readonly header?: {
    readonly origin?: string;
    readonly state?: string;
    readonly originMeta?: {
      readonly backgroundSource?: string;
      readonly workId?: string;
      readonly title?: string;
      readonly batchId?: string;
      readonly councilId?: string;
      readonly councilKind?: "plan" | "acceptance";
      readonly councilRound?: 1 | 2;
      readonly councilPhase?:
        "seating" | "deliberation" | "tally" | "moderation";
      readonly councilSeat?: {
        readonly index: number;
        readonly lens: CouncilSeatLensId;
      };
    };
  };
  readonly latestAssistantTextRow?: { readonly text?: string };
  readonly assistantTextRows?: readonly { readonly text?: string }[];
}

/** 裁定行解析（镜像内核 parseCouncilVerdictLine：全文取最后一个匹配，容忍全角/空格；变体不认）。 */
const VERDICT_LINE_SOURCE =
  "裁定\\s*[:：]\\s*(通过|打回|弃权)\\s*[|｜]\\s*信心\\s*[:：]\\s*(高|中|低)\\s*[|｜]\\s*否决\\s*[:：]\\s*(是|否)";

/**
 * 从一段发言里取裁定行：返回解析结果与剥掉全部裁定行后的正文（票在行尾徽章上，
 * 正文不重复印票）。缺行/槽位越界 → undefined（散文不算票，也不从散文猜）。
 */
export function extractCouncilVerdict(
  text: string,
): { verdict: CouncilVerdict; body: string } | undefined {
  const pattern = new RegExp(VERDICT_LINE_SOURCE, "gu");
  let match: RegExpExecArray | null;
  let last: RegExpExecArray | null = null;
  while ((match = pattern.exec(text)) !== null) last = match;
  if (!last) return undefined;
  const stance = last[1];
  const confidence = last[2];
  const veto = last[3];
  if (!stance || !confidence || !veto) return undefined;
  const body = text.replace(new RegExp(VERDICT_LINE_SOURCE, "gu"), "").trim();
  return {
    verdict: {
      stance:
        stance === "通过"
          ? "approve"
          : stance === "打回"
            ? "reject"
            : "abstain",
      confidence:
        confidence === "高" ? "high" : confidence === "中" ? "medium" : "low",
      veto: veto === "是",
    },
    body,
  };
}

/** 内核决议判定的镜像（resolveCouncilDecision）：round=1 全票早退，round=2 过半制+一票否决。 */
function resolveCouncilViewDecision(
  tally: CouncilMeetingTally,
  round: 1 | 2,
): "approved" | "rejected" | "deliberate" | "deadlocked" {
  if (round === 1) {
    if (tally.voted === 0) return "deliberate";
    if (
      tally.approve === tally.voted &&
      tally.vetoes === 0 &&
      tally.abstain === 0
    ) {
      return "approved";
    }
    if (tally.reject === tally.voted) return "rejected";
    return "deliberate";
  }
  if (tally.vetoes > 0) return "rejected";
  if (tally.approve * 2 > tally.voted) return "approved";
  if (tally.reject * 2 > tally.voted) return "rejected";
  return "deadlocked";
}

/** 发言摘要在裁决卡上的字符上界（整篇原话仍在，点开可见）。 */
const COUNCIL_SNIPPET_MAX_CHARS = 80;

/** 同人同轮短发言的合并阈值：超过即各自成条（长发言合并可读性反而降）。 */
const COUNCIL_MERGE_MAX_CHARS = 160;

function receiptFullText(unit: CouncilEvidenceUnit): string {
  const text =
    unit.latestAssistantTextRow?.text ?? unit.assistantTextRows?.at(-1)?.text;
  return text?.trim() ?? "";
}

/** 一句话摘要：正文首个非空行，有界（原话全文永远可点开）。 */
function snippetOf(body: string): string {
  const line = body
    .split("\n")
    .map((candidate) => candidate.trim())
    .find(Boolean);
  if (!line) return "";
  return line.length > COUNCIL_SNIPPET_MAX_CHARS
    ? `${line.slice(0, COUNCIL_SNIPPET_MAX_CHARS - 1)}…`
    : line;
}

interface MeetingDraft {
  kind?: "plan" | "acceptance";
  round: 1 | 2;
  seats: Map<number, CouncilMeetingSeatModel>;
  flow: CouncilFlowEntry[];
  hostUnitKey: string;
}

/** 回执终态口径同工地卡：标题解出交活方=completed；被中断=cancelled；其余=failed。 */
function seatStatusFromTitle(title: string): CouncilSeatReceiptStatus {
  if (parseReceiptDelivererName(title)) return "completed";
  if (title.includes("被中断")) return "cancelled";
  return "failed";
}

/** 回执标题里的员工名（completed 直解；失败/中断按 CLI 后缀解，解不出留空）。 */
function seatAgentNameFromTitle(title: string): string {
  return (
    parseReceiptDelivererName(title) ?? parseFailedReceiptAgentName(title) ?? ""
  );
}

export function selectCouncilMeetings(
  units: readonly CouncilEvidenceUnit[],
): CouncilMeetingModel[] {
  const drafts = new Map<string, MeetingDraft>();
  const ensureDraft = (id: string, unitKey: string): MeetingDraft => {
    let draft = drafts.get(id);
    if (!draft) {
      draft = { round: 1, seats: new Map(), flow: [], hostUnitKey: unitKey };
      drafts.set(id, draft);
      return draft;
    }
    // host 跟着最新证据走：席位回执每到一轮、主席合议轮一到，圆桌卡就搬一轮。
    draft.hostUnitKey = unitKey;
    return draft;
  };

  for (const unit of units) {
    const originMeta =
      unit.header?.origin === "backgroundResult"
        ? unit.header.originMeta
        : undefined;
    if (!originMeta) continue;
    const councilId = originMeta.councilId;
    if (!councilId) continue;
    const round = originMeta.councilRound === 2 ? 2 : 1;
    const draft = ensureDraft(councilId, unit.key);
    if (originMeta.councilKind) draft.kind = originMeta.councilKind;
    if (round > draft.round) draft.round = round;

    if (
      originMeta.backgroundSource === "agent_work_order_receipt" &&
      originMeta.councilPhase !== "moderation" &&
      originMeta.councilSeat
    ) {
      const seatIndex = originMeta.councilSeat.index;
      const title = originMeta.title ?? "";
      const status = seatStatusFromTitle(title);
      const fullText = receiptFullText(unit);
      const extracted = extractCouncilVerdict(fullText);
      const body = extracted?.body ?? fullText;
      const agentName = seatAgentNameFromTitle(title);

      const existing = draft.seats.get(seatIndex);
      // 回执按到达顺序覆盖：后到的回执是这一席的最近事实（重试/第二轮都成立）。
      // 名字只在这次解出时覆盖（失败回执解不出名时不抹掉已有的）。
      draft.seats.set(seatIndex, {
        index: seatIndex,
        agentName: agentName || existing?.agentName || "",
        lens: originMeta.councilSeat.lens,
        round,
        status,
        ...(status === "completed" && body ? { statement: body } : {}),
        ...(extracted ? { verdict: extracted.verdict } : {}),
        unitKey: unit.key,
      });
      draft.flow.push({
        key: `${unit.key}:seat${seatIndex}`,
        agentName: agentName || existing?.agentName || "",
        round,
        type: round === 1 ? "statement" : "cross",
        status,
        text: status === "completed" ? body : "",
        ...(extracted ? { verdict: extracted.verdict } : {}),
        unitKey: unit.key,
      });
    }
  }

  const models: CouncilMeetingModel[] = [];
  for (const [id, draft] of drafts) {
    models.push(buildCouncilMeetingModel(id, draft));
  }
  return models;
}

function buildCouncilMeetingModel(
  councilId: string,
  draft: MeetingDraft,
): CouncilMeetingModel {
  const seats = [...draft.seats.values()].sort((a, b) => a.index - b.index);
  const round = draft.round;
  // 最新一轮票面：每席一票——席位模型存的是最近一次回执，但其轮次可能早于全场
  // 最新轮（该席在最新轮还没发言），计票只认 round 一致的席位。
  const latestRoundSeats = seats.filter((seat) => seat.round === round);
  const verdicts = latestRoundSeats.filter((seat) => seat.verdict);
  const tally: CouncilMeetingTally = {
    approve: 0,
    reject: 0,
    abstain: 0,
    vetoes: 0,
    voted: 0,
  };
  for (const seat of verdicts) {
    const verdict = seat.verdict;
    if (!verdict) continue;
    tally.voted += 1;
    if (verdict.stance === "approve") tally.approve += 1;
    else if (verdict.stance === "reject") tally.reject += 1;
    else tally.abstain += 1;
    if (verdict.veto) tally.vetoes += 1;
  }
  // 交齐 = 全部已知席位都在最新一轮交了有效票；没交齐绝不假报终局。
  const votesComplete = seats.length > 0 && verdicts.length === seats.length;
  const status: CouncilMeetingViewStatus = !votesComplete
    ? "running"
    : (() => {
        const outcome = resolveCouncilViewDecision(tally, round);
        return outcome === "deliberate" ? "running" : outcome;
      })();

  const toEntry = (seat: CouncilMeetingSeatModel): CouncilStanceEntry => {
    const verdict = seat.verdict;
    const statement = seat.statement ?? "";
    return {
      seatIndex: seat.index,
      agentName: seat.agentName,
      lens: seat.lens,
      stance: verdict?.stance ?? "abstain",
      confidence: verdict?.confidence ?? "low",
      veto: verdict?.veto ?? false,
      snippet: snippetOf(statement),
      statement,
      unitKey: seat.unitKey,
    };
  };
  const consensus = verdicts
    .filter((seat) => seat.verdict?.stance === "approve")
    .map(toEntry);
  const disagreements = verdicts
    .filter((seat) => seat.verdict && seat.verdict.stance !== "approve")
    .map(toEntry);

  // 最新一轮没交票的席位：失败/被中断的回执、新一轮还没轮到他、或没有有效裁定
  // 行（散文不算票）。判据 = 没有最新一轮的有效票。
  const pending: CouncilPendingSeat[] = seats
    .filter((seat) => !(seat.round === round && seat.verdict))
    .map((seat) => ({
      seatIndex: seat.index,
      agentName: seat.agentName,
      lens: seat.lens,
      reason:
        seat.status === "failed"
          ? ("receiptFailed" as const)
          : seat.status === "cancelled"
            ? ("receiptCancelled" as const)
            : seat.round < round
              ? ("awaitingRound" as const)
              : ("noVerdictLine" as const),
      unitKey: seat.unitKey,
    }));

  // 盲区：攻角分了座但从未有效发言（从未 completed 过——第二轮没跟上但首轮说过
  // 话的角度已经听到了，不算盲区）。
  const blindSpots: CouncilBlindSpot[] = [];
  for (const seat of seats) {
    if (seat.status === "completed") continue;
    const spot = blindSpots.find((candidate) => candidate.lens === seat.lens);
    if (spot) {
      if (seat.agentName && !spot.agentNames.includes(seat.agentName)) {
        spot.agentNames.push(seat.agentName);
      }
    } else {
      blindSpots.push({
        lens: seat.lens,
        agentNames: seat.agentName ? [seat.agentName] : [],
      });
    }
  }

  return {
    councilId,
    ...(draft.kind ? { kind: draft.kind } : {}),
    round,
    status,
    seats,
    tally,
    votesComplete,
    consensus,
    disagreements,
    pending,
    blindSpots,
    flow: mergeCouncilFlowEntries(draft.flow),
    hostUnitKey: draft.hostUnitKey,
  };
}

/** 过程流的同人同轮短发言自动合并：相邻 + 同人 + 同轮 + 每段都短 → 拼成一条（裁定取最后）。 */
function mergeCouncilFlowEntries(
  entries: CouncilFlowEntry[],
): CouncilFlowEntry[] {
  const merged: CouncilFlowEntry[] = [];
  for (const entry of entries) {
    const previous = merged.at(-1);
    const canMerge =
      previous &&
      previous.status === "completed" &&
      entry.status === "completed" &&
      previous.agentName === entry.agentName &&
      previous.round === entry.round &&
      previous.text.length <= COUNCIL_MERGE_MAX_CHARS &&
      entry.text.length <= COUNCIL_MERGE_MAX_CHARS;
    if (previous && canMerge) {
      previous.text = `${previous.text}\n\n${entry.text}`;
      if (entry.verdict) previous.verdict = entry.verdict;
      previous.unitKey = entry.unitKey;
      continue;
    }
    merged.push({ ...entry });
  }
  return merged;
}

/** 每轮的挂载信息（Timeline 算一次）：圆桌卡挂在与会议相关的最新证据轮上。 */
export function selectCouncilMeetingRenderInfo(
  units: readonly CouncilEvidenceUnit[],
): Map<string, CouncilMeetingModel> {
  const byHostUnitKey = new Map<string, CouncilMeetingModel>();
  for (const meeting of selectCouncilMeetings(units)) {
    byHostUnitKey.set(meeting.hostUnitKey, meeting);
  }
  return byHostUnitKey;
}
