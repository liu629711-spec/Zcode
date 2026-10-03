// 圆桌画布两态挂载（2026-10-03 刀2）：会话⇄圆桌的切换状态 + 画布数据桥。
//
// 两态挂载形态：会议卡「查看」/侧栏条目触发后，当前会话 pane 顶部叠一层
// 不透明的圆桌画布层（CouncilStageLayer，挂 SessionPane 根部）——会话视图
// 与圆桌视图互斥可见，退回即摘层。不开新 BrowserWindow、不加 WindowTabState
// kind、不动 pane 布局树。
//
// 数据桥：CouncilMeetingModel 只能从会话轮证据聚合（selectCouncilMeetings），
// 而 renderUnits 只有 ConversationTimeline 看得见全量——timeline 用
// syncCouncilMeetings 把本会话的会议模型同步进来（画布层开/关都不停更，
// timeline 在层下继续活着）；层按 openCouncilId 对号取用。
//
// ponytail: openCouncilId 全局单值——会议模型只存在于其召集会话的证据里，
// 「哪个 pane 显示画布」由本会话有没有这个会议隐式决定，不需要再存 sessionId；
// 会议证据滚出数据层窗口时模型消失，画布自动让位回会话视图，属已知边界。

import { create } from "zustand";
import type { CouncilMeetingModel } from "@/v4/councilMeeting.js";

interface CouncilStageState {
  /** 当前打开的圆桌画布对应的会议；null=会话态。 */
  openCouncilId: string | null;
  /** 各会话最新聚合出的会议模型（ConversationTimeline 同步，画布消费）。 */
  meetingsBySession: Record<string, readonly CouncilMeetingModel[]>;
  /** 进入圆桌态（会议卡「查看」/侧栏定向打开都走这里）。 */
  openCouncilStage: (councilId: string) => void;
  /** 退回会话态。 */
  closeCouncilStage: () => void;
  /** timeline 同步：本会话全量会议模型（后到覆盖先到）。 */
  syncCouncilMeetings: (
    sessionKey: string,
    meetings: readonly CouncilMeetingModel[],
  ) => void;
  /** timeline 卸载时清掉本会话的同步残留。 */
  clearCouncilMeetings: (sessionKey: string) => void;
}

export const useCouncilStageStore = create<CouncilStageState>((set) => ({
  openCouncilId: null,
  meetingsBySession: {},
  openCouncilStage: (councilId) => set({ openCouncilId: councilId }),
  closeCouncilStage: () => set({ openCouncilId: null }),
  syncCouncilMeetings: (sessionKey, meetings) =>
    set((state) => ({
      meetingsBySession:
        state.meetingsBySession[sessionKey] === meetings
          ? state.meetingsBySession
          : { ...state.meetingsBySession, [sessionKey]: meetings },
    })),
  clearCouncilMeetings: (sessionKey) =>
    set((state) => {
      if (!(sessionKey in state.meetingsBySession)) return state;
      const { [sessionKey]: _removed, ...rest } = state.meetingsBySession;
      return { meetingsBySession: rest };
    }),
}));
