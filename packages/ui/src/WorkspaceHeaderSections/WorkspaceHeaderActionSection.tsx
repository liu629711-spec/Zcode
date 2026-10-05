import { SquareKanbanIcon } from "lucide-react";
import { WorkspaceEditorButtonGroup } from "@/WorkspaceEditorButtonGroup.js";
import { WorkspaceSidePaneToggleButton } from "@/WorkspaceSidePaneToggleButton.js";
import { WorkspaceTerminalToggleButton } from "@/WorkspaceTerminalToggleButton.js";
import { cn } from "@/components/lib/utils.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { WINDOWS_CAPTION_CONTROL_CLASS } from "@/windowCaptionControls.js";
import type { WorkspaceHeaderActionSectionProps } from "@/WorkspaceHeaderSections/shared.js";
import { WorkspaceHelpMenuButton } from "@/WorkspaceHelpMenuButton.js";
import { ConversationShareMenu } from "@/ConversationShareMenu.js";
import { DesktopWindowControls } from "@/DesktopWindowControls.js";

export type { WorkspaceHeaderActionSectionProps } from "@/WorkspaceHeaderSections/shared.js";

export function WorkspaceHeaderActionSection({
  variant = "task",
  activeTaskId,
  user,
  readOnlyReason,
  workspaceAbsPath,
  workspaceIdentity,
  remoteTarget,
  isDesktop,
  isTerminalOpen,
  isSidePaneOpen,
  onToggleTerminal,
  onToggleSidePane,
  onOpenTeamBoard,
  activeSessionId,
  toggleSidePaneShortcutLabel,
  onSelectedEditorChange,
  simplifyForNarrowRemote = false,
  hideHelpMenu = false,
  showWindowControls = false,
  useWindowsCaptionSpacing = false,
}: WorkspaceHeaderActionSectionProps) {
  const { intl } = useZCodeIntl();
  return (
    <div
      className={cn(
        "flex shrink-0 items-center [app-region:no-drag]",
        // Windows header 内容区有 p-2，普通工具栏按钮 hover 只覆盖 32px 高度。
        // 标题栏按钮需要抵消这层垂直内边距，和原生窗控/右侧菜单保持同一个 48px hover 面。
        useWindowsCaptionSpacing ? "-my-2 h-12 gap-0" : "gap-0.5",
      )}
    >
      {variant === "task" ? (
        <WorkspaceEditorButtonGroup
          disabledReason={readOnlyReason}
          workspaceAbsPath={workspaceAbsPath}
          workspaceIdentity={workspaceIdentity}
          remoteTarget={remoteTarget}
          onSelectedEditorChange={onSelectedEditorChange}
        />
      ) : null}
      {/* 分享发布接口依赖登录态；未登录时隐藏入口，避免用户打开后只能得到鉴权失败。 */}
      {activeTaskId && user && isDesktop !== false ? (
        <ConversationShareMenu
          taskId={activeTaskId}
          useWindowsCaptionSpacing={useWindowsCaptionSpacing}
        />
      ) : null}
      {!simplifyForNarrowRemote ? (
        <>
          {!hideHelpMenu ? <WorkspaceHelpMenuButton isDesktop={Boolean(isDesktop)} /> : null}
          {/* 远程控制移动端头部空间过窄，终端入口在这里会和核心操作争抢宽度。*/}
          <WorkspaceTerminalToggleButton
            isTerminalOpen={isTerminalOpen}
            onToggleTerminal={onToggleTerminal}
            disabledReason={readOnlyReason}
            useWindowsCaptionSpacing={useWindowsCaptionSpacing}
          />
        </>
      ) : null}
      {/* 远程控制移动端只保留图标，避免 diff 数字把按钮撑宽导致标题拥挤。 */}
      {!isSidePaneOpen && onOpenTeamBoard && activeSessionId ? (
        <button
          type="button"
          onClick={() => onOpenTeamBoard({ sessionId: activeSessionId })}
          data-testid="workspace-header-team-board"
          aria-label={intl.formatMessage({ id: "sidePane.teamBoard" })}
          title={intl.formatMessage({ id: "sidePane.teamBoard" })}
          className={cn(
            "flex size-8 shrink-0 cursor-pointer items-center justify-center rounded-[6px] text-foreground-subtle transition-colors hover:bg-hover hover:text-foreground [app-region:no-drag] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/30",
            useWindowsCaptionSpacing && WINDOWS_CAPTION_CONTROL_CLASS,
          )}
        >
          <SquareKanbanIcon className="size-4" aria-hidden="true" />
        </button>
      ) : null}
      {!isSidePaneOpen ? (
        <WorkspaceSidePaneToggleButton
          isSidePaneOpen={isSidePaneOpen}
          onToggleSidePane={onToggleSidePane}
          shortcutLabel={toggleSidePaneShortcutLabel}
          useWindowsCaptionSpacing={useWindowsCaptionSpacing}
        />
      ) : null}
      {showWindowControls && !isSidePaneOpen ? <DesktopWindowControls /> : null}
    </div>
  );
}
