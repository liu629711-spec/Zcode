// ============================================================
// Agent Dispatch Port - cross-session work order boundary (D29/D2)
// ============================================================

import type {
  CouncilMeetingKind,
  CouncilMeetingRound,
  CouncilSeatLensId,
} from "./council.js";

/**
 * 派单的 inputId/queryId 统一前缀：工单唤醒轮的身份信号之一。
 * core turn-loop（denylist 兜底）与协议端口（自检）都认它，
 * 与 automation- / offpeak- 前缀同一执法家族。
 */
export const WORK_ORDER_INPUT_ID_PREFIX = "workorder-";

/** 工单信封：随工单正文落库 + 提交入参携带（谁派的单、派给谁的会话、干什么）。 */
export interface AgentWorkOrderEnvelope {
  /** 本次工单身份（uuid）；目标唤醒轮的 inputId 以 `workorder-<id>` 派生。 */
  workOrderId: string;
  /** 发起方智能体工号；发起方不是驻场智能体（用户会话直接派单）时缺席。 */
  fromAgentId?: string;
  /** 发起方档案名；用户会话直接派单时为空串占位。 */
  fromAgentName: string;
  /** 发起方会话 id（回执的归还地址）。 */
  fromSessionId: string;
  /** 任务正文（与工单 carrier 的 text 相同，元数据里再带一份供 UI 卡片用）。 */
  task: string;
  /**
   * 批次身份（uuid，缺席 = 散单）：同批多张工单在发起方会话聚合为一张工地卡。
   * 跨派单调用稳定——模型回传上次结果里的 batchId 即并入同批，缺席才新铸。
   */
  batchId?: string;
  /** 批次人类短标题（工地卡标题）；与 batchId 同进退。 */
  batchTitle?: string;
  /**
   * 评审单标记（评审会批，2026-10-02）：review=true 的工单是评审单——评审人只
   * 评审不动手，工单正文随评审要求块；同批全为评审单时批次收口触发合议轮而非
   * 质检轮。缺席 = 普通施工单。
   */
  review?: boolean;
  /**
   * 圆桌会席位单（真会议，2026-10-03）：本单是某场圆桌会某轮的一个席位。与
   * batchId 互斥使用（席位单按 councilId 聚合，不进工地卡）；缺席 = 普通工单。
   * councilId 是全场分组键，回执轮头 originMeta.council* 与它对号。
   */
  councilId?: string;
  /** 会议类型（plan=方案评审 / acceptance=验收）；与 councilId 同进退。 */
  councilKind?: CouncilMeetingKind;
  /** 本单所属审议轮次（内核封顶 2）；与 councilId 同进退。 */
  councilRound?: CouncilMeetingRound;
  /** 本单席位座次（buildCouncilSeatPlan 的 index 原样随行）。 */
  councilSeatIndex?: number;
  /** 本单席位攻角；回执据此把发言归位到圆桌卡的对应席位。 */
  councilSeatLens?: CouncilSeatLensId;
}

export interface AgentDispatchRequest {
  /**
   * 目标档案：agentId（uuid）优先，其次精确名（仅限本项目作用域）。歧义即拒绝。
   * 缺省 = 不点名员工：必须配合 newSession=true，派生一段【无 persona 的普通会话】
   * （无工牌/无档案/无记忆，对齐稿 §三 #2/#5），task 作为它的第一条工单；
   * 缺 agent 且 newSession!==true 时端口拒绝。
   */
  agent?: string;
  task: string;
  /** 新会话的人类短标题（会话列表展示用）；仅在新建会话时作为首输入标题种子。 */
  title?: string;
  /** true 强制开新会话：带 agent 开员工新段；缺 agent 开无 persona 普通会话。默认投目标最新一段 persona 会话（无则自动开）。 */
  newSession?: boolean;
  /**
   * 本单指定模型（发起方目录里解析出的注册表拼写）。缺席 = 用目标智能体自己的模型；
   * 只作用于工单这一轮，不改写目标会话的常驻选择（闲时任务同款语义）。
   */
  modelSelection?: {
    providerId: string;
    modelId: string;
    options?: { reasoningLevel?: string };
  };
  /** 发起方会话（信封的 fromSessionId 由执行器注入，不来自模型输入）。 */
  sourceSessionId: string;
  /** 批次标题（工具输入 batch_title 逐字透传）；缺席 = 散单。 */
  batchTitle?: string;
  /** 批次身份（工具输入 batch_id 回传）；有 batchTitle 缺它时端口新铸。 */
  batchId?: string;
  /** 评审单标记（工具输入 review 逐字透传）；缺席 = 普通施工单。 */
  review?: boolean;
  /**
   * 圆桌会席位单（真会议）：由召集方**代码**在用户批准后逐席直派（CouncilConvene
   * 工具路径不把会议编排交给模型）——席位解析、round 推进与收票都在代码侧，模型
   * 面不暴露这些参数。信封的 council* 字段逐字来自这里。
   */
  councilId?: string;
  councilKind?: CouncilMeetingKind;
  councilRound?: CouncilMeetingRound;
  councilSeatIndex?: number;
  councilSeatLens?: CouncilSeatLensId;
  /** 发起方档案名/工号（信封署名；执行器注入）。 */
  sourceAgentId?: string;
  sourceAgentName?: string;
}

export interface AgentDispatchResult {
  targetSessionId: string;
  /** 目标档案名；无 persona 普通会话（agent 缺省派生）为空串占位。 */
  agentName: string;
  agentId?: string;
  /** started = 已开跑（或已入队即刻可跑）；queued = 目标忙，已按既有消息排队。投递成功 ≠ 已处理。 */
  delivery: "started" | "queued";
  /** 目标会话是本次新建的（员工新段或无 persona 普通会话）。 */
  createdSession: boolean;
  /** 本单实际使用的模型 modelId；指定模型时必在。 */
  model?: string;
  /** 本单工单 id（回执对账键）；结果里回显，供派单方把后续工单并进同批/对账。 */
  workOrderId?: string;
  /** 批次身份回显（本次派单带批次时必在）：模型回传它即可往同一工地续派。 */
  batchId?: string;
  /** 批次标题回显（与 batchId 同进退）。 */
  batchTitle?: string;
  /** 圆桌会身份回显（席位单必在）：召集方代码据它对账收票进度。 */
  councilId?: string;
}

/**
 * 跨会话派单端口：core 的 AgentDispatch 工具经它把工单投进同 workspace
 * 目标智能体的常驻会话。协议层实现（bootstrap），宿主不跨界；嵌套上限=1
 * 由工单轮 denylist + handler 终审 + 端口自检三重执法，不靠提示词。
 */
export interface AgentDispatchPort {
  dispatch(input: AgentDispatchRequest): Promise<AgentDispatchResult>;
}
