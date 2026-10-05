/**
 * 员工工位会话侧板（团队看板批3b 挂账③）：看板里点成员/通讯跳到员工的工位
 * transcript——只读复用 SessionPane，与 SubagentSessionSidePane 同一渲染路径，
 * 但用独立的 team-desk tab 类型：工位是顶层会话，绝不进子智能体 tab 的 GC
 * 清算集合（WorkflowActorSession 同款理由链，见 workspaceSidePane.ts 注释）。
 * @module team desk session pane
 */
import { useMemo } from "react";
import type { MessageFileLinkTarget } from "@/components/ai-elements/message.js";
import { V4PaneConversationProvider } from "@/v4/V4ConversationContext.js";
import { SessionPane } from "@/v4/SessionPane.js";
import type {
  OpenBackgroundBashSideTabRequest,
  OpenScopedSubagentSideTabRequest,
  TeamDeskSessionSidePaneTab,
} from "@/lib/workspaceSidePane.js";

export function TeamDeskSessionSidePane({
  tab,
  focused,
  onOpenBrowserUrl,
  onOpenCodeViewer,
  onOpenFileLink,
  onOpenSubagentSession,
  onOpenBackgroundBash,
}: {
  tab: TeamDeskSessionSidePaneTab;
  focused: boolean;
  onOpenBrowserUrl: (url: string) => void;
  onOpenCodeViewer: (source: import("@/lib/codeViewer.js").CodeViewerSource) => void;
  onOpenFileLink?: (target: MessageFileLinkTarget) => void;
  onOpenSubagentSession: (request: OpenScopedSubagentSideTabRequest) => void;
  onOpenBackgroundBash?: (request: OpenBackgroundBashSideTabRequest) => void;
}) {
  const scope = useMemo(
    () => ({
      workspacePath: tab.workspacePath,
      ...(tab.workspaceIdentity ? { workspaceIdentity: tab.workspaceIdentity } : {}),
      ...(tab.remoteSessionId ? { remoteSessionId: tab.remoteSessionId } : {}),
    }),
    [tab.remoteSessionId, tab.workspaceIdentity, tab.workspacePath],
  );

  return (
    <V4PaneConversationProvider scope={scope}>
      <SessionPane
        paneId={tab.id}
        sessionId={tab.deskSessionId}
        openTrigger="subagent"
        rootSessionId={tab.sessionId}
        readOnly
        allowWorkspaceFileRewind
        focused={focused}
        telemetryVisible={focused}
        workspacePath={tab.workspacePath}
        workspaceIdentity={tab.workspaceIdentity}
        remoteSessionId={tab.remoteSessionId}
        onOpenBrowserUrl={onOpenBrowserUrl}
        onOpenCodeViewer={onOpenCodeViewer}
        onOpenFileLink={onOpenFileLink}
        onOpenSubagentSession={onOpenSubagentSession}
        onOpenBackgroundBash={onOpenBackgroundBash}
      />
    </V4PaneConversationProvider>
  );
}
