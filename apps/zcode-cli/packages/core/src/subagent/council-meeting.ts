// ============================================================
// 圆桌会（真会议）编排纯规则：席位信封构造、匿名化、台账行 id、票决明细形状。
// 内核协议（攻角/裁定行/算票/决议判定）在 council.ts；本文件只做编排侧的
// 纯函数与形状——轮次推进/收票/重询的状态机在 runtime/methods/council-meeting.ts。
// 所有信封构造都是确定性的：同样的会议状态铸出同样的正文（测试可对账）。
// ============================================================

import type {
  CouncilMeetingKind,
  CouncilMeetingRound,
  CouncilMeetingState,
  CouncilSeatLensId,
} from "@zcode/contracts";
import {
  COUNCIL_SEAT_LENSES,
  type CouncilConfidence,
  type CouncilOutcome,
  type CouncilStance,
  type CouncilTally,
} from "./council.js";
import { neutralizeEnvelopeTags } from "./work-order.js";

// ── 台账行 id（会话命名空间，与 batchQcLedgerId 同纪律）────────────────

/** 会议记录行（kind "councilMeeting"，payload 见 CouncilMeetingRecordPayload）。 */
export function councilMeetingLedgerId(sessionId: string, councilId: string): string {
  return `councilMeeting:${sessionId}:${councilId}`;
}

/** 单轮收票决议行（kind "councilRound"）：在册 = 该轮已收票，绝不二算。 */
export function councilRoundLedgerId(sessionId: string, councilId: string, round: 1 | 2): string {
  return `councilRound:${sessionId}:${councilId}:${round}`;
}

/** 席位裁定行补交行（kind "councilSeatRequery"）：在册 = 该席该轮已重询过一次。 */
export function councilSeatRequeryLedgerId(
  sessionId: string,
  councilId: string,
  round: 1 | 2,
  seatIndex: number,
): string {
  return `councilSeatRequery:${sessionId}:${councilId}:${round}:${seatIndex}`;
}

// ── 会议记录 payload 形状 ───────────────────────────────────────────

/** 主持人插话：注入下一轮席位信封的材料；点名座位只进被点席位。 */
export interface CouncilMeetingInterjection {
  id: string;
  text: string;
  /** 点名座位（0 起，buildCouncilSeatPlan 的 index）；缺席 = 全体席位可见。 */
  targetSeatIndexes?: readonly number[];
}

/** 派单失败的席位（按缺席弃权如实标注；不重试——重派是老板/模型的新决定）。 */
export interface CouncilMeetingFailedDispatch {
  round: 1 | 2;
  seatIndex: number;
  error?: string;
}

/**
 * 会议台账行 payload（core 私有形状；跨包流动的会议级信息以 contracts 的
 * CouncilMeetingState 为权威，这里只是它的载体 + 编排侧附件）。
 */
export interface CouncilMeetingRecordPayload {
  council: CouncilMeetingState;
  /** 会议短标题（轮头卡/主席轮标题；提议时给）。 */
  title?: string;
  /** 提议时附带的材料（随每轮席位信封下发）。 */
  materials?: string;
  /** true = 会议暂停：冻结下一轮派单与推进，进行中的席位发言自然跑完。 */
  paused?: boolean;
  interjections?: readonly CouncilMeetingInterjection[];
  failedDispatches?: readonly CouncilMeetingFailedDispatch[];
}

/** 单席一票（决议落库明细；散文不算票——source 记录票从哪来）。 */
export interface CouncilVoteRecord {
  index: number;
  agentName: string;
  lens: CouncilSeatLensId;
  stance: CouncilStance;
  confidence?: CouncilConfidence;
  veto: boolean;
  /**
   * 票的来源：verdict_line=发言自带裁定行；requery_verdict_line=补交取得；
   * missing_verdict=缺行且重询一次仍缺（按弃权）；no_response=该席交活失败/被中断
   * （按缺席弃权）；dispatch_failed=该席派单失败（按缺席弃权）。
   */
  source:
    | "verdict_line"
    | "requery_verdict_line"
    | "missing_verdict"
    | "no_response"
    | "dispatch_failed";
  /** 本席发言（或缺席事实）对应的工单 id（发言索引；缺席席缺席）。 */
  workOrderId?: string;
}

/** 一个席位某轮的发言（completed 回执的 response 原文）。 */
export interface CouncilSeatStatement {
  index: number;
  statement: string;
}

// ── 标签与匿名化 ────────────────────────────────────────────────────

/** 席位匿名标签：按座次字母编号（评审A/B/C…），真实姓名不出现在跨席材料里。 */
export function councilSeatLabel(index: number): string {
  return `评审${String.fromCharCode(65 + (index % 26))}`;
}

/**
 * 匿名化一轮发言（跨席材料用）：按座次定序、字母编号、工单家族标签中和、
 * 姓名不随行——第 2 轮的质询对象只能是「评审B」，不是真人名（去身份去从众）。
 */
export function anonymizeCouncilStatements(
  statements: readonly CouncilSeatStatement[],
): { label: string; statement: string }[] {
  return [...statements]
    .sort((a, b) => a.index - b.index)
    .map((s) => ({
      label: councilSeatLabel(s.index),
      statement: neutralizeEnvelopeTags(s.statement).trim(),
    }));
}

function lensById(lensId: CouncilSeatLensId) {
  const lens = COUNCIL_SEAT_LENSES.find((candidate) => candidate.id === lensId);
  if (!lens) {
    throw new Error(`Unknown council seat lens: ${lensId}`);
  }
  return lens;
}

function meetingKindLabel(kind: CouncilMeetingKind): string {
  return kind === "plan" ? "方案评审（开工前：定方向定内容）" : "验收（完工后：对照需求验收成果）";
}

// ── 席位信封构造 ────────────────────────────────────────────────────

/** 单条任务/材料的正文上界：与批次质检清单同量级（信封塞得下、上下文不爆）。 */
const COUNCIL_MATERIAL_MAX_CHARS = 8_192;

function clipMaterial(text: string): string {
  const normalized = neutralizeEnvelopeTags(text).trim();
  return normalized.length > COUNCIL_MATERIAL_MAX_CHARS
    ? `${normalized.slice(0, COUNCIL_MATERIAL_MAX_CHARS - 1)}…`
    : normalized;
}

/**
 * 席位任务正文（<work-order> 标签内的 task）。首行（圆桌会定调）与压轴（裁定行）
 * 都由载体 buildWorkOrderEnvelopeText 的 councilId 分支钉死，这里只造中段：
 * 议题 + 会议类型 + 该席攻角 brief（内核原文）+ 材料 + 发言要求；
 * 第 2 轮附匿名质询材料与抗从众条款；插话点名谁就进谁的信封。
 */
export function buildCouncilSeatTaskText(input: {
  kind: CouncilMeetingKind;
  round: CouncilMeetingRound;
  seatIndex: number;
  lens: CouncilSeatLensId;
  seatCount: number;
  motion: string;
  materials?: string;
  /** 第 2 轮质询材料：其他席位第 1 轮发言的匿名版（本人发言也在列，便于对照）。 */
  peerStatements?: readonly { label: string; statement: string }[];
  /** 本席可见的主持人插话（未点名的全体可见；点名的只进被点席位并标注）。 */
  interjections?: readonly { text: string; targeted: boolean }[];
}): string {
  const lens = lensById(input.lens);
  const lines = [
    `【议题】${clipMaterial(input.motion)}`,
    `【会议类型】${meetingKindLabel(input.kind)}`,
    `【你的席位】第 ${input.seatIndex + 1} 席 / 共 ${input.seatCount} 席 · 攻角「${lens.label}」`,
    lens.brief,
  ];
  if (input.materials?.trim()) {
    lines.push("【材料】", clipMaterial(input.materials));
  }
  if (input.peerStatements?.length) {
    lines.push(
      "【其他席位发言（匿名，按座次编号）】",
      ...input.peerStatements.map(
        (peer) => `${peer.label}：${clipMaterial(peer.statement)}`,
      ),
    );
  }
  if (input.interjections?.length) {
    lines.push(
      "【主持人插话】",
      ...input.interjections.map(
        (item) => `${item.targeted ? "（点名你）" : ""}主持人：${clipMaterial(item.text)}`,
      ),
    );
  }
  if (input.round === 2) {
    lines.push(
      "【质询要求】",
      "1. 点名表态：先写明你最认同的评审（如 评审B）与最反对的评审（如 评审A），并各给至少一条理由（可引用对方原话）。",
      // 抗从众条款（同行评审纪律）：改立场必须有自我否定的具体理由，不许随风倒。
      "2. 抗从众：改立场必须点名你自己第 1 轮论证的具体缺陷（哪一条站不住、为什么）；仅因别人反对就改票视为从众，无效。",
      "3. 插话回应：若上方有主持人插话且点名了你，必须先回应插话再继续质询。",
    );
  } else {
    lines.push(
      "【发言要求】",
      "1. 独立发言：只从你的攻角说话，不猜测、不迎合其他席位。",
      "2. 讲具体：点名具体模块/接口/需求原话/边界场景/成本数字，不写空话套话。",
      "3. 篇幅克制：把最要紧的 2-4 个点说透即可。",
    );
  }
  return lines.join("\n");
}

/**
 * 裁定行补交（重询一次）的任务正文：指名只补裁定行——不重写发言、不寒暄。
 * 压轴的裁定行格式指令仍由载体统一钉死（同一份 COUNCIL_VERDICT_INSTRUCTION）。
 */
export function buildCouncilRequeryTaskText(input: {
  round: CouncilMeetingRound;
  seatIndex: number;
}): string {
  return [
    `【裁定行补交】你在第 ${input.round} 轮圆桌会的发言已收到，但结尾缺少格式合规的裁定行（缺行或格式不符）。`,
    "现在只补交裁定：只回复那一行裁定，不要重写发言，不要补充解释或寒暄。",
  ].join("\n");
}

// ── 主席合成轮信封 ──────────────────────────────────────────────────

const STANCE_LABELS: Record<CouncilStance, string> = {
  approve: "通过",
  reject: "打回",
  abstain: "弃权",
};

const CONFIDENCE_LABELS: Record<CouncilConfidence, string> = {
  high: "高",
  medium: "中",
  low: "低",
};

const OUTCOME_LABELS: Record<CouncilOutcome, string> = {
  approved: "通过",
  rejected: "打回",
  deliberate: "进入下一轮",
  deadlocked: "分歧未决（交老板裁决）",
};

function voteSourceNote(source: CouncilVoteRecord["source"]): string {
  switch (source) {
    case "requery_verdict_line":
      return "（裁定行由补交取得）";
    case "missing_verdict":
      return "（缺裁定行，重询一次仍缺，按弃权计）";
    case "no_response":
      return "（该席交活失败或被中断，按缺席弃权计）";
    case "dispatch_failed":
      return "（该席派单失败，按缺席弃权计）";
    default:
      return "";
  }
}

/** 主席合成轮头卡标题（backgroundResult 轮头链按 originMeta.title 原样透传）。 */
export function buildCouncilMeetingTitle(
  kind: CouncilMeetingKind,
  title?: string,
): string {
  const short = title?.trim();
  if (short) return `圆桌会 · ${short}`;
  return kind === "acceptance" ? "圆桌会验收" : "圆桌会评审";
}

/**
 * 主席合成轮的任务正文（发起方会话的模型输入 carrier）。主席只组织程序：
 * 票数由代码算好并随信封下发（权威明细，不得改写）；分歧不许说成已解决；
 * 结论必须引用席位原话。发言以匿名标签呈现（与席位间质询同一套编号）。
 */
export function buildCouncilModerationTaskText(input: {
  kind: CouncilMeetingKind;
  motion: string;
  round: CouncilMeetingRound;
  outcome: CouncilOutcome;
  tally: CouncilTally;
  votes: readonly CouncilVoteRecord[];
  statements: readonly CouncilSeatStatement[];
}): string {
  const labelsByIndex = new Map(
    anonymizeCouncilStatements(input.statements).map((entry) => [entry.label, entry.statement]),
  );
  // 匿名标签按座次稳定映射：statements 缺席的席位仍要有标签行（裁定如实呈现）。
  const labelOf = (index: number): string => councilSeatLabel(index);
  const verdictLines = input.votes.map((vote) => {
    const confidence = vote.confidence ? CONFIDENCE_LABELS[vote.confidence] : "—";
    return `${labelOf(vote.index)}（${lensById(vote.lens).label}）：${STANCE_LABELS[vote.stance]}｜信心 ${confidence}｜否决 ${vote.veto ? "是" : "否"}${voteSourceNote(vote.source)}`;
  });
  const statementLines = [...input.statements]
    .sort((a, b) => a.index - b.index)
    .map((entry) => {
      const label = labelOf(entry.index);
      const known = labelsByIndex.get(label);
      return `${label}：${known ?? clipMaterial(entry.statement)}`;
    });
  const lines = [
    "【圆桌会决议整理】会议已由系统收票收口。你是主持人：只整理，不代笔结论、不改票。",
    `【议题】${clipMaterial(input.motion)}`,
    `【会议类型】${meetingKindLabel(input.kind)}`,
    `【审议轮次】第 ${input.round} 轮（系统封顶 2 轮）`,
    `【系统核算票数（权威，照抄不得改写）】${OUTCOME_LABELS[input.outcome]} —— 共 ${input.tally.total} 席：通过 ${input.tally.approve}、打回 ${input.tally.reject}、弃权 ${input.tally.abstain}、否决票 ${input.tally.vetoes}。`,
    "【各席裁定】",
    ...verdictLines,
    "【各席最终发言（匿名，按座次编号）】",
    ...statementLines,
    "【整理要求】",
    "1. 产出四段，用小标题：①共识 ②分歧 ③盲区 ④建议下一步。",
    "2. 引用原文：每段至少引用一位评审的原话短句（用「」括起并注明 评审X），不许凭空转述。",
    "3. 票数只准照抄上面的系统核算明细；你无权重、不得增改或重新解读票数。",
    // 分歧不许被均值洗掉（内核同一条纪律在合成端的落点）。
    "4. 分歧如实呈现：有分歧就是有分歧，不许写成「已达成一致」；打回/否决不许写成通过。",
    input.outcome === "deadlocked"
      ? "5. 建议下一步：给老板 2-3 个可选裁决方向（各一句话），注明各自的代价；不要替老板拍板。"
      : "5. 建议下一步：给一句可直接执行的下一步（通过→按方案开工/验收通过；打回→按席位指出的缺陷返工）。",
  ];
  return lines.join("\n");
}
