// ============================================================
// Team Plan Port（团队看板批3b）：排班草案的协议层实现。
// ============================================================
// 两阶段协议下半场：TeamPlan 工具出草案（staged）→ 看板编辑器改（update）→
// 确认开工（approve，逐单走既有派单端口：频控三道闸兜底 + 批2 依赖持派自动
// 生效）→ 放弃（discard）。纪律（拍板 P1~P4）：
//  - 草案 = 队长会话台账一行（upsert 幂等），成员名单=assignee 集合不单列；
//  - approve 后草案不可再编辑（开工即普通批次，重派走既有抓手）；有失败单
//    可重试（只重派 error 在场的任务）；
//  - assignee 必须点名（无名新桌走散单，不进草案）；
//  - 校验纯函数化：assignee 可解析到名册、依赖引用本草案 taskKey、无环。

import type { SessionId, TeamPlanPort } from "@zcode/contracts";
import type {
  TeamPlan,
  TeamPlanError,
  TeamPlanTask,
} from "@zcode/shared/zcode-protocol-v4";
import { resolveWorkOrderTarget, type AgentProfile } from "@zcode/core";
import type {
  ZCodeProtocolAgentServerContext,
  ZCodeProtocolSessionRecord,
} from "./server-types.js";
import type { ProtocolAgentDispatchPortDeps } from "./agent-dispatch-port.js";
import { createProtocolAgentDispatchPort } from "./agent-dispatch-port.js";

export function teamPlanRowId(sessionId: string, planId: string): string {
  return `agentWorkOrderTeamPlan:${sessionId}:${planId}`;
}

function readString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

/** 校验错误：可读、面向模型/面板（reasonCode 进 guard 词表分流）。 */
export function teamPlanGuardError(reasonCode: string, message: string): Error {
  return Object.assign(new Error(message), { name: "AgentWorkOrderGuardError", reasonCode });
}

export const TEAM_PLAN_GUARDS = {
  invalid: "guard.teamPlanInvalid",
  notFound: "guard.teamPlanNotFound",
  state: "guard.teamPlanState",
} as const;

/** 草案台账行的窄视图（TeamPlanRowPayload + 行壳；测试夹具照此造）。 */
export interface TeamPlanLedgerRow {
  id: string;
  kind: string;
  status: string;
  payload: TeamPlanRowPayload;
}

/** 台账行 → 看板草案卡（快照用）：只列 staged 的（approved 已是真批次）。 */
export function extractStagedPlanDrafts(
  rows: readonly TeamPlanLedgerRow[],
): (TeamPlan & { errors?: TeamPlanError[] })[] {
  return rows
    .filter((row) => row.kind === "agentWorkOrderTeamPlan" && row.payload.status === "staged")
    .map((row) => ({
      planId: row.payload.planId,
      title: row.payload.title,
      status: "staged" as const,
      tasks: row.payload.tasks,
      ...(row.payload.batchId ? { batchId: row.payload.batchId } : {}),
      createdAt: row.payload.createdAt,
    }))
    .slice(0, 8);
}

/** 依赖图无环（DFS；环 = 谁也开不了工，创建时就得拒绝）。 */
export function hasPlanDependencyCycle(tasks: readonly TeamPlanTask[]): boolean {
  const byKey = new Map(tasks.map((task) => [task.taskKey, task]));
  const visiting = new Set<string>();
  const done = new Set<string>();
  const visit = (taskKey: string): boolean => {
    if (done.has(taskKey)) return false;
    if (visiting.has(taskKey)) return true;
    visiting.add(taskKey);
    const task = byKey.get(taskKey);
    for (const dep of task?.dependsOn ?? []) {
      if (byKey.has(dep) && visit(dep)) return true;
    }
    visiting.delete(taskKey);
    done.add(taskKey);
    return false;
  };
  return tasks.some((task) => visit(task.taskKey));
}

/**
 * 草案任务校验（纯函数）：assignee 全部可解析到名册、依赖引用本草案、无环、
 * taskKey 不重复。返回错误列表——空 = 通过。
 */
export function validatePlanTasks(
  tasks: readonly TeamPlanTask[],
  profiles: readonly AgentProfile[],
): TeamPlanError[] {
  const errors: TeamPlanError[] = [];
  const keys = new Set<string>();
  for (const task of tasks) {
    if (keys.has(task.taskKey)) {
      errors.push({ taskKey: task.taskKey, error: `任务符号名 "${task.taskKey}" 重复` });
    }
    keys.add(task.taskKey);
  }
  if (errors.length > 0) return errors;
  for (const task of tasks) {
    const resolution = resolveWorkOrderTarget(task.assignee, profiles);
    if (resolution.kind === "not_found") {
      const availableNames = profiles.map((profile) => profile.name).join(", ") || "none";
      errors.push({
        taskKey: task.taskKey,
        error: `负责人 "${task.assignee}" 不在名册里。可用员工：${availableNames}`,
      });
    } else if (resolution.kind === "ambiguous") {
      errors.push({
        taskKey: task.taskKey,
        error: `负责人 "${task.assignee}" 命中多个档案（${resolution.matchedNames.join(", ")}），请改用工号`,
      });
    }
    for (const dep of task.dependsOn ?? []) {
      if (!keys.has(dep)) {
        errors.push({
          taskKey: task.taskKey,
          error: `依赖 "${dep}" 不是本草案里定义的任务符号名`,
        });
      }
    }
  }
  if (errors.length === 0 && hasPlanDependencyCycle(tasks)) {
    errors.push({ taskKey: tasks[0]!.taskKey, error: "依赖成环：谁也开不了工，请调整依赖" });
  }
  return errors;
}

interface TeamPlanRowPayload {
  /** 台账行正文约定：存草案标题（saveSessionInput 的 payload 形状要求）。 */
  text: string;
  planId: string;
  title: string;
  status: "staged" | "approved" | "discarded";
  tasks: (TeamPlanTask & { workOrderId?: string; error?: string })[];
  batchId?: string;
  createdAt: number;
  approvedAt?: number;
  [key: string]: unknown;
}

export function createProtocolTeamPlanPort(
  context: ZCodeProtocolAgentServerContext,
  deps: Pick<ProtocolAgentDispatchPortDeps, "activateSessionRecord" | "createPersonaSessionRecord">,
): TeamPlanPort {
  const loadPlanRow = async (
    sessionId: string,
    planId: string,
  ): Promise<{ rowId: string; payload: TeamPlanRowPayload } | undefined> => {
    const store = context.deps.sessionStore;
    const rows = (await store?.listSessionInputs?.({
      sessionID: sessionId as SessionId,
    })) as unknown as
      | { id: string; status: string; payload: TeamPlanRowPayload }[]
      | undefined;
    const rowId = teamPlanRowId(sessionId, planId);
    const row = rows?.find((candidate) => candidate.id === rowId);
    if (!row) return undefined;
    return { rowId, payload: row.payload };
  };

  return {
    async create(input) {
      const ownRecord =
        context.sessions.get(input.sourceSessionId) ??
        (await deps.activateSessionRecord(input.sourceSessionId));
      const profiles = ownRecord.app.runtime.getAgentProfiles();
      const errors = validatePlanTasks(input.tasks, profiles);
      if (errors.length > 0) {
        throw teamPlanGuardError(
          TEAM_PLAN_GUARDS.invalid,
          `Plan draft rejected: ${errors.map((error) => `[${error.taskKey}] ${error.error}`).join("；")}`,
        );
      }
      const planId = crypto.randomUUID();
      const store = context.deps.sessionStore;
      if (!store?.saveSessionInput) {
        throw teamPlanGuardError(TEAM_PLAN_GUARDS.invalid, "Plan drafts need ledger support.");
      }
      await store.saveSessionInput({
        id: teamPlanRowId(ownRecord.app.sessionId, planId),
        sessionID: ownRecord.app.sessionId as SessionId,
        kind: "agentWorkOrderTeamPlan",
        delivery: "queue",
        payload: {
          text: input.title,
          planId,
          title: input.title,
          status: "staged",
          tasks: input.tasks,
          createdAt: Date.now(),
        },
      });
      context.logger?.info("Team plan draft staged", {
        event: "team_plan.staged",
        module: "bootstrap.zcode_protocol",
        planId,
        taskCount: input.tasks.length,
        sessionId: ownRecord.app.sessionId,
      });
      return { planId, taskCount: input.tasks.length };
    },

    async update(input) {
      const ownRecord =
        context.sessions.get(input.sourceSessionId) ??
        (await deps.activateSessionRecord(input.sourceSessionId));
      const sessionId = ownRecord.app.sessionId;
      const loaded = await loadPlanRow(sessionId, input.planId);
      if (!loaded) {
        throw teamPlanGuardError(TEAM_PLAN_GUARDS.notFound, `Plan draft ${input.planId} not found.`);
      }
      if (loaded.payload.status !== "staged") {
        throw teamPlanGuardError(
          TEAM_PLAN_GUARDS.state,
          `Plan ${input.planId} is ${loaded.payload.status}; only staged drafts are editable (approved plans are ordinary batches - use re-dispatch).`,
        );
      }
      const profiles = ownRecord.app.runtime.getAgentProfiles();
      const errors = validatePlanTasks(input.tasks, profiles);
      if (errors.length > 0) {
        throw teamPlanGuardError(
          TEAM_PLAN_GUARDS.invalid,
          `Plan update rejected: ${errors.map((error) => `[${error.taskKey}] ${error.error}`).join("；")}`,
        );
      }
      const store = context.deps.sessionStore;
      await store?.saveSessionInput?.({
        id: loaded.rowId,
        sessionID: sessionId as SessionId,
        kind: "agentWorkOrderTeamPlan",
        delivery: "queue",
        payload: { ...loaded.payload, tasks: input.tasks },
      });
      context.logger?.info("Team plan draft updated", {
        event: "team_plan.updated",
        module: "bootstrap.zcode_protocol",
        planId: input.planId,
        sessionId,
      });
      return { planId: input.planId };
    },

    async approve(input) {
      const ownRecord =
        context.sessions.get(input.sourceSessionId) ??
        (await deps.activateSessionRecord(input.sourceSessionId));
      const sessionId = ownRecord.app.sessionId;
      const loaded = await loadPlanRow(sessionId, input.planId);
      if (!loaded) {
        throw teamPlanGuardError(TEAM_PLAN_GUARDS.notFound, `Plan draft ${input.planId} not found.`);
      }
      const payload = loaded.payload;
      if (payload.status === "discarded") {
        throw teamPlanGuardError(TEAM_PLAN_GUARDS.state, `Plan ${input.planId} was discarded.`);
      }
      // P2：已批准的草案只剩失败单可重试；staged 全量派。
      const retriable =
        payload.status === "approved"
          ? payload.tasks.filter((task) => task.error !== undefined)
          : payload.tasks;
      if (payload.status === "approved" && retriable.length === 0) {
        throw teamPlanGuardError(
          TEAM_PLAN_GUARDS.state,
          `Plan ${input.planId} is already running with no failed tasks to retry.`,
        );
      }
      const dispatchPort = createProtocolAgentDispatchPort(context, {
        resolveOwnSession: () => ownRecord,
        createPersonaSessionRecord: deps.createPersonaSessionRecord,
        activateSessionRecord: deps.activateSessionRecord,
      });
      const failed: TeamPlanError[] = [];
      let dispatched = 0;
      let batchId: string | undefined;
      for (const task of payload.tasks) {
        const isFirstDispatch = payload.status === "staged";
        const isRetry = task.error !== undefined;
        if (!isFirstDispatch && !isRetry) continue;
        try {
          const result = await dispatchPort.dispatch({
            agent: task.assignee,
            task: task.task,
            batchTitle: payload.title,
            taskKey: task.taskKey,
            ...(task.dependsOn?.length ? { dependsOn: [...task.dependsOn] } : {}),
            sourceSessionId: sessionId,
          });
          task.workOrderId = result.workOrderId;
          delete task.error;
          dispatched += 1;
          batchId = batchId ?? result.batchId;
        } catch (error) {
          task.error = (error instanceof Error ? error.message : String(error)).slice(0, 300);
          failed.push({ taskKey: task.taskKey, error: task.error });
        }
      }
      const store = context.deps.sessionStore;
      await store?.saveSessionInput?.({
        id: loaded.rowId,
        sessionID: sessionId as SessionId,
        kind: "agentWorkOrderTeamPlan",
        delivery: "queue",
        payload: {
          ...payload,
          status: "approved",
          ...(batchId ? { batchId } : {}),
          approvedAt: payload.approvedAt ?? Date.now(),
          tasks: payload.tasks,
        },
      });
      context.logger?.info("Team plan approved and dispatched", {
        batchId,
        dispatched,
        event: "team_plan.approved",
        failedCount: failed.length,
        module: "bootstrap.zcode_protocol",
        planId: input.planId,
        sessionId,
      });
      return { planId: input.planId, ...(batchId ? { batchId } : {}), dispatched, failed };
    },

    async discard(input) {
      const ownRecord =
        context.sessions.get(input.sourceSessionId) ??
        (await deps.activateSessionRecord(input.sourceSessionId));
      const sessionId = ownRecord.app.sessionId;
      const loaded = await loadPlanRow(sessionId, input.planId);
      if (!loaded) {
        throw teamPlanGuardError(TEAM_PLAN_GUARDS.notFound, `Plan draft ${input.planId} not found.`);
      }
      if (loaded.payload.status !== "staged") {
        throw teamPlanGuardError(
          TEAM_PLAN_GUARDS.state,
          `Plan ${input.planId} is ${loaded.payload.status}; only staged drafts can be discarded.`,
        );
      }
      const store = context.deps.sessionStore;
      await store?.saveSessionInput?.({
        id: loaded.rowId,
        sessionID: sessionId as SessionId,
        kind: "agentWorkOrderTeamPlan",
        delivery: "queue",
        payload: { ...loaded.payload, status: "discarded" },
      });
      context.logger?.info("Team plan draft discarded", {
        event: "team_plan.discarded",
        module: "bootstrap.zcode_protocol",
        planId: input.planId,
        sessionId,
      });
      return { planId: input.planId };
    },
  };
}

export type { ZCodeProtocolSessionRecord };
