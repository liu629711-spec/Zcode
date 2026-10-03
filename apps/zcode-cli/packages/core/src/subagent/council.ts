// ============================================================
// 圆桌会（真会议）内核协议：对立面席位、裁定行、代码算票、决议判定。
// 与批次质检的"合议轮"（单模型扮一圈）不同：真会议的每个席位都派回员工
// 自己的会话与模型（经派单管线独立思考），主席只组织程序、不代笔结论；
// 票由代码从裁定行解析——散文里的"我同意"永不计票（弱模型兜底）。
// ============================================================

/** 裁定行三态结论（学 GitHub PR 评审：只有打回阻断；弃权不阻断但压死通过）。 */
export type CouncilStance = "approve" | "reject" | "abstain";

/** 信心度（学术同行评审：低信心评审系统性打高分——必须显式留痕给老板看）。 */
export type CouncilConfidence = "high" | "medium" | "low";

export interface CouncilVerdict {
  stance: CouncilStance;
  confidence: CouncilConfidence;
  /** 一票否决：终轮出现即否决整案，不按票数稀释。 */
  veto: boolean;
}

export interface CouncilTally {
  total: number;
  approve: number;
  reject: number;
  abstain: number;
  vetoes: number;
}

export type CouncilOutcome = "approved" | "rejected" | "deliberate" | "deadlocked";

/**
 * 对立面席位清单（按攻角分工，覆盖单个智能体看不见的关联面）。顺序即优先级：
 * 席位比清单少时从头部取（老板的班底常为 4 人 → 影响面/需求本意/边界/成本），
 * 席位多则轮转复用攻角——同人不同座仍因个人记忆与模型差异而异。
 */
export const COUNCIL_SEAT_LENSES = [
  {
    id: "impact",
    label: "影响面",
    brief:
      "你专门攻影响面：这个方案会牵连、破坏或波及哪些别的模块、调用方、数据与既有行为？逐一点名被牵连处，不许空谈。",
  },
  {
    id: "requirement",
    label: "需求本意",
    brief:
      "你专门守需求本意：老板要的到底是什么？方案有没有理解偏、做多了、做偏了？用方案原文对照需求原话。",
  },
  {
    id: "edge",
    label: "边界与异常",
    brief:
      "你专门攻边界与异常：极端输入、失败路径、并发、回滚、迁移。挑最疼的三个边界说透。",
  },
  {
    id: "cost",
    label: "成本与复杂度",
    brief:
      "你专门攻成本：token、工期、维护负担。有没有更便宜能达到八成效果的替代方案？",
  },
  {
    id: "minimal",
    label: "最小改动",
    brief:
      "你专门攻最小改动：哪些部分可以不做、可以砍掉、属于过度设计？砍到不能再砍还成立吗？",
  },
] as const;

export type CouncilLens = (typeof COUNCIL_SEAT_LENSES)[number];

export interface CouncilSeat {
  index: number;
  /** 员工真名（派单对象）。 */
  agentName: string;
  lens: CouncilLens;
}

/**
 * 席位分配：员工按给定顺序对攻角轮转，纯函数、确定性——同样的名单铸出同样的
 * 座次（测试可对账，重开会议不换座）。
 */
export function buildCouncilSeatPlan(employees: readonly string[]): CouncilSeat[] {
  const names = employees
    .map((name) => name.trim())
    .filter((name) => name.length > 0);
  if (names.length === 0) return [];
  const lensCount = COUNCIL_SEAT_LENSES.length;
  return names.map((agentName, index) => ({
    index,
    agentName,
    lens: COUNCIL_SEAT_LENSES[index % lensCount],
  }));
}

/**
 * 裁定行指令（压轴钉死进信封）：弱模型对开头/结尾权重最高，中间的要求等于没写；
 * 所以裁定行格式单独成块、要求"一字不改"，并明示缺行会被退回重交。
 */
export const COUNCIL_VERDICT_INSTRUCTION = [
  "发言的最后必须以一行裁定收尾，格式严格如下（除立场/信心/否决三个词外一字不改）：",
  "裁定：通过｜信心：高｜否决：否",
  "三个槽位的合法取值：立场=通过/打回/弃权；信心=高/中/低；否决=是/否。",
  "没有这一行或格式不符，主持人会把你整份发言退回重交，前面的讨论全部白费。",
].join("\n");

/**
 * 席位工单的首行（开头钉死，与裁定行的压轴钉死同一经验）：弱模型对首行权重最高，
 * 先一句话钉死「这是圆桌会发言、直接进议题」，压过工单家族的寒暄/角色扮演惯性。
 * 载体（buildWorkOrderEnvelopeText 的 councilId 分支）把它排在 <work-order> 之前。
 */
export const COUNCIL_SEAT_FIRST_LINE = "圆桌会发言：直接进入议题，不许寒暄不确认。";

/**
 * 从一段发言里解析裁定行：全文取**最后一个**匹配（信封钉死裁定行在结尾，
 * 中途改主意以后说的为准）。缺行或槽位越界 → undefined，由调用方退回重询。
 * 容忍全角/半角冒号与分隔符、多余空格；立场词之外的变体（"同意"等）一律不认
 * ——格式纪律靠信封钉死，解析端放松就会把散文当票。
 */
export function parseCouncilVerdictLine(text: string): CouncilVerdict | undefined {
  const pattern =
    /裁定\s*[:：]\s*(通过|打回|弃权)\s*[|｜]\s*信心\s*[:：]\s*(高|中|低)\s*[|｜]\s*否决\s*[:：]\s*(是|否)/g;
  let match: RegExpExecArray | null;
  let last: RegExpExecArray | null = null;
  while ((match = pattern.exec(text)) !== null) last = match;
  if (!last) return undefined;
  return {
    stance: last[1] === "通过" ? "approve" : last[1] === "打回" ? "reject" : "abstain",
    confidence: last[2] === "高" ? "high" : last[2] === "中" ? "medium" : "low",
    veto: last[3] === "是",
  };
}

export function countCouncilVotes(verdicts: readonly CouncilVerdict[]): CouncilTally {
  const tally: CouncilTally = { total: 0, approve: 0, reject: 0, abstain: 0, vetoes: 0 };
  for (const verdict of verdicts) {
    tally.total += 1;
    if (verdict.stance === "approve") tally.approve += 1;
    else if (verdict.stance === "reject") tally.reject += 1;
    else tally.abstain += 1;
    if (verdict.veto) tally.vetoes += 1;
  }
  return tally;
}

/**
 * 决议判定（代码算票，主席无权重、不许改票）：
 * - 首轮：全票同一立场 → 直接出决议（全票通过早退、全票打回也早退，省 token）；
 *   否则进入第二轮质询+再表决。
 * - 终轮（第 2 轮封顶）按**全体席位过半**判定（真会议规矩：过半才通过，相对多数
 *   不算数）：任一否决 → 否决；同意过半 → 通过；打回过半 → 否决；其余（平票、
 *   弃权拖垮、无过半方）→ 分歧未决，交老板裁决。
 * 二轮上限与透明上报是终结护栏（防两个评审员来回互改到天荒地老）；
 * "分歧未决"如实上抛而不是折中放行——分歧模式是多席位最值钱的信号，不许被均值洗掉。
 */
export function resolveCouncilDecision(
  tally: CouncilTally,
  round: 1 | 2,
): { outcome: CouncilOutcome; nextRound?: 2 } {
  if (round === 1) {
    if (tally.total === 0) return { outcome: "deliberate", nextRound: 2 };
    if (tally.approve === tally.total && tally.vetoes === 0 && tally.abstain === 0) {
      return { outcome: "approved" };
    }
    if (tally.reject === tally.total) return { outcome: "rejected" };
    return { outcome: "deliberate", nextRound: 2 };
  }
  if (tally.vetoes > 0) return { outcome: "rejected" };
  if (tally.approve * 2 > tally.total) return { outcome: "approved" };
  if (tally.reject * 2 > tally.total) return { outcome: "rejected" };
  return { outcome: "deadlocked" };
}
