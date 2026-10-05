// 团队看板快照（团队看板批 2026-10-05，参照件 dsh-agent-teams 的原生落法）。
//
// 数据全来自发起会话的既有台账（session_input 的派单/回执/质检闸门行）+ 会话
// 注册表的工位在跑状态——不建新的状态存储（台账即真相源）。CLI 端聚合，UI 经
// v4 teamBoardState 命令拉取轮询。schema 是 UI 渲染的唯一契约：发射侧有界铸造
// （taskPreview 200 字符、dependsOn ≤16、teams/orders 上界防呆），zod 剥未知键。
import { z } from "zod";

/** 单张工单在看板上的状态：终态来自最新回执，没回执的都是"在途"。 */
export const teamBoardOrderStatusSchema = z.enum([
  "in_flight",
  "completed",
  "failed",
  "cancelled",
]);
export type TeamBoardOrderStatus = z.infer<typeof teamBoardOrderStatusSchema>;

export const teamBoardOrderSchema = z.object({
  workOrderId: z.string().min(1),
  /** 批内符号名（派单 task_key）；旧批次/散单缺席——没有它就不能被别人依赖。 */
  taskKey: z.string().min(1).max(128).optional(),
  /** 前置工单符号名（派单 depends_on 原样）。 */
  dependsOn: z.array(z.string().min(1).max(128)).max(16).optional(),
  agentName: z.string(),
  agentId: z.string().optional(),
  deskSessionId: z.string().optional(),
  /** 任务正文预览（发射侧截 200 字符；全文在工位会话与工地卡里）。 */
  taskPreview: z.string().max(200).optional(),
  model: z.string().optional(),
  status: teamBoardOrderStatusSchema,
  /** 依赖是否全部满足（无依赖恒 true）；批1 只展示，解锁调度随批2。 */
  unlocked: z.boolean(),
});
export type TeamBoardOrder = z.infer<typeof teamBoardOrderSchema>;

export const teamBoardMemberLiveSchema = z.enum(["running", "idle", "unknown"]);
export type TeamBoardMemberLive = z.infer<typeof teamBoardMemberLiveSchema>;

export const teamBoardMemberSchema = z.object({
  name: z.string(),
  agentId: z.string().optional(),
  /** 该员工在本团队里最近一次派单落点的工位会话。 */
  deskSessionId: z.string().optional(),
  /** 工位实时状态：本进程在场的 record 查真实在跑，冷会话 unknown（不为看板激活冷会话）。 */
  live: teamBoardMemberLiveSchema,
  /** 本团队最近一次派给他的模型（派单台账 payload.model）。 */
  model: z.string().optional(),
});
export type TeamBoardMember = z.infer<typeof teamBoardMemberSchema>;

/**
 * 团队质检灯（台账口径）：skipped=全绿免检（闸门行 payload.skipped）；
 * ran=质检/合议轮已开跑（闸门行已 promote）；pending=闸门已落、轮未跑。
 * 比对话流里的三态灯粗（看不到轮失败）——面板上只做徽章，权威结论仍在质检卡。
 */
export const teamBoardQcStateSchema = z.enum(["skipped", "ran", "pending"]);
export type TeamBoardQcState = z.infer<typeof teamBoardQcStateSchema>;

export const teamBoardTeamSchema = z.object({
  batchId: z.string().min(1),
  title: z.string().optional(),
  review: z.boolean().optional(),
  createdAt: z.number().optional(),
  updatedAt: z.number().optional(),
  qc: teamBoardQcStateSchema.optional(),
  orders: z.array(teamBoardOrderSchema).max(64),
  members: z.array(teamBoardMemberSchema).max(16),
});
export type TeamBoardTeam = z.infer<typeof teamBoardTeamSchema>;

export const teamBoardSnapshotSchema = z.object({
  sessionId: z.string().min(1),
  generatedAt: z.number(),
  teams: z.array(teamBoardTeamSchema).max(32),
});
export type TeamBoardSnapshot = z.infer<typeof teamBoardSnapshotSchema>;
