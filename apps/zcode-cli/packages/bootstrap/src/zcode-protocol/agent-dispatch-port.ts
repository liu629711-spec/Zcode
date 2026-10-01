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
} from "@zcode/contracts";
import type { ModelSelection, ZCodeSessionPersona, ZCodeWorkspaceRef } from "@zcode/shared";
import {
  isSelfDispatch,
  resolveDispatchModelSelection,
  resolveWorkOrderTarget,
  type AgentProfile,
} from "@zcode/core";
import { scheduleWorkOrderReceiptRelay } from "./agent-dispatch-receipts.js";

import type {
  ZCodeProtocolAgentServerContext,
  ZCodeProtocolSessionRecord,
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
} as const;

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
  const sessions = await store.listSessions();
  const normalizedWorkspace = normalizeWorkspacePath(workspacePath);
  let latest: { sessionId: string; updatedAt: number } | undefined;
  for (const session of sessions) {
    if (!session.persona) continue;
    if (!personaMatchesProfile(session.persona, profile)) continue;
    // workspace 护栏：目录前缀判据（同 UI），别的 workspace 的同名档案会话不参与。
    if (!normalizeWorkspacePath(session.directory).startsWith(normalizedWorkspace)) continue;
    const updatedAt = session.time.updated;
    if (!latest || updatedAt > latest.updatedAt) {
      latest = { sessionId: session.id, updatedAt };
    }
  }
  return latest?.sessionId;
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
      };

      // 人类短标题（模型派单时给的 title；缺席回落任务正文）：工位落行首输入、
      // 无名工位的回执署名都用它——回执卡不再退化成「智能体 交活」（评审低危⑤）。
      const workerTitleSeed = input.title?.trim() || input.task;

      // 员工默认模型护栏（2026-10-01，真机事故 9-30 员工配了下线模型工单三连炸）：
      // 派单当场校验模型在不在注册表里，没配/已下线 → 回落主会话当前模型（老板
      // 正用它说话，必然可用），别等工单轮跑起来才被供应商拒收。
      const fallbackModel = ownRecord.app.runtime.getSessionModelSelection();
      const getModelOption = ownRecord.app.getModelOption;
      const isModelAvailable = getModelOption
        ? (selection: ModelSelection) => getModelOption(selection) !== undefined
        : undefined;
      // 新段出生模型：本单指定（D32）→ 档案默认 → 回落。落到 persona 快照/出生
      // 常驻上的就是工单实际要跑的模型，回执与日志对账同源。
      const birthModel = resolveDispatchModelSelection({
        requested: input.modelSelection,
        ambient: namedProfile?.modelSelection,
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
      // 复用会话的护栏：本单指定 / 会话常驻过了校验才放行。ambient（会话常驻）
      // 有效时**不传** per-order 覆盖——让会话自己的常驻模型跑，保持既有语义；
      // 只有指定/回落才带 modelSelection。
      const dispatchModel = resolveDispatchModelSelection({
        requested: input.modelSelection,
        ambient: targetRecord.app.runtime.getSessionModelSelection(),
        fallback: fallbackModel,
        isModelAvailable,
      });
      const enqueueModelSelection =
        dispatchModel.source === "ambient" ? undefined : dispatchModel.modelSelection;
      if (dispatchModel.unavailable) {
        context.logger?.warn("Agent work order model unavailable and no fallback", {
          event: "agent_dispatch.model_fallback",
          module: "bootstrap.zcode_protocol",
          reason: dispatchModel.reason,
          targetSessionId,
          workOrderId: envelope.workOrderId,
        });
      } else if (dispatchModel.source === "fallback" && dispatchModel.modelSelection) {
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
      };
    },
  };
}
