import { useCallback, useEffect, useRef, useState, type FormEvent } from "react";
import { Bot } from "lucide-react";
import {
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
import { useWorkspaceServicesResolution } from "@/hooks/useWorkspaceServices.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { createCommandEnvelope } from "@/v4/commandFactory.js";
import { acquireWorkspaceConnection } from "@/v4/workspaceConnectionRegistry.js";
import { launchWorkspaceId } from "@/settings/saved-workflows/useSavedWorkflowLauncher.js";
import {
  selectProjectAgents,
  toProjectAgentCreateConfig,
  toProjectAgentPersona,
  validateProjectAgentDraft,
  type ProjectAgentDraft,
  type ProjectAgentDraftError,
} from "./projectAgentsModel.js";

// 列表与创建共用一份服务解析：分组和创建弹窗分别挂载，状态由 WorkspaceSidebar 持有。
export function useWorkspaceProjectAgents(
  workspacePath: string,
  workspaceIdentity: string | undefined,
  workspaceRemoteSessionId: string | undefined,
  enabled: boolean,
) {
  const resolution = useWorkspaceServicesResolution(
    workspacePath,
    workspaceRemoteSessionId,
    workspaceIdentity,
  );
  const subagentsService = resolution.services.subagentsService;
  const [agents, setAgents] = useState<AgentSummary[]>([]);
  const [creating, setCreating] = useState(false);

  const reload = useCallback(async () => {
    if (!enabled || !resolution.rpcReady) {
      return;
    }
    try {
      const result = await subagentsService.list({
        workspacePath,
        workspaceIdentity,
        provider: ZCODE_AGENT_PROVIDER,
      });
      setAgents(selectProjectAgents(result.agents));
    } catch (error) {
      // 远程断连等场景下分组直接隐藏，不阻塞侧栏其余内容。
      setAgents([]);
      logger.warn("[projectAgents] 列表加载失败", error);
    }
  }, [enabled, resolution.rpcReady, subagentsService, workspaceIdentity, workspacePath]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const createAgent = useCallback(
    async (draft: ProjectAgentDraft): Promise<boolean> => {
      setCreating(true);
      try {
        await subagentsService.createAgent({
          config: toProjectAgentCreateConfig(draft),
          provider: ZCODE_AGENT_PROVIDER,
          scope: "workspace",
          workspacePath,
          workspaceIdentity,
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
    [reload, subagentsService, workspaceIdentity, workspacePath],
  );

  return { agents, creating, createAgent };
}

/**
 * 点开驻场智能体 = 以它的身份开一段对话：v4 createSession 携 persona（无 firstInput，
 * 空草稿不进侧栏），accepted 后交给调用方导航到新会话。失败 toast，不静默。
 */
export function useOpenProjectAgentChat(params: {
  workspacePath: string;
  workspaceIdentity?: string;
  workspaceRemoteSessionId?: string;
  onSessionCreated: (sessionId: string) => void;
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
    async (agent: AgentSummary) => {
      if (openingRef.current || !resolution.rpcReady) {
        return;
      }
      openingRef.current = true;
      setOpening(true);
      const lease = acquireWorkspaceConnection(
        {
          workspacePath,
          ...(workspaceIdentity ? { workspaceIdentity } : {}),
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
                workspacePath,
                workspaceIdentity,
                remoteSessionId: workspaceRemoteSessionId,
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
        onSessionCreated(ack.result.sessionId);
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
      workspaceIdentity,
      workspacePath,
      workspaceRemoteSessionId,
    ],
  );

  return { openAgentChat, opening };
}

export function WorkspaceProjectAgentsGroup({
  agents,
  onOpenAgent,
}: {
  agents: AgentSummary[];
  onOpenAgent: (agent: AgentSummary) => void;
}) {
  const { intl } = useZCodeIntl();
  if (agents.length === 0) {
    return null;
  }
  return (
    <div className="pb-4">
      <div className="flex items-center gap-1.5 px-2.5 pb-1 pt-2 text-ui-base font-medium text-foreground-subtlest">
        <Bot aria-hidden="true" className="size-3.5" />
        {intl.formatMessage({ id: "workspaceSidebar.projectAgents" })}
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
