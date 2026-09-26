import { memo, useEffect, useState } from "react";
import { ChevronRightIcon } from "lucide-react";
import {
  TID_WORKFLOW_REPLAY_ATTEMPT,
  TID_WORKFLOW_REPLAY_SECTION,
  TID_WORKFLOW_REPLAY_TOGGLE,
} from "@zcode/shared";
import type { WorkflowRunReplayInstance } from "@zcode/shared/zcode-protocol-v4";
import { cn } from "@/components/lib/utils.js";
import { useWorkflowRunReplay } from "@/hooks/useWorkflowRunReplay.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";

/**
 * T1 行车记录仪：workflow run 详情页的**回放**区。
 *
 * journal 事件折叠成按 attempt 分组的节点状态序列（折叠核与 CLI 的服务端投影共用一份，
 * 见 useWorkflowRunReplay 的文件头）。读面是证据，不是实况：实况（当前跑到哪、谁在跑）
 * 由上方的阶段清单读权威投影，这一区回答的是「当时每一步发生了什么」——含被后一轮
 * attempt 覆盖掉的前几轮的失败。
 *
 * 与产物区相反**默认收起**：产物是交付物，回放是审计动作。attempt 切换器是简单的
 * 全部 + 逐轮按钮；实例行一条一行，轨迹连成一条状态词流（queued → executing → …），
 * 失败原因单独一行。状态词保留引擎原词（证据面不翻译，与 logTail 同一姿态）。
 */
export const WorkflowRunReplaySection = memo(function WorkflowRunReplaySection({
  sessionId,
  runId,
}: {
  sessionId: string;
  runId: string;
}) {
  const { intl } = useZCodeIntl();
  const [expanded, setExpanded] = useState(false);
  // 选中的 attempt（显示层 1 起）；`all` = 不分组。切换器在时间线到手后才渲染，
  // 所以选中态只可能是真实存在的那一组。
  const [selectedAttempt, setSelectedAttempt] = useState<number | "all">("all");
  // 取数门在展开上：折叠着不为没人看的证据付 journal 读（见 hook 文件头）。
  const replay = useWorkflowRunReplay({ sessionId, runId, enabled: expanded });

  // 切 run：回放的选中轮次跟着作废——上一个 run 的「第 2 轮」对这一个 run 无意义。
  useEffect(() => {
    setSelectedAttempt("all");
  }, [runId]);

  // 查过且确实没有（empty）/ 能力缺席（unavailable）→ 整区缺席；「无则缺席」惯例，不留空壳。
  if (replay.status === "unavailable" || replay.status === "empty") {
    return null;
  }

  const timeline = replay.timeline;
  const groups = timeline?.attempts ?? [];
  const selectedInstances =
    selectedAttempt === "all"
      ? (timeline?.instances ?? [])
      : (groups.find((group) => group.attempt === selectedAttempt)?.instances ?? []);

  return (
    <section
      className="wf-motion shrink-0 border-t border-border/60"
      data-testid={TID_WORKFLOW_REPLAY_SECTION}
    >
      <button
        aria-expanded={expanded}
        aria-label={intl.formatMessage({
          id: expanded
            ? "chat.toolCall.workflow.run.replay.collapse"
            : "chat.toolCall.workflow.run.replay.expand",
        })}
        className="flex h-9 w-full items-center gap-2 px-3 text-left outline-none transition-colors hover:bg-surface focus-visible:ring-2 focus-visible:ring-ring/40"
        data-testid={TID_WORKFLOW_REPLAY_TOGGLE}
        onClick={() => setExpanded((previous) => !previous)}
        type="button"
      >
        <span className="min-w-0 shrink-0 truncate text-ui-base font-medium text-foreground">
          {intl.formatMessage({ id: "chat.toolCall.workflow.run.replay.title" })}
        </span>
        <span
          className="shrink-0 font-mono text-ui-xs tabular-nums text-foreground-subtlest"
          data-testid="workflow-run-replay-count"
        >
          {/* idle / loading 时还没有数：占位而不装作 0（0 是「确实没有」的答案）。 */}
          {timeline === undefined ? "—" : timeline.instances.length.toLocaleString()}
        </span>
        <ChevronRightIcon
          aria-hidden
          className={cn(
            "ml-auto size-4 shrink-0 text-foreground-subtle transition-transform",
            expanded && "rotate-90",
          )}
        />
      </button>

      {expanded && replay.status === "loading" ? (
        <p className="px-4 pb-3 text-ui-xs text-foreground-subtle">…</p>
      ) : null}

      {expanded && replay.status === "error" ? (
        <p className="px-4 pb-3 text-ui-xs text-foreground-subtle">
          {intl.formatMessage({ id: "chat.toolCall.workflow.run.replay.error" })}
          {replay.error === null ? null : `: ${replay.error}`}
        </p>
      ) : null}

      {expanded && timeline !== undefined && replay.status !== "error" ? (
        <div className="pb-3">
          {replay.truncated ? (
            <p className="px-4 pb-2 text-ui-xs text-foreground-subtle">
              {intl.formatMessage({ id: "chat.toolCall.workflow.run.replay.truncated" })}
            </p>
          ) : null}

          {/* attempt 切换器：全部 + 逐轮。只有多于一轮时才值得切。 */}
          {groups.length > 1 ? (
            <div
              aria-label={intl.formatMessage({ id: "chat.toolCall.workflow.run.replay.title" })}
              className="flex flex-wrap gap-1 px-4 pb-2"
              role="group"
            >
              <AttemptChip
                active={selectedAttempt === "all"}
                label={intl.formatMessage({ id: "chat.toolCall.workflow.run.replay.attemptAll" })}
                onSelect={() => setSelectedAttempt("all")}
              />
              {groups.map((group) => (
                <AttemptChip
                  active={selectedAttempt === group.attempt}
                  key={group.attempt}
                  label={intl.formatMessage(
                    { id: "chat.toolCall.workflow.run.replay.attempt" },
                    { attempt: group.attempt + 1 },
                  )}
                  onSelect={() => setSelectedAttempt(group.attempt)}
                />
              ))}
            </div>
          ) : null}

          <ul data-testid="workflow-run-replay-instances">
            {selectedInstances.map((instance) => (
              <ReplayInstanceRow instance={instance} key={`${instance.siteId}@${instance.attempt}`} />
            ))}
          </ul>
        </div>
      ) : null}
    </section>
  );
});

const AttemptChip = memo(function AttemptChip({
  active,
  label,
  onSelect,
}: {
  active: boolean;
  label: string;
  onSelect: () => void;
}) {
  return (
    <button
      aria-pressed={active}
      className={cn(
        "rounded-full border px-2.5 py-0.5 text-ui-xs transition-colors",
        active
          ? "border-transparent bg-foreground text-background"
          : "border-border text-foreground-subtle hover:bg-surface",
      )}
      data-testid={TID_WORKFLOW_REPLAY_ATTEMPT}
      onClick={onSelect}
      type="button"
    >
      {label}
    </button>
  );
});

const ReplayInstanceRow = memo(function ReplayInstanceRow({
  instance,
}: {
  instance: WorkflowRunReplayInstance;
}) {
  return (
    <li className="px-4 py-1.5" data-testid="workflow-run-replay-instance">
      <div className="flex min-w-0 items-baseline gap-2">
        <span className="shrink-0 font-mono text-ui-xs text-foreground-subtlest">
          {instance.siteId}@{instance.attempt}
        </span>
        {instance.label === undefined ? null : (
          <span className="min-w-0 truncate text-ui-xs font-medium text-foreground">
            {instance.label}
          </span>
        )}
        <span
          className={cn(
            "ml-auto shrink-0 font-mono text-ui-xs",
            instance.outcome === "failed"
              ? "text-destructive"
              : instance.outcome === undefined
                ? "text-foreground-subtlest"
                : "text-foreground-subtle",
          )}
        >
          {instance.steps.at(-1)?.state ?? "—"}
        </span>
      </div>
      <div className="mt-0.5 font-mono text-ui-xs text-foreground-subtle">
        {instance.steps.map((step) => step.state).join(" → ")}
      </div>
      {instance.summary === undefined ? null : (
        <div className="mt-0.5 truncate text-ui-xs text-foreground-subtle">{instance.summary}</div>
      )}
      {instance.error === undefined ? null : (
        <div className="mt-0.5 text-ui-xs text-destructive">{instance.error}</div>
      )}
    </li>
  );
});
