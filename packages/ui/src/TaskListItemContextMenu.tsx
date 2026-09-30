import {
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
} from "@/components/ui/context-menu.js";
import { TaskActionMenuContent } from "@/TaskActionMenuContent.js";

export function TaskListItemContextMenu({
  intl,
  isPinned,
  fileManagerLabel,
  taskSessionFile,
  activeSessionId,
  taskNativeSessionLogFile,
  onTogglePinTask,
  onStartRenameTask,
  onArchiveTask,
  onMarkTaskAsUnread,
  onOpenInSplitPane,
  openInSplitPaneDisabled,
  onOpenTaskFeedback,
  onOpenTaskPathInFileManager,
  onCopyWorkspacePath,
  onCopyTaskPath,
  onCopyTaskLogPath,
  onCopySessionId,
  onViewModelTrajectory,
  onEditProjectAgent,
  onPromoteProjectAgent,
  onDeleteProjectAgent,
  disableTaskActions = false,
  disabledReason,
}: {
  intl: {
    formatMessage: (desc: { id: string }, values?: Record<string, string>) => string;
  };
  isPinned: boolean;
  fileManagerLabel: string;
  taskSessionFile: { loading: boolean; path: string | null; exists: boolean };
  activeSessionId?: string | null;
  taskNativeSessionLogFile: {
    loading: boolean;
    path: string | null;
    exists: boolean;
  };
  onTogglePinTask: () => void;
  onStartRenameTask: () => void;
  onArchiveTask: () => void;
  onMarkTaskAsUnread: () => void;
  /** 「在分屏打开」（仅桌面 shell 传入）。 */
  onOpenInSplitPane?: () => void;
  /** 叶子数达上限且该 session 未在任何 pane 时禁用。 */
  openInSplitPaneDisabled?: boolean;
  onOpenTaskFeedback: () => void;
  onOpenTaskPathInFileManager: () => void;
  onCopyWorkspacePath: () => void;
  onCopyTaskPath: () => void;
  onCopyTaskLogPath: () => void;
  onCopySessionId?: () => void;
  onViewModelTrajectory?: () => void;
  /** 驻场智能体行的档案操作（G5/D4）：由调用方按徽章判定后传入，缺席不渲染。 */
  onEditProjectAgent?: () => void;
  /** 收编（身份轴终局 §九④）：项目档案升级为用户级全局员工，调用方按 scope 判定后传入。 */
  onPromoteProjectAgent?: () => void;
  onDeleteProjectAgent?: () => void;
  disableTaskActions?: boolean;
  disabledReason?: string;
}) {
  return (
    <ContextMenuContent className="w-52">
      <TaskActionMenuContent
        intl={intl}
        isPinned={isPinned}
        fileManagerLabel={fileManagerLabel}
        taskSessionFile={taskSessionFile}
        activeSessionId={activeSessionId}
        taskNativeSessionLogFile={taskNativeSessionLogFile}
        Item={ContextMenuItem}
        Separator={ContextMenuSeparator}
        onTogglePinTask={onTogglePinTask}
        onStartRenameTask={onStartRenameTask}
        onArchiveTask={onArchiveTask}
        onMarkTaskAsUnread={onMarkTaskAsUnread}
        onOpenInSplitPane={onOpenInSplitPane}
        openInSplitPaneDisabled={openInSplitPaneDisabled}
        onOpenTaskFeedback={onOpenTaskFeedback}
        onOpenTaskPathInFileManager={onOpenTaskPathInFileManager}
        onCopyWorkspacePath={onCopyWorkspacePath}
        onCopyTaskPath={onCopyTaskPath}
        onCopyTaskLogPath={onCopyTaskLogPath}
        onCopySessionId={onCopySessionId}
        onViewModelTrajectory={onViewModelTrajectory}
        disableTaskActions={disableTaskActions}
        disabledReason={disabledReason}
      />
      {onEditProjectAgent || onPromoteProjectAgent || onDeleteProjectAgent ? (
        <>
          <ContextMenuSeparator />
          {onEditProjectAgent ? (
            <ContextMenuItem onSelect={onEditProjectAgent}>
              {intl.formatMessage({ id: "workspaceSidebar.projectAgentEditMenu" })}
            </ContextMenuItem>
          ) : null}
          {onPromoteProjectAgent ? (
            <ContextMenuItem onSelect={onPromoteProjectAgent}>
              {intl.formatMessage({ id: "workspaceSidebar.projectAgentPromoteMenu" })}
            </ContextMenuItem>
          ) : null}
          {onDeleteProjectAgent ? (
            <ContextMenuItem onSelect={onDeleteProjectAgent}>
              {intl.formatMessage({ id: "workspaceSidebar.projectAgentDeleteMenu" })}
            </ContextMenuItem>
          ) : null}
        </>
      ) : null}
    </ContextMenuContent>
  );
}
