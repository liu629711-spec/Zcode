/**
 * 派单失败回执的「大白话失败说明 + 一键重派」（2026-10-01 员工可靠性批）。
 * 数据全部来自轮头 originMeta 的结构化失败线索（CLI 权威下发，describeReceiptFailure
 * 做大白话映射），绝不从回执文本反推。重派 = 原员工 + 原任务原文，走与转交同一条
 * dispatchAgentWorkOrder 宿主能力；员工不在名册（无名工位/档案已删）时只说明、不给按钮。
 *
 * inline 变体：工地卡行内使用——短语与按钮同行随 flex-wrap 换行。
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
import { TID_CHAT_RECEIPT_REDISPATCH, testId } from "@zcode/shared";
import type { ConversationRowRenderContext } from "@/v4/conversationRowContext.js";
import {
  describeReceiptFailure,
  type AgentWorkOrderReceiptMeta,
} from "@/v4/agentWorkOrderTurn.js";
import {
  parseFailedReceiptAgentName,
  parseReceiptDelivererName,
} from "@/v4/workOrderForward.js";

export function ReceiptFailureNotice({
  title,
  meta,
  context,
  unitKey,
  inline = false,
}: {
  /** 回执标题（CLI 权威铸造；解员工名用）。 */
  title: string;
  meta: Pick<
    AgentWorkOrderReceiptMeta,
    "task" | "failureCode" | "failureModelId" | "failureReason" | "retried"
  >;
  context: ConversationRowRenderContext;
  unitKey: string;
  /** 工地卡行内模式：短语与按钮同行（随父级 flex-wrap 换行），不带独立面板。 */
  inline?: boolean;
}) {
  const { intl } = useZCodeIntl();
  const [pending, setPending] = useState(false);
  const [done, setDone] = useState(false);
  const directory = useProjectAgentDirectoryStore((state) =>
    selectProjectAgentDirectoryForWorkspace(
      state,
      buildTaskWorkspaceKey(context.workspacePath, context.workspaceIdentity),
    ),
  );
  const presentation = describeReceiptFailure(meta);
  if (!presentation) return null;
  const agentName = parseReceiptDelivererName(title) ?? parseFailedReceiptAgentName(title);
  const task = meta.task?.trim();
  const dispatchWorkOrder = context.onDispatchAgentWorkOrder;
  // 重派三道门：有宿主派单能力、解得出原员工、员工还在名册里（无名工位的标题
  // 种子是任务正文，解出的「名字」多半不在册——那种只能报告老板，不冒名重派）。
  const canRedispatch =
    !done &&
    Boolean(dispatchWorkOrder) &&
    Boolean(agentName && task) &&
    directory.some(
      (agent) => agent.name.trim().toLowerCase() === agentName?.trim().toLowerCase(),
    );
  const redispatch = async () => {
    if (!dispatchWorkOrder || !agentName || !task || pending) return;
    setPending(true);
    try {
      const delivery = await dispatchWorkOrder(agentName, task);
      toast(
        intl.formatMessage(
          {
            id:
              delivery === "queued"
                ? "chat.mention.agents.dispatchQueued"
                : "chat.mention.agents.dispatchAccepted",
          },
          { name: agentName },
        ),
      );
      setDone(true);
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error));
    } finally {
      setPending(false);
    }
  };
  const phrase = intl.formatMessage(
    { id: presentation.messageId },
    presentation.values ?? {},
  );
  const content = (
    <>
      {presentation.retried ? (
        <span className="min-w-0 text-ui-xs leading-4 text-foreground-subtle">
          {intl.formatMessage({ id: "chat.receipt.failure.retried" })}
        </span>
      ) : null}
      <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
        <span className="min-w-0 flex-1 text-ui-sm leading-5 text-destructive">{phrase}</span>
        {canRedispatch ? (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="shrink-0"
            disabled={pending}
            data-testid={testId(TID_CHAT_RECEIPT_REDISPATCH, unitKey)}
            onClick={() => void redispatch()}
          >
            {intl.formatMessage({ id: "chat.receipt.redispatch" })}
          </Button>
        ) : null}
      </div>
      {presentation.detail ? (
        <span className="min-w-0 break-words text-ui-xs leading-4 text-foreground-subtlest">
          {presentation.detail}
        </span>
      ) : null}
    </>
  );
  return (
    <div
      className={
        inline
          ? "flex w-full min-w-0 flex-col gap-0.5"
          : "mt-1 flex w-full min-w-0 flex-col gap-0.5"
      }
      data-testid={testId(TID_CHAT_RECEIPT_REDISPATCH, `${unitKey}:notice`)}
    >
      {content}
    </div>
  );
}
