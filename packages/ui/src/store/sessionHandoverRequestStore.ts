import { create } from "zustand";

/**
 * 会话内「换个新会话继续」（换班）的请求通道：
 * SessionPane（横幅所在处）只知道当前会话的 id/工作区，而建新会话（persona 构造、
 * createSession transport、firstInput 播种）、归档旧会话与跳转导航的全套能力都在
 * WorkspaceSidebar —— 照 projectAgentProfileActionStore 的 D27 先例，Pane 只发请求，
 * 侧栏未挂载（处理器缺席）时横幅按钮不渲染，不留死入口。
 */
export interface SessionHandoverRequest {
  sessionId: string;
  workspacePath: string;
  workspaceIdentity?: string;
  /** 会话标题（侧栏按标题前缀反推员工档案用；普通会话可为空串）。 */
  title: string;
}

export interface SessionHandoverHandlers {
  handover: (request: SessionHandoverRequest) => void;
}

interface SessionHandoverState {
  handlers: SessionHandoverHandlers | null;
  setHandlers: (handlers: SessionHandoverHandlers | null) => void;
}

export const useSessionHandoverStore = create<SessionHandoverState>()((set) => ({
  handlers: null,
  setHandlers: (handlers) => set({ handlers }),
}));

/** 发一条换班请求；处理器缺席返回 false（调用方据此隐藏/降级入口）。 */
export function requestSessionHandover(request: SessionHandoverRequest): boolean {
  const handlers = useSessionHandoverStore.getState().handlers;
  if (!handlers) {
    return false;
  }
  handlers.handover(request);
  return true;
}
