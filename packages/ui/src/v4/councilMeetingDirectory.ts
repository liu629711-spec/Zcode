// ============================================================
// 会议室侧栏分组的目录纯规则（2026-10-03 圆桌会刀1）：council/list 返回的
// 会议摘要 → 进行中/已收口两小节。收口含流会（deadlocked）与取消（全场派单
// 失败如实取消的哑会）——终态都是「不再推进」，统一进已收口，徽章仍分词。
// 纯函数零依赖可单测（councilMeetingDirectory.test.ts）：shared 的
// ZCodeCouncilMeetingSummary 在结构上兼容本文件的窄形状（同 councilMeeting.ts
// 的 CouncilEvidenceUnit 纪律——@/ 别名与 shared 类型不进编译面）。
// ============================================================

/** council/list 会议摘要的窄形状（与 shared ZCodeCouncilMeetingSummary 手工同步）。 */
export interface CouncilMeetingSummaryLike {
  councilId: string;
  /** 召集方会话（UI taskId ≡ sessionId）：点击定位目标。 */
  sessionId: string;
  kind: "plan" | "acceptance";
  status: "proposed" | "running" | "approved" | "rejected" | "deadlocked" | "cancelled";
  motion?: string;
  title?: string;
  timeUpdated: number;
}

/** 目录小节：running=进行中；closed=已收口（通过/否决/流会/取消）。 */
export type CouncilMeetingDirectoryGroup = "running" | "closed";

export interface CouncilMeetingDirectoryEntry {
  councilId: string;
  sessionId: string;
  kind: "plan" | "acceptance";
  status: CouncilMeetingSummaryLike["status"];
  group: CouncilMeetingDirectoryGroup;
  motion?: string;
  title?: string;
  timeUpdated: number;
}

export interface CouncilMeetingDirectory {
  running: CouncilMeetingDirectoryEntry[];
  closed: CouncilMeetingDirectoryEntry[];
}

/** 小节归属：proposed/running 在册即进行中；四个终态都算已收口（含流会）。 */
function groupOf(status: CouncilMeetingSummaryLike["status"]): CouncilMeetingDirectoryGroup {
  return status === "running" || status === "proposed" ? "running" : "closed";
}

function toEntry(meeting: CouncilMeetingSummaryLike): CouncilMeetingDirectoryEntry {
  return {
    councilId: meeting.councilId,
    sessionId: meeting.sessionId,
    kind: meeting.kind,
    status: meeting.status,
    group: groupOf(meeting.status),
    ...(meeting.motion ? { motion: meeting.motion } : {}),
    ...(meeting.title ? { title: meeting.title } : {}),
    timeUpdated: meeting.timeUpdated,
  };
}

/** 单小节内按更新时间倒序（最近推进的会议排最上）。 */
function byRecency(left: CouncilMeetingDirectoryEntry, right: CouncilMeetingDirectoryEntry) {
  return right.timeUpdated - left.timeUpdated;
}

export function buildCouncilMeetingDirectory(
  meetings: readonly CouncilMeetingSummaryLike[],
): CouncilMeetingDirectory {
  const entries = meetings.map(toEntry);
  return {
    running: entries.filter((entry) => entry.group === "running").sort(byRecency),
    closed: entries.filter((entry) => entry.group === "closed").sort(byRecency),
  };
}

/** 条目主文案：议题原话优先，退召集方短标题；都没有退 undefined（组件再退类型词）。 */
export function councilMeetingEntrySubject(
  entry: Pick<CouncilMeetingDirectoryEntry, "motion" | "title">,
): string | undefined {
  return entry.motion?.trim() || entry.title?.trim() || undefined;
}
