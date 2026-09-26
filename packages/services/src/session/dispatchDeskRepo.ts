/* eslint-disable max-lines -- 与 automationRepo/offPeakTaskRepo 同理：派活台仓库集中维护
   tasks 表旁路列的 sqlite 状态机写入与调度认领，稳定后再按读写职责拆分。 */
import {
  isTasksStorageMigrated,
  isTasksStoragePrepared,
} from "#src/session/tasksDatabase/prepared.js";
/* 派活台仓库：tasks 表旁路列（dispatch_state/review_state/acceptance_criteria/deliverables）
   的状态机写入与 sweeper 认领。与 automationRepo / offPeakTaskRepo 同库 tasks-index.sqlite、
   同 Repo 模式，但状态机独立，禁止往 automations 表或 ZCodeAutomation 类型上加字段。

   不变量：
   1. task_status 三值枚举（running/completed/error）是 session 生命周期，本仓库只读不写；
   2. approved 只能经 recordReviewDecision（人工通道）设置；代理可调用的
      submitForReview 在签名上没有决策参数，永远只能写 pending_review；
   3. 每 workspace 单飞：同一 workspace 同时至多一张票处于 claimed/dispatched。 */
import { mkdir } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname } from "node:path";
import { resolveWorkspaceKey } from "@zcode/shared";
import { getTasksIndexDatabasePath } from "#src/paths.js";
import { runTasksDatabaseMigrations } from "#src/session/tasksDatabase/migrations.js";

const require = createRequire(import.meta.url);
const { DatabaseSync } = require("node:sqlite") as typeof import("node:sqlite");
type DatabaseSyncInstance = InstanceType<typeof DatabaseSync>;

/** 评审态（可空：NULL=未进入评审流）。列上另有 CHECK 兜底。 */
export type TaskReviewState = "pending_review" | "approved" | "changes_requested";
/** 派发四态，语义模板 = shared/automation-types.ts 的 ZCodeAutomationDispatchStatus。 */
export type TaskDispatchState = "idle" | "claimed" | "dispatched" | "failed_to_dispatch";
/** tasks.task_status 的既有三值枚举；本仓库只读投影。 */
export type DeskTaskStatus = "running" | "completed" | "error";

/** 认领超时回收：claimed 超过该时长仍未结算，视为持有者已崩溃，允许重新认领。
    独立于 automation/off-peak 的同名常量（语义相同、常量独立一份，勿互相引用）。 */
export const DESK_CLAIM_STALE_MS = 10 * 60_000;

/** 派发失败退避常量；公式与 automationRepo.computeRetryAt 相同，常量独立一份。 */
export const DESK_DISPATCH_RETRY_BASE_MS = 30_000;
export const DESK_DISPATCH_RETRY_CAP_MS = 15 * 60_000;

/** 重试次数封顶：达限票停在 failed_to_dispatch 不再自动重派，人工接手由看板承接。
    语义对齐 automationRepo 的 DISPATCH_MAX_ATTEMPTS，常量独立一份勿互相引用。 */
export const DESK_DISPATCH_MAX_ATTEMPTS = 5;

/** 退避重试时间：now + min(BASE * 2^(attempts-1), CAP)。 */
export function computeDispatchRetryAt(now: number, attempts: number): number {
  const backoff = Math.min(
    DESK_DISPATCH_RETRY_BASE_MS * 2 ** Math.max(0, attempts - 1),
    DESK_DISPATCH_RETRY_CAP_MS,
  );
  return now + backoff;
}

/** 定位一张票：tasks 主键 (workspace_key, task_id)，key 由 workspacePath/Identity 解析。 */
export interface DeskTaskRef {
  workspacePath: string;
  workspaceIdentity?: string;
  taskId: string;
}

export interface DeskTicket {
  workspaceKey: string;
  workspacePath: string;
  taskId: string;
  title: string;
  /** session 生命周期投影，只读。 */
  taskStatus: DeskTaskStatus | null;
  dispatchState: TaskDispatchState;
  reviewState: TaskReviewState | null;
  acceptanceCriteria: string | null;
  /** json text 解析结果；写侧恒为合法 JSON，读侧解析失败时 undefined（防御手工改库）。 */
  deliverables?: unknown;
  claimedAt: number | null;
  dispatchAttempts: number;
  retryAt: number | null;
  lastDispatchError: string | null;
  createdAt: number;
  updatedAt: number;
}

interface DeskTicketRow {
  workspace_key: string;
  workspace_path: string;
  task_id: string;
  title: string;
  task_status: string | null;
  dispatch_state: string;
  review_state: string | null;
  acceptance_criteria: string | null;
  deliverables: string | null;
  claimed_at: number | null;
  dispatch_attempts: number;
  retry_at: number | null;
  last_dispatch_error: string | null;
  created_at: number;
  updated_at: number;
}

function rowToTicket(row: DeskTicketRow): DeskTicket {
  let deliverables: unknown;
  if (row.deliverables !== null) {
    try {
      deliverables = JSON.parse(row.deliverables);
    } catch {
      deliverables = undefined;
    }
  }
  return {
    workspaceKey: row.workspace_key,
    workspacePath: row.workspace_path,
    taskId: row.task_id,
    title: row.title,
    taskStatus:
      row.task_status === "running" || row.task_status === "completed" || row.task_status === "error"
        ? row.task_status
        : null,
    dispatchState: row.dispatch_state as TaskDispatchState,
    reviewState: (row.review_state ?? null) as TaskReviewState | null,
    acceptanceCriteria: row.acceptance_criteria,
    ...(deliverables !== undefined ? { deliverables } : {}),
    claimedAt: row.claimed_at,
    dispatchAttempts: row.dispatch_attempts,
    retryAt: row.retry_at,
    lastDispatchError: row.last_dispatch_error,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

function serializeDeliverables(deliverables: unknown): string | null {
  if (deliverables === undefined) return null;
  const json = JSON.stringify(deliverables);
  if (json === undefined) {
    throw new Error("deliverables 必须是可 JSON 序列化的值");
  }
  return json;
}

/** 派活台存储仓库（tasks-index.sqlite，WAL、多进程安全）。 */
export class DispatchDeskRepo {
  private db: DatabaseSyncInstance | null = null;
  private dbPath: string | null = null;
  private initializePromise: Promise<void> | null = null;
  // 同 OffPeakTaskRepo：db 路径构造期固定，测试注入临时库路径，生产回退默认。
  private readonly resolvedDbPath: string | null;

  constructor(
    dbPath?: string,
    private readonly startupBusyTimeoutMs = 5000,
  ) {
    this.resolvedDbPath = dbPath?.trim() || null;
  }

  private resolveDbPath(): string {
    return this.resolvedDbPath ?? getTasksIndexDatabasePath();
  }

  async ensureReady(): Promise<void> {
    const path = this.resolveDbPath();
    if (this.dbPath && this.dbPath !== path) {
      this.close();
    }
    if (!this.initializePromise) {
      this.initializePromise = this.initialize(path).catch((error) => {
        this.close();
        throw error;
      });
    }
    await this.initializePromise;
  }

  close(options?: { throwOnError?: boolean }): void {
    let closeError: unknown;
    try {
      this.db?.close();
    } catch (error) {
      closeError = error;
      // ignore
    }
    this.db = null;
    this.dbPath = null;
    this.initializePromise = null;
    if (options?.throwOnError && closeError) throw closeError;
  }

  private async initialize(path: string): Promise<void> {
    await mkdir(dirname(path), { recursive: true });
    if (!this.db) {
      this.db = new DatabaseSync(path);
      this.dbPath = path;
      this.db.exec(`PRAGMA busy_timeout = ${this.startupBusyTimeoutMs}`);
      this.db.exec("PRAGMA journal_mode = WAL");
      this.db.exec("PRAGMA synchronous = NORMAL");
    }
    if (isTasksStoragePrepared(path, this.db)) return;
    if (!isTasksStorageMigrated(path, this.db)) runTasksDatabaseMigrations(this.db);
  }

  private getDatabase(): DatabaseSyncInstance {
    if (!this.db) {
      throw new Error("DispatchDeskRepo 未初始化：请先 await ensureReady()");
    }
    return this.db;
  }

  private getRow(workspaceKey: string, taskId: string): DeskTicketRow | null {
    const row = this.getDatabase()
      .prepare(
        `SELECT workspace_key, workspace_path, task_id, title, task_status, dispatch_state,
          review_state, acceptance_criteria, deliverables, claimed_at, dispatch_attempts,
          retry_at, last_dispatch_error, created_at, updated_at
        FROM tasks WHERE workspace_key = ? AND task_id = ?`,
      )
      .get(workspaceKey, taskId) as DeskTicketRow | undefined;
    return row ?? null;
  }

  async getTicket(ref: DeskTaskRef): Promise<DeskTicket | null> {
    await this.ensureReady();
    const row = this.getRow(resolveWorkspaceKey(ref), ref.taskId);
    return row ? rowToTicket(row) : null;
  }

  // ---- 票据登记（作者通道） ----

  /**
   * 写入验收标准（票据的派活台准入标记：acceptance_criteria 非空 = 进台）。可选首版交付物。
   * 不做派发状态守卫：台主在任意阶段都允许补改 spec；评审态不受影响。
   */
  async setTicketSpec(
    ref: DeskTaskRef,
    params: { acceptanceCriteria: string; deliverables?: unknown; now: number },
  ): Promise<DeskTicket | null> {
    await this.ensureReady();
    const criteria = params.acceptanceCriteria.trim();
    if (!criteria) {
      throw new Error("acceptance_criteria 不能为空：空 spec 会让票据永远可被认领却无法验收");
    }
    const result = this.getDatabase()
      .prepare(
        `UPDATE tasks
        SET acceptance_criteria = @criteria,
            deliverables = COALESCE(@deliverables, deliverables),
            updated_at = @now
        WHERE workspace_key = @workspace_key AND task_id = @task_id`,
      )
      .run({
        criteria,
        deliverables: serializeDeliverables(params.deliverables),
        now: params.now,
        workspace_key: resolveWorkspaceKey(ref),
        task_id: ref.taskId,
      });
    if (result.changes !== 1) return null;
    return rowToTicket(this.getRow(resolveWorkspaceKey(ref), ref.taskId)!);
  }

  // ---- 调度状态机（sweeper 通道） ----

  /**
   * single-flight 认领到期票据（模板 = automationRepo.claimDue）：原子写 dispatch_state
   * guarded UPDATE + changes 校验（SQLite 无 SKIP LOCKED，BEGIN IMMEDIATE 串行写者）。
   * 参数化每 tick 认领至多 limit 张 + 可选按 workspace 过滤；每 workspace 单飞由候选过滤、
   * tick 内已认领集合与认领事务共同保证。同时回收认领超时的僵尸项（claimed 超时回到 idle）。
   *
   * 认领条件：
   * - acceptance_criteria 非空（进台标记）；
   * - 评审闸机：review_state 为 NULL 或 changes_requested（pending_review/approved 不可再派）；
   * - done stays human：task_status=completed 且从未进入评审的票不自动重跑，
   *   仅 changes_requested（人工打回）可作为返工重新派发；
   * - dispatch_state=idle，或 failed_to_dispatch 且 retry_at 到期且未达 DESK_DISPATCH_MAX_ATTEMPTS
   *   封顶（退避绕不过；达限票停在该状态，人工接手由看板承接）。
   */
  async claimDueTickets(options: {
    now: number;
    /** 每 tick 认领上限。 */
    limit: number;
    workspacePath?: string;
    workspaceIdentity?: string;
  }): Promise<DeskTicket[]> {
    if (!Number.isInteger(options.limit) || options.limit < 1) {
      throw new Error(`limit 必须是正整数，收到 ${options.limit}`);
    }
    await this.ensureReady();
    const db = this.getDatabase();
    const workspaceKey = options.workspacePath
      ? resolveWorkspaceKey({
          workspacePath: options.workspacePath,
          workspaceIdentity: options.workspaceIdentity,
        })
      : null;
    db.exec("BEGIN IMMEDIATE");
    try {
      // 先回收僵尸认领（持有者崩溃，claimed 超时未结算）。
      db.prepare(
        `UPDATE tasks
        SET dispatch_state = 'idle', claimed_at = NULL, updated_at = @now
        WHERE dispatch_state = 'claimed' AND deleted = 0
          AND claimed_at IS NOT NULL AND claimed_at <= @stale`,
      ).run({ now: options.now, stale: options.now - DESK_CLAIM_STALE_MS });

      // dispatched 阶段的僵尸回收：持有会话已终态却从未交活（代理在提交前崩溃）。
      // - task_status=error：转 failed_to_dispatch 走基础退避重派（失败重试是派活台本职）；
      // - task_status=completed：只释放 workspace 单飞槽回 idle，票本身因 done-stays-human
      //   仍不可自动重派，等人工在 recordReviewDecision 强制打回返工；
      // - task_status IS NULL：Host 标记 dispatched 后、session 落 task_status 前崩溃，
      //   活未开始干不适用 done-stays-human，同 error 走退避重派——放回 idle 会让崩溃
      //   路径每 tick 立即重派成快循环，退避+次数封顶让它最终停在 failed_to_dispatch 等人接手。
      db.prepare(
        `UPDATE tasks
        SET dispatch_state = 'failed_to_dispatch',
            dispatch_attempts = dispatch_attempts + 1,
            retry_at = @now + @retry_base,
            last_dispatch_error = 'session ended without review submission',
            claimed_at = NULL,
            updated_at = @now
        WHERE dispatch_state = 'dispatched' AND review_state IS NULL
          AND task_status = 'error' AND deleted = 0`,
      ).run({ now: options.now, retry_base: DESK_DISPATCH_RETRY_BASE_MS });
      db.prepare(
        `UPDATE tasks
        SET dispatch_state = 'idle', claimed_at = NULL, updated_at = @now
        WHERE dispatch_state = 'dispatched' AND review_state IS NULL
          AND task_status = 'completed' AND deleted = 0`,
      ).run({ now: options.now });
      db.prepare(
        `UPDATE tasks
        SET dispatch_state = 'failed_to_dispatch',
            dispatch_attempts = dispatch_attempts + 1,
            retry_at = @now + @retry_base,
            last_dispatch_error = 'session ended before reporting task status',
            claimed_at = NULL,
            updated_at = @now
        WHERE dispatch_state = 'dispatched' AND review_state IS NULL
          AND task_status IS NULL AND deleted = 0`,
      ).run({ now: options.now, retry_base: DESK_DISPATCH_RETRY_BASE_MS });

      // 每 workspace 单飞在候选查询内过滤：workspace 里已有 claimed/dispatched 的票则整队跳过。
      // 候选结果是快照，本 tick 刚认领出的 claimed 不在其 NOT EXISTS 视野里，认领循环再以
      // 已认领集合跳过同 workspace 的后续行；认领 UPDATE 在同一写事务内复检状态值，
      // guard+changes 兜住任何并发路径。
      const dueRows = db
        .prepare(
          `SELECT workspace_key, workspace_path, task_id, title, task_status, dispatch_state,
            review_state, acceptance_criteria, deliverables, claimed_at, dispatch_attempts,
            retry_at, last_dispatch_error, created_at, updated_at
          FROM tasks
          WHERE deleted = 0
            AND acceptance_criteria IS NOT NULL
            AND (review_state IS NULL OR review_state = 'changes_requested')
            AND (task_status IS NULL OR task_status != 'completed'
                 OR review_state = 'changes_requested')
            AND (
              dispatch_state = 'idle'
              OR (dispatch_state = 'failed_to_dispatch'
                  AND (retry_at IS NULL OR retry_at <= @now)
                  AND dispatch_attempts < @max_attempts)
            )
            AND (@workspace_key IS NULL OR workspace_key = @workspace_key)
            AND NOT EXISTS (
              SELECT 1 FROM tasks busy
              WHERE busy.workspace_key = tasks.workspace_key
                AND busy.dispatch_state IN ('claimed', 'dispatched')
                AND busy.deleted = 0
                AND busy.rowid != tasks.rowid
            )
          ORDER BY created_at ASC, task_id ASC
          LIMIT @limit`,
        )
        .all({
          now: options.now,
          workspace_key: workspaceKey,
          limit: options.limit,
          max_attempts: DESK_DISPATCH_MAX_ATTEMPTS,
        }) as unknown as DeskTicketRow[];

      const claimed: DeskTicket[] = [];
      // 本 tick 已认领的 workspace：同一次调用内至多认领一张（快照 SELECT 看不见本次认领）。
      const claimedWorkspaces = new Set<string>();
      const claim = db.prepare(
        `UPDATE tasks
        SET dispatch_state = 'claimed', claimed_at = @now, updated_at = @now
        WHERE workspace_key = @workspace_key AND task_id = @task_id
          AND (
            dispatch_state = 'idle'
            OR (dispatch_state = 'failed_to_dispatch'
                AND (retry_at IS NULL OR retry_at <= @now))
          )`,
      );
      for (const row of dueRows) {
        if (claimedWorkspaces.has(row.workspace_key)) continue;
        const result = claim.run({
          now: options.now,
          workspace_key: row.workspace_key,
          task_id: row.task_id,
        });
        if (result.changes === 1) {
          claimedWorkspaces.add(row.workspace_key);
          claimed.push(
            rowToTicket({
              ...row,
              dispatch_state: "claimed",
              claimed_at: options.now,
              updated_at: options.now,
            }),
          );
        }
      }
      db.exec("COMMIT");
      return claimed;
    } catch (error) {
      db.exec("ROLLBACK");
      throw error;
    }
  }

  /**
   * 认领成功、会话已交由 Host 拉起：claimed→dispatched。claimedAt 来自认领结果，
   * 作为持有凭据复检（防止把他人/新一轮认领误标）。返回 false=凭据失效。
   */
  async markDispatched(
    ref: DeskTaskRef,
    params: { claimedAt: number; now: number },
  ): Promise<boolean> {
    await this.ensureReady();
    const result = this.getDatabase()
      .prepare(
        `UPDATE tasks
        SET dispatch_state = 'dispatched', updated_at = @now
        WHERE workspace_key = @workspace_key AND task_id = @task_id
          AND dispatch_state = 'claimed' AND claimed_at = @claimed_at`,
      )
      .run({
        now: params.now,
        workspace_key: resolveWorkspaceKey(ref),
        task_id: ref.taskId,
        claimed_at: params.claimedAt,
      });
    return result.changes === 1;
  }

  /**
   * 派发失败：claimed→failed_to_dispatch，累计尝试并按指数退避设定 retry_at。
   * 退避公式同 automationRepo.computeRetryAt；attempts 取累计后的次数。
   */
  async markDispatchFailed(
    ref: DeskTaskRef,
    params: { claimedAt: number; now: number; error: string },
  ): Promise<boolean> {
    await this.ensureReady();
    const db = this.getDatabase();
    const workspaceKey = resolveWorkspaceKey(ref);
    // 读-改-写包进写事务：指数退避只能在 JS 侧算（SQLite math 函数不可依赖），
    // BEGIN IMMEDIATE 保证 attempts 读取与自增取同一快照；BUSY 上抛契约同 claimDueTickets。
    db.exec("BEGIN IMMEDIATE");
    try {
      const row = db
        .prepare(
          `SELECT dispatch_attempts FROM tasks
          WHERE workspace_key = ? AND task_id = ? AND dispatch_state = 'claimed' AND claimed_at = ?`,
        )
        .get(workspaceKey, ref.taskId, params.claimedAt) as
        | { dispatch_attempts: number }
        | undefined;
      if (!row) {
        db.exec("COMMIT");
        return false;
      }
      const attempts = row.dispatch_attempts + 1;
      const result = db
        .prepare(
          `UPDATE tasks
          SET dispatch_state = 'failed_to_dispatch',
              dispatch_attempts = dispatch_attempts + 1,
              retry_at = @retry_at,
              last_dispatch_error = @error,
              claimed_at = NULL,
              updated_at = @now
          WHERE workspace_key = @workspace_key AND task_id = @task_id
            AND dispatch_state = 'claimed' AND claimed_at = @claimed_at`,
        )
        .run({
          retry_at: computeDispatchRetryAt(params.now, attempts),
          error: params.error,
          now: params.now,
          workspace_key: workspaceKey,
          task_id: ref.taskId,
          claimed_at: params.claimedAt,
        });
      db.exec("COMMIT");
      return result.changes === 1;
    } catch (error) {
      db.exec("ROLLBACK");
      throw error;
    }
  }

  // ---- 交活（代理通道） ----

  /**
   * 代理交活：dispatched→pending_review 并释放 workspace 单飞槽（dispatch_state 回 idle）。
   * 签名上没有决策参数——代理路径在类型层面无法写出 approved；
   * SQL 守卫再兜一层：仅 dispatched 且评审态为 NULL/changes_requested 的票可提交。
   * 返回 null = 票据不在可交活状态（未派发 / 已 approved / 已提交）。
   */
  async submitForReview(
    ref: DeskTaskRef,
    params: { now: number; deliverables?: unknown },
  ): Promise<DeskTicket | null> {
    await this.ensureReady();
    const workspaceKey = resolveWorkspaceKey(ref);
    const result = this.getDatabase()
      .prepare(
        `UPDATE tasks
        SET review_state = 'pending_review',
            deliverables = COALESCE(@deliverables, deliverables),
            dispatch_state = 'idle',
            claimed_at = NULL,
            last_dispatch_error = NULL,
            retry_at = NULL,
            updated_at = @now
        WHERE workspace_key = @workspace_key AND task_id = @task_id
          AND dispatch_state = 'dispatched'
          AND (review_state IS NULL OR review_state = 'changes_requested')`,
      )
      .run({
        now: params.now,
        deliverables: serializeDeliverables(params.deliverables),
        workspace_key: workspaceKey,
        task_id: ref.taskId,
      });
    if (result.changes !== 1) return null;
    return rowToTicket(this.getRow(workspaceKey, ref.taskId)!);
  }

  // ---- 评审（人工通道） ----

  /**
   * 人工评审裁决：唯一能写 approved 的入口。只能裁决 pending_review 的票，
   * approved 由此成为终态（守卫使已 approved 的行 changes=0）。
   * 补充人工通道：对"已完成但代理未交活就崩溃"的卡死票（review_state NULL 且
   * task_status completed），允许人工强制 changes_requested 打回返工——这是
   * 该状态唯一的解锁路径；approved 不允许从 NULL 直接写出（没交活的活不批）。
   */
  async recordReviewDecision(
    ref: DeskTaskRef,
    params: { decision: "approved" | "changes_requested"; now: number },
  ): Promise<DeskTicket | null> {
    await this.ensureReady();
    const workspaceKey = resolveWorkspaceKey(ref);
    const result = this.getDatabase()
      .prepare(
        `UPDATE tasks
        SET review_state = @decision, updated_at = @now
        WHERE workspace_key = @workspace_key AND task_id = @task_id
          AND (
            review_state = 'pending_review'
            OR (review_state IS NULL AND @decision = 'changes_requested'
                AND task_status = 'completed')
          )`,
      )
      .run({
        decision: params.decision,
        now: params.now,
        workspace_key: workspaceKey,
        task_id: ref.taskId,
      });
    if (result.changes !== 1) return null;
    return rowToTicket(this.getRow(workspaceKey, ref.taskId)!);
  }
}
