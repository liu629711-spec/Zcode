import type { ZCodeTaskMeta } from "@zcode/shared";
import {
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
} from "@/components/ui/context-menu.js";
import { TaskGroupColorDot } from "@/workspace-grouped-tasks/colors.js";
import type { TaskGroupMenuItem } from "@/workspace-grouped-tasks/types.js";

export function GroupedTaskContextMenuContent({
  task,
  currentGroupId,
  groups,
  intl,
  fileManagerLabel,
  taskSessionFile,
  taskNativeSessionLogFile,
  onMoveTaskToGroup,
  onMoveTaskToTop,
  onStartRenameTask,
  onWriteTicketSpec,
  onArchiveTask,
  onMarkTaskAsUnread,
  onOpenTaskPathInFileManager,
  onCopyText,
  onViewModelTrajectory,
  onOpenTaskFeedback,
  disabledReason,
}: {
  task: ZCodeTaskMeta;
  currentGroupId?: string;
  groups: TaskGroupMenuItem[];
  intl: {
    formatMessage: (desc: { id: string }) => string;
  };
  fileManagerLabel: string;
  taskSessionFile: { loading: boolean; path: string | null };
  taskNativeSessionLogFile: { loading: boolean; path: string | null };
  onMoveTaskToGroup: (task: ZCodeTaskMeta, groupId: string | null) => void;
  onMoveTaskToTop: (task: ZCodeTaskMeta) => void;
  onStartRenameTask: (task: ZCodeTaskMeta) => void;
  onWriteTicketSpec: (task: ZCodeTaskMeta) => void;
  onArchiveTask: (task: ZCodeTaskMeta) => void;
  onMarkTaskAsUnread: (task: ZCodeTaskMeta) => void;
  onOpenTaskPathInFileManager: () => void;
  onCopyText: (label: string, text: string | null) => void;
  onViewModelTrajectory?: () => void;
  onOpenTaskFeedback: () => void;
  disabledReason?: string;
}) {
  return (
    <ContextMenuContent className="w-56">
      <ContextMenuSub>
        <ContextMenuSubTrigger disabled={Boolean(disabledReason)} title={disabledReason}>
          {intl.formatMessage({ id: "taskGroup.moveToGroup" })}
        </ContextMenuSubTrigger>
        <ContextMenuSubContent className="w-52">
          <ContextMenuItem
            disabled={Boolean(disabledReason) || !currentGroupId}
            title={disabledReason}
            onSelect={() => {
              if (!disabledReason) {
                onMoveTaskToGroup(task, null);
              }
            }}
          >
            {intl.formatMessage({ id: "taskGroup.removeFromGroup" })}
          </ContextMenuItem>
          <ContextMenuSeparator />
          {groups.map((group) => (
            <ContextMenuItem
              key={group.id}
              disabled={Boolean(disabledReason) || group.id === currentGroupId}
              title={disabledReason}
              onSelect={() => {
                if (!disabledReason) {
                  onMoveTaskToGroup(task, group.id);
                }
              }}
            >
              <TaskGroupColorDot color={group.color} />
              <span className="truncate">{group.title}</span>
            </ContextMenuItem>
          ))}
        </ContextMenuSubContent>
      </ContextMenuSub>
      <ContextMenuItem
        disabled={Boolean(disabledReason)}
        title={disabledReason}
        onSelect={() => {
          if (!disabledReason) {
            onMoveTaskToTop(task);
          }
        }}
      >
        {intl.formatMessage({ id: "taskGroup.moveToTop" })}
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem
        disabled={Boolean(disabledReason)}
        title={disabledReason}
        onSelect={() => {
          if (!disabledReason) {
            onStartRenameTask(task);
          }
        }}
      >
        {intl.formatMessage({ id: "taskList.rename" })}
      </ContextMenuItem>
      {/* 上票入口（作者通道）：写验收标准是任务进派活台看板的唯一门票；未进台的任务不在看板上。 */}
      <ContextMenuItem
        disabled={Boolean(disabledReason)}
        title={disabledReason}
        onSelect={() => {
          if (!disabledReason) {
            onWriteTicketSpec(task);
          }
        }}
      >
        {intl.formatMessage({ id: "dispatchDesk.writeSpec" })}
      </ContextMenuItem>
      <ContextMenuItem
        disabled={Boolean(disabledReason)}
        title={disabledReason}
        onSelect={() => {
          if (!disabledReason) {
            onArchiveTask(task);
          }
        }}
      >
        {intl.formatMessage({ id: "taskList.archive" })}
      </ContextMenuItem>
      <ContextMenuItem
        disabled={Boolean(disabledReason)}
        title={disabledReason}
        onSelect={() => {
          if (!disabledReason) {
            onMarkTaskAsUnread(task);
          }
        }}
      >
        {intl.formatMessage({ id: "taskList.markAsUnread" })}
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem
        disabled={Boolean(disabledReason)}
        title={disabledReason}
        onSelect={() => {
          if (!disabledReason) {
            onOpenTaskPathInFileManager();
          }
        }}
      >
        {fileManagerLabel}
      </ContextMenuItem>
      <ContextMenuItem
        onSelect={() =>
          onCopyText(intl.formatMessage({ id: "appHeader.copyPath" }), task.workspacePath)
        }
      >
        {intl.formatMessage({ id: "appHeader.copyPath" })}
      </ContextMenuItem>
      <ContextMenuItem
        disabled={taskSessionFile.loading || !taskSessionFile.path}
        onSelect={() =>
          onCopyText(intl.formatMessage({ id: "appHeader.copyTaskPath" }), taskSessionFile.path)
        }
      >
        {intl.formatMessage({ id: "appHeader.copyTaskPath" })}
      </ContextMenuItem>
      <ContextMenuItem
        disabled={taskNativeSessionLogFile.loading || !taskNativeSessionLogFile.path}
        onSelect={() =>
          onCopyText(
            intl.formatMessage({ id: "appHeader.copyLogPath" }),
            taskNativeSessionLogFile.path,
          )
        }
      >
        {intl.formatMessage({ id: "appHeader.copyLogPath" })}
      </ContextMenuItem>
      <ContextMenuItem
        onSelect={() =>
          onCopyText(intl.formatMessage({ id: "appHeader.copySessionId" }), task.taskId)
        }
      >
        {intl.formatMessage({ id: "appHeader.copySessionId" })}
      </ContextMenuItem>
      {onViewModelTrajectory ? (
        <>
          <ContextMenuSeparator />
          {/* 调用轨迹查看：从 ~/.zcode/cli 的 model-io 还原该 task 的模型请求/响应/工具调用。
              只依赖 taskId（即 sessionId），不依赖快照文件是否落盘。与侧栏行菜单同款。 */}
          <ContextMenuItem
            disabled={Boolean(disabledReason)}
            title={disabledReason}
            onSelect={() => {
              if (!disabledReason) {
                onViewModelTrajectory();
              }
            }}
          >
            {intl.formatMessage({ id: "taskList.viewModelTrajectory" })}
          </ContextMenuItem>
        </>
      ) : null}
      <ContextMenuSeparator />
      <ContextMenuItem onSelect={onOpenTaskFeedback}>
        {intl.formatMessage({ id: "taskList.feedback" })}
      </ContextMenuItem>
    </ContextMenuContent>
  );
}
