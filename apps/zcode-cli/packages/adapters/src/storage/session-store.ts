export {
  createSqliteSessionStore,
  getDefaultSessionDbPath,
  openStartupSqliteSessionStore,
  SqliteSessionStore,
} from "./session-store/sqlite-session-store.js";
export { createDwfJournalStore } from "./session-store/repositories/dwf-journal.js";
// 回放投影（T1 行车记录仪）：三表齐全校验器 + runId → 按 attempt 分组的节点状态序列。
// 折叠核在 @zcode/shared（与桌面回放区共用一份），这里负责校验与装载。
export {
  DwfRunReplayError,
  loadDwfRunReplay,
} from "./session-store/repositories/dwf-journal-replay.js";
export type {
  DwfReplayTable,
  DwfRunReplay,
} from "./session-store/repositories/dwf-journal-replay.js";
// run 内省查询的类型住在 adapters 而不是 contracts：它们说的是 **journal 行**的词汇
// （dwf_run 的列 + 时间戳），不是跨边界的工具载荷；而 bootstrap 已经依赖 @zcode/adapters，
// 能力探测的窄接口因此可以直接复用这份签名，不必在宿主侧再抄一遍（抄一遍就会漂移）。
export type {
  DwfArtifactItem,
  DwfArtifactItemsQuery,
  DwfListRunsQuery,
  DwfNodeStatusCounts,
  DwfRunIntrospectionQueries,
  DwfRunLifeSpan,
} from "./session-store/repositories/dwf-journal.js";
export type {
  DwfRunDetailRow,
  DwfRunListItem,
  DwfRunSessionListItem,
  DwfRunSessionRow,
  DwfRunTimestamps,
  DwfWorldNodeRow,
} from "./session-store/repositories/dwf-journal-codecs.js";
export { SqliteSessionMigrationError } from "./session-store/errors.js";
export type {
  SqliteSessionMigrationErrorKind,
  SqliteSessionMigrationErrorOptions,
} from "./session-store/errors.js";
export type {
  SessionStoreDebugCounts,
  SqliteSessionStoreOptions,
} from "./session-store/options.js";

export type {
  AsyncSqliteMigrationOptions,
  SqliteMigrationProgress,
} from "./session-store/migration-runner.js";
