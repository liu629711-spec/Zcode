/**
 * 回执卡「转交」控件（接力协作第一刀，2026-09-30）：把一张已完成的回执一键转给
 * 名册里的另一位员工——成果原文全文+老板附言拼成新工单，走与 @ 面板同一条
 * dispatchAgentWorkOrder 宿主能力。从 ConversationTurnGroup 抽出独立文件：
 * 工地卡（AgentWorkOrderBatchCard）的行内转交也复用它，避免 UI 互相引用成环。
 *
 * inline 变体：工地卡行内使用——收起态只渲染按钮本身（不带右对齐包裹层），
 * 展开面板靠父级 flex-wrap 换行占满整行。
 */
import { useState } from "react";
import { Button } from "@/components/ui/button.js";
import { toast } from "@/components/ui/toast.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { buildTaskWorkspaceKey } from "@/lib/taskQueryCache.js";
import {
  selectProjectAgentDirectoryForWorkspace,
  useProjectAgentDirectoryStore,
} from "@/store/projectAgentDirectoryStore.js";
import { testId } from "@/lib/testId.js";
import { TID_CHAT_RECEIPT_FORWARD } from "@zcode/shared";
import type { ConversationRowRenderContext } from "@/v4/conversationRowContext.js";
import {
  buildWorkOrderForwardTask,
  parseReceiptDelivererName,
  selectForwardTargets,
} from "@/v4/workOrderForward.js";

export function ReceiptForwardControl({
  title,
  answer,
  context,
  unitKey,
  inline = false,
}: {
  title: string;
  answer: string;
  context: ConversationRowRenderContext;
  unitKey: string;
  /** 工地卡行内模式：收起态不带右对齐包裹层，展开面板随父级 flex-wrap 换行。 */
  inline?: boolean;
}) {
  const { intl } = useZCodeIntl();
  const [open, setOpen] = useState(false);
  const [target, setTarget] = useState("");
  const [note, setNote] = useState("");
  const [pending, setPending] = useState(false);
  const delivererName = parseReceiptDelivererName(title);
  const directory = useProjectAgentDirectoryStore((state) =>
    selectProjectAgentDirectoryForWorkspace(
      state,
      buildTaskWorkspaceKey(context.workspacePath, context.workspaceIdentity),
    ),
  );
  const targets = selectForwardTargets(directory, delivererName);
  const dispatchWorkOrder = context.onDispatchAgentWorkOrder;
  if (!delivererName || !dispatchWorkOrder || targets.length === 0) {
    return null;
  }
  const submit = async () => {
    const agent = target.trim();
    if (!agent || pending) return;
    setPending(true);
    try {
      const delivery = await dispatchWorkOrder(
        agent,
        buildWorkOrderForwardTask({ delivererName, answer, note }),
      );
      toast(
        intl.formatMessage(
          {
            id:
              delivery === "queued"
                ? "chat.mention.agents.dispatchQueued"
                : "chat.mention.agents.dispatchAccepted",
          },
          { name: agent },
        ),
      );
      setOpen(false);
      setNote("");
      setTarget("");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error));
    } finally {
      setPending(false);
    }
  };
  const button = (
    <Button
      type="button"
      variant="ghost"
      size="sm"
      data-testid={testId(TID_CHAT_RECEIPT_FORWARD, unitKey)}
      onClick={() => setOpen(true)}
    >
      {intl.formatMessage({ id: "chat.receipt.forward" })}
    </Button>
  );
  return open ? (
    <div
      data-testid={testId(TID_CHAT_RECEIPT_FORWARD, unitKey)}
      className={cnPanel(inline)}
    >
      <div className="flex items-center gap-2">
        <span className="shrink-0 text-ui-sm text-foreground-subtle">
          {intl.formatMessage({ id: "chat.receipt.forward.target" })}
        </span>
        <select
          className="h-7 min-w-0 flex-1 rounded-md border border-input-border bg-input px-2 text-ui-sm text-foreground outline-none"
          value={target}
          onChange={(event) => setTarget(event.target.value)}
        >
          <option value="">…</option>
          {targets.map((agent) => (
            <option key={agent.name} value={agent.name}>
              {agent.name}
            </option>
          ))}
        </select>
      </div>
      <textarea
        rows={2}
        className="w-full resize-none rounded-md border border-input-border bg-input/20 px-2 py-1.5 text-ui-sm text-foreground outline-none placeholder:text-muted-foreground"
        placeholder={intl.formatMessage({ id: "chat.receipt.forward.note" })}
        value={note}
        onChange={(event) => setNote(event.target.value)}
      />
      <div className="flex justify-end gap-2">
        <Button type="button" variant="outline" size="sm" onClick={() => setOpen(false)}>
          {intl.formatMessage({ id: "common.cancel" })}
        </Button>
        <Button
          type="button"
          size="sm"
          disabled={pending || target.trim().length === 0}
          onClick={() => void submit()}
        >
          {intl.formatMessage({ id: "chat.receipt.forward.confirm" })}
        </Button>
      </div>
    </div>
  ) : inline ? (
    button
  ) : (
    <div className="flex w-full justify-end">{button}</div>
  );
}

function cnPanel(inline: boolean): string {
  return inline
    ? "flex w-full min-w-0 flex-col gap-1.5 rounded-md border border-border bg-background px-2.5 py-2"
    : "mt-1 flex w-full flex-col gap-1.5 rounded-md border border-border bg-background px-2.5 py-2";
}
