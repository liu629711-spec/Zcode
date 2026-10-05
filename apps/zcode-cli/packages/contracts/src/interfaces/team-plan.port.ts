// ============================================================
// Team Plan Port - 排班草案端口（团队看板批3b）
// ============================================================
// 草案 = 队长会话台账的一行（agentWorkOrderTeamPlan，upsert 幂等）。create 由
// 队长的 TeamPlan 工具调；update/approve/discard 由看板命令调（编辑器在面板）。
// approve 逐单走既有派单端口（频控三道闸兜底），依赖持派自动生效。

import type { TeamPlanError, TeamPlanTask } from "@zcode/shared/zcode-protocol-v4";

export interface TeamPlanCreateInput {
  title: string;
  tasks: TeamPlanTask[];
  sourceSessionId: string;
}

export interface TeamPlanUpdateInput {
  planId: string;
  tasks: TeamPlanTask[];
  sourceSessionId: string;
}

export interface TeamPlanDecisionInput {
  planId: string;
  sourceSessionId: string;
}

export interface TeamPlanApproveResult {
  planId: string;
  batchId?: string;
  dispatched: number;
  failed: TeamPlanError[];
}

export interface TeamPlanPort {
  create(input: TeamPlanCreateInput): Promise<{ planId: string; taskCount: number }>;
  update(input: TeamPlanUpdateInput): Promise<{ planId: string }>;
  approve(input: TeamPlanDecisionInput): Promise<TeamPlanApproveResult>;
  discard(input: TeamPlanDecisionInput): Promise<{ planId: string }>;
}
