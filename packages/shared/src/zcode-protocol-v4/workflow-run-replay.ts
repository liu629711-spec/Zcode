// ============================================================
// workflow run 的**回放折叠**：journal 事件流 → 节点状态序列（按 attempt 分组）
// ============================================================
// 行车记录仪（T1 证据重放）的折叠核。与 workflow-runs-reducer.ts 同居同规，理由同那边的
// 文件头：折叠必须单一实现——adapters 的服务端投影（校验器 + 装载 + 本折叠）与桌面 UI 的
// 回放区（既有 workflowRunEvents 查询 + 本折叠）若各写一份，迟早会在「哪条事件算状态跃迁」
// 这种地方分叉，两边的证据就对不上账了。
//
// 本模块是纯函数：无 Date.now、无随机、无 I/O、无时钟。事件由调用方按 sequence 升序递进来
// （journal 的 `listEvents` 与 v4 workflowRunEvents 分页天然如此）；折叠不重排——回放的轴
// 就是「事件当时落下的顺序」，排序等于偷偷改写历史。
//
// 依赖方向：contracts → shared。所以事件在这里**结构化定义**（`{sequence, type, payload}`，
// 与 contracts 的 DynamicWorkflowRunEvent 同形同词），不 import contracts / 引擎类型；
// payload 的字段按 reducer 同一条纪律读——「能归约多少算多少」，残缺键即缺席，不抛错。
//
// 与 reducer 的分工：reducer 归约的是**现在**（活投影，有界表），本折叠还原的是**当时**
// （完整事件轨，逐实例的步进轨迹）。两条轨读同一份 journal，答案互补而不互相替代。

/** 一条回放入参事件。与线上 `workflowRunEvents` 查询的元素同形（payload 已在 CLI 侧有界）。 */
export interface WorkflowRunReplayEvent {
  sequence: number;
  type: string;
  payload: Record<string, unknown>;
}

/**
 * 一个实例在事件轨上的**一步**。只收生命周期与交付事实，`node-progress` 刻意不进——
 * 它一次 turn 一条，几千事件的长 run 会让轨迹变成进度流水，而「节点当时走到哪一步」
 * 由 reducer 的活投影回答，不是回放的职责。
 */
export interface WorkflowRunReplayStep {
  sequence: number;
  /** 引擎事件类型原词——回放是证据面，不翻译（与 logTail 同一姿态）。 */
  type: string;
  /** 展示状态词：由事件类型映射；少数旁路事件（report / artifact）用事件种类本身。 */
  state: string;
  /** 人读摘要：等待原因 / 修复轮次 / 结算错误等。缺席即这条事件没有可说的一句。 */
  detail?: string;
}

/** 一个节点实例（站点的一次执行）的完整轨迹。 */
export interface WorkflowRunReplayInstance {
  siteId: string;
  /**
   * 同站点的第几次执行（引擎 `InstanceRef.ordinal`，0 起）。任务卡所称 attempt 的 dwf
   * 对应物：引擎对同一站点的重试 / 循环再入都铸新 ordinal，事件轨上没有别的重试维度。
   * （`node-repairing` 上的 attempt 字段是 schema 修复轮次，只是步进里的一条 detail。）
   */
  attempt: number;
  /** 节点种类（ask / world-read / world-run / report / artifact）。只随出生事件携带。 */
  kind?: string;
  /** 出生戳（phaseName）：节点出生时声明的阶段，不是结算时的当前阶段。 */
  label?: string;
  /** ask 节点的作者指令开头——「它当时看到什么」的有界答案。 */
  summary?: string;
  /** 命中缓存（replay / amend-resume）结算。 */
  cached?: boolean;
  /** 结算结果。缺席 = 事件轨上还没走到结算（崩溃 / 仍在跑）。 */
  outcome?: "ok" | "failed" | "cancelled";
  /** 结算失败原因的一句话摘要。 */
  error?: string;
  firstSequence: number;
  lastSequence: number;
  steps: WorkflowRunReplayStep[];
}

/** 按 attempt 分好组的一份实例清单（attempt 升序，组内出场序）。 */
export interface WorkflowRunReplayAttemptGroup {
  attempt: number;
  instances: WorkflowRunReplayInstance[];
}

/** 一个 run 的回放时间线。 */
export interface WorkflowRunReplayTimeline {
  /** 全部实例，出场序（首次被事件提到 = 出生，升序）。 */
  instances: WorkflowRunReplayInstance[];
  /** {@link WorkflowRunReplayAttemptGroup}：attempt 切换器的直接数据源。 */
  attempts: WorkflowRunReplayAttemptGroup[];
  /** `phase-entered` 刻度，原序——控制流走过哪些阶段的证据。 */
  phases: Array<{ name: string; sequence: number }>;
  eventCount: number;
  /** 最后一条事件的 sequence；空事件 run 缺席。 */
  lastSequence?: number;
}

/** 摘要字符串的硬上限。服务端装载路径读的是**未**有界化的 journal 原文，折叠产出的
 * 每一个字符串都必须就地封顶，才能保证无界输入不流进无界输出；UI 路径的载荷虽已在线上
 * 有界化，同一道闸再过一次是无害的（更短者胜）。 */
const REPLAY_DETAIL_MAX_CHARS = 240;

/** 事件类型 → 展示状态词。引擎的节点相位词表（见 reducer 文件头的事实 1）。 */
const REPLAY_STATE_BY_EVENT_TYPE: Readonly<Record<string, string>> = {
  "node-queued": "queued",
  "node-dispatched": "dispatched",
  "node-executing": "executing",
  "node-waiting": "waiting",
  "node-repairing": "repairing",
  "node-nudged": "nudged",
};

/** 非相位跃迁但仍进轨迹的事件 → 状态词。 */
const REPLAY_STATE_BY_DELIVERY_EVENT_TYPE: Readonly<Record<string, string>> = {
  report: "report",
  "artifact-published": "artifact",
  "import-cache-closed": "import-closed",
};

function isPlainRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function nonEmptyString(value: unknown): string | undefined {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

/** 折叠产出的字符串只有这一条出料口——封顶纪律靠它守。 */
function clip(value: string, max = REPLAY_DETAIL_MAX_CHARS): string {
  return value.length <= max ? value : value.slice(0, max);
}

/** 结算错误的可读摘要：引擎的错误信封（`{name,message,…}`）或裸串，读不出就缺席。 */
function errorDetail(value: unknown): string | undefined {
  const message = isPlainRecord(value) ? nonEmptyString(value.message) : nonEmptyString(value);
  return message === undefined ? undefined : clip(message);
}

function waitDetail(payload: Record<string, unknown>): string | undefined {
  const cause = nonEmptyString(payload.cause) ?? "unknown";
  // 证据面不翻译：cause / reason 都是引擎或治理器的原词，这里只做拼装。
  const parts = [`wait ${cause}`];
  const reason = nonEmptyString(payload.reason);
  if (reason !== undefined) parts.push(reason);
  const retryAfterMs = payload.retryAfterMs;
  if (typeof retryAfterMs === "number" && Number.isFinite(retryAfterMs) && retryAfterMs > 0) {
    parts.push(`retry ${Math.round(retryAfterMs)}ms`);
  }
  return clip(parts.join(" · "));
}

/**
 * 折叠一个 run 的事件流。事件必须 sequence 升序（重复本模块的纪律：不排序、不查重——
 * journal 的 sequence 契约由写入侧保证，读侧重复验证等于不信任自己的取数面）。
 */
export function foldWorkflowRunReplay(
  events: readonly WorkflowRunReplayEvent[],
): WorkflowRunReplayTimeline {
  const instances = new Map<string, WorkflowRunReplayInstance>();
  const phases: WorkflowRunReplayTimeline["phases"] = [];
  let lastSequence: number | undefined;

  for (const event of events) {
    const { sequence, type, payload } = event;
    if (!Number.isFinite(sequence) || !isPlainRecord(payload)) continue;
    lastSequence = sequence;

    if (type === "phase-entered") {
      const name = nonEmptyString(payload.name);
      if (name !== undefined) phases.push({ name, sequence });
      continue;
    }

    // 节点实例事件：instance 三元组缺了就无从归属，整条跳过（reducer 同一姿态）。
    const ref = isPlainRecord(payload.instance)
      ? workflowReplayInstanceRef(payload.instance)
      : undefined;
    if (ref === undefined) continue;
    const instance = instanceOf(instances, ref.siteId, ref.ordinal, sequence);

    const settledState = settleStateOf(type, payload);
    if (settledState !== undefined) {
      // 缓存命中的 settle 是该实例的出生事件（没有 queued），带阶段戳——同一条先到先得纪律。
      instance.label ??= nonEmptyString(payload.phaseName);
      if (payload.cached === true) instance.cached = true;
      instance.outcome ??= settleOutcomeOf(payload.outcome);
      instance.error ??= errorDetail(payload.error);
      instance.steps.push({ sequence, type, state: settledState.state, ...(settledState.detail === undefined ? {} : { detail: settledState.detail }) });
      instance.lastSequence = sequence;
      continue;
    }

    const lifecycle = REPLAY_STATE_BY_EVENT_TYPE[type];
    if (lifecycle !== undefined) {
      // 出生事实（kind / 阶段戳 / 指令开头）只在 queued / dispatched 上携带，先到先得——
      // dispatched 逐字重复 queued 的那份，取哪条都一样，取先到的少一次写。
      if (type === "node-queued" || type === "node-dispatched") {
        instance.kind ??= nonEmptyString(payload.kind);
        instance.label ??= nonEmptyString(payload.phaseName);
        instance.summary ??= (() => {
          const head = nonEmptyString(payload.instructionsHead);
          return head === undefined ? undefined : clip(head, 160);
        })();
      }
      const detail =
        type === "node-waiting" ? waitDetail(payload) : type === "node-repairing" ? repairDetail(payload) : undefined;
      instance.steps.push({ sequence, type, state: lifecycle, ...(detail === undefined ? {} : { detail }) });
      instance.lastSequence = sequence;
      continue;
    }

    const delivery = REPLAY_STATE_BY_DELIVERY_EVENT_TYPE[type];
    if (delivery !== undefined) {
      const detail =
        delivery === "artifact" ? nonEmptyString(readArtifactId(payload)) : undefined;
      instance.steps.push({ sequence, type, state: delivery, ...(detail === undefined ? {} : { detail }) });
      instance.lastSequence = sequence;
    }
    // 其余（run-* / log / usage-* / concurrency-*）：run 级上下文，归详情页头部，不进轨迹。
  }

  // 出场序：Map 保持插入序（首提序），无需再排。
  const flat = [...instances.values()];
  const byAttempt = new Map<number, WorkflowRunReplayInstance[]>();
  for (const instance of flat) {
    const group = byAttempt.get(instance.attempt);
    if (group === undefined) byAttempt.set(instance.attempt, [instance]);
    else group.push(instance);
  }
  const attempts = [...byAttempt.entries()]
    .sort(([left], [right]) => left - right)
    .map(([attempt, grouped]) => ({ attempt, instances: grouped }));
  return {
    instances: flat,
    attempts,
    phases,
    eventCount: events.length,
    ...(lastSequence === undefined ? {} : { lastSequence }),
  };
}

/** node-settled 的 outcome 词 → 实例终局。认不出的词保持缺席（不猜）。 */
function settleOutcomeOf(value: unknown): WorkflowRunReplayInstance["outcome"] {
  if (value === "ok" || value === "failed" || value === "cancelled") return value;
  return undefined;
}

function instanceOf(
  instances: Map<string, WorkflowRunReplayInstance>,
  siteId: string,
  ordinal: number,
  sequence: number,
): WorkflowRunReplayInstance {
  const key = `${siteId}@${ordinal}`;
  const existing = instances.get(key);
  if (existing !== undefined) return existing;
  const created: WorkflowRunReplayInstance = {
    siteId,
    attempt: ordinal,
    firstSequence: sequence,
    lastSequence: sequence,
    steps: [],
  };
  instances.set(key, created);
  return created;
}

/** 与 reducer 的 workflowInstanceRef 同一条结构化读法：siteId 非空串 + ordinal 是数。 */
function workflowReplayInstanceRef(
  value: Record<string, unknown>,
): { siteId: string; ordinal: number } | undefined {
  const siteId = nonEmptyString(value.siteId);
  const ordinal = value.ordinal;
  if (siteId === undefined || typeof ordinal !== "number" || !Number.isInteger(ordinal)) {
    return undefined;
  }
  return { siteId, ordinal };
}

/** node-settled → 终态步进（outcome / cached / error 摘要）。 */
function settleStateOf(
  type: string,
  payload: Record<string, unknown>,
): { state: string; detail?: string } | undefined {
  if (type !== "node-settled") return undefined;
  const outcome = nonEmptyString(payload.outcome);
  const cached = payload.cached === true;
  const state = outcome === "ok" ? "completed" : outcome === "failed" ? "failed" : outcome === "cancelled" ? "cancelled" : outcome === undefined && cached ? "completed" : "settled";
  const error = errorDetail(payload.error);
  return {
    state,
    ...(error === undefined ? {} : { detail: error }),
  };
}

function repairDetail(payload: Record<string, unknown>): string | undefined {
  const attempt = payload.attempt;
  if (typeof attempt !== "number" || !Number.isFinite(attempt)) return undefined;
  return `repair #${attempt}`;
}

function readArtifactId(payload: Record<string, unknown>): unknown {
  const artifact = payload.artifact;
  return isPlainRecord(artifact) ? artifact.id : undefined;
}
