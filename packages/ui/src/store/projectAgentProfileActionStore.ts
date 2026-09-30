import { create } from "zustand";
import type { AgentColor, ZCodeTaskMeta } from "@zcode/shared";

/** 设置页改名的补链请求（D27）：侧栏才有会话行与任务服务，设置页只报"这个人改名了"。 */
export interface ProjectAgentRenameRelinkRequest {
  workspacePath: string;
  workspaceIdentity?: string;
  agentId?: string;
  oldName: string;
  newName: string;
  color?: AgentColor;
}

/**
 * Header「···」菜单借用侧栏档案操作链路的请求通道（D4 一致性）：
 * 编辑对话框、删除确认与列表刷新都住在 WorkspaceSidebar（它才持有档案列表与对话框状态），
 * Header 只发请求；侧栏未挂载（处理器缺席）时菜单项不渲染，不留死入口。
 */
export interface ProjectAgentProfileActionHandlers {
  edit: (task: ZCodeTaskMeta) => void;
  remove: (task: ZCodeTaskMeta) => void;
  /** 收编（身份轴终局 §九④）：把项目档案升级为用户级全局员工（工号随迁）。 */
  promote: (task: ZCodeTaskMeta) => void;
  /** 改名补链（D27）：设置页改过名后请侧栏把该员工的历史会话标题一起换牌。 */
  renameRelink: (request: ProjectAgentRenameRelinkRequest) => void;
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
  kind: "edit" | "remove" | "promote",
  task: ZCodeTaskMeta,
): boolean {
  const handlers = useProjectAgentProfileActionStore.getState().handlers;
  if (!handlers) {
    return false;
  }
  const handler =
    kind === "edit" ? handlers.edit : kind === "remove" ? handlers.remove : handlers.promote;
  handler(task);
  return true;
}

/**
 * 设置页改名的补链请求（D27）。侧栏没挂载（处理器缺席）就返回 false：
 * 会话标题保持旧名前缀，工牌靠号登记与档案目录刷新仍然对得上，
 * 只是那批"只能按标题前缀反推"的老会话要等下一次从会话行改名才跟上。
 */
export function requestProjectAgentRenameRelink(
  request: ProjectAgentRenameRelinkRequest,
): boolean {
  const handlers = useProjectAgentProfileActionStore.getState().handlers;
  if (!handlers) {
    return false;
  }
  handlers.renameRelink(request);
  return true;
}