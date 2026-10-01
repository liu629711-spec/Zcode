// 派单临时工位的记忆隔离标记（审计 2026-10-01 P1：隔离原来只在 create 期
// 一次性传参，重启后 resume 按全局开关恢复，员工又读到老板的项目记忆）。
// nullable：null = 未声明（普通会话，跟随全局开关）；1 = 该会话出生即隔离工位。
// 写侧只有派单端口一处（createSession upsert 首写为准）；读侧 bootstrap resume。
export const SESSION_MEMORY_ISOLATION_MIGRATION_SQL = `
alter table session add column memory_isolation integer;
`;
