// 点将（D24）的 turn-start 注入：解析 canonical 正文里的 `agent://<名字>` 引用，
// 与现役档案取交集，得到本轮的派遣归属名单，并把同一事实固化成 model-only 提醒。
// 失败语义与 plugin 引用一致：对话 fail open（异常不阻塞本轮），注入 fail closed（异常时不写）。
import {
  buildAgentCallReminderBody,
  extractAgentReferences,
  resolveAgentCallTargets,
} from "../../subagent/agent-call.js";
import { createMessageId, traceContextToLogContext } from "../deps.js";
import type { TraceContext } from "../deps.js";
import type { AgentRuntimeInternal } from "../internal.js";

export async function applyAgentCallFromTurn(
  this: AgentRuntimeInternal,
  userInput: string,
  traceContext: TraceContext,
): Promise<void> {
  const extraction = extractAgentReferences(userInput);
  // 每轮先清名单：点将只覆盖用户开口的那一轮，续跑/model-only 轮不得继承。
  this.turnPinnedAgentNames = [];
  if (extraction.names.length === 0) {
    if (extraction.invalidCount > 0 || extraction.truncatedCount > 0) {
      this.logger?.debug("Agent call reference parse rejected or truncated", {
        ...traceContextToLogContext(traceContext),
        event: "agent_call.parse.rejected",
        invalidCount: extraction.invalidCount,
        module: "core.runtime",
        truncatedCount: extraction.truncatedCount,
      });
    }
    return;
  }

  const { resolved, unresolved } = resolveAgentCallTargets(
    extraction.names,
    this.config.subagents?.profiles ?? [],
  );
  this.turnPinnedAgentNames = resolved.map((profile) => profile.name);
  const body = buildAgentCallReminderBody({
    resolved: resolved.map((profile) => ({ name: profile.name, description: profile.description })),
    unresolved,
  });
  if (!body) return;

  try {
    this.messageHistory.addAttachment("agent_call", body);
    // 与 plugin 引用同一纪律：先注入再以 model-only notice 原文落库，
    // 通用 hydration 按同一 source 重建 attachment；只写内存会让 cold resume 丢它，
    // 历史序列错位还会砸穿 provider 前缀缓存。
    await this.persistSyntheticUserNoticeForSession({
      messageID: createMessageId(),
      sessionId: this.sessionId,
      source: "agent_call",
      text: body,
      traceContext,
    });
    this.logger?.debug("Agent call roster pinned", {
      ...traceContextToLogContext(traceContext),
      event: "agent_call.pinned",
      module: "core.runtime",
      pinnedAgentNames: this.turnPinnedAgentNames,
      referenceCount: extraction.names.length,
      unresolvedAgentNames: unresolved,
    });
  } catch (error) {
    // 提醒写失败不阻塞本轮；名单已就位——硬约束在端口上，不依赖这段文字。
    this.logger?.debug("Agent call reminder persistence failed", {
      ...traceContextToLogContext(traceContext),
      error: error instanceof Error ? error.message : String(error),
      event: "agent_call.reminder.failed",
      module: "core.runtime",
      referenceCount: extraction.names.length,
    });
  }
}
