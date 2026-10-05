/**
 * 排班草案卡（团队看板批3b）：staged 草案的结构化编辑器。
 *
 * 纪律（对齐稿 §十，P1~P4 拍板）：每任务一行——负责人下拉（名册）、依赖
 * 多选（同草案 taskKey 互斥排除自身）、删除；加任务=内联表单（P1）；保存=
 * teamPlanUpdate 原子 upsert；确认开工=teamPlanApprove（P2 已派照跑、失败
 * 红字可重试）；放弃=P4 二次确认（按钮两段式，失焦复位）。校验执法在端口，
 * 拒绝原文显示（guard.teamPlan* 的 reasonCode 上行）。
 * @module team board plan card
 */
import { useCallback, useMemo, useState } from "react";
import { ListPlusIcon, Trash2Icon } from "lucide-react";
import type { TeamPlan, TeamPlanTask } from "@zcode/shared/zcode-protocol-v4";
import { createCommandEnvelope } from "@/v4/commandFactory.js";
import { useV4Conversation } from "@/v4/V4ConversationContext.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";

const EDITOR_BUTTON_CLASS =
  "cursor-pointer rounded-[4px] px-2 py-0.5 text-ui-2xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30 disabled:cursor-not-allowed disabled:opacity-50";
const INPUT_CLASS =
  "rounded-[4px] border border-card-border bg-card px-1 py-0.5 text-ui-2xs text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30";

type PlanCommandType = "teamPlanUpdate" | "teamPlanApprove" | "teamPlanDiscard";

export function TeamBoardPlanCard({
  plan,
  roster,
  sessionId,
  sendCommand,
  onSettled,
}: {
  plan: TeamPlan & { errors?: { taskKey: string; error: string }[] };
  roster: { name: string; agentId?: string }[];
  sessionId: string;
  sendCommand: ReturnType<typeof useV4Conversation>["sendCommand"];
  onSettled: () => void;
}) {
  const { intl } = useZCodeIntl();
  const [draft, setDraft] = useState<TeamPlanTask[]>(() =>
    plan.tasks.map((task) => ({
      ...task,
      ...(task.dependsOn ? { dependsOn: [...task.dependsOn] } : {}),
    })),
  );
  const [dirty, setDirty] = useState(false);
  const [adding, setAdding] = useState(false);
  const [newTask, setNewTask] = useState<TeamPlanTask>({ taskKey: "", task: "", assignee: "" });
  const [pending, setPending] = useState<"save" | "approve" | "discard" | undefined>(undefined);
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const [errorText, setErrorText] = useState<string | undefined>(undefined);
  const errorByTask = useMemo(() => {
    const map = new Map<string, string>();
    for (const entry of plan.errors ?? []) map.set(entry.taskKey, entry.error);
    return map;
  }, [plan.errors]);

  const mutate = (next: TeamPlanTask[]) => {
    setDraft(next);
    setDirty(true);
  };

  const runCommand = useCallback(
    (
      type: PlanCommandType,
      payload:
        | { planId: string; tasks: TeamPlanTask[] }
        | { planId: string },
    ) => {
      setPending(type === "teamPlanUpdate" ? "save" : type === "teamPlanApprove" ? "approve" : "discard");
      setErrorText(undefined);
      void sendCommand(createCommandEnvelope({ type, payload, sessionId }))
        .then((ack) => {
          if (ack.status !== "accepted") {
            setErrorText(ack.message ?? intl.formatMessage({ id: "chat.teamBoard.planRejected" }));
            return;
          }
          if (ack.result?.type === "teamPlanApprove") {
            const failures = ack.result.failed ?? [];
            if (failures.length > 0) {
              setErrorText(
                intl.formatMessage(
                  { id: "chat.teamBoard.planPartial" },
                  { count: String(failures.length) },
                ),
              );
            }
          }
          onSettled();
        })
        .catch((cause: unknown) => {
          setErrorText(cause instanceof Error ? cause.message : String(cause));
        })
        .finally(() => setPending(undefined));
    },
    [intl, onSettled, sendCommand, sessionId],
  );

  const saveDraft = () => {
    runCommand("teamPlanUpdate", { planId: plan.planId, tasks: draft });
    setDirty(false);
  };

  const assigneeValue = (assignee: string): string =>
    roster.find((agent) => agent.name === assignee)?.agentId ?? assignee;

  return (
    <section
      className="flex flex-col gap-2 rounded-xl border border-dashed border-card-border bg-card p-3 shadow-xs"
      data-testid="team-board-plan"
    >
      <div className="flex min-w-0 items-center gap-2">
        <span
          className="min-w-0 flex-1 truncate text-ui-sm font-medium text-foreground"
          title={plan.title}
        >
          {plan.title}
        </span>
        <span className="shrink-0 rounded-[4px] bg-brand/10 px-1.5 text-ui-2xs font-medium leading-4 text-brand">
          {intl.formatMessage({ id: "chat.teamBoard.planChip" })}
        </span>
      </div>
      {draft.map((task, index) => {
        const serverError = errorByTask.get(task.taskKey);
        return (
          <div
            key={task.taskKey}
            className="flex flex-col gap-1 rounded-lg border border-card-border/60 p-2"
          >
            <div className="flex min-w-0 items-center gap-2">
              <span className="shrink-0 rounded-[4px] bg-muted px-1.5 font-mono text-ui-2xs leading-4 text-foreground-subtle">
                {task.taskKey}
              </span>
              <select
                value={assigneeValue(task.assignee)}
                onChange={(event) => {
                  const next = [...draft];
                  next[index] = { ...task, assignee: event.target.value };
                  mutate(next);
                }}
                aria-label={intl.formatMessage({ id: "chat.teamBoard.planAssignee" })}
                className={cn("min-w-0 max-w-40 flex-1 cursor-pointer", INPUT_CLASS)}
              >
                {roster.map((agent) => (
                  <option key={agent.agentId ?? agent.name} value={agent.agentId ?? agent.name}>
                    {agent.name}
                  </option>
                ))}
              </select>
              <button
                type="button"
                aria-label={intl.formatMessage({ id: "chat.teamBoard.planRemove" })}
                onClick={() => mutate(draft.filter((_, candidate) => candidate !== index))}
                className="ml-auto shrink-0 cursor-pointer rounded-[4px] p-0.5 text-foreground-subtlest transition-colors hover:bg-surface-hover hover:text-destructive focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30"
              >
                <Trash2Icon className="size-3" aria-hidden="true" />
              </button>
            </div>
            <p
              className="line-clamp-2 text-ui-2xs leading-4 text-foreground-subtlest"
              title={task.task}
            >
              {task.task}
            </p>
            {draft.length > 1 ? (
              <div className="flex min-w-0 flex-wrap items-center gap-1">
                <span className="shrink-0 text-ui-2xs text-foreground-subtlest">
                  {intl.formatMessage({ id: "chat.teamBoard.planDeps" })}:
                </span>
                {draft
                  .filter((candidate) => candidate.taskKey !== task.taskKey)
                  .map((candidate) => {
                    const active = (task.dependsOn ?? []).includes(candidate.taskKey);
                    return (
                      <button
                        key={candidate.taskKey}
                        type="button"
                        onClick={() => {
                          const deps = new Set(task.dependsOn ?? []);
                          if (active) deps.delete(candidate.taskKey);
                          else deps.add(candidate.taskKey);
                          const next = [...draft];
                          next[index] = {
                            ...task,
                            ...(deps.size > 0 ? { dependsOn: [...deps] } : {}),
                          };
                          if (deps.size === 0) delete next[index].dependsOn;
                          mutate(next);
                        }}
                        className={cn(
                          "cursor-pointer rounded-[4px] px-1 font-mono text-ui-2xs leading-4 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30",
                          active
                            ? "bg-brand/10 text-brand"
                            : "bg-muted text-foreground-subtlest hover:bg-surface-hover hover:text-foreground",
                        )}
                      >
                        {candidate.taskKey}
                      </button>
                    );
                  })}
              </div>
            ) : null}
            {serverError ? <p className="text-ui-2xs text-destructive">{serverError}</p> : null}
          </div>
        );
      })}
      {adding ? (
        <div className="flex flex-col gap-1 rounded-lg border border-dashed border-card-border p-2">
          <div className="flex min-w-0 items-center gap-1">
            <input
              value={newTask.taskKey}
              onChange={(event) =>
                setNewTask((current) => ({ ...current, taskKey: event.target.value }))
              }
              placeholder={intl.formatMessage({ id: "chat.teamBoard.planTaskKey" })}
              className={cn("w-28 font-mono", INPUT_CLASS)}
              aria-label={intl.formatMessage({ id: "chat.teamBoard.planTaskKey" })}
            />
            <select
              value={newTask.assignee}
              onChange={(event) =>
                setNewTask((current) => ({ ...current, assignee: event.target.value }))
              }
              aria-label={intl.formatMessage({ id: "chat.teamBoard.planAssignee" })}
              className={cn("min-w-0 flex-1 cursor-pointer", INPUT_CLASS)}
            >
              <option value="">{intl.formatMessage({ id: "chat.teamBoard.planAssignee" })}</option>
              {roster.map((agent) => (
                <option key={agent.agentId ?? agent.name} value={agent.agentId ?? agent.name}>
                  {agent.name}
                </option>
              ))}
            </select>
          </div>
          <textarea
            value={newTask.task}
            onChange={(event) => setNewTask((current) => ({ ...current, task: event.target.value }))}
            placeholder={intl.formatMessage({ id: "chat.teamBoard.planTaskBody" })}
            rows={2}
            aria-label={intl.formatMessage({ id: "chat.teamBoard.planTaskBody" })}
            className={INPUT_CLASS}
          />
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => {
                const key = newTask.taskKey.trim();
                const body = newTask.task.trim();
                const assignee = newTask.assignee.trim();
                if (!key || !body || !assignee) return;
                mutate([...draft, { taskKey: key, task: body, assignee }]);
                setNewTask({ taskKey: "", task: "", assignee: "" });
                setAdding(false);
              }}
              disabled={!newTask.taskKey.trim() || !newTask.task.trim() || !newTask.assignee.trim()}
              className={cn(EDITOR_BUTTON_CLASS, "bg-brand/10 text-brand hover:bg-brand/20")}
            >
              {intl.formatMessage({ id: "chat.teamBoard.planAddConfirm" })}
            </button>
            <button
              type="button"
              onClick={() => setAdding(false)}
              className={cn(
                EDITOR_BUTTON_CLASS,
                "bg-transparent text-foreground-subtle hover:bg-surface-hover hover:text-foreground",
              )}
            >
              {intl.formatMessage({ id: "chat.teamBoard.planCancel" })}
            </button>
          </div>
        </div>
      ) : (
        <button
          type="button"
          onClick={() => setAdding(true)}
          className="flex cursor-pointer items-center justify-center gap-1 rounded-lg border border-dashed border-card-border py-1 text-ui-2xs text-foreground-subtle transition-colors hover:bg-surface-hover hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30"
        >
          <ListPlusIcon className="size-3" aria-hidden="true" />
          {intl.formatMessage({ id: "chat.teamBoard.planAddTask" })}
        </button>
      )}
      {errorText ? <p className="text-ui-2xs text-destructive">{errorText}</p> : null}
      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          disabled={!dirty || pending !== undefined}
          onClick={saveDraft}
          className={cn(EDITOR_BUTTON_CLASS, "bg-muted text-foreground hover:bg-surface-hover")}
        >
          {intl.formatMessage({ id: "chat.teamBoard.planSave" })}
        </button>
        <button
          type="button"
          disabled={pending !== undefined}
          onClick={() => runCommand("teamPlanApprove", { planId: plan.planId })}
          data-testid="team-board-plan-approve"
          className={cn(
            EDITOR_BUTTON_CLASS,
            "bg-brand text-primary-foreground hover:opacity-90",
          )}
        >
          {intl.formatMessage({ id: "chat.teamBoard.planApprove" })}
        </button>
        <button
          type="button"
          disabled={pending !== undefined}
          onClick={() => {
            if (!confirmDiscard) {
              setConfirmDiscard(true);
              return;
            }
            runCommand("teamPlanDiscard", { planId: plan.planId });
          }}
          onBlur={() => setConfirmDiscard(false)}
          className={cn(
            EDITOR_BUTTON_CLASS,
            "ml-auto bg-transparent font-normal text-foreground-subtle hover:bg-surface-hover hover:text-destructive",
          )}
        >
          {intl.formatMessage({
            id: confirmDiscard ? "chat.teamBoard.planDiscardConfirm" : "chat.teamBoard.planDiscard",
          })}
        </button>
      </div>
    </section>
  );
}
