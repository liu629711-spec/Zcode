// ============================================================
// dwf journal 的**回放投影**：runId → 校验 → 事件流折叠成节点状态序列（按 attempt 分组）
// ============================================================
// T1 行车记录仪的服务端读面。三段职责钉在这一处：
//   1. **三表齐全校验器**——dwf_run / dwf_node / dwf_event 任一缺表（migration 未应用、
//      库被外来工具动过）都在读之前大声报错，错误里点名缺哪张表；
//   2. **数据校验**——run 行不存在、payload_json 解不开，各自报出所在表与定位（runId /
//      sequence），绝不让一条坏行变成整份回放的静默空白；
//   3. **折叠**——复用 @zcode/shared 的 foldWorkflowRunReplay（与桌面回放区同一份实现，
//      单一实现纪律见那个文件的文件头）。
//
// 与 dwf-journal.ts 的关系：那是引擎端口（JournalStorePort）的适配器，本文件是宿主侧的
// 证据读面——端口上没有「回放」这个领域概念，刻意不往端口上加方法（引擎不必知道回放）。
// 输入是裸 DatabaseSync，调用方持有 session 库句柄即可，与 dwf-journal-introspection 的
// SQL 读面同一条规矩。

import type { DatabaseSync } from "node:sqlite";
import {
  foldWorkflowRunReplay,
  type WorkflowRunReplayEvent,
  type WorkflowRunReplayTimeline,
} from "@zcode/shared/zcode-protocol-v4";

/** 回放要读的三张表。缺任一张，journal 都答不出一份完整的证据。 */
const REPLAY_TABLES = ["dwf_run", "dwf_node", "dwf_event"] as const;

export type DwfReplayTable = (typeof REPLAY_TABLES)[number];

/** 回放装载失败。`table` 在场即「哪张表的数据不齐」，消息与属性说的是同一件事。 */
export class DwfRunReplayError extends Error {
  readonly table?: DwfReplayTable;

  constructor(message: string, options?: { table?: DwfReplayTable; cause?: unknown }) {
    super(message, options?.cause === undefined ? undefined : { cause: options.cause });
    this.name = "DwfRunReplayError";
    this.table = options?.table;
  }
}

/** 一条 run 的回放投影：run 行的元数据 + 折叠出的时间线。 */
export interface DwfRunReplay {
  run: {
    runId: string;
    name?: string;
    /** dwf_run.status 原词（物理编码：failed/cancelled 是逻辑 errored/stopped 的落库形态）。 */
    status: string;
    resumedFrom?: string;
    timeCreated: number;
    timeUpdated: number;
  };
  timeline: WorkflowRunReplayTimeline;
}

interface DwfRunReplayRow {
  id: string;
  name: string | null;
  status: string;
  resumed_from: string | null;
  time_created: number;
  time_updated: number;
}

interface DwfEventRow {
  sequence: number;
  type: string;
  payload_json: string | null;
}

/**
 * 装载一个 run 的回放投影。
 *
 * 空事件 run（run 行在、journal 里一条事件都没有——进程在 run-launched 之前就死了）是
 * **合法状态**，返回空时间线，不是错误：「数据不齐」指表缺失 / 行缺失 / 行解不开，
 * 不指「还没来得及写」。
 */
export function loadDwfRunReplay(db: DatabaseSync, runId: string): DwfRunReplay {
  assertReplayTables(db);
  const run = mustGetRunRow(db, runId);
  return { run, timeline: foldWorkflowRunReplay(loadReplayEvents(db, runId)) };
}

/** 校验器①：三表齐全。缺哪张点名哪张（一次点名全部缺失的，少跑一趟）。 */
function assertReplayTables(db: DatabaseSync): void {
  const placeholders = REPLAY_TABLES.map(() => "?").join(", ");
  const rows = db
    .prepare(`select name from sqlite_master where type = 'table' and name in (${placeholders})`)
    .all(...REPLAY_TABLES) as Array<{ name: string }>;
  const present = new Set(rows.map((row) => row.name));
  const missing = REPLAY_TABLES.filter((table) => !present.has(table));
  if (missing.length > 0) {
    throw new DwfRunReplayError(
      `工作流回放加载失败：缺表 ${missing.map((table) => `"${table}"`).join("、")}（dwf journal migration 未应用或库不完整）`,
      { table: missing[0] },
    );
  }
}

/** 校验器②：run 行必须存在——「这个 runId 没有任何记录」与「journal 空」是两句话。 */
function mustGetRunRow(db: DatabaseSync, runId: string): DwfRunReplay["run"] {
  const row = db
    .prepare(
      "select id, name, status, resumed_from, time_created, time_updated from dwf_run where id = ?",
    )
    .get(runId) as DwfRunReplayRow | undefined;
  if (row === undefined) {
    throw new DwfRunReplayError(`工作流回放加载失败：dwf_run 里没有这个 run：${runId}`, {
      table: "dwf_run",
    });
  }
  return {
    runId: row.id,
    ...(row.name === null ? {} : { name: row.name }),
    status: row.status,
    ...(row.resumed_from === null ? {} : { resumedFrom: row.resumed_from }),
    timeCreated: row.time_created,
    timeUpdated: row.time_updated,
  };
}

/**
 * 校验器③ + 取数：全量事件、sequence 升序（unique(run_id, sequence) 索引即查询形状，
 * 不扫整表）。payload_json 是唯一「可能被外来写入弄坏」的单元格——解不开就报 sequence，
 * 让人能直接定位那一行，而不是拿到一份静默缺页的回放。
 */
function loadReplayEvents(db: DatabaseSync, runId: string): WorkflowRunReplayEvent[] {
  const rows = db
    .prepare(
      "select sequence, type, payload_json from dwf_event where run_id = ? order by sequence asc",
    )
    .all(runId) as unknown as DwfEventRow[];
  return rows.map((row) => {
    let payload: unknown;
    try {
      payload = row.payload_json === null ? undefined : JSON.parse(row.payload_json);
    } catch (error) {
      throw new DwfRunReplayError(
        `工作流回放加载失败：dwf_event 的 payload_json 解不开（run ${runId} sequence ${row.sequence}）`,
        { table: "dwf_event", cause: error },
      );
    }
    if (payload === undefined || payload === null || typeof payload !== "object") {
      throw new DwfRunReplayError(
        `工作流回放加载失败：dwf_event 的 payload_json 不是 JSON 对象（run ${runId} sequence ${row.sequence}）`,
        { table: "dwf_event" },
      );
    }
    // journal 的 payload_json 落的是**整条**事件（appendEvent 里 JSON.stringify(event)），
    // 折叠入参的 type 用列值（权威），载荷里重复的那个判别式留着无害——折叠只读已知键。
    return { sequence: row.sequence, type: row.type, payload: payload as Record<string, unknown> };
  });
}
