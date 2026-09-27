import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { Bot, Plus } from "lucide-react";
import {
  TID_PROJECT_AGENT_CREATE,
  TID_PROJECT_AGENT_CREATE_DIALOG,
  TID_PROJECT_AGENT_ROW,
  ZCODE_AGENT_PROVIDER,
  testId,
  type AgentSummary,
} from "@zcode/shared";
import { Button } from "@/components/ui/button.js";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog.js";
import { Input } from "@/components/ui/input.js";
import { Textarea } from "@/components/ui/textarea.js";
import { toast } from "@/components/ui/toast.js";
import { logger } from "@/logger.js";
import { buildTaskWorkspaceKey } from "@/lib/taskQueryCache.js";
import { useWorkspaceServicesResolution } from "@/hooks/useWorkspaceServices.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { createCommandEnvelope } from "@/v4/commandFactory.js";
import { acquireWorkspaceConnection } from "@/v4/workspaceConnectionRegistry.js";
import { launchWorkspaceId } from "@/settings/saved-workflows/useSavedWorkflowLauncher.js";
import {
  selectProjectAgentsForWorkspace,
  toProjectAgentCreateConfig,
  toProjectAgentPersona,
  validateProjectAgentDraft,
  type ProjectAgentDraft,
  type ProjectAgentDraftError,
} from "./projectAgentsModel.js";

/** 驻场智能体操作的目标工作区坐标：创建和开会话都按它定向（照 useSavedWorkflowLauncher 先例）。 */
export interface ProjectAgentTarget {
  workspacePath: string;
  workspaceIdentity?: string;
}

/** 侧栏待取数的工作区条目：remoteSessionId 只用于区分本地/远程取数路径。 */
export interface ProjectAgentWorkspaceTab extends ProjectAgentTarget {
  workspaceRemoteSessionId?: string;
}

// 列表与创建共用一份服务解析（绑侧栏活动工作区）；分组按工作区各自挂载，状态由 WorkspaceSidebar 持有。
export function useWorkspaceProjectAgents(params: {
  tabs: readonly ProjectAgentWorkspaceTab[];
  boundWorkspacePath: string;
  boundWorkspaceIdentity?: string;
  boundWorkspaceRemoteSessionId?: string;
  enabled: boolean;
}) {
  const {
    tabs,
    boundWorkspacePath,
    boundWorkspaceIdentity,
    boundWorkspaceRemoteSessionId,
    enabled,
  } = params;
  const resolution = useWorkspaceServicesResolution(
    boundWorkspacePath,
    boundWorkspaceRemoteSessionId,
    boundWorkspaceIdentity,
  );
  const subagentsService = resolution.services.subagentsService;
  const [agentsByWorkspaceKey, setAgentsByWorkspaceKey] = useState<Map<string, AgentSummary[]>>(
    () => new Map(),
  );
  const [creating, setCreating] = useState(false);

  // 取数集合：bound 本地 → 一次解析并行列所有本地工作区；bound 远程 → 只列与 bound 同路径的 tab（现行为）。
  const fetchTargets = useMemo(() => {
    if (boundWorkspaceRemoteSessionId) {
      return tabs.filter((tab) => tab.workspacePath === boundWorkspacePath);
    }
    return tabs.filter((tab) => !tab.workspaceRemoteSessionId);
  }, [boundWorkspacePath, boundWorkspaceRemoteSessionId, tabs]);
  // effect 依赖用稳定签名（workspaceKey join），避免 tabs 数组每次渲染换引用导致反复拉列表。
  const fetchSignature = useMemo(
    () =>
      fetchTargets
        .map((tab) => buildTaskWorkspaceKey(tab.workspacePath, tab.workspaceIdentity))
        .join("\n"),
    [fetchTargets],
  );
  const fetchTargetsRef = useRef(fetchTargets);
  fetchTargetsRef.current = fetchTargets;

  const reload = useCallback(async () => {
    if (!enabled || !resolution.rpcReady) {
      return;
    }
    try {
      const entries = await Promise.all(
        fetchTargetsRef.current.map(async (tab) => {
          const result = await subagentsService.list({
            workspacePath: tab.workspacePath,
            workspaceIdentity: tab.workspaceIdentity,
            provider: ZCODE_AGENT_PROVIDER,
          });
          return [
            buildTaskWorkspaceKey(tab.workspacePath, tab.workspaceIdentity),
            selectProjectAgentsForWorkspace(result.agents, tab.workspacePath),
          ] as const;
        }),
      );
      setAgentsByWorkspaceKey(new Map(entries));
    } catch (error) {
      // 远程断连等场景下分组退化为仅剩创建入口，不阻塞侧栏其余内容。
      setAgentsByWorkspaceKey(new Map());
      logger.warn("[projectAgents] 列表加载失败", error);
    }
  }, [enabled, fetchSignature, resolution.rpcReady, subagentsService]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const createAgent = useCallback(
    async (target: ProjectAgentTarget, draft: ProjectAgentDraft): Promise<boolean> => {
      setCreating(true);
      try {
        await subagentsService.createAgent({
          config: toProjectAgentCreateConfig(draft),
          provider: ZCODE_AGENT_PROVIDER,
          scope: "workspace",
          workspacePath: target.workspacePath,
          workspaceIdentity: target.workspaceIdentity,
        });
        await reload();
        return true;
      } catch (error) {
        toast(error instanceof Error ? error.message : String(error));
        return false;
      } finally {
        setCreating(false);
      }
    },
    [reload, subagentsService],
  );

  return { agentsByWorkspaceKey, creating, createAgent };
}

/**
 * 点开驻场智能体 = 以它的身份开一段对话：v4 createSession 携 persona（无 firstInput，
 * 空草稿不进侧栏），accepted 后交给调用方导航到新会话。失败 toast，不静默。
 * 服务解析与 transport 绑定侧栏活动工作区三件套；会话目标按 openAgentChat 传入的
 * target 定向（照 useSavedWorkflowLauncher 先例：一条连接、按调用传目标工作区）。
 */
export function useOpenProjectAgentChat(params: {
  workspacePath: string;
  workspaceIdentity?: string;
  workspaceRemoteSessionId?: string;
  onSessionCreated: (sessionId: string, target: ProjectAgentTarget) => void;
}) {
  const { workspacePath, workspaceIdentity, workspaceRemoteSessionId, onSessionCreated } = params;
  const resolution = useWorkspaceServicesResolution(
    workspacePath,
    workspaceRemoteSessionId,
    workspaceIdentity,
  );
  const { intl } = useZCodeIntl();
  const [opening, setOpening] = useState(false);
  const openingRef = useRef(false);

  const openAgentChat = useCallback(
    async (agent: AgentSummary, target: ProjectAgentTarget) => {
      if (openingRef.current || !resolution.rpcReady) {
        return;
      }
      openingRef.current = true;
      setOpening(true);
      // 连接端点仍取 bound 三件套的 remoteSessionId；scope 用目标工作区。
      const lease = acquireWorkspaceConnection(
        {
          workspacePath: target.workspacePath,
          ...(target.workspaceIdentity ? { workspaceIdentity: target.workspaceIdentity } : {}),
          ...(workspaceRemoteSessionId ? { remoteSessionId: workspaceRemoteSessionId } : {}),
        },
        resolution.services.zcodeAgentService,
      );
      try {
        const ack = await lease.transport.sendCommand(
          createCommandEnvelope({
            type: "createSession",
            payload: {
              workspaceId: launchWorkspaceId({
                workspacePath: target.workspacePath,
                workspaceIdentity: target.workspaceIdentity,
              }),
              persona: toProjectAgentPersona(agent),
            },
            sessionId: null,
          }),
        );
        if (ack.status !== "accepted" || ack.result?.type !== "createSession") {
          toast(
            `${intl.formatMessage({ id: "workspaceSidebar.projectAgentOpenFailed" })}${
              ack.reasonCode ? ` (${ack.reasonCode})` : ""
            }`,
          );
          return;
        }
        onSessionCreated(ack.result.sessionId, target);
      } catch (error) {
        logger.warn("[projectAgents] 打开智能体会话失败", error);
        toast(intl.formatMessage({ id: "workspaceSidebar.projectAgentOpenFailed" }));
      } finally {
        lease.release();
        openingRef.current = false;
        setOpening(false);
      }
    },
    [
      intl,
      onSessionCreated,
      resolution.rpcReady,
      resolution.services.zcodeAgentService,
      workspaceRemoteSessionId,
    ],
  );

  return { openAgentChat, opening };
}

// 工作区级分组：每个项目块的任务列表之下各自渲染，头行常驻（空列表时它就是该工作区的创建入口）。
export function WorkspaceProjectAgentsGroup({
  agents,
  onCreateClick,
  onOpenAgent,
}: {
  agents: AgentSummary[];
  onCreateClick: () => void;
  onOpenAgent: (agent: AgentSummary) => void;
}) {
  const { intl } = useZCodeIntl();
  return (
    <div className="pb-4">
      <div className="flex items-center gap-1.5 px-2.5 pb-1 pt-2 text-ui-base font-medium text-foreground-subtlest">
        <Bot aria-hidden="true" className="size-3.5" />
        <span className="min-w-0 flex-1">
          {intl.formatMessage({ id: "workspaceSidebar.projectAgents" })}
        </span>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          className="shrink-0 text-foreground-subtlest hover:text-foreground"
          data-testid={TID_PROJECT_AGENT_CREATE}
          aria-label={intl.formatMessage({ id: "workspaceSidebar.createProjectAgent" })}
          onClick={onCreateClick}
        >
          <Plus className="size-3.5" />
        </Button>
      </div>
      <ul className="space-y-0.5">
        {agents.map((agent) => (
          <li key={agent.id}>
            <button
              type="button"
              data-testid={testId(TID_PROJECT_AGENT_ROW, agent.name)}
              className="flex w-full flex-col gap-0.5 rounded-md px-2.5 py-1 text-left outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-ring/30"
              onClick={() => onOpenAgent(agent)}
            >
              <span className="truncate text-ui-base text-foreground">{agent.name}</span>
              {agent.description ? (
                <span className="truncate text-ui-sm text-foreground-subtle">
                  {agent.description}
                </span>
              ) : null}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

function formatDraftError(intl: ReturnType<typeof useZCodeIntl>["intl"], error: ProjectAgentDraftError): string {
  switch (error) {
    case "nameLength":
      return intl.formatMessage(
        { id: "settings.subagents.form.validation.nameLength" },
        { min: "3", max: "50" },
      );
    case "nameCharacters":
      return intl.formatMessage({ id: "settings.subagents.form.validation.nameCharacters" });
    case "descriptionRequired":
      return intl.formatMessage({ id: "settings.subagents.form.validation.descriptionRequired" });
    case "promptRequired":
      return intl.formatMessage({ id: "settings.subagents.form.validation.promptRequired" });
  }
}

export function WorkspaceProjectAgentCreateDialog({
  open,
  onOpenChange,
  creating,
  onCreate,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  creating: boolean;
  onCreate: (draft: ProjectAgentDraft) => Promise<boolean>;
}) {
  const { intl } = useZCodeIntl();
  const [draft, setDraft] = useState<ProjectAgentDraft>({
    name: "",
    description: "",
    systemPrompt: "",
  });
  const [errors, setErrors] = useState<ProjectAgentDraftError[]>([]);

  useEffect(() => {
    if (open) {
      setDraft({ name: "", description: "", systemPrompt: "" });
      setErrors([]);
    }
  }, [open]);

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    const nextErrors = validateProjectAgentDraft(draft);
    if (nextErrors.length > 0) {
      setErrors(nextErrors);
      return;
    }
    if (await onCreate(draft)) {
      onOpenChange(false);
    }
  };

  const fieldError = (error: ProjectAgentDraftError) =>
    errors.includes(error) ? (
      <p className="text-ui-base text-destructive">{formatDraftError(intl, error)}</p>
    ) : null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[min(520px,calc(100vw-2rem))] max-w-none" data-testid={TID_PROJECT_AGENT_CREATE_DIALOG}>
        <DialogTitle className="text-ui-lg font-medium text-foreground">
          {intl.formatMessage({ id: "workspaceSidebar.createProjectAgent" })}
        </DialogTitle>
        <form className="space-y-3" onSubmit={handleSubmit}>
          <div className="space-y-1.5">
            <label className="block text-ui-base font-medium text-foreground-subtle">
              {intl.formatMessage({ id: "settings.subagents.form.name.label" })}
            </label>
            <Input
              type="text"
              value={draft.name}
              onChange={(event) => setDraft({ ...draft, name: event.target.value })}
              placeholder={intl.formatMessage({ id: "settings.subagents.form.name.placeholder" })}
            />
            {fieldError("nameLength")}
            {fieldError("nameCharacters")}
          </div>
          <div className="space-y-1.5">
            <label className="block text-ui-base font-medium text-foreground-subtle">
              {intl.formatMessage({ id: "settings.subagents.form.description.label" })}
            </label>
            <Input
              type="text"
              value={draft.description}
              onChange={(event) => setDraft({ ...draft, description: event.target.value })}
              placeholder={intl.formatMessage({
                id: "settings.subagents.form.description.placeholder",
              })}
            />
            {fieldError("descriptionRequired")}
          </div>
          <div className="space-y-1.5">
            <label className="block text-ui-base font-medium text-foreground-subtle">
              {intl.formatMessage({ id: "settings.subagents.form.systemPrompt.label" })}
            </label>
            <Textarea
              rows={5}
              value={draft.systemPrompt}
              onChange={(event) => setDraft({ ...draft, systemPrompt: event.target.value })}
              placeholder={intl.formatMessage({
                id: "settings.subagents.form.systemPrompt.placeholder",
              })}
            />
            {fieldError("promptRequired")}
          </div>
          <p className="text-ui-sm text-foreground-subtle">
            {intl.formatMessage({ id: "workspaceSidebar.projectAgentScopeHint" })}
          </p>
          <div className="flex justify-end gap-2 pt-1">
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              {intl.formatMessage({ id: "common.cancel" })}
            </Button>
            <Button type="submit" disabled={creating}>
              {intl.formatMessage({ id: "common.save" })}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
