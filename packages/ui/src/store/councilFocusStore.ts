// 会议室侧栏 → 会话内会议卡的定向打开（2026-10-03 圆桌会刀1）：侧栏点击 =
// 打开召集方会话 + 自动展开该会议的评审专区。跨组件握手走最小 zustand 单例：
// 侧栏 request，CouncilMeetingCard 挂载时 consume（对上号才开，一次性）。
// ponytail: 无过期清理——consume 只认最新一次 request，同会议重复点击会覆盖，
// 最坏情况是「点了侧栏但很久后才进那个会话」时专区自动弹开一次，可接受；
// 若产品要严格时效，给 request 加 requestedAt 过期判断即可。

import { create } from "zustand";

interface CouncilFocusState {
  focus: { councilId: string } | null;
  /** 侧栏点击：登记要打开的会议（覆盖上一次未消费的请求）。 */
  requestCouncilFocus: (councilId: string) => void;
  /** 会议卡挂载时对号：命中即清空并返回 true（调用方借此展开专区）。 */
  consumeCouncilFocus: (councilId: string) => boolean;
}

export const useCouncilFocusStore = create<CouncilFocusState>((set, get) => ({
  focus: null,
  requestCouncilFocus: (councilId) => set({ focus: { councilId } }),
  consumeCouncilFocus: (councilId) => {
    const current = get().focus;
    if (!current || current.councilId !== councilId) return false;
    set({ focus: null });
    return true;
  },
}));
