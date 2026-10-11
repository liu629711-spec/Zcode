/**
 * 圆桌会评审专区（四段式弹层）：一行摘要卡点开后的会议全貌。
 * ① 局面条：状态灯 + 轮次 pill + 票数条（同意/打回/弃权比例，语义色）——本文件；
 * ② 裁决卡：共识/分歧/盲区/建议下一步——见 CouncilMeetingVerdict.tsx；
 * ③ 过程流（默认折叠）：席位发言时间线——见 CouncilMeetingFlow.tsx；
 * ④ 控场条：暂停/恢复+插话——本文件，走 v4 命令面 councilPause/councilInterject
 *    （人工控场批）。命令幂等且 core 对终态会议自行忽略；进行中的会议才出控件，
 *    插话 v1 不做 @点名解析（协议已留 targetSeatIndexes，点名解析随下批）。
 * 数据只来自 CouncilMeetingModel（纯函数聚合 originMeta.council*），这里只管画。
 */
import { useCallback, useState } from "react";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import { Button } from "@/components/ui/button.js";
import { Input } from "@/components/ui/input.js";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog.js";
import { TooltipProvider } from "@/components/ui/tooltip.js";
import { CirclePause, CirclePlay, SendHorizontal } from "lucide-react";
import { STATUS_DOT } from "@/components/workflow-graph/run-status-presentation.js";
import type { CouncilMeetingModel } from "@/v4/councilMeeting.js";
import { useV4Conversation } from "@/v4/V4ConversationContext.js";
import { createCommandEnvelope } from "@/v4/commandFactory.js";
import {
  COUNCIL_STATUS_BADGE_CLASS,
  COUNCIL_STATUS_DOT_KEY,
} from "@/v4/councilMeetingVisuals.js";
import { CouncilVerdictCard } from "@/v4/CouncilMeetingVerdict.js";
import { CouncilProcessFlow } from "@/v4/CouncilMeetingFlow.js";

// ── ① 局面条 ────────────────────────────────────────────────────────

function CouncilStatusStrip({ meeting }: { meeting: CouncilMeetingModel }) {
  const { intl } = useZCodeIntl();
  const votes = meeting.tally;
  const total = votes.approve + votes.reject + votes.abstain;
  const segments = [
    { key: "approve" as const, count: votes.approve, className: "bg-success" },
    {
      key: "reject" as const,
      count: votes.reject,
      className: "bg-destructive",
    },
    {
      key: "abstain" as const,
      count: votes.abstain,
      className: "bg-foreground/25",
    },
  ];
  return (
    <section
      aria-label={intl.formatMessage({ id: "chat.council.section.status" })}
      className="rounded-xl border border-card-border bg-card/50 px-3.5 py-3"
      data-testid="council-zone-status"
    >
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <span
          aria-hidden="true"
          className={cn(
            "size-1.5 rounded-full",
            STATUS_DOT[COUNCIL_STATUS_DOT_KEY[meeting.status]],
          )}
        />
        <span
          className={cn(
            "rounded-[4px] px-1.5 py-0.5 text-ui-2xs font-medium leading-4",
            COUNCIL_STATUS_BADGE_CLASS[meeting.status],
          )}
        >
          {intl.formatMessage({ id: `chat.council.status.${meeting.status}` })}
        </span>
        <span className="rounded-full border border-card-border px-2 py-0.5 text-ui-2xs text-foreground-subtle">
          {intl.formatMessage(
            { id: "chat.council.roundPill" },
            { round: String(meeting.round) },
          )}
        </span>
        <span className="min-w-0 flex-1" />
        {votes.vetoes > 0 ? (
          <span className="rounded-[4px] bg-destructive/10 px-1.5 py-0.5 text-ui-2xs font-medium text-destructive">
            {intl.formatMessage(
              { id: "chat.council.veto.note" },
              { count: String(votes.vetoes) },
            )}
          </span>
        ) : null}
      </div>
      <div className="mt-2.5 flex items-center gap-3">
        {/* 票数条：三段比例。无票时整条是空轨，绝不用假比例占位。 */}
        <div
          role="img"
          aria-label={
            total > 0
              ? intl.formatMessage(
                  { id: "chat.council.votes.barAria" },
                  {
                    approve: String(votes.approve),
                    reject: String(votes.reject),
                    abstain: String(votes.abstain),
                    total: String(total),
                  },
                )
              : intl.formatMessage({ id: "chat.council.votes.none" })
          }
          className="h-1.5 min-w-0 flex-1 overflow-hidden rounded-full bg-muted"
        >
          {total > 0 ? (
            <div className="flex h-full">
              {segments.map((segment) =>
                segment.count > 0 ? (
                  <span
                    key={segment.key}
                    aria-hidden="true"
                    className={cn("h-full", segment.className)}
                    style={{ width: `${(segment.count / total) * 100}%` }}
                  />
                ) : null,
              )}
            </div>
          ) : null}
        </div>
        <span className="flex shrink-0 items-center gap-2 text-ui-2xs text-foreground-subtlest">
          {segments.map((segment) => (
            <span key={segment.key} className="flex items-center gap-1">
              <span
                aria-hidden="true"
                className={cn("size-1.5 rounded-full", segment.className)}
              />
              {intl.formatMessage({ id: `chat.council.votes.${segment.key}` })}
              <span className="font-mono tabular-nums">{segment.count}</span>
            </span>
          ))}
        </span>
      </div>
    </section>
  );
}

// ── ④ 控场条 ────────────────────────────────────────────────────────

/**
 * 控场命令走 v4 命令面（councilPause / councilInterject，人工控场批）：
 * 一次一条串行发（busy 期间控件整体禁用），ACK 即本行状态（暂停旗/插话入账），
 * 失败原样落状态行。暂停旗是 ACK 驱动的本地态——弹层重开后回到「未暂停」
 * 显示，再按一次暂停是幂等操作无副作用（台账权威在 core，v1 不为显示拉查询命令）。
 * 会议进行中才渲染本区（终局会议没有可控行为）；插话点名（targetSeatIndexes）
 * 协议已留，UI 的 @解析随下批。
 */
function CouncilControlBar({
  meeting,
  sessionId,
}: {
  meeting: CouncilMeetingModel;
  sessionId: string;
}) {
  const { intl } = useZCodeIntl();
  const { sendCommand } = useV4Conversation();
  const [paused, setPaused] = useState<boolean | null>(null);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<{ tone: "ok" | "error"; text: string } | null>(null);

  const togglePause = useCallback(
    async (next: boolean) => {
      setBusy(true);
      setNotice(null);
      try {
        const ack = await sendCommand(
          createCommandEnvelope({
            type: "councilPause",
            payload: { councilId: meeting.councilId, paused: next },
            sessionId,
          }),
        );
        if (ack.status === "accepted" && ack.result?.type === "councilPause") {
          setPaused(ack.result.paused);
          setNotice({
            tone: "ok",
            text: intl.formatMessage({
              id: ack.result.paused
                ? "chat.council.controls.pauseAck"
                : "chat.council.controls.resumeAck",
            }),
          });
        } else {
          setNotice({
            tone: "error",
            text: intl.formatMessage(
              { id: "chat.council.controls.failed" },
              { error: ack.message ?? ack.reasonCode ?? ack.status },
            ),
          });
        }
      } catch (error) {
        setNotice({
          tone: "error",
          text: intl.formatMessage(
            { id: "chat.council.controls.failed" },
            { error: error instanceof Error ? error.message : String(error) },
          ),
        });
      } finally {
        setBusy(false);
      }
    },
    [sendCommand, meeting.councilId, sessionId, intl],
  );

  const sendInterjection = useCallback(async () => {
    const text = draft.trim();
    if (!text) return;
    setBusy(true);
    setNotice(null);
    try {
      const ack = await sendCommand(
        createCommandEnvelope({
          type: "councilInterject",
          payload: { councilId: meeting.councilId, text },
          sessionId,
        }),
      );
      if (ack.status === "accepted" && ack.result?.type === "councilInterject") {
        setDraft("");
        setNotice({
          tone: "ok",
          text: intl.formatMessage({ id: "chat.council.controls.interjectAck" }),
        });
      } else {
        setNotice({
          tone: "error",
          text: intl.formatMessage(
            { id: "chat.council.controls.failed" },
            { error: ack.message ?? ack.reasonCode ?? ack.status },
          ),
        });
      }
    } catch (error) {
      setNotice({
        tone: "error",
        text: intl.formatMessage(
          { id: "chat.council.controls.failed" },
          { error: error instanceof Error ? error.message : String(error) },
        ),
      });
    } finally {
      setBusy(false);
    }
  }, [draft, sendCommand, meeting.councilId, sessionId, intl]);

  const isPaused = paused === true;
  return (
    <TooltipProvider>
      <section
        aria-label={intl.formatMessage({ id: "chat.council.section.controls" })}
        className="flex flex-wrap items-center gap-2 border-t border-card-border/60 px-3.5 py-3"
        data-testid="council-zone-controls"
      >
        {isPaused ? (
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={busy}
            onClick={() => void togglePause(false)}
          >
            <CirclePlay aria-hidden="true" className="size-3.5" />
            {intl.formatMessage({ id: "chat.council.controls.resume" })}
          </Button>
        ) : (
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={busy}
            onClick={() => void togglePause(true)}
          >
            <CirclePause aria-hidden="true" className="size-3.5" />
            {intl.formatMessage({ id: "chat.council.controls.pause" })}
          </Button>
        )}
        <Input
          type="text"
          aria-label={intl.formatMessage({ id: "chat.council.controls.interjectAria" })}
          placeholder={intl.formatMessage({ id: "chat.council.controls.interjectPlaceholder" })}
          className="min-w-0 flex-1 basis-48"
          value={draft}
          disabled={busy}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.nativeEvent.isComposing) {
              event.preventDefault();
              void sendInterjection();
            }
          }}
        />
        <Button
          type="button"
          variant="secondary"
          size="sm"
          disabled={busy || draft.trim().length === 0}
          onClick={() => void sendInterjection()}
        >
          <SendHorizontal aria-hidden="true" className="size-3.5" />
          {intl.formatMessage({ id: "chat.council.controls.send" })}
        </Button>
        {notice ? (
          <p
            role="status"
            className={cn(
              "w-full text-ui-2xs",
              notice.tone === "ok" ? "text-foreground-subtle" : "text-destructive",
            )}
          >
            {notice.text}
          </p>
        ) : null}
      </section>
    </TooltipProvider>
  );
}

// ── 专区弹层 ─────────────────────────────────────────────────────────

export function CouncilMeetingZone({
  meeting,
  open,
  onOpenChange,
  sessionId,
}: {
  meeting: CouncilMeetingModel;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** 召集方会话：控场命令挂在它的 runtime 上；缺席（草稿态）不出控件。 */
  sessionId?: string | null;
}) {
  const { intl } = useZCodeIntl();
  const controlsLive = meeting.status === "running" && typeof sessionId === "string";
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="flex max-w-2xl flex-col gap-0 p-0"
        data-testid="council-meeting-zone"
      >
        <DialogHeader className="gap-1.5 border-b border-card-border/60 px-4 py-3.5 pr-12 text-left">
          <DialogTitle className="flex items-center gap-2">
            {intl.formatMessage({ id: "chat.council.title" })}
            {meeting.kind ? (
              <span className="text-ui-sm font-normal text-foreground-subtlest">
                {intl.formatMessage({
                  id: `chat.council.kind.${meeting.kind}`,
                })}
              </span>
            ) : null}
          </DialogTitle>
          <DialogDescription>
            {intl.formatMessage({ id: "chat.council.zone.subtitle" })}
          </DialogDescription>
        </DialogHeader>
        <div className="flex max-h-[60vh] flex-col gap-3 overflow-y-auto px-4 py-3.5">
          <CouncilStatusStrip meeting={meeting} />
          <CouncilVerdictCard meeting={meeting} />
          <CouncilProcessFlow meeting={meeting} />
        </div>
        {controlsLive ? (
          <CouncilControlBar meeting={meeting} sessionId={sessionId} />
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
