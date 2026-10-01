// ============================================================
// workflow actor 的工具面（AgentRuntime 工具配置）
// ============================================================
//
// 子代理的工具面必须落到 child runtime 的工具注册上，否则实盘 transcript
// 里裁判 actor 也拿到了整套交互工具。两类风险：
//   1. 悬挂：AskUserQuestion / EnterPlanMode 在 headless child 里没有人可问，turn 永远不结束；
//   2. 越权与递归：CreateWorkflow 让 actor 能再提交一条工作流，ReadSessionContext 越界读父会话。
// 本模块是那个缺失的映射，由 driver 侧的 runtime 工厂在造 AgentRuntime 时展开。
//
// persona 无工具档位：每个 actor 都拿完整工作工具集减去下面这份减法表；
// 「裁判不要改文件」由 ask 文本说清——普通子代理也是这么做的（Explore 保留 Bash，
// 只读靠提示）。

import {
  ASK_USER_QUESTION_TOOL_NAME,
  ENTER_PLAN_MODE_TOOL_NAME,
  EXIT_PLAN_MODE_TOOL_NAME,
  READ_SESSION_CONTEXT_TOOL_NAME,
  RESOLVE_WORKFLOW_QUESTION_TOOL_NAME,
} from "@zcode/contracts";

/**
 * 员工岗位限制的窄视图（AgentProfile.tools/disallowedTools 的结构切片，
 * 不引 core 类型保持本模块轻）。
 */
export interface WorkflowActorProfileToolLimits {
  tools?: readonly string[];
  disallowedTools?: readonly string[];
}

/** AgentRuntimeConfig 的工具面切片。 */
export interface WorkflowActorToolPolicy {
  toolDisallowlist: readonly string[];
  /** 员工岗位白名单（班底进图纸）：在场 = actor 只能用这份清单减安全底线。 */
  toolAllowlist?: readonly string[];
  /** true = 岗位规矩真的收窄了工具面（调用方据此留一条「为什么变窄」的日志）。 */
  narrowed: boolean;
}

/**
 * 从全集里减掉的工具：前三个会阻塞在一个不存在的人类上（headless child 无人应答，
 * turn 不结束）；CreateWorkflow 会让 actor 递归提交工作流；ReadSessionContext 越界读父会话。
 * 其余（Bash / Edit / Write / 搜索 / web）照常保留——actor 就是要干活的。
 */
const ACTOR_DISALLOWED_TOOLS: readonly string[] = [
  ASK_USER_QUESTION_TOOL_NAME,
  ENTER_PLAN_MODE_TOOL_NAME,
  EXIT_PLAN_MODE_TOOL_NAME,
  "CreateWorkflow",
  // 修订入口与 CreateWorkflow 同一种嵌套编排，同一个根因入列。
  "AmendWorkflow",
  READ_SESSION_CONTEXT_TOOL_NAME,
  // 子代理不许替主代理回答升级问题。
  // 与上面几条的根因不同：这不是悬挂也不是越权读，而是**身份**——升级的整个意义是把判断权
  // 交给创建这条工作流的那一方；让另一个 actor 顺手作答，等于把它悄悄退化成 actor 之间的
  // 互相说服。actor 提问用 `escalate`（恒注册），作答只属于主会话。
  RESOLVE_WORKFLOW_QUESTION_TOOL_NAME,
];

/**
 * workflow actor 的 AgentRuntime 工具配置。纯函数，供 driver 侧 runtime 工厂展开进
 * AgentRuntimeConfig：不收窄，只减掉会悬挂或越权的交互/元工具。
 *
 * 只覆盖内建工具；MCP / plugin 工具的过滤留待生产接线时处理（同一个工厂 seam）。
 */
/**
 * 班底进图纸（2026-10-01 拍板）：点名员工的**岗位禁令随行**——员工被限定"只读"
 * 是岗位属性，进图纸不失效。合成规则（与派单路径的 profileToPersona 同源）：
 * - 岗位白名单（profile.tools）在场 → actor 只能用「白名单 − 安全底线」：
 *   底线（不许问人/不许递归编排/不许越界读）压过任何岗位白名单；
 * - 岗位禁令（profile.disallowedTools）→ 并进减法表；
 * - 都缺席 → 行为与从前逐字节一致（narrowed=false）。
 * 只覆盖内建工具；MCP / plugin 工具的过滤留待生产接线时处理（同一个工厂 seam）。
 */
export function workflowActorToolPolicy(
  profileLimits?: WorkflowActorProfileToolLimits,
): WorkflowActorToolPolicy {
  const profileAllowlist = profileLimits?.tools;
  const profileDisallowed = profileLimits?.disallowedTools ?? [];
  const narrowed =
    (profileAllowlist !== undefined && profileAllowlist.length > 0) ||
    profileDisallowed.length > 0;
  const toolDisallowlist = [...new Set([...ACTOR_DISALLOWED_TOOLS, ...profileDisallowed])];
  // 岗位白名单在场：actor 只能用「白名单 − 安全底线」。白名单里混进底线工具时，
  // 底线赢——岗位规矩再宽也请不回"问人/递归编排/越界读"这三类。
  const toolAllowlist = profileAllowlist?.length
    ? profileAllowlist.filter((tool) => !ACTOR_DISALLOWED_TOOLS.includes(tool))
    : undefined;
  return {
    toolDisallowlist,
    ...(toolAllowlist === undefined ? {} : { toolAllowlist }),
    narrowed,
  };
}
