import { create } from "zustand";
import type { ZCodeTaskMeta } from "@zcode/shared";

/**
 * Header「···」菜单借用侧栏档案操作链路的请求通道（D4 一致性）：
 * 编辑对话框、删除确认与列表刷新都住在 WorkspaceSidebar（它才持有档案列表与对话框状态），
 * Header 只发请求；侧栏未挂载（处理器缺席）时菜单项不渲染，不留死入口。
 */
export interface ProjectAgentProfileActionHandlers {
  edit: (task: ZCodeTaskMeta) => void;
  remove: (task: ZCodeTaskMeta) => void;
}

interface ProjectAgentProfileActionState {
  handlers: ProjectAgentProfileActionHandlers | null;
  setHandlers: (handlers: ProjectAgentProfileActionHandlers | null) => void;
}

export const useProjectAgentProfileActionStore = create<ProjectAgentProfileActionState>()((set) => ({
  handlers: null,
  setHandlers: (handlers) => set({ handlers }),
}));

/** 发一条档案操作请求；处理器缺席返回 false（调用方据此降级）。 */
export function requestProjectAgentProfileAction(
  kind: "edit" | "remove",
  task: ZCodeTaskMeta,
): boolean {
  const handlers = useProjectAgentProfileActionStore.getState().handlers;
  if (!handlers) {
    return false;
  }
  (kind === "edit" ? handlers.edit : handlers.remove)(task);
  return true;
}