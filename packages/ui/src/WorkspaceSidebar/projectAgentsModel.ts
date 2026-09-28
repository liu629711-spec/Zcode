import type {
  AgentColor,
  AgentSummary,
  SubAgentConfig,
  ZCodeSessionPersona,
} from "@zcode/shared";

export interface ProjectAgentDraft {
  name: string;
  description: string;
  systemPrompt: string;
}

/** 任务列表行的驻场智能体徽章（D2）：名字给悬停大白话，颜色缺省 = 只画图标。 */
export interface PersonaChatBadge {
  name: string;
  color?: AgentColor;
}

/** 行级徽章的挂载键（UI 内联标记；持久化 producer 落地前由打开会话的入口登记）。 */
export interface PersonaChatBadgeCarrier {
  /** 徽章标记挂在 task meta 上的字段名与持久化方案（tasks-index meta_json）预留同名，producer 落地后零改消费端。 */
  agentPersona?: PersonaChatBadge;
}

export type ProjectAgentDraftError =
  | "nameLength"
  | "nameCharacters"
  | "descriptionRequired"
  | "promptRequired";

// 校验规则与 settings/SubagentsSection 的 agent 表单一致，报错文案复用其 locale 键。
export function validateProjectAgentDraft(draft: ProjectAgentDraft): ProjectAgentDraftError[] {
  const errors: ProjectAgentDraftError[] = [];
  const name = draft.name.trim();
  if (name.length < 3 || name.length > 50) {
    errors.push("nameLength");
  } else if (!/^[a-zA-Z0-9-]+$/u.test(name)) {
    errors.push("nameCharacters");
  }
  if (!draft.description.trim()) {
    errors.push("descriptionRequired");
  }
  if (!draft.systemPrompt.trim()) {
    errors.push("promptRequired");
  }
  return errors;
}

// 侧栏只展示驻场（workspace 作用域）智能体；顺序沿用 service 已按名称排好的列表。
// agent.path = `<workspacePath>/.zcode/agents/<name>.md` 全路径，一次服务解析可列任意本地工作区，
// 所以按路径前缀把 agent 归到具体工作区（斜杠/盘符大小写容错的简单前缀匹配即可，不引 path 库）。
export function selectProjectAgentsForWorkspace<T extends Pick<AgentSummary, "scope" | "path">>(
  agents: readonly T[],
  workspacePath: string,
): T[] {
  const normalize = (value: string): string =>
    value
      .replace(/\\/g, "/")
      .replace(/^[A-Za-z]:/, (drive) => drive.toLowerCase())
      .replace(/\/+$/, "");
  const prefix = `${normalize(workspacePath)}/.zcode/agents/`;
  return agents.filter(
    (agent) => agent.scope === "workspace" && normalize(agent.path).startsWith(prefix),
  );
}

export function toProjectAgentCreateConfig(draft: ProjectAgentDraft): SubAgentConfig {
  // memory 固定 project：驻场智能体只服务本项目（AgentMemoryScope 由 services 侧定义）。
  return {
    name: draft.name.trim(),
    description: draft.description.trim(),
    systemPrompt: draft.systemPrompt.trim(),
    memory: "project",
  };
}

/**
 * 任务行上的徽章标记读取（D2 内联标记）：徽章渲染与 G5 编辑/删除入口同源，避免第二处读法漂移。
 * 形参放宽为 object：ZCodeTaskMeta 不带该可选字段的声明，弱类型判定会拒绝直传（调用方都是 meta 实值）。
 */
export function getPersonaChatBadge(task: object): PersonaChatBadge | undefined {
  return (task as PersonaChatBadgeCarrier).agentPersona;
}

/**
 * 侧栏三框编辑提交的完整配置（G5）：只让用户改 名字/介绍/人设，其余档案字段
 * （模型/工具/颜色/记忆范围…）原样带回——updateAgent 是按 config 整文件重写
 * （serializeSubagentMarkdown），漏带字段会把档案里设置页配好的模型/工具抹掉。
 * 记忆范围缺省 project（与建档同约定）。
 */
export function toProjectAgentUpdateConfig(
  agent: AgentSummary,
  draft: ProjectAgentDraft,
): SubAgentConfig {
  return {
    name: draft.name.trim(),
    description: draft.description.trim(),
    systemPrompt: draft.systemPrompt.trim(),
    ...(agent.color ? { color: agent.color } : {}),
    ...(agent.modelSelection ? { modelSelection: agent.modelSelection } : {}),
    ...(agent.tools?.length ? { tools: [...agent.tools] } : {}),
    ...(agent.disallowedTools?.length ? { disallowedTools: [...agent.disallowedTools] } : {}),
    ...(agent.skills?.length ? { skills: [...agent.skills] } : {}),
    ...(agent.permissionMode ? { permissionMode: agent.permissionMode } : {}),
    memory: agent.memory ?? "project",
    ...(agent.maxTurns !== undefined ? { maxTurns: agent.maxTurns } : {}),
    ...(agent.background !== undefined ? { background: agent.background } : {}),
    ...(agent.injectAgentsMd !== undefined ? { injectAgentsMd: agent.injectAgentsMd } : {}),
    ...(agent.mcpServers?.length ? { mcpServers: agent.mcpServers } : {}),
  };
}

/**
 * 删除确认弹窗里的记事本路径（G5/D7：删档案≠删记忆，路径要说真话）。
 * 与 core persistent-memory 的目录推导同源：project → <ws>/.zcode/agent-memory/<key>/，
 * local → <ws>/.zcode/agent-memory-local/<key>/；key 对合法档案名（[a-zA-Z0-9-]）即原名。
 * user 记事本在用户数据目录，UI 不知道绝对路径 → 返回 null，由调用方换用不带路径的文案。
 */
export function buildAgentMemoryDirectoryHint(
  agent: Pick<AgentSummary, "name" | "memory">,
  workspacePath: string,
): string | null {
  const scope = agent.memory ?? "project";
  if (scope === "user") {
    return null;
  }
  const dir = scope === "local" ? "agent-memory-local" : "agent-memory";
  return `${workspacePath.replace(/\\/gu, "/")}/.zcode/${dir}/${agent.name}/`;
}

/**
 * createSession.persona 载荷：随会话创建一次性进入 runtime config。
 * 即 shared 的 ZCodeSessionPersona（单一来源，防两处形状漂移）：
 * G2 起除 name/systemPrompt/memoryScope 外还携带档案的模型/工具/颜色，
 * 让驻场会话与档案同一副面孔；模型/工具/颜色可选，缺席 = 跟随会话缺省；
 * memoryScope 必带（档案缺 memory 字段时缺省 project）——core 以 persona.memory
 * 在场为记忆注入/补记忆工具的门，载荷缺席会让面板承诺的记事本变成摆设。
 */
export type ProjectAgentPersona = ZCodeSessionPersona;

export function toProjectAgentPersona(agent: AgentSummary): ProjectAgentPersona {
  return {
    name: agent.name,
    systemPrompt: agent.systemPrompt,
    // 记忆范围缺省 project（与建档/三框更新同约定）：面板与删除提示都按
    // `agent.memory ?? "project"` 展示记事本路径，载荷缺省必须同一口径。
    memoryScope: agent.memory ?? "project",
    ...(agent.modelSelection ? { modelSelection: agent.modelSelection } : {}),
    // 空数组与缺席同义 = 继承全部工具（与子代理派遣的 allowedTools 语义一致）。
    ...(agent.tools?.length ? { tools: [...agent.tools] } : {}),
    ...(agent.disallowedTools?.length ? { disallowedTools: [...agent.disallowedTools] } : {}),
    ...(agent.color ? { color: agent.color } : {}),
  };
}

/**
 * 侧栏建档（三框简表）产出的 persona：记忆范围固定 project（与 toProjectAgentCreateConfig 同一约定），
 * 模型/工具/颜色缺省 = 跟随会话缺省。建档成功即按它直接开会话（孤儿档案闭环）。
 */
export function toProjectAgentPersonaFromDraft(draft: ProjectAgentDraft): ProjectAgentPersona {
  return {
    name: draft.name.trim(),
    systemPrompt: draft.systemPrompt.trim(),
    memoryScope: "project",
  };
}

/**
 * 工作区新建智能体入口的可达性闸（原 agentsSection 渲染闸，随入口搬迁，语义不变）：
 * 创建与开会话都走侧栏 bound 连接，只有与 bound 同通道的工作区可达（同为本地，或同一远程会话）。
 * 不可达的远程 tab 不给入口，防止经本机服务把目录建到错误的机器上。
 */
export function resolveWorkspaceProjectAgentReachability(params: {
  boundWorkspacePath: string;
  boundWorkspaceRemoteSessionId?: string;
  tabWorkspacePath: string;
  tabRemoteSessionId?: string;
}): boolean {
  const { boundWorkspacePath, boundWorkspaceRemoteSessionId, tabWorkspacePath, tabRemoteSessionId } =
    params;
  return boundWorkspaceRemoteSessionId
    ? tabWorkspacePath === boundWorkspacePath
    : !tabRemoteSessionId;
}

/**
 * 标题反推徽章（D2 冷重启补缺的轻量 producer）：persona 会话标题恒为
 * 「智能体名 · 首条输入」（core buildProjectAgentSessionTitle 落盘，且 persona 会话
 * 关闭标题二次生成，前缀跨重启稳定在 tasks-index 里），拿工作区现役档案按前缀
 * 对号即得徽章——零新增持久化。名字与「 · 」分隔符的边界让前缀陷阱（ui-pro vs
 * ui-pro-team）天然不误标。内存登记（badge store）仍优先，本函数只补重启后
 * 登记丢失的行；已知上限：任务被手动改名（标题不再带前缀）后反推失效，
 * 彻底解法仍是 tasks-index meta_json 的 agentPersona 落盘（第二落点）。
 */
export function applyDerivedPersonaChatBadges<
  T extends { taskId: string; title: string },
>(items: readonly T[], agents: readonly Pick<AgentSummary, "name" | "color">[]): T[] {
  if (agents.length === 0) {
    return [...items];
  }
  return items.map((item) => {
    // 已带徽章的行原样保引用：登记层优先，反推不做二次覆盖。
    if ((item as T & PersonaChatBadgeCarrier).agentPersona) {
      return item;
    }
    const badge = derivePersonaChatBadge(item, agents);
    return badge ? { ...item, agentPersona: badge } : item;
  });
}

/** 徽章的标题反推单行版（D2）：标题「名字 · 」前缀 × 现役档案名单对号，前缀陷阱不误标。 */
export function derivePersonaChatBadge<T extends { title: string }>(
  row: T,
  agents: readonly Pick<AgentSummary, "name" | "color">[],
): PersonaChatBadge | undefined {
  const matched = agents.find((agent) => row.title.startsWith(`${agent.name} · `));
  if (!matched) {
    return undefined;
  }
  return { name: matched.name, ...(matched.color ? { color: matched.color } : {}) };
}

/**
 * 单行徽章三路解析（D2，与侧栏多行合并同一优先级）：行内标记 → 持久登记 → 标题反推。
 * Header「···」菜单等只拿到裸 task meta 的消费方用它复现侧栏行上同样的对号结果，
 * 避免"侧栏有徽章、Header 没菜单"的第二处漂移。
 */
export function resolvePersonaChatBadgeForTask(
  task: { taskId: string; title: string } & PersonaChatBadgeCarrier,
  registeredByTaskId: ReadonlyMap<string, PersonaChatBadge> | undefined,
  agents: readonly Pick<AgentSummary, "name" | "color">[] | undefined,
): PersonaChatBadge | undefined {
  return (
    getPersonaChatBadge(task) ??
    registeredByTaskId?.get(task.taskId) ??
    derivePersonaChatBadge(task, agents ?? [])
  );
}

/**
 * 改名跟走的标题换牌（D2 会话侧）：persona 会话标题 =「员工名 · 首条输入」，
 * 员工名 [a-zA-Z0-9-] 不含「 · 」，首个分隔符即名字段边界。返回换牌后的完整
 * 标题；无分隔符（用户手改过的自定义标题）或名字段为空返回 null，调用方跳过
 * 改写只补徽章登记——用户的自定义标题不被我们动。
 */
export function buildRetitledPersonaTitle(title: string, newName: string): string | null {
  const separator = title.indexOf(" · ");
  if (separator <= 0) {
    return null;
  }
  return `${newName} · ${title.slice(separator + 3)}`;
}

/**
 * 改名跟走的最终标题（D2 治愈版）：有「 · 」前缀照换；无分隔符（用户手动删过前缀，
 * 或历史行只剩余下正文）自动补回「新名 · 」前缀——否则这类行卡在旧标题，
 * 无论后续怎么改名都不再更新（真机验收抓到的卡死行）。空标题返回 null 由调用方跳过。
 */
export function buildRenamedPersonaTitle(title: string, newName: string): string | null {
  const replaced = buildRetitledPersonaTitle(title, newName);
  if (replaced) {
    return replaced;
  }
  return title.trim().length > 0 ? `${newName} · ${title}` : null;
}

/**
 * 标题显示去前缀（D4 展示层）：工牌已承担身份表达，行/头部标题里的「员工名 · 」
 * 不再重复——但只影响显示，存储标题原样保留：冷重启的标题反推徽章
 * （applyDerivedPersonaChatBadges）依赖这个前缀，剥存储会引发"工牌丢了"。
 * 只剥形似员工名的段（合法字符集 [a-zA-Z0-9-]{3,50}，与工牌名一致或陈旧旧名皆可）；
 * 用户自起的「装修 · 二期」这类中文自由标题不动。
 */
export function stripPersonaTitlePrefix(title: string, agentName: string): string {
  const separator = title.indexOf(" · ");
  if (separator <= 0) {
    return title;
  }
  const head = title.slice(0, separator);
  if (head !== agentName && !/^[a-zA-Z0-9-]{3,50}$/u.test(head)) {
    return title;
  }
  return title.slice(separator + 3);
}

/**
 * 重命名弹窗的编辑初值（D4 配套）：弹窗既然只让人改正文，初值就该与行上显示的一致
 * （去掉「员工名 · 」）。判据与显示层同一处（stripPersonaTitlePrefix），避免行里去了
 * 前缀、弹窗还挂着前缀的第二次漂移。无徽章 = 普通会话，原样编辑。
 */
export function personaChatRenameDraft(
  title: string,
  badge: PersonaChatBadge | undefined,
): string {
  return badge ? stripPersonaTitlePrefix(title, badge.name) : title;
}

/**
 * 重命名弹窗的回写（D4 配套）：弹窗里用户只看到正文，存回时必须把「员工名 · 」拼回
 * 存储标题——冷重启的工牌反推（applyDerivedPersonaChatBadges）只认这份前缀，弹窗图省事
 * 存正文就会重演「工牌丢了」那次事故。名字段以**盘上原标题**为准（不取徽章名）：
 * 徽章可能因登记/档案名与旧标题不同名而指向新名，原标题的前缀才是这段历史真正的主人，
 * 改名跟走另有链路（buildRenamedPersonaTitle）负责换牌，这里不越权。
 * 正文里用户自己打的「 · 」不再当名字边界（写什么就是什么）；已带同前缀的正文不重复拼。
 */
export function restorePersonaTitlePrefix(previousTitle: string, draft: string): string {
  // 空正文不拼（否则落库成「员工名 · 」这种悬挂前缀，行上看着像标题丢了）。
  if (!draft.trim()) {
    return draft;
  }
  const separator = previousTitle.indexOf(" · ");
  if (separator <= 0) {
    return draft;
  }
  const head = previousTitle.slice(0, separator);
  // 与显示层同一判据：只有形似员工名（档案名字符集 [a-zA-Z0-9-]{3,50}）才算前缀，
  // 用户自起的「装修 · 二期」这类中文标题原样交回，不被我们当成身份牌改写。
  if (!/^[a-zA-Z0-9-]{3,50}$/u.test(head)) {
    return draft;
  }
  const prefix = `${head} · `;
  return draft.startsWith(prefix) ? draft : `${prefix}${draft}`;
}

/**
 * 员工最近一段已归号会话（D3 续接）：行集合按三路对号找名字命中的行，
 * 取 updatedAt 最新的一条——打开员工时跳回它，历史与记忆原样续上；
 * 没有才新开一段（首见）。行集合是侧栏已加载分页，够用且零额外取数。
 */
export function findLatestPersonaChatRow<
  T extends { taskId: string; title: string; updatedAt: number } & PersonaChatBadgeCarrier,
>(
  items: readonly T[],
  agentName: string,
  registeredByTaskId: ReadonlyMap<string, PersonaChatBadge> | undefined,
  agents: readonly Pick<AgentSummary, "name" | "color">[] | undefined,
): T | undefined {
  let latest: T | undefined;
  for (const item of items) {
    const badge = resolvePersonaChatBadgeForTask(item, registeredByTaskId, agents);
    if (badge?.name !== agentName) {
      continue;
    }
    if (!latest || item.updatedAt > latest.updatedAt) {
      latest = item;
    }
  }
  return latest;
}

/** 「按智能体」视图的分栏（D3）：员工组带栏头工牌；未归号会话进无徽章的兜底组。 */
export interface PersonaBadgeTaskGroup<T> {
  key: string;
  /** 员工组才有：栏头画彩色工牌用；普通会话组缺席。 */
  badge?: PersonaChatBadge;
  items: T[];
}

/**
 * 按智能体分组（D3）：员工各一栏（栏序=条目传入顺序，即最近活动的员工在前，
 * 与"更新时间排序"的列表语义一致），未归号会话收进最后"普通会话"组。
 * 传入行须已合并徽章（usePersonaBadgedTaskItems 的产物）。
 */
export function groupPersonaBadgedTaskItems<T extends PersonaChatBadgeCarrier>(
  items: readonly T[],
): PersonaBadgeTaskGroup<T>[] {
  const groups = new Map<string, PersonaBadgeTaskGroup<T>>();
  const plain: T[] = [];
  for (const item of items) {
    const badge = item.agentPersona;
    if (!badge) {
      plain.push(item);
      continue;
    }
    const key = `agent:${badge.name}`;
    const existing = groups.get(key);
    if (existing) {
      existing.items.push(item);
    } else {
      groups.set(key, { key, badge, items: [item] });
    }
  }
  const result = [...groups.values()];
  if (plain.length > 0) {
    result.push({ key: "plain", items: plain });
  }
  return result;
}

/**
 * 档案改名补链的行扫描（D2）：标题前缀=旧名+「 · 」的 persona 历史行 id 列表。
 * 调用方（侧栏改名成功钩子）按这些行逐个 register 新名徽章——标题反推对新名
 * 永远命中不了旧行，登记是它们唯一的持久工牌来源。
 */
export function findPersonaRowIdsByTitlePrefix<
  T extends { taskId: string; title: string },
>(items: readonly T[], agentName: string): string[] {
  const prefix = `${agentName} · `;
  return items.filter((item) => item.title.startsWith(prefix)).map((item) => item.taskId);
}

/**
 * 把徽章登记合并进任务行：命中的行挂 agentPersona 标记，未命中的行保持原引用——
 * 侧栏流式刷新按引用判等打穿 memo，这里不能给无关行换新引用。
 * 徽章登记为空时原样返回同一数组。
 */
export function applyPersonaChatBadges<T extends { taskId: string }>(
  items: readonly T[],
  badgeByTaskId: ReadonlyMap<string, PersonaChatBadge>,
): (T & PersonaChatBadgeCarrier)[] {
  if (badgeByTaskId.size === 0) {
    return [...items];
  }
  return items.map((item) => {
    const badge = badgeByTaskId.get(item.taskId);
    return badge ? { ...item, agentPersona: badge } : item;
  });
}
