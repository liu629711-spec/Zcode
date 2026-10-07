/**
 * 派单失败回执的「大白话失败说明 + 一键重派」（2026-10-01 员工可靠性批）。
 * 数据全部来自轮头 originMeta 的结构化失败线索（CLI 权威下发，describeReceiptFailure
 * 做大白话映射），绝不从回执文本反推。重派 = 原员工 + 原任务原文，走与转交同一条
 * dispatchAgentWorkOrder 宿主能力；目标按工号优先（员工改名不误派，audit 2026-10-01），
 * 名册对不上时明说原因，不再无声消失；批次内失败行重派带原批次（结果归回原工地卡）。
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
  batchId,
  batchTitle,
  review,
  inline = false,
}: {
  /** 回执标题（CLI 权威铸造；解员工名用）。 */
  title: string;
  meta: Pick<
    AgentWorkOrderReceiptMeta,
    | "task"
    | "agentId"
    | "batchId"
    | "batchTitle"
    | "review"
    | "failureCode"
    | "failureModelId"
    | "failureReason"
    | "retried"
  >;
  context: ConversationRowRenderContext;
  unitKey: string;
  /** 工地卡行内模式：批次身份由卡片显式传入（行模型不携带批次键）。 */
  batchId?: string;
  batchTitle?: string;
  /** 工地卡行内模式：批次口味由卡片显式传入（行模型不携带 review）；单卡读 meta.review。 */
  review?: boolean;
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
  const titleName = parseReceiptDelivererName(title) ?? parseFailedReceiptAgentName(title);
  const task = meta.task?.trim();
  const dispatchWorkOrder = context.onDispatchAgentWorkOrder;
  // 目标按工号优先（终审评委B P2）：工号在场时它就是权威——在册按号派（改名
  // 不影响）；不在册说明员工被删，**绝不退回按名**（旧名可能被新员工顶了，会派错
  // 人）。只有无工号的旧回执才按名字对号。两个都落空 → 只说明，不给按钮。
  const rosterEntry = meta.agentId
    ? directory.find((agent) => agent.agentId === meta.agentId)
    : undefined;
  const nameEntry =
    !meta.agentId && titleName
      ? directory.find(
          (agent) => agent.name.trim().toLowerCase() === titleName.trim().toLowerCase(),
        )
      : undefined;
  const target = rosterEntry?.name ?? nameEntry?.name ?? titleName;
  const redispatchBatchId = meta.batchId ?? batchId;
  const redispatchBatchTitle = meta.batchTitle ?? batchTitle;
  // 评审单口味透传（2026-10-07）：单卡读轮头 meta.review，工地卡行内由卡片传批次口味。
  const redispatchReview = review === true || meta.review === true;
  const canRedispatch =
    !done &&
    Boolean(dispatchWorkOrder) &&
    Boolean(target && task) &&
    Boolean(rosterEntry || nameEntry);
  const redispatch = async () => {
    if (!dispatchWorkOrder || !target || !task || pending) return;
    setPending(true);
    try {
      const delivery = await dispatchWorkOrder(target, task, {
        ...(redispatchBatchId ? { batchId: redispatchBatchId } : {}),
        ...(redispatchBatchTitle ? { batchTitle: redispatchBatchTitle } : {}),
        ...(redispatchReview ? { review: true } : {}),
      });
      toast(
        intl.formatMessage(
          {
            id:
              delivery === "queued"
                ? "chat.mention.agents.dispatchQueued"
                : "chat.mention.agents.dispatchAccepted",
          },
          { name: target },
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
      {!canRedispatch && target && task ? (
        // 按钮缺席不再无声：员工不在名册（被删/改名对不上）时明说，老板知道为何不能重派。
        <span className="min-w-0 text-ui-xs leading-4 text-foreground-subtlest">
          {intl.formatMessage({ id: "chat.receipt.redispatch.unavailable" })}
        </span>
      ) : null}
      {presentation.detail ? (
        <span
          className={
            // 行内（工地卡失败行）必须截断：根因原文可达 500 字符，不截会把状态板
            // 撑出十几行（评审 B 的 P2）；单卡有整卡空间，照常全文。
            inline
              ? "min-w-0 break-words text-ui-xs leading-4 text-foreground-subtlest line-clamp-2"
              : "min-w-0 break-words text-ui-xs leading-4 text-foreground-subtlest"
          }
        >
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
