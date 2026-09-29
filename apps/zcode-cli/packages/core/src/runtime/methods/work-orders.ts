// ============================================================
// 派单（D29/D2）工单投递：core 侧的窄公共入口与唤醒轮执行。
// bootstrap 协议端口解析目标/定位会话后，经公共 AgentRuntime 接口投递；
// 目标忙 → 命令在 runtime 命令队列排队，回合边界独立成轮（绝不打断进行中回合）。
// ============================================================

import { createMessageId, traceContextToLogContext } from "../deps.js";
import type { QueryId, TraceContext } from "../deps.js";
import type { AgentWorkOrderEnvelope } from "@zcode/contracts";
import { WORK_ORDER_INPUT_ID_PREFIX, boundAgentWorkOrderMeta } from "@zcode/contracts";
import type { ModelSelection } from "@zcode/shared";
import { createRuntimeCommandId, type WorkOrderRuntimeCommand } from "../command-queue.js";
import { runtimeInputMetadata } from "../../agent/runtime-input-presentation.js";
import {
  buildWorkOrderEnvelopeText,
  WORK_ORDER_RESTRICTED_TOOL_NAMES,
} from "../../subagent/work-order.js";
import type { AgentRuntimeInternal } from "../internal.js";
import { isStaleBranchRuntimeCommand } from "./runtime-command-generation.js";
import { beginForegroundExecution, finishForegroundExecution } from "./runtime-command-queue.js";

export interface EnqueueAgentWorkOrderResult {
  /** started = 空闲即刻成轮；queued = 目标忙，已按既有消息排队。投递成功 ≠ 已处理。 */
  delivery: "started" | "queued";
  inputId: string;
  workOrderId: string;
}

/**
 * 公共入口（AgentRuntime 面上）：把一条工单 carrier 投进本会话。
 * 与 enqueueBackgroundTaskNotification 同型（排队 + 账本 admission），但来源是
 * agent_work_order 且独占成轮；崩溃后残余 admission 照既有收口语义在 resume 时
 * 落 discarded——工单在死进程里本就不可能再跑，发起侧拿到的是诚实口径的受理。
 */
export async function enqueueAgentWorkOrder(
  this: AgentRuntimeInternal,
  input: {
    envelope: AgentWorkOrderEnvelope;
    traceContext?: TraceContext;
    /** 本单一次性指定模型（注册表拼写）；只造工单轮的 Model，不改写目标会话常驻选择。 */
    modelSelection?: ModelSelection;
  },
): Promise<EnqueueAgentWorkOrderResult> {
  const traceContext = input.traceContext ?? this.rootTraceContext;
  if (this.shuttingDown) {
    // teardown 期间不能再启动模型轮次；工单如实在发起侧报失败。
    throw new Error("Cannot accept an agent work order during session shutdown");
  }
  const workOrderId = input.envelope.workOrderId;
  const inputId = `${WORK_ORDER_INPUT_ID_PREFIX}${workOrderId}`;
  const text = buildWorkOrderEnvelopeText(input.envelope);
  const delivery: EnqueueAgentWorkOrderResult["delivery"] = this.hasActiveOrQueuedTurnWork()
    ? "queued"
    : "started";
  const command: WorkOrderRuntimeCommand = {
    branchGeneration: this.branchGeneration,
    createdAt: new Date(),
    envelope: input.envelope,
    id: createRuntimeCommandId(),
    inputId,
    mode: "work-order",
    priority: "next",
    source: "agent_work_order",
    text,
    traceContext,
    workOrderId,
    ...(input.modelSelection === undefined ? {} : { modelSelection: input.modelSelection }),
  };
  this.enqueueRuntimeCommand(command);
  // 账本 admission（durable 痕迹）：runtime 命令队列是纯内存的，崩溃后由 resume 统一收口。
  const admission = this.sessionStore?.saveSessionInput?.({
    id: String(command.id),
    sessionID: this.sessionId,
    kind: "agentWorkOrder",
    delivery: "queue",
    payload: {
      text,
      workOrderId,
      envelope: input.envelope,
    },
  });
  if (admission) {
    void this.trackResidencyBlockingWork(admission).catch((error) => {
      this.logger?.warn("Failed to admit agent work order to ledger", {
        ...traceContextToLogContext(traceContext),
        errorMessage: error instanceof Error ? error.message : String(error),
        event: "session_input.admit_failed",
        module: "core.runtime",
        status: "failed",
        workOrderId,
      });
    });
  }
  this.logger?.info("Agent work order enqueued", {
    ...traceContextToLogContext(traceContext),
    delivery,
    event: "agent_work_order.enqueued",
    fromSessionId: input.envelope.fromSessionId,
    inputId,
    module: "core.runtime",
    sessionId: this.sessionId,
    workOrderId,
  });
  return { delivery, inputId, workOrderId };
}

/**
 * 工单唤醒轮：落库 synthetic notice（provider 可见、UI 不画气泡）→ 以工单身份
 * 独立成轮。轮上 options.workOrderId + turn denylist 双信号进入执行边界终审，
 * 端口自检兜底（照 automation 三件套），嵌套上限=1。
 */
export async function runWorkOrderCommand(
  this: AgentRuntimeInternal,
  command: WorkOrderRuntimeCommand,
): Promise<void> {
  if (isStaleBranchRuntimeCommand(this, command)) return;
  const foregroundExecution = beginForegroundExecution.call(this, command);
  try {
    const messageID = createMessageId();
    // 工单卡元数据（D29/D5）：落库与 TurnStarted 用**同一份有界值**（boundAgentWorkOrderMeta
    // 的铸造侧承诺）。工具路径的 task 没有上限，信封若原样落库，冷恢复的
    // agentWorkOrderMetaSchema（max 8192）会解析失败回 undefined、重启后工单卡消失——
    // 活投影侧有界所以卡在，冷热就此分叉。元数据只喂 UI 卡（回执对账走内存信封），
    // 截断不影响模型面：carrier（command.text）与模型历史仍是全文。
    const agentWorkOrderMeta = boundAgentWorkOrderMeta({
      workOrderId: command.envelope.workOrderId,
      fromAgentName: command.envelope.fromAgentName,
      ...(command.envelope.fromAgentId ? { fromAgentId: command.envelope.fromAgentId } : {}),
      fromSessionId: command.envelope.fromSessionId,
      task: command.envelope.task,
    });
    // 内存历史与持久化同一份原文（<work-order> 信封）；执行期投影再补
    // 「非用户权威」框架（provider-entry-origins 的 agent_work_order 分支）。
    this.messageHistory.addUser(command.text, runtimeInputMetadata("agent_work_order"));
    await this.persistSyntheticUserNoticeForSession({
      messageID,
      metadata: {
        envelope: agentWorkOrderMeta,
        inputPresentation: "agent_work_order",
        visibility: "model-only",
      },
      sessionId: this.sessionId,
      source: "agent_work_order",
      text: command.text,
      traceContext: command.traceContext,
      visibility: "model-only",
    });
    await this.sessionStore
      ?.markSessionInputPromoted?.({
        id: String(command.id),
        sessionID: this.sessionId,
        promotedMessageID: messageID,
      })
      .catch((error) => {
        this.logger?.warn("Failed to mark agent work order promoted", {
          ...traceContextToLogContext(command.traceContext),
          commandId: command.id,
          errorMessage: error instanceof Error ? error.message : String(error),
          event: "session_input.promote_mark_failed",
          module: "core.runtime",
          status: "failed",
          workOrderId: command.workOrderId,
        });
      });
    this.logger?.info("Agent work order turn started", {
      ...traceContextToLogContext(command.traceContext),
      commandId: command.id,
      event: "agent_work_order.turn_started",
      inputId: command.inputId,
      messageId: messageID,
      module: "core.runtime",
      workOrderId: command.workOrderId,
    });
    // 侧栏标题来源：persona 会话标题 =「智能体名 · 首条输入」且二次生成已关
    // （ensureSessionPersisted 吃 displayInput），不传 displayInput 会让首次派单开出的
    // 新会话标题铸自英文 carrier 噪音（[AGENT WORK ORDER - NOT USER INPUT] …），
    // 且 persona 会话改不回来。任务正文才是人话标题；displayInput 只进 TurnStarted
    // 与标题——本轮 model-only + skipInputRecord，投影不会因此冒用户行，
    // plugin/点将的 mention 解析也被 turn.ts 的 model-only 门挡住。
    const displayTask = command.envelope.task.trim();
    await this.executeTurnCommand(command.text, undefined, {
      abortSignal: foregroundExecution.controller.signal,
      inputId: command.inputId,
      inputPresentation: "agent_work_order",
      inputSource: "agent_work_order",
      inputVisibility: "model-only",
      // workorder- 前缀的 inputId 即本轮 queryId（turn-loop 前缀兜底信号）。
      queryId: command.inputId as QueryId,
      recordedInputMessageId: messageID,
      skipInputRecord: true,
      skipUserPromptSubmitHooks: true,
      toolDisallowlist: [...WORK_ORDER_RESTRICTED_TOOL_NAMES],
      traceContext: command.traceContext,
      workOrderId: command.workOrderId,
      // 工单卡元数据（D29/D5）：随 TurnStarted 下发，目标会话画「来自 X 的任务」卡。
      agentWorkOrder: agentWorkOrderMeta,
      ...(displayTask ? { displayInput: displayTask } : {}),
    });
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    this.logger?.warn("Agent work order turn failed", {
      ...traceContextToLogContext(command.traceContext),
      commandId: command.id,
      errorMessage: reason,
      event: "agent_work_order.turn_failed",
      module: "core.runtime",
      workOrderId: command.workOrderId,
    });
    // 工单失败也要在目标会话留终态（真机教训：轮死在 provider 风控时，会话里
    // 只剩工单卡和一个没有终态的空轮，打开像"卡住了"）。model-only：模型下轮
    // 知道上一单为什么没成；用户侧的失败告知由发起方会话的回执承担（D3）。
    try {
      const failureMessageId = createMessageId();
      const failureText = `Work order execution failed: ${reason}`;
      this.messageHistory.addUser(failureText, runtimeInputMetadata("agent_work_order"));
      await this.persistSyntheticUserNoticeForSession({
        messageID: failureMessageId,
        metadata: {
          inputPresentation: "agent_work_order",
          visibility: "model-only",
        },
        sessionId: this.sessionId,
        source: "agent_work_order",
        text: failureText,
        traceContext: command.traceContext,
        visibility: "model-only",
      });
    } catch (noticeError) {
      // 终态留痕失败不能覆盖原始失败原因；回执链已经把失败带给发起方。
      this.logger?.warn("Failed to persist agent work order failure notice", {
        ...traceContextToLogContext(command.traceContext),
        commandId: command.id,
        errorMessage: noticeError instanceof Error ? noticeError.message : String(noticeError),
        event: "agent_work_order.failure_notice_failed",
        module: "core.runtime",
        status: "failed",
        workOrderId: command.workOrderId,
      });
    }
  } finally {
    finishForegroundExecution.call(this, foregroundExecution);
  }
}
