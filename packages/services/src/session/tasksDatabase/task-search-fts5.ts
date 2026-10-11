// 全文搜索（migration 0005）：把 queryTaskList 的 LIKE 全表扫描升级成 FTS5 倒排索引。
// 中文没有空格分词，unicode61 会把整段连续汉字当一个 token，子串搜不到；
// 这里用业界标准做法：索引时把每段汉字展开成重叠二字词（bigram），
// 查询时把关键词同样展开成短语（phrase），相邻二字词同时命中 ⇔ 原子串存在。
// 纯拉丁查询不走 FTS（保留 LIKE 的词中缀命中，如 "error" 命中 "TypeError"）。
import type { DatabaseSync } from "node:sqlite";

export const TASK_SEARCH_FTS_MIGRATION_SQL = `
      CREATE VIRTUAL TABLE IF NOT EXISTS task_search_fts USING fts5(
        body,
        cjk,
        workspace_key UNINDEXED,
        task_id UNINDEXED,
        tokenize = 'unicode61 remove_diacritics 2'
      );
    `;

// 平假名/片假名、CJK 扩展A、统一表意文字、兼容表意文字、谚文：同类"无空格连写"文字一并按二字词处理。
const CJK_RUN_PATTERN = /[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\uac00-\ud7af]+/g;
const LATIN_WORD_PATTERN = /[A-Za-z0-9]+/g;

/** 索引侧：把文本里的连续汉字段展开成空格分隔的重叠二字词，供 unicode61 分词。 */
export function buildCjkBigramText(input: string): string {
  const runs = input.match(CJK_RUN_PATTERN);
  if (!runs) {
    return "";
  }
  const pieces: string[] = [];
  for (const run of runs) {
    if (run.length < 2) {
      pieces.push(run);
      continue;
    }
    for (let i = 0; i < run.length - 1; i++) {
      pieces.push(run.slice(i, i + 2));
    }
  }
  return pieces.join(" ");
}

/**
 * 查询侧：把用户关键词编译成 FTS5 MATCH 表达式。
 * 返回 null 表示本关键词不适合 FTS，调用方回落 LIKE：
 * - 含单个汉字的关键词（二字词无法覆盖单字）；
 * - 纯拉丁/数字（保住 LIKE 的词中缀命中）。
 */
export function buildTaskFtsMatch(search: string): string | null {
  const trimmed = search.trim();
  if (!trimmed) {
    return null;
  }
  const cjkRuns = trimmed.match(CJK_RUN_PATTERN) ?? [];
  if (!cjkRuns.some((run) => run.length >= 2)) {
    return null;
  }
  if (cjkRuns.some((run) => run.length === 1)) {
    return null;
  }

  const terms: string[] = [];
  for (const run of cjkRuns) {
    // 短语内空格 = 二字词相邻；相邻二字词序列 ⇔ 原子串。run 只含汉字，无需转义引号。
    const grams: string[] = [];
    for (let i = 0; i < run.length - 1; i++) {
      grams.push(run.slice(i, i + 2));
    }
    terms.push(`"${grams.join(" ")}"`);
  }
  for (const word of trimmed.match(LATIN_WORD_PATTERN) ?? []) {
    // 前缀短语：fix 命中 fixing；单词内的引号不可能出现，无需转义。
    terms.push(`"${word}"*`);
  }
  return terms.length > 0 ? terms.join(" AND ") : null;
}

type TaskSearchBackfillRow = {
  workspace_key: string;
  task_id: string;
  title: string;
  searchable_text: string;
};

/** 组合 FTS 行正文：标题在前（标题命中也走同一张索引）。 */
export function buildTaskSearchFtsRowText(title: string, searchableText: string): string {
  return `${title}\n${searchableText}`;
}

/** migration 0005 执行器：建虚拟表 + 一次性回填存量行（已 deleted=0 的任务）。 */
export function adoptTaskSearchFts(db: DatabaseSync): void {
  db.exec(TASK_SEARCH_FTS_MIGRATION_SQL);
  const rows = db
    .prepare(
      `SELECT workspace_key, task_id, title, searchable_text
       FROM tasks
       WHERE deleted = 0 AND (title <> '' OR searchable_text <> '')`,
    )
    .iterate() as IterableIterator<TaskSearchBackfillRow>;
  const insert = db.prepare(
    `INSERT INTO task_search_fts (body, cjk, workspace_key, task_id) VALUES (?, ?, ?, ?)`,
  );
  for (const row of rows) {
    const body = buildTaskSearchFtsRowText(row.title, row.searchable_text);
    insert.run(body, buildCjkBigramText(body), row.workspace_key, row.task_id);
  }
}
