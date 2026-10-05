// ============================================================
// AgentDispatch Port (D29/D2): 协议层实现的跨会话工单投递。
// ============================================================
// 落在 bootstrap 协议层（不是宿主进程）：context.sessions 会话注册表与
// sessionStore 都在这里，一个 CLI 进程只服务一个 workspace，派单因此天然
// 只能投同 workspace 的档案/会话；跨 workspace/跨机器目标在这个进程里根本
// 不可见，等同于被结构性拒绝。
//
// 嵌套上限=1 的端口级自检：工单唤醒轮的 inputId 带 workorder- 前缀
// （core turn-loop 据此进 denylist 终审），工单轮内再调 AgentDispatch 在这里
// 就被拒绝，不会触达目标会话。

import {
  WORK_ORDER_INPUT_ID_PREFIX,
  type AgentDispatchPort,
  type AgentDispatchRequest,
  type AgentDispatchResult,
  type AgentWorkOrderEnvelope,
  type SessionId,
  type UsageStorePort,
} from "@zcode/contracts";
import type { ModelSelection, ZCodeSessionPersona, ZCodeWorkspaceRef } from "@zcode/shared";
import {
  zcodeAgentDispatchCheckTaskRetiredResultSchema,
  zcodeProtocolMethods,
} from "@zcode/shared";
import {
  isSelfDispatch,
  resolveDispatchModelSelection,
  resolveWorkOrderTarget,
  type AgentProfile,
} from "@zcode/core";
import { scheduleWorkOrderReceiptRelay } from "./agent-dispatch-receipts.js";
import {
  computeCompletedTaskKeys,
  type TeamBoardInputRow,
} from "./team-board.js";

import {
  ProtocolRequestError,
  type ZCodeProtocolAgentServerContext,
  type ZCodeProtocolSessionRecord,
} from "./server-types.js";

export interface ProtocolAgentDispatchPortDeps {
  /** 归属会话解析器（automation-port 同款惰性绑定）：本端口所服务的 session record。 */
  resolveOwnSession?: () => ZCodeProtocolSessionRecord | undefined;
  /**
   * 既有 createSession 链（createSessionRecordForV4 包装）；由 server-operations 注入。
   * persona 缺省 = 派生无 persona 的普通会话（对齐稿 §三 #2/#5，无工牌/档案/记忆）；
   * model 是普通会话的出生常驻模型（createSession 的 model 参数），员工新段不经过它。
   * memoryEnabled=false 只由普通工位传（干净工位不读工作区记忆笔记，对齐稿 §六）。
   */
  createPersonaSessionRecord: (input: {
    workspace: ZCodeWorkspaceRef;
    persona?: ZCodeSessionPersona;
    model?: ModelSelection;
    memoryEnabled?: boolean;
  }) => Promise<{ sessionId: string }>;
  /** 冷会话恢复（activateSessionForResume 包装）；由 server-operations 注入。 */
  activateSessionRecord: (sessionId: string) => Promise<ZCodeProtocolSessionRecord>;
}

/**
 * 工单拒绝 guard id（UI 按它分流成人话，不走错误文本匹配——网关约定：
 * 携带 reasonCode 的领域错误原样上行进 ack.reasonCode，error.message 进 ack.message）。
 */
export const AGENT_WORK_ORDER_GUARDS = {
  nested: "guard.agentWorkOrderNested",
  targetNotFound: "guard.agentWorkOrderTargetNotFound",
  targetAmbiguous: "guard.agentWorkOrderTargetAmbiguous",
  selfTarget: "guard.agentWorkOrderSelfTarget",
  /** 未点名派单缺 newSession=true（core 工具 schema refine 之外的第二道墙，端口级执法）。 */
  input: "guard.agentWorkOrderInput",
  /** 频控（audit A 盲区 2026-10-05）：同会话在飞工单超上限 / 派得太勤。 */
  rate: "guard.agentWorkOrderRate",
  /** 成本熔断（audit A 盲区 2026-10-05）：全进程近窗 token 花费超安全线。 */
  cost: "guard.agentWorkOrderCost",
} as const;

// ── 派单频控 / 成本熔断（audit A 盲区，2026-10-05 老板拍板）────────────
// 三道闸全挂在既有台账上：在飞=派单台账行 admitted（回执销账即 discarded），
// 频窗=台账行 time.created，花费=store.queryAppUsage 的近窗合计。阈值取
// 「正常重活绝不触发、失控循环几分钟内必触发」的宽档；查询失败一律 fail-open
// （守的是资源护栏不是授权边界，对照 isReuseSessionRetired 同判据）。
// ponytail: 阈值硬编码，超重度的合法使用可能偏紧；升级路径=搬进会话配置。
const DISPATCH_INFLIGHT_MAX = 16;
const DISPATCH_RATE_WINDOW_MS = 10 * 60_000;
const DISPATCH_RATE_MAX = 20;
const DISPATCH_COST_WINDOW_MS = 60 * 60_000;
const DISPATCH_COST_MAX_TOTAL_TOKENS = 100_000_000;

/**
 * 派单三道闸（频控/成本熔断）：在飞上限 → 频窗上限 → 全局近窗花费熔断。
 * 自派重试（retryWorkOrderOnce）直走 runtime 不经本端口，天然不受闸；圆桌
 * 席位单经端口但一次 2~3 张，宽档下永不误伤。
 */
export async function assertDispatchBudget(
  context: Pick<ZCodeProtocolAgentServerContext, "logger">,
  store: NonNullable<ZCodeProtocolAgentServerContext["deps"]["sessionStore"]>,
  sourceSessionId: string,
): Promise<void> {
  // 用量方法按 getTaskTokenUsage/getAppUsage 同款窄视取（sqlite store 双端口同体）。
  const usageStore = store as Partial<UsageStorePort> | undefined;
  try {
    const now = Date.now();
    if (store.listSessionInputs) {
      const rows = await store.listSessionInputs({
        sessionID: sourceSessionId as SessionId,
      });
      const dispatchRows = rows.filter((row) => row.kind === "agentWorkOrderDispatch");
      // 持派行（团队看板批2 payload.held）是有意挂起的承诺，没在烧钱——不计在飞。
      const inflight = dispatchRows.filter(
        (row) =>
          row.status === "admitted" &&
          (row.payload as { held?: unknown } | undefined)?.held === undefined,
      ).length;
      if (inflight >= DISPATCH_INFLIGHT_MAX) {
        throw agentWorkOrderGuardError(
          AGENT_WORK_ORDER_GUARDS.rate,
          `Dispatch rate limit reached: ${inflight} work orders from this session are still awaiting receipts. Wait for receipts to come back before dispatching more, or report to the user that dispatching is temporarily paused.`,
        );
      }
      const recent = dispatchRows.filter(
        (row) => now - row.time.created < DISPATCH_RATE_WINDOW_MS,
      ).length;
      if (recent >= DISPATCH_RATE_MAX) {
        throw agentWorkOrderGuardError(
          AGENT_WORK_ORDER_GUARDS.rate,
          `Dispatch rate limit reached: ${recent} work orders were dispatched from this session in the last 10 minutes. Wait for some receipts before dispatching more, or report to the user that dispatching is temporarily paused.`,
        );
      }
    }
    if (usageStore?.queryAppUsage) {
      const usage = await usageStore.queryAppUsage({
        since: now - DISPATCH_COST_WINDOW_MS,
        until: now,
        tzOffsetMs: 0,
      });
      if (usage.totals.totalTokens >= DISPATCH_COST_MAX_TOTAL_TOKENS) {
        throw agentWorkOrderGuardError(
          AGENT_WORK_ORDER_GUARDS.cost,
          "Spending circuit breaker tripped: total token usage across all sessions in the last hour exceeded the safety line. Do not dispatch more work orders; report to the user and wait, or let them finish reviewing existing receipts first.",
        );
      }
    }
  } catch (error) {
    if (
      error instanceof Error &&
      (error as { name?: string }).name === "AgentWorkOrderGuardError"
    ) {
      throw error;
    }
    // 台账/用量查询失败 ≠ 派单该被拦：护栏 fail-open，留痕即可。
    context.logger?.warn("Dispatch budget check failed; failing open", {
      errorMessage: error instanceof Error ? error.message : String(error),
      event: "agent_dispatch.budget_check_failed",
      module: "bootstrap.zcode_protocol",
      sourceSessionId,
    });
  }
}

function agentWorkOrderGuardError(reasonCode: string, message: string): Error {
  return Object.assign(new Error(message), { name: "AgentWorkOrderGuardError", reasonCode });
}

/**
 * persona 快照 × 目标档案对号（照 findLatestPersonaChatRow 判据，D26）：
 * 两边都有号只认号；任一侧无号才比名字。
 */
export function personaMatchesProfile(
  persona: ZCodeSessionPersona,
  profile: Pick<AgentProfile, "name" | "agentId">,
): boolean {
  if (persona.agentId !== undefined && profile.agentId !== undefined) {
    return persona.agentId === profile.agentId;
  }
  return persona.name === profile.name;
}

/** 工作区路径归一（与 UI selectProjectAgentsForWorkspace 同判据：斜杠/盘符大小写容错）。 */
export function normalizeWorkspacePath(value: string): string {
  return value
    .replace(/\\/g, "/")
    .replace(/^[A-Za-z]:/, (drive) => drive.toLowerCase())
    .replace(/\/+$/u, "");
}

function profileToPersona(profile: AgentProfile): ZCodeSessionPersona {
  // 与 UI toProjectAgentPersona 同一映射纪律：可选字段缺席 = 跟随会话缺省；
  // memoryScope 缺省 project（档案缺 memory 字段时同约定）。
  return {
    name: profile.name,
    ...(profile.agentId ? { agentId: profile.agentId } : {}),
    systemPrompt: profile.systemPrompt,
    memoryScope: profile.memory ?? "project",
    ...(profile.modelSelection ? { modelSelection: profile.modelSelection } : {}),
    ...(profile.tools?.length ? { tools: [...profile.tools] } : {}),
    ...(profile.disallowedTools?.length ? { disallowedTools: [...profile.disallowedTools] } : {}),
    ...(profile.color ? { color: profile.color } : {}),
  };
}

/**
 * 点名派单的目标解析终审：not_found/ambiguous 在这里带 guard 词表抛出，
 * resolved 直接回档案（调用方因此拿到非空类型，无需断言）。
 */
function requireWorkOrderTarget(agent: string, profiles: readonly AgentProfile[]): AgentProfile {
  const resolution = resolveWorkOrderTarget(agent, profiles);
  if (resolution.kind === "not_found") {
    // 报错带全名单（照 selfTarget 先例）：派单方看不见名册时，这一行是它唯一
    // 能拿到正确名字的地方——拿到后重试，而不是抓个临时工冒名。
    const availableNames = profiles.map((profile) => profile.name).join(", ") || "none";
    throw agentWorkOrderGuardError(
      AGENT_WORK_ORDER_GUARDS.targetNotFound,
      `No agent profile matches "${resolution.agent}" here. Available agents: ${availableNames}. Retry with one of these exact names, or ask the user who should do the task.`,
    );
  }
  if (resolution.kind === "ambiguous") {
    throw agentWorkOrderGuardError(
      AGENT_WORK_ORDER_GUARDS.targetAmbiguous,
      `Agent "${resolution.agent}" matches multiple profiles (${resolution.matchedNames.join(", ")}). Dispatch by the agent's unique agentId instead.`,
    );
  }
  return resolution.profile;
}

/**
 * 目标最新 persona 会话：listSessions 内存过滤（persona 快照 × 档案对号 +
 * 同 workspace），按 time.updated 取最新。persona 快照随会话落盘
 * （session-store codecs select *），服务端此前没有「按 agentId 找最新 persona
 * 会话」的现成服务，这里在 bootstrap 侧补一个。
 */
export async function findLatestPersonaSessionId(
  store: NonNullable<ZCodeProtocolAgentServerContext["deps"]["sessionStore"]>,
  workspacePath: string,
  profile: Pick<AgentProfile, "name" | "agentId">,
): Promise<string | undefined> {
  // 提速（地基清理 2026-10-05，审计 C 资源 P1）：此前每次派单都 listSessions()
  // 全表扫全部会话（5k 行 × JSON 解码，几十 ms × 并发派单连发）。工位判定只认
  // interactive 会话且只要最新一间——taskTypes/limit 下推给 sqlite；task_type 过滤
  // 与内存侧的 taskType 检查同语义，limit 1000 只影响"最新工位已经旧过 1000 间
  // 更新会话"的极端边角（那时新建工位本就是更合理的选择）。已归档工位由
  // time_archived 过滤直接挡掉（归档补时间戳落地后的免费收益）。
  const sessions = await store.listSessions({
    taskTypes: ["interactive"],
    limit: 1000,
  });
  const normalizedWorkspace = normalizeWorkspacePath(workspacePath);
  let latest: { sessionId: string; updatedAt: number } | undefined;
  for (const session of sessions) {
    if (!session.persona) continue;
    if (!personaMatchesProfile(session.persona, profile)) continue;
    // workspace 护栏：目录前缀判据（同 UI），别的 workspace 的同名档案会话不参与。
    if (!normalizeWorkspacePath(session.directory).startsWith(normalizedWorkspace)) continue;
    // 工位必须落在侧栏看得见的会话上（2026-10-03 真机实证）：划词侧聊
    // （selection_side_chat）等工作会话不进主会话列表，续用它们=给员工造
    // 隐身工位——frontend-design 的席位发言交活都正常，卡片却永远找不到。
    // 只认 interactive；档案的最新交互会话不是正经工位时，落到新建分支开新的。
    if (session.taskType !== "interactive") continue;
    // 已归档的工位不参与（归档补时间戳 2026-10-04 后本进程可见）。旧数据
    // time_archived 可能仍为 NULL，兜底是复用前的 isReuseSessionRetired 反查。
    if (session.time.archived !== undefined) continue;
    const updatedAt = session.time.updated;
    if (!latest || updatedAt > latest.updatedAt) {
      latest = { sessionId: session.id, updatedAt };
    }
  }
  return latest?.sessionId;
}

/**
 * 复用前的工位退役反查：任务索引里已删/已归档的会话不再复用（2026-10-02 拍板：
 * 派单只续用侧栏看得见的工位）。tombstone/归档只记账、不动 CLI 会话行，本进程
 * 看不见，必须问 Host。查询失败 fail-open：这里守的是可见性优化而非授权边界
 * （对照 automation-port 的 fail-closed），退化为修复前的复用行为，不能让派单炸。
 */
export async function isReuseSessionRetired(
  context: Pick<ZCodeProtocolAgentServerContext, "requestClient" | "logger">,
  sessionId: string,
): Promise<boolean> {
  try {
    const result = await context.requestClient(
      zcodeProtocolMethods.agentDispatchCheckTaskRetired,
      { targetTaskId: sessionId },
      zcodeAgentDispatchCheckTaskRetiredResultSchema,
    );
    return result.retired;
  } catch (error) {
    if (error instanceof ProtocolRequestError && error.code === -32601) {
      // 旧 Host：方法未上线是能力差异，按未退役继续，两端同版本后自然生效。
      return false;
    }
    context.logger?.warn("Failed to check task retirement before dispatch reuse", {
      errorMessage: error instanceof Error ? error.message : String(error),
      event: "agentDispatch.reuse_retired_check.failed",
      sessionId,
    });
    return false;
  }
}

/**
 * 批次（工地卡）身份的进程内登记：同一发起轮内并发多张工单彼此拿不到对方的
 * 派单结果（scheduler 并行跑），batch_id 传不回去——同轮同 batch_title 必须在这里
 * 领到同一个 batchId，跨轮续批则靠模型回传 batch_id（结果回显）。键以发起轮
 * turnId 定界：新的一轮同名单独开批，绝不静默并入旧批。
 * ponytail: 登记项进程生命周期内不清（上界 ≈ 带批次的派单轮数 × 批名数，CLI 进程
 * 体量下可忽略）；若将来长驻进程要控内存，按 record 关 teardown 时清空即可。
 */
const batchMintPerTurn = new Map<string, string>();

export function createProtocolAgentDispatchPort(
  context: ZCodeProtocolAgentServerContext,
  deps: ProtocolAgentDispatchPortDeps,
): AgentDispatchPort {
  return {
    async dispatch(input: AgentDispatchRequest): Promise<AgentDispatchResult> {
      const ownRecord = deps.resolveOwnSession?.();
      if (!ownRecord) {
        throw agentWorkOrderGuardError(
          AGENT_WORK_ORDER_GUARDS.nested,
          "AgentDispatch is not bound to a session record",
        );
      }
      // 端口自检：工单唤醒轮内再派单 → 直接拒绝（照 automation-port 自检先例）。
      const activeInputId = ownRecord.app.runtime.getActiveTurnInfo()?.inputId;
      if (activeInputId?.trim().startsWith(WORK_ORDER_INPUT_ID_PREFIX)) {
        throw agentWorkOrderGuardError(
          AGENT_WORK_ORDER_GUARDS.nested,
          "Cannot dispatch a work order while running an agent work order turn.",
        );
      }

      // 频控/成本熔断（audit A 盲区 2026-10-05）：在飞上限 + 频窗 + 全局近窗花费，
      // 全挂在既有台账上（assertDispatchBudget 内 fail-open）。store 缺席的旧宿主跳过。
      if (context.deps.sessionStore) {
        await assertDispatchBudget(
          context,
          context.deps.sessionStore,
          ownRecord.app.sessionId,
        );
      }

      const profiles = ownRecord.app.runtime.getAgentProfiles();
      // 未点名派单（对齐稿 §三 #2/#5）：不带 agent 只允许 newSession=true —— 派生
      // 无 persona 普通会话；否则无落点。core 工具 schema refine 之外的端口级第二道墙。
      if (input.agent === undefined && input.newSession !== true) {
        throw agentWorkOrderGuardError(
          AGENT_WORK_ORDER_GUARDS.input,
          "Dispatching without an agent requires newSession=true: name a target agent to deliver into its resident session, or set newSession=true to spawn an unbadged ordinary session.",
        );
      }
      const namedProfile =
        input.agent === undefined ? undefined : requireWorkOrderTarget(input.agent, profiles);

      const sourcePersona = ownRecord.app.runtime.getProjectAgentPersona();
      // 自派拒绝（对齐稿 §三规则 6）：员工给自己派工单只会复制自己的对话，
      // 构造信封前端口级硬墙。错误信息教会模型改道：换人/直接干。
      // 仅点名员工的派单有「自派」可言；无 persona 普通会话没有目标档案。
      if (namedProfile && isSelfDispatch(sourcePersona, namedProfile)) {
        const otherNames = profiles
          .filter((candidate) => candidate !== namedProfile)
          .map((candidate) => candidate.name);
        throw agentWorkOrderGuardError(
          AGENT_WORK_ORDER_GUARDS.selfTarget,
          `You cannot dispatch a work order to yourself ("${namedProfile.name}") — that would just copy your own conversation. Dispatch to another agent instead (available: ${
            otherNames.join(", ") || "none - ask the user"
          }), or do the task directly in this conversation.`,
        );
      }
      // 批次（工地卡）身份：模型回传 batch_id（上次结果的 batchId）即并批；只有
      // batch_title 时按「发起轮 × 批名」领号——同轮并派的多张工单共享一批，跨轮
      // 续批必须显式回传 batch_id（新轮同名单独开批）。不带 batch_title 是散单。
      const batchTitle = input.batchTitle?.trim();
      const activeTurnId = ownRecord.app.runtime.getActiveTurnInfo()?.turnId;
      let batchId: string | undefined;
      if (batchTitle) {
        if (input.batchId?.trim()) {
          batchId = input.batchId.trim();
        } else {
          const mintKey = `${ownRecord.app.sessionId}|${activeTurnId ?? "no-turn"}|${batchTitle}`;
          let minted = activeTurnId ? batchMintPerTurn.get(mintKey) : undefined;
          if (!minted) {
            minted = crypto.randomUUID();
            if (activeTurnId) batchMintPerTurn.set(mintKey, minted);
          }
          batchId = minted;
        }
      }
      const envelope: AgentWorkOrderEnvelope = {
        workOrderId: crypto.randomUUID(),
        ...(sourcePersona?.agentId ? { fromAgentId: sourcePersona.agentId } : {}),
        fromAgentName: sourcePersona?.name ?? "",
        fromSessionId: ownRecord.app.sessionId,
        task: input.task,
        ...(batchId ? { batchId } : {}),
        ...(batchTitle ? { batchTitle } : {}),
        // 批内符号名与依赖（团队看板 2026-10-05）：逐字随信封落台账，看板快照
        // 与批2 解锁调度据它对号。
        ...(input.taskKey ? { taskKey: input.taskKey } : {}),
        ...(input.dependsOn?.length ? { dependsOn: [...input.dependsOn] } : {}),
        // 评审单标记（评审会批）：随信封落台账——批次收口时触发侧据它判合议口味。
        ...(input.review === true ? { review: true } : {}),
        // 返修链（团队看板批2）：reviews=被审工单 id（模型透传）；repairOf/repairRound
        // 由调度器直派铸造——三者随信封落台账，看板与调度器据它们对账。
        ...(input.reviews ? { reviews: input.reviews } : {}),
        ...(input.repairOf ? { repairOf: input.repairOf } : {}),
        ...(input.repairRound !== undefined ? { repairRound: input.repairRound } : {}),
        // 圆桌会席位单（真会议，2026-10-03）：council* 五参逐字随信封落台账——
        // 发起方的收票推进（maybeAdvanceCouncilRound）按 councilId:round 聚拢席位单、
        // 回执轮头 originMeta.council* 据它对号。与 batchId 互斥（召集方代码保证）。
        ...(input.councilId
          ? {
              councilId: input.councilId,
              ...(input.councilKind ? { councilKind: input.councilKind } : {}),
              ...(input.councilRound ? { councilRound: input.councilRound } : {}),
              ...(input.councilSeatIndex !== undefined
                ? { councilSeatIndex: input.councilSeatIndex }
                : {}),
              ...(input.councilSeatLens ? { councilSeatLens: input.councilSeatLens } : {}),
            }
          : {}),
      };

      // 人类短标题（模型派单时给的 title；缺席回落任务正文）：工位落行首输入、
      // 无名工位的回执署名都用它——回执卡不再退化成「智能体 交活」（评审低危⑤）。
      const workerTitleSeed = input.title?.trim() || input.task;

      // 员工默认模型护栏（2026-10-01，真机事故 9-30 员工配了下线模型工单三连炸）：
      // 派单当场校验模型在不在注册表里。2026-10-02 拍板修订：**默认跟随发起会话
      // 当前模型**——员工档案自配的模型不再自动候选（各档案押不同供应商，有的
      // 一分钱不剩，默认分散不合理）；要跑某个特定模型，派单时明说（本单指定）。
      const fallbackModel = ownRecord.app.runtime.getSessionModelSelection();
      const getModelOption = ownRecord.app.getModelOption;
      const isModelAvailable = getModelOption
        ? (selection: ModelSelection) => getModelOption(selection) !== undefined
        : undefined;
      // 新段出生模型：本单指定 → 跟随发起会话当前模型。落到 persona 快照/出生
      // 常驻上的就是工单实际要跑的模型，回执与日志对账同源。
      const birthModel = resolveDispatchModelSelection({
        requested: input.modelSelection,
        fallback: fallbackModel,
        isModelAvailable,
      });

      let targetSessionId: string | undefined;
      let createdSession = false;
      if (namedProfile && input.newSession !== true) {
        targetSessionId = context.deps.sessionStore
          ? await findLatestPersonaSessionId(
              context.deps.sessionStore,
              ownRecord.workspace.workspacePath,
              namedProfile,
            )
          : undefined;
        if (targetSessionId && (await isReuseSessionRetired(context, targetSessionId))) {
          // 工位已收走（任务已删/已归档，侧栏看不见）：不复用隐身会话，
          // 落到下面的新建分支开新工位。
          targetSessionId = undefined;
        }
      }
      if (!targetSessionId) {
        // 既有 createSession persona 链开新段；工作区用发起方自己的 workspace ref
        // （一个 CLI 进程一个 workspace，跨 workspace 目标在这里就不存在）。
        // 本单指定模型（D32）覆盖档案默认：员工新段从出生就带着它（persona
        // modelSelection 走 create 路径落会话常驻选择），后续轮不回退。
        // 可见性（真机反馈 2026-09-30）：用户点名“新建一个会话”就是要看得见的新
        // 会话——不盖 workOrderOnly（隐藏机制保留备用）；派单堆积的真正解法是
        // 复用最近会话（不带 newSession 的派单永远不新增行）+ 守住 CLI 后门。
        // 未点名（对齐稿 §三 #2/#5）：不带 persona 开普通会话——无工牌/档案/记忆，
        // 不占任何员工身份；指定模型经 createSession 的 model 参数成为出生常驻模型；
        // memoryEnabled=false 是工位隔离（真机事故：worker 读到老板记忆笔记，
        // 把"发你好"干成了"今天想干点啥"）——干净工位不读老板的记忆。
        const created = await deps.createPersonaSessionRecord(
          namedProfile
            ? {
                workspace: ownRecord.workspace,
                persona: {
                  ...profileToPersona(namedProfile),
                  ...(birthModel.modelSelection === undefined
                    ? {}
                    : { modelSelection: birthModel.modelSelection }),
                },
              }
            : {
                workspace: ownRecord.workspace,
                ...(birthModel.modelSelection === undefined
                  ? {}
                  : { model: birthModel.modelSelection }),
                memoryEnabled: false,
              },
        );
        targetSessionId = created.sessionId;
        createdSession = true;
      }

      // 冷会话先恢复再提交（照 v4-bridge 冷恢复先例）；对已在场 record 幂等返回。
      const targetRecord =
        context.sessions.get(targetSessionId) ??
        (await deps.activateSessionRecord(targetSessionId));
      // 工位卡标题随最近一张工单刷新（2026-10-03 老板验收反馈）：续用的旧工位标题
      // 可能还挂着建桌时的杂题——真机实证 frontend-design 的工位卡叫 "Selection
      // side chat"，老板找不到第四席去哪了（席位其实发言交活都正常）。新工位本来
      // 就落「智能体名 · 短标题」，续用工位对齐同一格式；工单标题盖过旧杂题是有意
      // 行为：工位卡语义 = 这个员工最近在干的活。
      if (namedProfile && !createdSession) {
        await targetRecord.app
          .setCustomSessionTitle({
            title: `${namedProfile.name} · ${workerTitleSeed.slice(0, 60)}`,
            traceContext: targetRecord.traceContext,
          })
          .catch((error) => {
            context.logger?.warn("Failed to refresh reused desk title", {
              errorMessage: error instanceof Error ? error.message : String(error),
              event: "agent_dispatch.desk_title_refresh_failed",
              module: "bootstrap.zcode_protocol",
              targetSessionId,
            });
          });
      }
      // 复用会话同样跟随发起会话当前模型（2026-10-02 拍板）：前口径是「会话常驻
      // 有效就不传 per-order 覆盖、让会话自己跑」，那等于员工各押各的供应商——
      // 现在除了本单指定，一律带 per-order 覆盖（老板当前模型），员工会话自己的
      // 常驻只管它自己的闲聊轮。指定了已下线模型 → 回落老板当前模型并留痕。
      const dispatchModel = resolveDispatchModelSelection({
        requested: input.modelSelection,
        fallback: fallbackModel,
        isModelAvailable,
      });
      const enqueueModelSelection = dispatchModel.modelSelection;
      if (dispatchModel.unavailable) {
        context.logger?.warn("Agent work order model unavailable and no fallback", {
          event: "agent_dispatch.model_fallback",
          module: "bootstrap.zcode_protocol",
          reason: dispatchModel.reason,
          targetSessionId,
          workOrderId: envelope.workOrderId,
        });
      } else if (
        dispatchModel.source === "fallback" &&
        dispatchModel.reason === "unavailable" &&
        dispatchModel.modelSelection
      ) {
        context.logger?.info("Agent work order model fell back to dispatcher's current model", {
          event: "agent_dispatch.model_fallback",
          module: "bootstrap.zcode_protocol",
          reason: dispatchModel.reason,
          targetSessionId,
          toModel: dispatchModel.modelSelection.modelId,
          workOrderId: envelope.workOrderId,
        });
      }
      // 真机事故修复（2026-09-29）：新建的 persona 会话此前不落行，draft 态不进会话区
      // （侧栏看不见“新开的会话”），且冷恢复按 id 查库落空。工单是外部活动，落行语义
      // 照既有的 ensureSessionPersistedForExternalActivity——首输入用人类短标题
      // （模型派单时给的 title；缺席回落任务正文），标题因此是「智能体名 · 短标题」。
      // 投递前后都幂等（sessionPersisted 置位即返回）。
      await targetRecord.app.runtime.ensureSessionPersistedForExternalActivity(
        workerTitleSeed,
        { traceContext: targetRecord.traceContext },
      );
      // 隔离工位标记落盘（审计 2026-10-01 P1）：无名临时工的 memoryEnabled=false
      // 原来只是 create 期一次性传参，重启后 resume 按全局开关恢复、员工又读到老板
      // 记忆。行上落 memory_isolation（首写为准），resume 读到即重建隔离；有 persona
      // 的员工会话隔离本就随快照穿越重启，不走这条路。
      if (createdSession && namedProfile === undefined && context.deps.sessionStore) {
        await context.deps.sessionStore
          .updateSession({ id: targetRecord.app.sessionId, memoryIsolated: true })
          .catch((error) => {
            context.logger?.warn("Failed to persist memory isolation flag", {
              errorMessage: error instanceof Error ? error.message : String(error),
              event: "agent_dispatch.memory_isolation_flag_failed",
              module: "bootstrap.zcode_protocol",
              targetSessionId,
            });
          });
      }
      // 依赖持派（团队看板批2）：信封带 dependsOn 且有前置未 completed → 有意挂起。
      // 落台账行（payload.held 记未满足的批内符号名）但**不投目标、不接回执线**——
      // 前置全部 completed 后由回执销账钩子（team-scheduler）以同一 workOrderId 释放
      // 投递。依赖 honored 与「自动流转」开关无关；台账查询失败 fail-open 照常直派。
      let heldPendingKeys: string[] | undefined;
      if (input.dependsOn?.length && context.deps.sessionStore?.listSessionInputs) {
        try {
          const dispatchRows = (await context.deps.sessionStore.listSessionInputs({
            sessionID: ownRecord.app.sessionId as SessionId,
          })) as unknown as TeamBoardInputRow[];
          const completed = computeCompletedTaskKeys(dispatchRows);
          const pending = input.dependsOn.filter((key) => !completed.has(key));
          if (pending.length > 0) heldPendingKeys = pending;
        } catch (error) {
          context.logger?.warn("Dependency hold check failed; dispatching without hold", {
            errorMessage: error instanceof Error ? error.message : String(error),
            event: "agent_dispatch.hold_check_failed",
            module: "bootstrap.zcode_protocol",
            workOrderId: envelope.workOrderId,
          });
        }
      }
      if (heldPendingKeys) {
        try {
          context.deps.sessionStore?.saveSessionInput?.({
            id: `agentWorkOrderDispatch:${envelope.workOrderId}`,
            sessionID: ownRecord.app.sessionId,
            kind: "agentWorkOrderDispatch",
            delivery: "queue",
            payload: {
              text: envelope.task,
              workOrderId: envelope.workOrderId,
              agentName: namedProfile?.name ?? workerTitleSeed,
              ...(namedProfile?.agentId ? { agentId: namedProfile.agentId } : {}),
              targetSessionId,
              ...(enqueueModelSelection
                ? {
                    model: enqueueModelSelection.modelId,
                    modelSelection: enqueueModelSelection,
                  }
                : {}),
              held: { pendingKeys: heldPendingKeys },
              envelope,
            },
          });
        } catch (ledgerError) {
          context.logger?.warn("Failed to admit held work order to ledger", {
            errorMessage: ledgerError instanceof Error ? ledgerError.message : String(ledgerError),
            event: "agent_dispatch.held_ledger_admit_failed",
            module: "bootstrap.zcode_protocol",
            workOrderId: envelope.workOrderId,
          });
        }
        context.logger?.info("Agent work order held pending dependencies", {
          event: "agent_dispatch.held",
          module: "bootstrap.zcode_protocol",
          pendingKeys: heldPendingKeys,
          targetSessionId,
          workOrderId: envelope.workOrderId,
        });
        return {
          targetSessionId,
          agentName: namedProfile?.name ?? "",
          ...(namedProfile?.agentId ? { agentId: namedProfile.agentId } : {}),
          delivery: "queued",
          createdSession,
          ...(input.modelSelection ? { model: input.modelSelection.modelId } : {}),
          workOrderId: envelope.workOrderId,
          ...(batchId ? { batchId } : {}),
          ...(batchTitle ? { batchTitle } : {}),
          held: heldPendingKeys,
        };
      }
      const admission = await targetRecord.app.runtime.enqueueAgentWorkOrder({
        envelope,
        traceContext: targetRecord.traceContext,
        ...(enqueueModelSelection === undefined
          ? {}
          : { modelSelection: enqueueModelSelection }),
      });
      // 完成钩子（D29/D3）：目标轮 TurnComplete/TurnError（按 workorder- inputId 对号）
      // 后把携带最终答案的回执投回发起方会话。投递成功 ≠ 已处理；本轮终态见回执。
      // 失败先自动重试一次（员工可靠性批），重试沿用本单生效的模型覆盖。
      scheduleWorkOrderReceiptRelay(context, deps, {
        targetRecord,
        inputId: admission.inputId,
        envelope,
        // 无名工位的回执署名用会话标题（"发个你好 交活"），不再退化成「智能体 交活」。
        agentName: namedProfile?.name ?? workerTitleSeed,
        ...(namedProfile?.agentId ? { agentId: namedProfile.agentId } : {}),
        ...(enqueueModelSelection === undefined
          ? {}
          : { modelSelection: enqueueModelSelection }),
      });
      // 派单台账（audit 2026-10-01 对账批）：发起方账本落下「已派未回」的凭据
      // （完整信封 + 工号 + 归还地址）。回执丢失/崩溃后，resume 清扫据此合成
      // 失败回执——钱花了账上必须有。best-effort：落不进账本不影响派单本身。
      try {
        context.deps.sessionStore?.saveSessionInput?.({
          id: `agentWorkOrderDispatch:${envelope.workOrderId}`,
          sessionID: ownRecord.app.sessionId,
          kind: "agentWorkOrderDispatch",
          delivery: "queue",
          payload: {
            text: envelope.task,
            workOrderId: envelope.workOrderId,
            agentName: namedProfile?.name ?? workerTitleSeed,
            // 员工工号（批次质检打回重派按号点名，改名不误派）；无名工位没有工号。
            ...(namedProfile?.agentId ? { agentId: namedProfile.agentId } : {}),
            targetSessionId,
            // 本单生效模型（团队看板 2026-10-05）：看板每张任务卡标注用哪个模型；
            // modelSelection 完整结构留给重派/持派释放（批2）按原模型投递。
            ...(enqueueModelSelection
              ? {
                  model: enqueueModelSelection.modelId,
                  modelSelection: enqueueModelSelection,
                }
              : {}),
            envelope,
          },
        });
      } catch (ledgerError) {
        context.logger?.warn("Failed to admit agent work order dispatch to ledger", {
          errorMessage: ledgerError instanceof Error ? ledgerError.message : String(ledgerError),
          event: "agent_dispatch.ledger_admit_failed",
          module: "bootstrap.zcode_protocol",
          workOrderId: envelope.workOrderId,
        });
      }
      context.logger?.info("Agent work order dispatched", {
        createdSession,
        delivery: admission.delivery,
        event: "agent_dispatch.port.dispatched",
        fromSessionId: envelope.fromSessionId,
        module: "bootstrap.zcode_protocol",
        targetSessionId,
        unbadged: namedProfile === undefined,
        workOrderId: envelope.workOrderId,
      });
      return {
        targetSessionId,
        agentName: namedProfile?.name ?? "",
        ...(namedProfile?.agentId ? { agentId: namedProfile.agentId } : {}),
        delivery: admission.delivery,
        createdSession,
        ...(input.modelSelection
          ? { model: input.modelSelection.modelId }
          : {}),
        // 批次回显（core 工具原样透传给模型）：batch_id 回传即往同一工地续派。
        workOrderId: envelope.workOrderId,
        ...(batchId ? { batchId } : {}),
        ...(batchTitle ? { batchTitle } : {}),
        // 圆桌会身份回显（席位单必在）：召集方代码据它对账收票进度。
        ...(envelope.councilId ? { councilId: envelope.councilId } : {}),
      };
    },
  };
}
