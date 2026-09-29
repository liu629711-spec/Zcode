import type { TraceContext, TurnState } from "./deps.js";
import type {
  AgentWorkOrderEnvelope,
  BackgroundResultOriginMeta,
  WorkflowLaunchMeta,
} from "@zcode/contracts";
import type { WorkOrderReceiptOutcome } from "../subagent/work-order.js";
import type {
  ActiveTurnStartReservation,
  ContinueActiveTargetLoopOptions,
  ExecuteTurnOptions,
  TurnResult,
} from "./types.js";

export type RuntimeCommandPriority = "now" | "next" | "later";
export type RuntimeCommandMode =
  | "prompt"
  | "target-continuation"
  | "target-continuation-loop"
  | "task-notification"
  | "subagent-message"
  | "work-order"
  | "work-order-receipt"
  | "control-only-turn";
export type RuntimeCommandId = string & {
  readonly __runtimeCommandId: unique symbol;
};

export interface RuntimeCommandBase {
  readonly createdAt: Date;
  readonly id: RuntimeCommandId;
  readonly mode: RuntimeCommandMode;
  readonly priority: RuntimeCommandPriority;
  readonly traceContext: TraceContext;
}

export interface PromptRuntimeCommand extends RuntimeCommandBase {
  readonly attachments?: TurnState["attachments"];
  readonly input: string;
  readonly mode: "prompt";
  readonly options?: ExecuteTurnOptions;
  /** admission 已建立的 reservation；执行阶段不得再次创建/竞争 turn。 */
  readonly startReservation?: ActiveTurnStartReservation;
  readonly reject: (error: unknown) => void;
  readonly resolve: (result: TurnResult) => void;
}

export interface TargetContinuationRuntimeCommandOptions {
  readonly abortSignal?: AbortSignal;
  readonly inputId?: string;
  readonly intent?: ExecuteTurnOptions["intent"];
  readonly traceContext: TraceContext;
  readonly verifyBeforeContinue?: boolean;
}

export interface TargetContinuationRuntimeCommand extends RuntimeCommandBase {
  readonly mode: "target-continuation";
  readonly options: TargetContinuationRuntimeCommandOptions;
  readonly reject: (error: unknown) => void;
  readonly resolve: (result: TurnResult | null) => void;
}

export interface TargetContinuationLoopRuntimeCommand extends RuntimeCommandBase {
  readonly mode: "target-continuation-loop";
  readonly options: ContinueActiveTargetLoopOptions & {
    readonly traceContext: TraceContext;
  };
  readonly reject: (error: unknown) => void;
  readonly resolve: (result: TurnResult | null) => void;
}

export interface TaskNotificationRuntimeCommand extends RuntimeCommandBase {
  readonly branchGeneration: number;
  readonly mode: "task-notification";
  readonly source: "background_task";
  readonly originMeta?: BackgroundResultOriginMeta;
  readonly taskId?: string;
  readonly text: string;
  readonly toolName?: string;
}

export interface SubagentMessageRuntimeCommand extends RuntimeCommandBase {
  readonly branchGeneration: number;
  readonly mode: "subagent-message";
  readonly source: "subagent_message";
  readonly responseId: string;
  readonly agentId: string;
  readonly agentType: string;
  readonly childSessionId: string;
  readonly childToolCallId: string;
  readonly parentToolCallId?: string;
  readonly summary: string;
  readonly messageLength: number;
  readonly text: string;
}

/**
 * 派单工单命令（D29/D2）：目标会话忙时在 runtime 命令队列排队，回合边界取件
 * 独立成轮（绝不打断进行中回合）。与 task-notification 同一家族但**刻意不合并批次**：
 * 工单轮必须携带自己的身份（workOrderId/inputId）与 tool denylist（嵌套上限=1）。
 */
export interface WorkOrderRuntimeCommand extends RuntimeCommandBase {
  readonly branchGeneration: number;
  readonly mode: "work-order";
  readonly source: "agent_work_order";
  /** 工单身份（uuid）；唤醒轮 options.workOrderId 携带同一值。 */
  readonly workOrderId: string;
  /** 唤醒轮 inputId/queryId，`workorder-<workOrderId>` 派生。 */
  readonly inputId: string;
  /** 信封结构体（落库 message metadata 供 UI 卡片/回执对账）。 */
  readonly envelope: AgentWorkOrderEnvelope;
  /** 信封拼装后的工单正文（<work-order> 包裹）。 */
  readonly text: string;
  /** 本单一次性指定模型（注册表拼写）；唤醒轮只造本轮 Model，不改写目标会话常驻选择。 */
  readonly modelSelection?: {
    providerId: string;
    modelId: string;
    options?: { reasoningLevel?: string };
  };
}

/**
 * 派单回执命令（D29/D3）：目标轮结束后投回发起方会话的完成通知。与 work-order
 * 同家族但独立成轮（不与 task-notification 合并批次——回执必须携带自己的身份
 * workOrderId/originMeta，混批会让回执轮头退化为无标题行）。
 */
export interface WorkOrderReceiptRuntimeCommand extends RuntimeCommandBase {
  readonly branchGeneration: number;
  readonly mode: "work-order-receipt";
  readonly source: "agent_work_order_receipt";
  /** 回执指回的工单身份。 */
  readonly workOrderId: string;
  /** 交活方档案名/工号与其 persona 会话（回执信封署名）。 */
  readonly targetAgentName: string;
  readonly targetAgentId?: string;
  readonly targetSessionId: string;
  /** 原工单信封（谁派的单/派给谁）。 */
  readonly envelope: AgentWorkOrderEnvelope;
  /** 目标轮终态（completed 带最终答案本体；cancelled/failed 如实带原因）。 */
  readonly outcome: WorkOrderReceiptOutcome;
  /** 后台结果轮头卡元信息（backgroundSource=agent_work_order_receipt）。 */
  readonly originMeta: BackgroundResultOriginMeta;
  /** 信封拼装后的回执正文（<work-order-receipt> 包裹）。 */
  readonly text: string;
}

/**
 * 一条排队的 controlOnly 用户轮：GUI「配置」
 * 已经把 run 修订掉了，这条命令只负责把这件事记进会话。它不进模型轮，却要落一条 user 消息，而
 * user 消息插不进一个正在跑的 turn（provider 语法：assistant 的 tool_use 与 tool_result 之间
 * 不能夹 user），所以它排在队列里、等当前 turn 结束。与通知同一个优先级，因而落在新 run 的任何
 * 通知之前；带 branchGeneration，rewind 后丢弃。
 */
export interface ControlOnlyTurnRuntimeCommand extends RuntimeCommandBase {
  readonly branchGeneration: number;
  readonly mode: "control-only-turn";
  /** 进 runtime history 与持久消息的规范句（模型下一回合读它）。 */
  readonly text: string;
  /** `ensureSessionPersisted` 的首输入标题种子。 */
  readonly titleInput: string;
  readonly inputId?: string;
  /** 轮与消息上的同一份元数据（冷热同形）。 */
  readonly workflowLaunch: WorkflowLaunchMeta;
}

export type RuntimeCommand =
  | PromptRuntimeCommand
  | TargetContinuationRuntimeCommand
  | TargetContinuationLoopRuntimeCommand
  | TaskNotificationRuntimeCommand
  | SubagentMessageRuntimeCommand
  | WorkOrderRuntimeCommand
  | WorkOrderReceiptRuntimeCommand
  | ControlOnlyTurnRuntimeCommand;

export interface RuntimeCommandQueue {
  clearCancelPending(id: RuntimeCommandId): void;
  consumeCancelPending(id: RuntimeCommandId): boolean;
  dequeue(): RuntimeCommand | undefined;
  dequeueNextBatch(): readonly RuntimeCommand[];
  enqueue(command: RuntimeCommand): void;
  getByMaxPriority(maxPriority: RuntimeCommandPriority): readonly RuntimeCommand[];
  hasPending(): boolean;
  markCancelPending(id: RuntimeCommandId): void;
  removeById(id: RuntimeCommandId): RuntimeCommand | undefined;
  size(): number;
  snapshot(): readonly RuntimeCommand[];
}

const RUNTIME_COMMAND_PRIORITY_ORDER: Record<RuntimeCommandPriority, number> = {
  now: 0,
  next: 1,
  later: 2,
};

let runtimeCommandIdSequence = 0;

export function createRuntimeCommandId(): RuntimeCommandId {
  runtimeCommandIdSequence += 1;
  return `runtime_command_${runtimeCommandIdSequence}` as RuntimeCommandId;
}

export function createRuntimeCommandQueue(): RuntimeCommandQueue {
  const commands: RuntimeCommand[] = [];
  const cancelPendingCommandIds = new Set<RuntimeCommandId>();

  const selectNextIndex = (maxPriority?: RuntimeCommandPriority): number => {
    let selectedIndex = -1;
    let selectedPriority = Number.POSITIVE_INFINITY;
    const maxPriorityRank =
      maxPriority === undefined
        ? Number.POSITIVE_INFINITY
        : RUNTIME_COMMAND_PRIORITY_ORDER[maxPriority];

    for (const [index, command] of commands.entries()) {
      const priority = RUNTIME_COMMAND_PRIORITY_ORDER[command.priority];
      if (priority > maxPriorityRank) continue;
      if (priority < selectedPriority) {
        selectedIndex = index;
        selectedPriority = priority;
      }
    }

    return selectedIndex;
  };

  return {
    clearCancelPending(id: RuntimeCommandId): void {
      cancelPendingCommandIds.delete(id);
    },

    consumeCancelPending(id: RuntimeCommandId): boolean {
      return cancelPendingCommandIds.delete(id);
    },

    dequeue(): RuntimeCommand | undefined {
      const index = selectNextIndex();
      if (index === -1) return undefined;
      const [command] = commands.splice(index, 1);
      return command;
    },

    dequeueNextBatch(): readonly RuntimeCommand[] {
      const index = selectNextIndex();
      if (index === -1) return Object.freeze([]);
      const selected = commands[index];
      if (selected?.mode !== "task-notification") {
        const [command] = commands.splice(index, 1);
        return Object.freeze(command ? [command] : []);
      }

      const batch = commands.filter(
        (command) => command.mode === "task-notification" && command.priority === selected.priority,
      );
      for (let commandIndex = commands.length - 1; commandIndex >= 0; commandIndex -= 1) {
        const command = commands[commandIndex];
        if (command?.mode === "task-notification" && command.priority === selected.priority) {
          commands.splice(commandIndex, 1);
        }
      }
      return Object.freeze(batch);
    },

    enqueue(command: RuntimeCommand): void {
      commands.push(command);
    },

    getByMaxPriority(maxPriority: RuntimeCommandPriority): readonly RuntimeCommand[] {
      const maxPriorityRank = RUNTIME_COMMAND_PRIORITY_ORDER[maxPriority];
      return Object.freeze(
        commands.filter(
          (command) => RUNTIME_COMMAND_PRIORITY_ORDER[command.priority] <= maxPriorityRank,
        ),
      );
    },

    hasPending(): boolean {
      return commands.length > 0;
    },

    markCancelPending(id: RuntimeCommandId): void {
      cancelPendingCommandIds.add(id);
    },

    removeById(id: RuntimeCommandId): RuntimeCommand | undefined {
      const index = commands.findIndex((command) => command.id === id);
      if (index === -1) return undefined;
      const [command] = commands.splice(index, 1);
      return command;
    },

    size(): number {
      return commands.length;
    },

    snapshot(): readonly RuntimeCommand[] {
      return Object.freeze([...commands]);
    },
  };
}
