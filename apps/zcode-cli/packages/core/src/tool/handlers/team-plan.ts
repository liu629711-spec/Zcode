// ============================================================
// TeamPlan Tool Handler（团队看板批3b）：排班草案——先排班、拍板了才开工。
// ============================================================
// 两阶段协议上半场：草案落台账 status=staged（不派单不开工不建工位）；用户在
// 看板上改（teamPlanUpdate）/确认开工（teamPlanApprove，逐单走既有派单端口，
// 依赖持派自动生效）/放弃（teamPlanDiscard，二次确认）。校验执法在端口
// （bootstrap team-plan.ts：assignee 可解析、依赖引用本草案、无环）。

import {
  TeamPlanInputSchema,
  TeamPlanOutputSchema,
  TeamPlanInputJsonSchema,
  TeamPlanOutputJsonSchema,
  CoreErrorType,
  createCoreError,
  type TeamPlanInput,
  type TeamPlanOutput,
  type ToolPermissionSpec,
} from "@zcode/contracts";
import type { ToolEntry, ToolExecutionContext, ToolHandler } from "../types.js";

const TEAM_PLAN_TOOL_TIMEOUT_MS = 15_000;
const TEAM_PLAN_MODEL_BYTES = 4_000;

function assertTeamPlanPort(context: ToolExecutionContext): asserts context is ToolExecutionContext & {
  teamPlanPort: NonNullable<ToolExecutionContext["teamPlanPort"]>;
} {
  if (context.teamPlanPort) return;
  throw createCoreError(
    CoreErrorType.ConfigurationError,
    "TeamPlanPort is not configured for TeamPlan",
    {
      context: {
        toolCallId: context.toolCallId,
        toolName: "TeamPlan",
      },
      recoverable: false,
    },
  );
}

const teamPlanHandler: ToolHandler = async (input, context) => {
  const parsed = TeamPlanInputSchema.parse(input) as TeamPlanInput;
  assertTeamPlanPort(context);
  const result = await context.teamPlanPort.create({
    title: parsed.title,
    tasks: parsed.tasks.map((task) => ({
      taskKey: task.task_key,
      task: task.task,
      assignee: task.assignee,
      ...(task.depends_on ? { dependsOn: [...task.depends_on] } : {}),
    })),
    sourceSessionId: context.sessionId,
  });
  return {
    planId: result.planId,
    taskCount: result.taskCount,
    message: `Plan draft staged with ${result.taskCount} tasks. Nothing is dispatched yet - tell the user to open the team board to review and edit the draft, then press approve to start the work. Do NOT dispatch these tasks yourself with AgentDispatch.`,
  } satisfies TeamPlanOutput;
};

const teamPlanPermission: ToolPermissionSpec = {
  permission: "agent.teamPlan",
  reason: "TeamPlan stages a plan draft (members, tasks, dependencies) for user review on the team board",
  riskLevel: "low",
  sideEffectScope: "workspace",
  needsApproval: false,
  patternSources: ["toolName"],
  alwaysAllowPatternSources: ["toolName"],
  denyPriority: "beforeAsk",
};

const teamPlanResultBudget = {
  maxInlineBytes: TEAM_PLAN_MODEL_BYTES,
  maxModelBytes: TEAM_PLAN_MODEL_BYTES,
  strategy: "truncate" as const,
  preview: {
    maxBytes: TEAM_PLAN_MODEL_BYTES,
    direction: "head" as const,
  },
};

const teamPlanTimeout = {
  defaultMs: TEAM_PLAN_TOOL_TIMEOUT_MS,
  maxMs: TEAM_PLAN_TOOL_TIMEOUT_MS,
  allowCallOverride: false,
};

export const teamPlanToolEntry: ToolEntry = {
  capability: "Stage a plan draft (tasks, assignees, dependencies) for user review on the team board",
  metadata: {
    name: "TeamPlan",
    description:
      "Stage a PLAN DRAFT for a multi-task job: a titled breakdown of 1-16 tasks, each with a named doer from the workspace roster and optional dependencies, saved to the team board for the user to review and edit. Nothing runs at staging time - no sessions are created, no work is dispatched. The user edits the draft on the board (swap assignees, change dependencies, add or remove tasks) and presses approve; only then does the system dispatch the work (dependencies are honored automatically). Use this when the user asks for a plan first (先出个计划/排班/怎么分工给我看看) or for a big multi-part job where review before execution is expected; dispatch directly with AgentDispatch only when the user says to just do it. Do not dispatch plan tasks yourself afterwards - approval on the board starts them, and double-dispatching creates duplicate work.",
    modelInstructions: [
      "When the user asks for a plan or review before execution (先出个计划 / 排个班 / 怎么分工给我看看先), call TeamPlan with a task breakdown instead of dispatching: each task needs a task_key, a complete standalone description, an assignee from the roster, and depends_on for ordering. The draft appears on the team board; say in one sentence that it is ready for review and stop.",
      "After staging a plan, do NOT also dispatch the same tasks with AgentDispatch - the user approves the draft on the board and the system dispatches it. If the user asks to change the plan, update the draft via the board (or stage a new one only if they discarded the old draft); never create a second draft for the same job.",
      "Every task must name its assignee from the workspace roster; unnamed work is rejected. Dependencies must reference task_keys in the same plan.",
    ],
    readOnly: false,
    destructive: false,
    concurrentSafe: true,
    timeoutMs: TEAM_PLAN_TOOL_TIMEOUT_MS,
    maxOutputBytes: TEAM_PLAN_MODEL_BYTES,
    sideEffectScope: "workspace",
    riskLevel: "low",
    needsApproval: false,
  },
  handler: teamPlanHandler,
  inputSchema: TeamPlanInputJsonSchema,
  outputSchema: TeamPlanOutputJsonSchema,
  runtimeInputSchema: TeamPlanInputSchema,
  runtimeOutputSchema: TeamPlanOutputSchema,
  permission: teamPlanPermission,
  resultBudget: teamPlanResultBudget,
  timeout: teamPlanTimeout,
  cancellation: {
    supported: true,
    cleanup: "none",
    userVisibleMessage: "TeamPlan was cancelled before the draft was staged",
  },
  trace: {
    required: true,
    propagateToAdapters: true,
    recordInput: "summary",
    recordOutput: "summary",
  },
};
