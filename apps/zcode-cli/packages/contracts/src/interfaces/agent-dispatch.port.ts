// ============================================================
// Agent Dispatch Port - cross-session work order boundary (D29/D2)
// ============================================================

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
}

export interface AgentDispatchRequest {
  /** 目标档案：agentId（uuid）优先，其次精确名（仅限本项目作用域）。歧义即拒绝。 */
  agent: string;
  task: string;
  /** true 强制开新 persona 会话；默认投目标最新一段 persona 会话（无则自动开）。 */
  newSession?: boolean;
  /** 发起方会话（信封的 fromSessionId 由执行器注入，不来自模型输入）。 */
  sourceSessionId: string;
  /** 发起方档案名/工号（信封署名；执行器注入）。 */
  sourceAgentId?: string;
  sourceAgentName?: string;
}

export interface AgentDispatchResult {
  targetSessionId: string;
  agentName: string;
  agentId?: string;
  /** started = 已开跑（或已入队即刻可跑）；queued = 目标忙，已按既有消息排队。投递成功 ≠ 已处理。 */
  delivery: "started" | "queued";
  /** 目标 persona 会话是本次新建的。 */
  createdSession: boolean;
}

/**
 * 跨会话派单端口：core 的 AgentDispatch 工具经它把工单投进同 workspace
 * 目标智能体的常驻会话。协议层实现（bootstrap），宿主不跨界；嵌套上限=1
 * 由工单轮 denylist + handler 终审 + 端口自检三重执法，不靠提示词。
 */
export interface AgentDispatchPort {
  dispatch(input: AgentDispatchRequest): Promise<AgentDispatchResult>;
}
