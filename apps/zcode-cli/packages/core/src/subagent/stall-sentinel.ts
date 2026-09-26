// ============================================================
// 停滞哨兵：一段安静期报一次，活动即归零（纯定时器，零依赖）
// ============================================================
// runner.ts 的活动看门狗只说「超时即中止」；这里补上**中止之前**的那一句：
// idle 越过阈值时旁路报一次停滞（每段安静期至多一次），让观察面能在 abort
// 落地之前把「还在跑但没动静」说出来。纪律：纯观察，不持有 abort、不改看门狗
// 的中止语义——与 bootstrap 的 RunStallClock 同一条「每段至多一条」的规矩。
//
// 零依赖是有意的：看门狗的判定与上报要能脱离 runtime 依赖图单独验证
// （stall-sentinel.test.ts 用 node:test 的假时钟直跑本文件）。

/** 一次停滞观察：静了多久、最后活动落在何时（epoch ms）。 */
export interface StallObservation {
  idleMs: number;
  lastActivityAt: number;
}

export interface StallSentinelOptions {
  /** 安静多久算停滞。非正或非有限值时哨兵退化为空操作（与看门狗同一条守卫）。 */
  afterMs: number;
  onStalled: (observation: StallObservation) => void;
}

export interface StallSentinel {
  /**
   * 活动信号：结束当前停滞段（允许下一段重新上报）并重新上膛。
   * 与看门狗的 reportActivity 一一配对调用。
   */
  noteActivity(): void;
  /** 撤表：不再上报。完成、中止、关闭时调用，不让定时器把进程钉住。 */
  stop(): void;
}

export function createStallSentinel(options: StallSentinelOptions): StallSentinel {
  if (!Number.isFinite(options.afterMs) || options.afterMs <= 0) {
    return { noteActivity: () => {}, stop: () => {} };
  }

  let lastActivityAt = Date.now();
  let notified = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const clear = () => {
    if (timer !== undefined) {
      clearTimeout(timer);
      timer = undefined;
    }
  };

  const fire = () => {
    timer = undefined;
    // 一段安静期只报一次：反复刷屏的「还没动」不是观察，是噪音。
    if (notified) return;
    notified = true;
    options.onStalled({ idleMs: Date.now() - lastActivityAt, lastActivityAt });
  };

  return {
    noteActivity: () => {
      lastActivityAt = Date.now();
      notified = false;
      clear();
      timer = setTimeout(fire, options.afterMs);
      // 停滞观察没有阻进程退出的投票权：run 结束、进程要走时它不该钉住谁。
      if (typeof timer === "object" && timer !== null && "unref" in timer) timer.unref();
    },
    stop: clear,
  };
}
