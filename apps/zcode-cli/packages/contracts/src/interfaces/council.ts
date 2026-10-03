// ============================================================
// 圆桌会（真会议）线级契约：会议类型、状态机、席位、轮次与阶段。
// ============================================================
// 内核协议（攻角清单、裁定行解析、代码算票、决议判定）在 core
// （core/src/subagent/council.ts），本文件只定跨包流动的形状：contracts 不依赖
// core，攻角 id 在此独立成闭集，与内核 COUNCIL_SEAT_LENSES 的 id 手工同步
// （qcKind 同一纪律）——内核增删攻角时这里漏改，派单信封与事件载荷里的攻角
// 会在 zod 边界被静默剥掉（shared 的 schema 同步见
// packages/shared/src/zcode-protocol-v4/workflow-row-meta.ts）。

/** 席位攻角 id（与 core COUNCIL_SEAT_LENSES[].id 同闭集、同顺序；内核是权威）。 */
export const COUNCIL_SEAT_LENS_IDS = [
  "impact",
  "requirement",
  "edge",
  "cost",
  "minimal",
] as const;
export type CouncilSeatLensId = (typeof COUNCIL_SEAT_LENS_IDS)[number];

/** 会议类型：plan=开工前方案评审（定方向定内容）；acceptance=完工后验收。 */
export type CouncilMeetingKind = "plan" | "acceptance";

/**
 * 会议状态机：proposed（老板/主智能体已提议，等用户批准）→ running（已批准，
 * 席位派单与发言进行中）→ approved / rejected / deadlocked（决议三终态，
 * 判定由内核 resolveCouncilDecision 给出）；用户撤回 → cancelled
 * （proposed/running 可入）。
 */
export type CouncilMeetingStatus =
  | "proposed"
  | "running"
  | "approved"
  | "rejected"
  | "deadlocked"
  | "cancelled";

/**
 * 运行阶段（status=running 才推进）：
 * - seating：席位派单中（批准后向各员工会话投席位工单）；
 * - deliberation：席位发言/质询进行中（第几轮见 round）；
 * - tally：收票算票（解析裁定行、代码算票，散文不计票）；
 * - moderation：主席整理决议（发起方会话的合议轮，禁派单）。
 */
export type CouncilMeetingPhase = "seating" | "deliberation" | "tally" | "moderation";

/** 审议轮次（内核封顶 2：首轮非全票才开质询轮）。 */
export type CouncilMeetingRound = 1 | 2;

/**
 * 决议终态闭集（主席合议轮随 originMeta.councilOutcome 权威下发）：内核
 * resolveCouncilDecision 的终局三态；"deliberate" 不是终局（转第 2 轮），不下发。
 */
export type CouncilMeetingOutcome = "approved" | "rejected" | "deadlocked";

/** 圆桌会的一个席位：员工真名（派单对象）+ 分到的攻角。 */
export interface CouncilMeetingSeat {
  /** 座次（0 起）：buildCouncilSeatPlan 的 index 原样随行，重开会议不换座。 */
  index: number;
  /** 员工真名（派单目标；目标解析与 AgentDispatch 同口径）。 */
  agentName: string;
  /** 本席攻角。 */
  lens: CouncilSeatLensId;
}

/**
 * 会议记录（台账/事件载荷里会议级信息的权威形状）：councilId 是全场唯一分组键
 * ——席位工单、席位回执、主席合议轮都靠它聚拢；单轮收票按 `councilId:round`
 * 分组（回执轮 originMeta.councilId + councilRound 对号）。
 */
export interface CouncilMeetingState {
  /** 会议身份（uuid）。 */
  councilId: string;
  kind: CouncilMeetingKind;
  status: CouncilMeetingStatus;
  /** 当前轮次（status=running 才推进；终态后保留最后一轮）。 */
  round: CouncilMeetingRound;
  phase: CouncilMeetingPhase;
  /** 座次表（批准时由内核 buildCouncilSeatPlan 铸出，确定性）。 */
  seats: readonly CouncilMeetingSeat[];
  /** 议题：老板要审的方案/成果（提议原话或摘要）。 */
  motion?: string;
  /** 召集方会话（圆桌卡挂载地；席位回执与合议轮的归还地址）。 */
  convenedBySessionId?: string;
}
