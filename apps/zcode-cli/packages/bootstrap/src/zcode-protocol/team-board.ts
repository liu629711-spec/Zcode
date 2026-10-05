// ============================================================
// 团队看板快照聚合（团队看板批 2026-10-05）：发起会话台账 → 看板 JSON。
// ============================================================
// 数据全来自既有台账（session_input）：派单行（信封带 batchId/taskKey/dependsOn）
// + 回执行（outcome 终态）+ 质检闸门行（skipped/promoted）+ 会话注册表的工位在跑
// 状态。不建新存储（台账即真相源）；聚合主体是纯函数（行数组 + live 查询回调），
// 测试不用起真实 runtime。上界防呆：teams 32 / orders 64 / members 16（快照 schema
// 同界），超出截断——看板是视图，不是审计工具。

import {
  TEAM_BOARD_MAX_REPAIR_ROUNDS,
  type TeamBoardMemberLive,
  TeamBoardOrderStatus,
  type TeamBoardQcState,
  type TeamBoardSnapshot,
  type TeamPlan,
} from "@zcode/shared/zcode-protocol-v4";

/** 台账行的窄视图（SessionInputRecord 的子集，测试夹具照此造）。 */
export interface TeamBoardInputRow {
  id: string;
  kind: string;
  status: string;
  admittedSequence: number;
  time: { created: number; updated: number };
  payload: {
    text?: unknown;
    workOrderId?: unknown;
    agentName?: unknown;
    agentId?: unknown;
    targetSessionId?: unknown;
    model?: unknown;
    batchId?: unknown;
    skipped?: unknown;
    envelope?: unknown;
    outcome?: unknown;
    [key: string]: unknown;
  };
}

export interface TeamBoardDeps {
  sessionId: string;
  rows: readonly TeamBoardInputRow[];
  /** 工位实时状态查询：本进程在场 record 查真实在跑，其余 unknown。 */
  liveStatusOf: (deskSessionId: string) => TeamBoardMemberLive;
  /** 排班草案（批3b）：bridge 用 extractStagedPlanDrafts(rows) 预计算后传入。 */
  plans?: TeamPlan[];
  /** 名册（批3b 草案编辑器负责人下拉）：bridge 从 runtime.getAgentProfiles() 预取。 */
  roster?: { name: string; agentId?: string }[];
  now?: number;
}

const MAX_TEAMS = 32;
const MAX_ORDERS = 64;
const MAX_MEMBERS = 16;
const TASK_PREVIEW_CHARS = 200;

function readString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function readEnvelopeRecord(envelope: unknown): Record<string, unknown> | undefined {
  return typeof envelope === "object" && envelope !== null && !Array.isArray(envelope)
    ? (envelope as Record<string, unknown>)
    : undefined;
}

interface OrderDraft {
  workOrderId: string;
  taskKey?: string;
  dependsOn?: string[];
  repairOf?: string;
  repairRound?: number;
  agentName: string;
  agentId?: string;
  deskSessionId?: string;
  taskPreview?: string;
  model?: string;
  status: TeamBoardOrderStatus;
  createdAt: number;
  updatedAt: number;
}

/** 组装看板快照：分组、终态对账、解锁判定、成员聚合（纯函数）。 */
export function buildTeamBoardSnapshotFromRows(deps: TeamBoardDeps): TeamBoardSnapshot {
  const completedTaskKeys = new Set<string>();
  interface TeamDraft {
    batchId: string;
    title?: string;
    review?: boolean;
    createdAt: number;
    updatedAt: number;
    orders: Map<string, OrderDraft>;
    memberOrder: string[];
    members: Map<string, { name: string; agentId?: string; deskSessionId?: string; model?: string }>;
  }
  const teams = new Map<string, TeamDraft>();

  const ensureTeam = (batchId: string, title?: string, createdAt = 0): TeamDraft => {
    let team = teams.get(batchId);
    if (!team) {
      team = {
        batchId,
        ...(title ? { title } : {}),
        createdAt,
        updatedAt: createdAt,
        orders: new Map(),
        memberOrder: [],
        members: new Map(),
      };
      teams.set(batchId, team);
    }
    if (title && !team.title) team.title = title;
    return team;
  };

  // 一遍扫派单行：分组 + 任务清单 + 成员名册（终态第二遍按回执行对账）。
  for (const row of deps.rows) {
    if (row.kind !== "agentWorkOrderDispatch") continue;
    const envelope = readEnvelopeRecord(row.payload.envelope);
    if (!envelope) continue;
    const batchId = readString(envelope.batchId);
    if (!batchId) continue;
    const workOrderId = readString(row.payload.workOrderId) ?? readString(envelope.workOrderId);
    if (!workOrderId) continue;
    const team = ensureTeam(batchId, readString(envelope.batchTitle), row.time.created);
    const taskKey = readString(envelope.taskKey);
    const repairOf = readString(envelope.repairOf);
    const repairRound =
      typeof envelope.repairRound === "number" && Number.isFinite(envelope.repairRound)
        ? Math.max(1, Math.floor(envelope.repairRound))
        : undefined;
    const rawDeps = Array.isArray(envelope.dependsOn) ? envelope.dependsOn : undefined;
    const dependsOn = rawDeps
      ?.map((key) => (typeof key === "string" && key.trim() ? key.trim() : ""))
      .filter(Boolean);
    const agentName = readString(row.payload.agentName) ?? "";
    const agentId = readString(row.payload.agentId);
    const deskSessionId = readString(row.payload.targetSessionId);
    const existing = team.orders.get(workOrderId);
    const order: OrderDraft = {
      workOrderId,
      ...(taskKey ? { taskKey } : {}),
      ...(dependsOn?.length ? { dependsOn } : {}),
      ...(repairOf ? { repairOf } : {}),
      ...(repairRound !== undefined ? { repairRound } : {}),
      agentName,
      ...(agentId ? { agentId } : {}),
      ...(deskSessionId ? { deskSessionId } : {}),
      ...(typeof row.payload.text === "string" && row.payload.text.trim()
        ? { taskPreview: row.payload.text.slice(0, TASK_PREVIEW_CHARS) }
        : {}),
      ...(readString(row.payload.model) ? { model: readString(row.payload.model) } : {}),
      // 重派/续批可能同 workOrderId 多行（自动重试同 id）：取台账最新一行的时间。
      status: "in_flight",
      createdAt: Math.min(existing?.createdAt ?? Number.POSITIVE_INFINITY, row.time.created),
      updatedAt: Math.max(existing?.updatedAt ?? 0, row.time.updated),
    };
    team.orders.set(workOrderId, order);
    team.updatedAt = Math.max(team.updatedAt, row.time.updated);
    const memberKey = agentId ?? agentName;
    if (memberKey) {
      if (!team.members.has(memberKey)) {
        team.memberOrder.push(memberKey);
      }
      const member = team.members.get(memberKey) ?? { name: agentName, ...(agentId ? { agentId } : {}) };
      // 同员工多次派单：署名/工位/模型取台账最新一行（改名后看板跟着新名走）。
      if (agentName) member.name = agentName;
      if (deskSessionId) member.deskSessionId = deskSessionId;
      if (order.model) member.model = order.model;
      team.members.set(memberKey, member);
    }
  }

  // review 口径（与 startBatchQcIfComplete 同规则）：同批**全是**评审单才算圆桌；
  // 混批或全普通批都不是。第二遍扫保证「先普通后评审」的混批不会误标。
  for (const team of teams.values()) {
    const envelopes = deps.rows.filter(
      (row) =>
        row.kind === "agentWorkOrderDispatch" &&
        readString(readEnvelopeRecord(row.payload.envelope)?.batchId) === team.batchId,
    );
    team.review =
      envelopes.length > 0 &&
      envelopes.every((row) => readEnvelopeRecord(row.payload.envelope)?.review === true);
  }

  // 回执行对账：每张工单取 admittedSequence 最新的终态（自动重试同 id 也成立）。
  const latestOutcome = new Map<string, { status: TeamBoardOrderStatus; sequence: number }>();
  for (const row of deps.rows) {
    if (row.kind !== "agentWorkOrderReceipt") continue;
    const workOrderId = readString(row.payload.workOrderId);
    const outcome =
      typeof row.payload.outcome === "object" && row.payload.outcome !== null
        ? (row.payload.outcome as { status?: unknown })
        : undefined;
    const statusValue = outcome?.status;
    if (!workOrderId || typeof statusValue !== "string") continue;
    const status: TeamBoardOrderStatus =
      statusValue === "completed" || statusValue === "cancelled" ? statusValue : "failed";
    const previous = latestOutcome.get(workOrderId);
    if (previous && previous.sequence >= row.admittedSequence) continue;
    latestOutcome.set(workOrderId, { status, sequence: row.admittedSequence });
    const batchId = readString(readEnvelopeRecord(row.payload.envelope)?.batchId);
    const team = batchId ? teams.get(batchId) : undefined;
    if (team?.orders.has(workOrderId)) {
      team.orders.get(workOrderId)!.status = status;
      team.updatedAt = Math.max(team.updatedAt, row.time.updated);
    }
  }
  // 解锁判定基准：taskKey → 该任务已有 completed 终态。
  for (const team of teams.values()) {
    for (const order of team.orders.values()) {
      if (order.taskKey && order.status === "completed") completedTaskKeys.add(order.taskKey);
    }
  }

  // 队内消息（批3）：通讯区按批次归拢，时间正序，最新 32 条。
  const messagesByBatch = new Map<
    string,
    { from: string; to: string; text: string; createdAt: number; toSessionId?: string }[]
  >();
  for (const row of deps.rows) {
    if (row.kind !== "agentWorkOrderTeamMessage") continue;
    const rowBatchId = readString(row.payload.batchId);
    const from = readString(row.payload.from);
    const to = readString(row.payload.to);
    const text = readString(row.payload.text);
    if (!rowBatchId || !from || !to || !text) continue;
    const list = messagesByBatch.get(rowBatchId) ?? [];
    const toSessionId = readString(row.payload.toSessionId);
    list.push({
      from,
      to,
      text: text.slice(0, 200),
      createdAt: row.time.created,
      ...(toSessionId ? { toSessionId } : {}),
    });
    messagesByBatch.set(rowBatchId, list);
  }

  // 质检灯（台账口径）：skipped > promoted(ran) > admitted(pending)。
  const qcByBatch = new Map<string, TeamBoardQcState>();
  for (const row of deps.rows) {
    if (row.kind !== "agentWorkOrderBatchQc") continue;
    const batchId = readString(row.payload.batchId);
    if (!batchId) continue;
    const state: TeamBoardQcState =
      row.payload.skipped === true ? "skipped" : row.status === "promoted" ? "ran" : "pending";
    // 免检是最强事实，其次已开跑；同一批多代闸门行（新旧键并存）取最强。
    const existing = qcByBatch.get(batchId);
    const rank: Record<TeamBoardQcState, number> = { skipped: 2, ran: 1, pending: 0 };
    if (!existing || rank[state] > rank[existing]) qcByBatch.set(batchId, state);
  }

  const teamList = [...teams.values()]
    .sort((a, b) => b.updatedAt - a.updatedAt)
    .slice(0, MAX_TEAMS)
    .map((team) => {
      const members = team.memberOrder
        .map((key) => team.members.get(key)!)
        .filter((member) => member !== undefined)
        .slice(0, MAX_MEMBERS)
        .map((member) => ({
          name: member.name,
          ...(member.agentId ? { agentId: member.agentId } : {}),
          ...(member.deskSessionId ? { deskSessionId: member.deskSessionId } : {}),
          live: (
            member.deskSessionId ? deps.liveStatusOf(member.deskSessionId) : "unknown"
          ) as TeamBoardMemberLive,
          ...(member.model ? { model: member.model } : {}),
        }));
      return {
        batchId: team.batchId,
        ...(team.title ? { title: team.title } : {}),
        ...(team.review === true ? { review: true } : {}),
        createdAt: team.createdAt,
        updatedAt: team.updatedAt,
        ...(qcByBatch.get(team.batchId) ? { qc: qcByBatch.get(team.batchId)! } : {}),
        // 自动流转开关（批2）：台账开关行是权威，快照带一份给面板画状态；
        // escalated = 返修轮次到上限（停手等人）。
        ...(isTeamAutoFlowEnabledRow(deps.rows, team.batchId) ? { autoFlow: true } : {}),
        ...([...team.orders.values()].some(
          (order) => (order.repairRound ?? 0) >= TEAM_BOARD_MAX_REPAIR_ROUNDS,
        )
          ? { escalated: true }
          : {}),
        ...(messagesByBatch.get(team.batchId)
          ? {
              messages: messagesByBatch
                .get(team.batchId)!
                .sort((a, b) => a.createdAt - b.createdAt)
                .slice(-32),
            }
          : {}),
        orders: [...team.orders.values()]
          .sort((a, b) => a.createdAt - b.createdAt || a.workOrderId.localeCompare(b.workOrderId))
          .slice(0, MAX_ORDERS)
          .map((order) => {
            const depsKeys = order.dependsOn ?? [];
            return {
              workOrderId: order.workOrderId,
              ...(order.taskKey ? { taskKey: order.taskKey } : {}),
              ...(order.dependsOn?.length ? { dependsOn: order.dependsOn } : {}),
              agentName: order.agentName,
              ...(order.agentId ? { agentId: order.agentId } : {}),
              ...(order.deskSessionId ? { deskSessionId: order.deskSessionId } : {}),
              ...(order.taskPreview ? { taskPreview: order.taskPreview } : {}),
              ...(order.model ? { model: order.model } : {}),
              status: order.status,
              unlocked: depsKeys.every((key) => completedTaskKeys.has(key)),
              ...(order.repairOf ? { repairOf: order.repairOf } : {}),
              ...(order.repairRound !== undefined ? { repairRound: order.repairRound } : {}),
            };
          }),
        members,
      };
    });

  return {
    sessionId: deps.sessionId,
    generatedAt: deps.now ?? Date.now(),
    teams: teamList,
    ...(deps.plans?.length ? { plans: deps.plans } : {}),
    ...(deps.roster?.length ? { roster: deps.roster } : {}),
  };
}

/** 已交活（completed）的批内符号名集合：解锁判定的唯一基准。 */
export function computeCompletedTaskKeys(rows: readonly TeamBoardInputRow[]): Set<string> {
  // 工单号 → 批内符号名（派单行；重派/续批可能同 key 多单，任一 completed 即算）。
  const taskKeyByWorkOrder = new Map<string, string>();
  const latestStatus = new Map<string, { status: unknown; sequence: number }>();
  for (const row of rows) {
    const envelope = readEnvelopeRecord(row.payload.envelope);
    const workOrderId = readString(row.payload.workOrderId) ?? readString(envelope?.workOrderId);
    if (!workOrderId) continue;
    if (row.kind === "agentWorkOrderDispatch") {
      const taskKey = readString(envelope?.taskKey);
      if (taskKey && !taskKeyByWorkOrder.has(workOrderId)) {
        taskKeyByWorkOrder.set(workOrderId, taskKey);
      }
    }
    if (row.kind !== "agentWorkOrderReceipt") continue;
    const outcome =
      typeof row.payload.outcome === "object" && row.payload.outcome !== null
        ? (row.payload.outcome as { status?: unknown })
        : undefined;
    const previous = latestStatus.get(workOrderId);
    if (previous && previous.sequence >= row.admittedSequence) continue;
    latestStatus.set(workOrderId, { status: outcome?.status, sequence: row.admittedSequence });
  }
  const completed = new Set<string>();
  for (const [workOrderId, taskKey] of taskKeyByWorkOrder) {
    if (latestStatus.get(workOrderId)?.status === "completed") completed.add(taskKey);
  }
  return completed;
}

/** 团队「自动流转」开关是否打开（台账开关行 payload.enabled 权威）。 */
export function isTeamAutoFlowEnabledRow(rows: readonly TeamBoardInputRow[], batchId: string): boolean {
  return rows.some(
    (row) =>
      row.kind === "agentWorkOrderTeamFlow" &&
      row.payload.batchId === batchId &&
      row.payload.enabled === true,
  );
}

/** CLI 端实时状态查询的窄接口：只在会话注册表里看，不为看板激活冷会话。 */
export function liveStatusFromSessions(
  sessions: { hasActiveOrQueuedTurnWork?: () => boolean } | undefined,
): TeamBoardMemberLive {
  if (!sessions) return "unknown";
  try {
    return sessions.hasActiveOrQueuedTurnWork?.() === true ? "running" : "idle";
  } catch {
    return "unknown";
  }
}
