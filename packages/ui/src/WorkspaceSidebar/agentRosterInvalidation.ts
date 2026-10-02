/**
 * 员工名册的跨界面失效通道（2026-10-02）：侧栏的员工列表是本地状态（挂载时拉一次），
 * 设置页增删改档案后没人通知它——名册旧名残留，@ 面板吃的又是侧栏下发的目录快照，
 * 于是「设置页删了的人还能被 @」。81d1fd6 给新建对话框补过"打开即刷新"，但那只是
 * 一个面；这里是把失效做成共享通道：任何界面动过档案就广播一声，侧栏听到即重拉，
 * 名单一新，目录快照与 @ 面板跟着一新。
 * 进程内即可（同窗口）：设置页与侧栏同属一个 renderer。
 */
const listeners = new Set<() => void>();

export function notifyAgentRosterChanged(): void {
  for (const listener of listeners) {
    listener();
  }
}

/** 订阅名册失效；返回退订函数（effect 清理直接用）。 */
export function onAgentRosterChanged(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
