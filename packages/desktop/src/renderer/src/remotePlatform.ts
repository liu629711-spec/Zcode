import type { IPlatformService } from "@zcode/shared";

/**
 * Web 远控壳的平台桩：Root 的 isDesktop=false 路径本就按"Web/mobile 空实现"设计
 * （Root.tsx 注释明示 telemetry/发送漏斗在 Web/mobile 不装 reporter），这里只兜住
 * 偶发的平台方法调用不让它炸——真机跑出缺口的口子再逐个补真实实现。
 *
 * ponytail: Proxy no-op 兜底是刻意的宽口径；出现"调了没效果"的功能缺口时，
 * 在 base 里补显式实现（如 openExternal 直接 window.open），不追着枚举全部方法。
 */
export function createWebRemotePlatform(): IPlatformService {
  const base = {
    canSelectFilePath: false,
    isLocalDevelopmentRuntime: import.meta.env.DEV === true,
    getDeviceId: () => "web-remote",
    getSystemLocale: () =>
      Promise.resolve(
        navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US",
      ),
    openExternal: (url: string) => {
      window.open(url, "_blank", "noopener");
      return Promise.resolve();
    },
    exportLogs: () => Promise.resolve(),
    syncWindowUnreadCount: () => Promise.resolve(),
    syncWindowTabs: () => Promise.resolve(),
    showTaskNotification: () => Promise.resolve(),
    reportTelemetryEvent: () => Promise.resolve(),
    reportArmsCustomEvent: () => Promise.resolve(),
  };

  return new Proxy(base, {
    get(target, prop, receiver) {
      if (prop in target) {
        return Reflect.get(target, prop, receiver);
      }
      // 订阅类方法（onXxx(handler): () => void）调用方会把返回值直接当退订函数调，
      // 返回 Promise 会炸 "not a function... instance of Promise"——必须返回退订函数。
      if (/^on[A-Z]/.test(String(prop))) {
        return () => () => {};
      }
      // 其余未实现的平台方法统一 no-op：返回 Promise.resolve(undefined) 兼容 await 调用
      return () => Promise.resolve(undefined);
    },
  }) as unknown as IPlatformService;
}
