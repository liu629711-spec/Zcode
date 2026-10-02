import {
  type CollaborationMode,
  type PermissionBrokerRequest,
  type PermissionBrokerResult,
  type PermissionRuleset,
  type TraceContext,
} from "@zcode/contracts";

import type { PermissionDecisionResult, PermissionContext } from "../../permission/service.js";
import type { ExecutableToolCall, ToolEntry } from "../types.js";
import { applyEmployeeDeskPermission } from "./employee-desk-permission.js";
import { applyMemoryFilePermission, targetsMemoryFile } from "./memory-file-permission.js";
import {
  resolveRuntimePermissionCapability,
  resolveRuntimePermissionContext,
} from "./permission-capability.js";
import { buildDefaultPermissionUpdates } from "./permission-suggestions.js";
import type { ToolExecutorDeps } from "./types.js";

interface PermissionHookInputRecheckResult {
  brokerResult?: PermissionBrokerResult;
  permissionDecision?: PermissionDecisionResult;
}

export async function recheckPermissionHookModifiedInput(input: {
  deps: ToolExecutorDeps;
  entry: ToolEntry;
  mode: CollaborationMode;
  modifiedInput: unknown;
  projectRules: PermissionRuleset | null;
  requestId: string;
  signal?: AbortSignal;
  toolCall: ExecutableToolCall;
  traceContext: TraceContext;
}): Promise<PermissionHookInputRecheckResult> {
  const runtimePermissionContext = resolveRuntimePermissionContext(input.deps);
  const permissionContext: PermissionContext = {
    input: input.modifiedInput,
    mode: input.mode,
    prePlanMode: input.deps.sessionModePort?.getPrePlanMode(),
    planEnabled: input.deps.sessionModePort?.isPlanEnabled?.(),
    riskLevel: input.entry.metadata.riskLevel,
    toolName: input.toolCall.name,
    // 与首次判定同源：hook 改过 input 之后，草稿免确认仍要按同一个工作目录复核。
    workingDirectory: input.deps.getWorkingDirectory(),
  };
  const rulePolicy = input.entry.resolvePermissionRulePolicy?.(
    input.modifiedInput,
    runtimePermissionContext,
  );
  let decision = input.deps.permissionService.checkPermission(
    permissionContext,
    resolveRuntimePermissionCapability(input.entry, input.modifiedInput, runtimePermissionContext),
    input.projectRules,
    rulePolicy,
  );
  decision = applyMemoryFilePermission({
    decision,
    executionInput: input.modifiedInput,
    memoryRoot: input.deps.getMemoryRoot?.(),
    toolName: input.toolCall.name,
    workingDirectory: input.deps.getWorkingDirectory(),
    workspaceRoot: input.deps.getWorkspaceRoot(),
  });
  // 员工桌子闸与首判同源：hook 改过 input 之后，桌外写入照样改判 ask 并进审批闸。
  decision = applyEmployeeDeskPermission({
    decision,
    executionInput: input.modifiedInput,
    toolName: input.toolCall.name,
    workingDirectory: input.deps.getWorkingDirectory(),
    workspaceRoot: input.deps.getWorkspaceRoot(),
    desk: input.deps.employeeDesk,
  });

  if (decision.decision === "deny") {
    return {
      brokerResult: { decision: "deny", reason: decision.reason },
      permissionDecision: decision,
    };
  }
  if (
    decision.decision !== "ask" ||
    (decision.ruleId !== "rule.project.ask" &&
      decision.ruleId !== "guard.employeeDesk" &&
      !targetsMemoryFile({
        executionInput: input.modifiedInput,
        memoryRoot: input.deps.getMemoryRoot?.(),
        toolName: input.toolCall.name,
        workingDirectory: input.deps.getWorkingDirectory(),
        workspaceRoot: input.deps.getWorkspaceRoot(),
      }))
  ) {
    return {};
  }

  const suggestedPermissionUpdates =
    rulePolicy?.suggestedPermissionUpdates ??
    buildDefaultPermissionUpdates(input.toolCall.name, input.modifiedInput);
  const brokerResult = await input.deps.permissionBroker.requestPermission(
    {
      input: input.modifiedInput,
      mode: input.mode,
      reason: decision.reason ?? `Tool ${input.toolCall.name} requires approval`,
      requestId: input.requestId,
      requestedAt: new Date(),
      riskLevel: decision.riskLevel,
      ruleId: decision.ruleId,
      sessionId: input.deps.sessionId,
      sideEffectScope: decision.sideEffectScope,
      suggestedPermissionUpdates,
      toolCallId: input.toolCall.id as PermissionBrokerRequest["toolCallId"],
      toolName: input.toolCall.name,
      traceId: input.traceContext.traceId,
      turnId: input.traceContext.turnId ?? input.deps.turnId,
    },
    {
      signal: input.signal,
      timeoutMs: input.deps.permissionTimeoutMs,
    },
  );
  return { brokerResult, permissionDecision: decision };
}
