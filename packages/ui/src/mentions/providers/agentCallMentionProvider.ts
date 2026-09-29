/**
 * 点将 @ 候选（D24）：在任意会话里 @ 一个项目员工，这条消息的活就固定派给他。
 *
 * 候选来自侧栏下发的项目档案目录（那份快照就是"这个项目里的驻场员工"），同步过滤、
 * 零额外取数——再开一路 RPC 会让 @ 面板和侧栏各说各话（侧栏有的这里没有，用户就
 * 会以为员工丢了）。选中后插入的是 `[@名字](agent://名字)`，身份在 destination，
 * 由 core 的 agent-call 解析器把派遣钉在名单内。
 */
import { useMemo } from "react";
import { buildTaskWorkspaceKey } from "@/lib/taskQueryCache.js";
import { buildAgentCallMentionMarkdown } from "@/mentions/mentionMarkdown.js";
import { getMentionGroupLimitForQuery } from "@/mentions/mentionSearch.js";
import type { MentionCategoryResult, MentionItem } from "@/mentions/mentionTypes.js";
import {
  selectProjectAgentDirectoryForWorkspace,
  useProjectAgentDirectoryStore,
} from "@/store/projectAgentDirectoryStore.js";
import type { PersonaBadgeAgentEntry } from "@/WorkspaceSidebar/projectAgentsModel.js";

// 与 core 的 agent-call 判据同一条：名字不合法就派不动，干脆不进候选，
// 免得用户点了个"看起来存在但没人接单"的名字。
const AGENT_NAME_PATTERN = /^[A-Za-z0-9-]{3,50}$/;

function matchesQuery(agent: PersonaBadgeAgentEntry, loweredQuery: string): boolean {
  if (!loweredQuery) return true;
  return (
    agent.name.toLowerCase().includes(loweredQuery) ||
    agent.description.toLowerCase().includes(loweredQuery)
  );
}

/** 纯映射（测试直接吃它）：过滤非法名与不匹配 query 的员工，按档案顺序出候选。 */
export function mapProjectAgentsToCallMentionItems(
  agents: readonly PersonaBadgeAgentEntry[],
  query: string,
): MentionItem[] {
  const loweredQuery = query.trim().toLowerCase();
  return agents
    .filter((agent) => AGENT_NAME_PATTERN.test(agent.name.trim()) && matchesQuery(agent, loweredQuery))
    .map((agent) => {
      const name = agent.name.trim();
      return {
        id: `agent-call:${name}`,
        category: "subagents",
        label: name,
        description: agent.description,
        value: name,
        markdown: buildAgentCallMentionMarkdown(name, name),
        keywords: [name, agent.description],
        // 档案色带进面板（D24）：与侧栏工牌同色，用户在两处认同一个人。
        data: { ...(agent.color ? { agentColor: agent.color } : {}) },
      } satisfies MentionItem;
    });
}

export function useAgentCallMentionProvider(params: {
  workspacePath: string;
  workspaceIdentity?: string;
  query: string;
  enabled: boolean;
  emptyText: string;
  title: string;
  limit: number;
}): MentionCategoryResult {
  const { workspacePath, workspaceIdentity, query, enabled, emptyText, title, limit } = params;
  const directory = useProjectAgentDirectoryStore((state) =>
    selectProjectAgentDirectoryForWorkspace(
      state,
      buildTaskWorkspaceKey(workspacePath, workspaceIdentity),
    ),
  );
  const items = useMemo(() => {
    if (!enabled) return [];
    return mapProjectAgentsToCallMentionItems(directory, query).slice(
      0,
      getMentionGroupLimitForQuery(query, limit),
    );
  }, [directory, enabled, limit, query]);

  return { items, loading: false, error: null, emptyText, title };
}
