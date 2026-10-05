// 团队看板快照（团队看板批 2026-10-05，参照件 dsh-agent-teams 的原生落法）。
//
// 数据全来自发起会话的既有台账（session_input 的派单/回执/质检闸门行）+ 会话
// 注册表的工位在跑状态——不建新的状态存储（台账即真相源）。CLI 端聚合，UI 经
// v4 teamBoardState 命令拉取轮询。schema 是 UI 渲染的唯一契约：发射侧有界铸造
// （taskPreview 200 字符、dependsOn ≤16、teams/orders 上界防呆），zod 剥未知键。
import { z } from "zod";

/**
 * 自动返修上限（团队看板批2 2026-10-05 老板拍板 T6）：同一张工单最多自动返修
 * 2 轮，到顶停手——看板标"升级给老板"，绝不无限循环。调度器与快照 escalated
 * 判定共用这一个常量，两侧不会漂移。
 */
export const TEAM_BOARD_MAX_REPAIR_ROUNDS = 2;

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
  /** 依赖是否全部满足（无依赖恒 true）；解锁释放由调度器执行（批2）。 */
  unlocked: z.boolean(),
  /** 返修单标记（批2）：repairOf=被返修的工单 id，repairRound 单调轮次。 */
  repairOf: z.string().optional(),
  repairRound: z.number().int().positive().optional(),
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
  /**
   * 团队「自动流转」开关（批2，老板拍板 T3 默认关）：开=评审不通过自动返修
   * （有界），关=失败照旧只报不修。台账行（agentWorkOrderTeamFlow）是权威，
   * 快照带一份给面板开关画状态。
   */
  autoFlow: z.boolean().optional(),
  /** 自动返修到上限停手（repairRound ≥ TEAM_BOARD_MAX_REPAIR_ROUNDS）：等人拍板。 */
  escalated: z.boolean().optional(),
  /** 队内直达消息（批3）：通讯区按时间正序，最新 32 条（发射侧截断）。 */
  messages: z
    .array(
      z.object({
        from: z.string().min(1),
        to: z.string().min(1),
        text: z.string().min(1).max(200),
        createdAt: z.number(),
        /** 收件人工位（跳转会话用）；缺席 = 投递时未落工位（旧数据）。 */
        toSessionId: z.string().optional(),
      }),
    )
    .max(32)
    .optional(),
});
export type TeamBoardTeam = z.infer<typeof teamBoardTeamSchema>;

// ── 排班草案（批3b）：staged → 面板编辑 → 确认开工 ─────────────────────
// 草案是队长会话台账的一行（agentWorkOrderTeamPlan，upsert 幂等）；成员名单
// 不单列——就是 tasks 的 assignee 集合（成员=名册档案，改人=改 assignee）。

export const teamPlanTaskSchema = z.object({
  taskKey: z.string().min(1).max(128),
  task: z.string().min(1).max(8192),
  /** 名册点名（工号优先，其次精确名）；P3 拍板：草案必须点名，无名新桌走散单。 */
  assignee: z.string().min(1).max(160),
  dependsOn: z.array(z.string().min(1).max(128)).max(16).optional(),
});
export type TeamPlanTask = z.infer<typeof teamPlanTaskSchema>;

export const teamPlanStatusSchema = z.enum(["staged", "approved", "discarded"]);
export type TeamPlanStatus = z.infer<typeof teamPlanStatusSchema>;

export const teamPlanSchema = z.object({
  planId: z.string().min(1).max(64),
  title: z.string().min(1).max(80),
  status: teamPlanStatusSchema,
  tasks: z.array(
    teamPlanTaskSchema.extend({
      /** approve 后回填：每张任务对应的真工单 id。 */
      workOrderId: z.string().optional(),
      /** P2 拍板：开工后单张失败照跑其余，失败红字挂卡可点重试。 */
      error: z.string().max(300).optional(),
    }),
  ).max(16),
  batchId: z.string().optional(),
  createdAt: z.number(),
  approvedAt: z.number().optional(),
});
export type TeamPlan = z.infer<typeof teamPlanSchema>;

export const teamPlanErrorSchema = z.object({
  taskKey: z.string().min(1).max(128),
  error: z.string().min(1).max(300),
});
export type TeamPlanError = z.infer<typeof teamPlanErrorSchema>;

export const teamBoardSnapshotSchema = z.object({
  sessionId: z.string().min(1),
  generatedAt: z.number(),
  teams: z.array(teamBoardTeamSchema).max(32),
  /** 排班草案（批3b）：只列 staged 的（approved 的已变真批次进 teams）。 */
  plans: z
    .array(teamPlanSchema.extend({ errors: z.array(teamPlanErrorSchema).max(16).optional() }))
    .max(8)
    .optional(),
  /** 名册（批3b 草案编辑器的负责人下拉数据源）：本 workspace 可派员工。 */
  roster: z
    .array(
      z.object({
        name: z.string().min(1),
        agentId: z.string().optional(),
      }),
    )
    .max(32)
    .optional(),
});
export type TeamBoardSnapshot = z.infer<typeof teamBoardSnapshotSchema>;
