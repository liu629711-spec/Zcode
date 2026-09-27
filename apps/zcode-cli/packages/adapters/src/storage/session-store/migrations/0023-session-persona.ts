// 驻场智能体会话的 persona 快照列（G1：冷重启保身份）。
// nullable、无 CHECK：SQLite 无法原地放宽 CHECK，形状校验放在 codecs
// （persona-json.ts，坏 JSON/坏形状 → undefined，不让历史行读不回来）。
// 历史行（含修复批之前的 persona 会话）该列恒为 NULL，读侧按「无 persona」处理。
export const SESSION_PERSONA_MIGRATION_SQL = `
alter table session add column persona_json text;
`;
