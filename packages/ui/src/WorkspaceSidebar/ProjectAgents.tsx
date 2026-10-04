/* eslint-disable max-lines -- 取数/开会话/建档/编辑/删除同属驻场智能体这一块状态面（G5 编辑复用建档表单后越限），
   拆文件会把同一套表单的服务与状态链打散，先按 TaskListItem 的先例整file托管。 */
import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import {
  TID_PROJECT_AGENT_CREATE_DIALOG,
  TID_PROJECT_AGENT_ROW,
  ZCODE_AGENT_PROVIDER,
  testId,
  type AgentSummary,
  type SubAgentConfig,
} from "@zcode/shared";
import {
  ModelConfigSelect,
  type ModelSelectFooterAction,
  type ModelSelectGroup,
} from "@/ModelConfigSelect.js";
import { Button } from "@/components/ui/button.js";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog.js";
import { Input } from "@/components/ui/input.js";
import { Textarea } from "@/components/ui/textarea.js";
import { toast } from "@/components/ui/toast.js";
import { onAgentRosterChanged } from "@/WorkspaceSidebar/agentRosterInvalidation.js";
import { logger } from "@/logger.js";
import { buildTaskWorkspaceKey } from "@/lib/taskQueryCache.js";
import { useModelSelectionServiceView } from "@/hooks/useModelSelectionView.js";
import { useWorkspaceServicesResolution, useBaseWorkspaceServices } from "@/hooks/useWorkspaceServices.js";
import { buildRegistryModelSelectGroups, resolveModelDisplayName } from "@/lib/modelSelectionGroups.js";
import { parseModelPickerValue } from "@/lib/zcodeSessionProjection.js";
import { encodeCustomModelValue } from "@/lib/zcodeCustomModelValue.js";
import { completeNewModelSelection } from "@zcode/provider";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { createCommandEnvelope } from "@/v4/commandFactory.js";
import { acquireWorkspaceConnection } from "@/v4/workspaceConnectionRegistry.js";
import { launchWorkspaceId } from "@/settings/saved-workflows/useSavedWorkflowLauncher.js";
import { AgentMemorySection } from "@/WorkspaceSidebar/AgentMemorySection.js";
import {
  selectWorkspaceRosterAgents,
  toProjectAgentCreateConfig,
  toProjectAgentUpdateConfig,
  validateProjectAgentDraft,
  type ProjectAgentDraft,
  type ProjectAgentDraftError,
  type ProjectAgentPersona,
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
// 取数不再挂"项目小节展开"闸门：建档选择器和任务行右键的编辑/删除入口都在小节之外，
// 小节折叠时列表若是空的，两处消费方都会把"还没取数"误判成"没有档案"。
export function useWorkspaceProjectAgents(params: {
  tabs: readonly ProjectAgentWorkspaceTab[];
  boundWorkspacePath: string;
  boundWorkspaceIdentity?: string;
  boundWorkspaceRemoteSessionId?: string;
}) {
  const { tabs, boundWorkspacePath, boundWorkspaceIdentity, boundWorkspaceRemoteSessionId } =
    params;
  const resolution = useWorkspaceServicesResolution(
    boundWorkspacePath,
    boundWorkspaceRemoteSessionId,
    boundWorkspaceIdentity,
  );
  const subagentsService = resolution.services.subagentsService;
  const [agentsByWorkspaceKey, setAgentsByWorkspaceKey] = useState<Map<string, AgentSummary[]>>(
    () => new Map(),
  );
  // 全局员工（user 级档案）机器上只有一份，每个本地 tab 的列表都会带回：
  // 「请进预置班底」的缺口按这份名册算（安装目标是用户级档案目录，不是项目目录）。
  const [userAgents, setUserAgents] = useState<AgentSummary[]>([]);
  const [saving, setSaving] = useState(false);
  // 列表首拉/重载进行中：对话框的选择器要区分"还没有智能体"和"正在取数"。
  const [loadingAgents, setLoadingAgents] = useState(false);

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
    if (!resolution.rpcReady) {
      return;
    }
    setLoadingAgents(true);
    try {
      const results = await Promise.all(
        fetchTargetsRef.current.map(async (tab) => {
          const result = await subagentsService.list({
            workspacePath: tab.workspacePath,
            workspaceIdentity: tab.workspaceIdentity,
            provider: ZCODE_AGENT_PROVIDER,
          });
          return {
            key: buildTaskWorkspaceKey(tab.workspacePath, tab.workspaceIdentity),
            workspacePath: tab.workspacePath,
            agents: result.agents,
          };
        }),
      );
      // user 档案全局一份，每个 tab 的列表都带全：按 id 去重只收一份。
      // 插件子代理档也是 scope:"user"（source:"plugin"），与名册同一不变量：
      // 不收——否则安装缺口统计/收编查重会把插件裸名别名当成在岗员工。
      const userAgentsById = new Map<string, AgentSummary>();
      for (const { agents } of results) {
        for (const agent of agents) {
          if (agent.scope === "user" && agent.source === "user") {
            userAgentsById.set(agent.id, agent);
          }
        }
      }
      setUserAgents([...userAgentsById.values()]);
      setAgentsByWorkspaceKey(
        new Map(
          results.map(({ key, workspacePath, agents }) => [
            key,
            // 名册合并（身份轴终局 §九②）：项目档 ∪ 全局员工（user 档），每个
            // 本地 tab 都见得到同一批员工；数据取自各 tab 自己的服务解析，
            // 远程 tab 吃远端进程的名册，本机全局员工不会混进远程工作区。
            selectWorkspaceRosterAgents(agents, workspacePath),
          ]),
        ),
      );
    } catch (error) {
      // 远程断连等场景下列表取数退化为空集，不阻塞侧栏其余内容。
      setAgentsByWorkspaceKey(new Map());
      setUserAgents([]);
      logger.warn("[projectAgents] 列表加载失败", error);
    } finally {
      setLoadingAgents(false);
    }
  }, [fetchSignature, resolution.rpcReady, subagentsService]);

  useEffect(() => {
    void reload();
  }, [reload]);
  // 跨界面失效（2026-10-02）：设置页增删改档案后广播一声，侧栏重拉名单——
  // 否则删掉的人还留在名册里，@ 面板吃的目录快照跟着说谎。
  useEffect(
    () =>
      onAgentRosterChanged(() => {
        void reload();
      }),
    [reload],
  );

  const createAgent = useCallback(
    async (
      target: ProjectAgentTarget,
      draft: ProjectAgentDraft,
    ): Promise<AgentSummary | null> => {
      setSaving(true);
      try {
        const { agent } = await subagentsService.createAgent({
          config: toProjectAgentCreateConfig(draft),
          provider: ZCODE_AGENT_PROVIDER,
          scope: "workspace",
          workspacePath: target.workspacePath,
          workspaceIdentity: target.workspaceIdentity,
        });
        await reload();
        // 回传服务刚写的档案本体（D26）：建档即发号，号在这里第一次进 UI，
        // 随后「自动开一段会话」的 persona 载荷才带得上号。
        return agent;
      } catch (error) {
        toast(error instanceof Error ? error.message : String(error));
        return null;
      } finally {
        setSaving(false);
      }
    },
    [reload, subagentsService],
  );

  // 编辑档案（G5/D4）：侧栏只露 名字/介绍/人设 三框，其余字段由 toProjectAgentUpdateConfig
  // 原样带回（updateAgent 整文件重写）；oldFilePath 让改名场景能删旧文件。
  // 全局员工（user 档）也在名册里可编辑：scope 按被编辑档案走——user 档写回
  // 用户级目录，绝不能按 workspace 落盘（那会在项目里复制出第二份同名档案）。
  const updateAgent = useCallback(
    async (
      target: ProjectAgentTarget,
      agent: AgentSummary,
      draft: ProjectAgentDraft,
    ): Promise<AgentSummary | null> => {
      setSaving(true);
      try {
        const isUserAgent = agent.scope === "user";
        const { agent: saved } = await subagentsService.updateAgent({
          agentId: agent.id,
          config: toProjectAgentUpdateConfig(agent, draft),
          oldFilePath: agent.path,
          provider: ZCODE_AGENT_PROVIDER,
          scope: isUserAgent ? "user" : "workspace",
          ...(isUserAgent
            ? {}
            : {
                workspacePath: target.workspacePath,
                workspaceIdentity: target.workspaceIdentity,
              }),
        });
        await reload();
        // 回传写盘后的档案本体：老档案这次才补上号（D26），调用方改名跟走要用它。
        return saved;
      } catch (error) {
        toast(error instanceof Error ? error.message : String(error));
        return null;
      } finally {
        setSaving(false);
      }
    },
    [reload, subagentsService],
  );

  // 删除档案（G5/D7）：服务端只删 profile 文件，记事本目录本来就不在删除范围里。
  const deleteAgent = useCallback(
    async (target: ProjectAgentTarget, agent: AgentSummary): Promise<boolean> => {
      try {
        await subagentsService.deleteAgent({
          agentId: agent.id,
          filePath: agent.path,
        });
        await reload();
        return true;
      } catch (error) {
        toast(error instanceof Error ? error.message : String(error));
        return false;
      }
    },
    [reload, subagentsService],
  );

  // 收编（身份轴终局 §九④）：项目档案升级为用户级全局员工。config 由
  // planProjectAgentPromotion 组装（工号随迁、memory=user、整档字段原样带回），
  // 这里只管两步落盘：先在用户级建档，成功后再删项目档。create 成功而 delete
  // 失败会两边各留一份——返回 cleanupFailed 让调用方把半成品状态说清楚
  // （重试时同名建档会被 services 拦下，需手动清旧档），绝不静默。
  // 服务报错原文交调用方拼进回执（这里不 toast，一条回执说全）。
  const promoteAgentToUser = useCallback(
    async (
      agent: AgentSummary,
      config: SubAgentConfig,
    ): Promise<{ status: "promoted" | "cleanupFailed" | "failed"; error?: string }> => {
      try {
        await subagentsService.createAgent({
          config,
          provider: ZCODE_AGENT_PROVIDER,
          scope: "user",
          // 收编=同一员工升舱（工号随迁），接管自己的记忆柜是本意：显式带确认，
          // 防静默接管的 createAgent 守卫不会拦自己人。
          confirmMemoryTakeover: true,
        });
      } catch (error) {
        return { status: "failed", error: error instanceof Error ? error.message : String(error) };
      }
      try {
        await subagentsService.deleteAgent({
          agentId: agent.id,
          filePath: agent.path,
        });
      } catch (error) {
        await reload();
        return {
          status: "cleanupFailed",
          error: error instanceof Error ? error.message : String(error),
        };
      }
      await reload();
      return { status: "promoted" };
    },
    [reload, subagentsService],
  );

  return {
    agentsByWorkspaceKey,
    userAgents,
    saving,
    loadingAgents,
    reload,
    createAgent,
    updateAgent,
    deleteAgent,
    promoteAgentToUser,
  };
}

/**
 * 点开驻场智能体 = 以它的身份开一段对话：v4 createSession 携 persona（无 firstInput，
 * 空草稿不进侧栏），accepted 后交给调用方导航到新会话。失败 toast，不静默。
 * 服务解析与 transport 绑定侧栏活动工作区三件套；会话目标按 openAgentChat 传入的
 * target 定向（照 useSavedWorkflowLauncher 先例：一条连接、按调用传目标工作区）。
 * persona 由调用方构造（既有档案走 toProjectAgentPersona，新建走 toProjectAgentPersonaFromDraft）。
 */
export function useOpenProjectAgentChat(params: {
  workspacePath: string;
  workspaceIdentity?: string;
  workspaceRemoteSessionId?: string;
  onSessionCreated: (
    sessionId: string,
    target: ProjectAgentTarget,
    persona: ProjectAgentPersona | undefined,
  ) => void;
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
    async (
      persona: ProjectAgentPersona | undefined,
      target: ProjectAgentTarget,
      // 换班（对齐稿 §2.2）：首条消息随 createSession 播种（原生 prompt-turn 同一条
      // 写路径），接班会话出生即在读交接单，不需要导航后再补发。
      opts?: { firstInputText?: string },
    ) => {
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
              ...(persona ? { persona } : {}),
              ...(opts?.firstInputText
                ? { firstInput: { text: opts.firstInputText } }
                : {}),
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
        onSessionCreated(ack.result.sessionId, target, persona);
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

/**
 * 工作区新建智能体对话框，兼做已有档案选择器（D3 收口后智能体从这里开聊）：
 * 上半是已有档案列表（点按直接开聊，空态/加载态各自成行），下半是建档表单（名字/介绍/人设三框，
 * 与设置页同一套校验文案）；建档成功由调用方负责自动开一段会话。
 * 编辑档案（G5/D4）复用同一套表单：传入 editingAgent 时隐藏选择器、三框预填，
 * 提交交给 onUpdate（不跳设置页）；G3 记忆区同屏挂在表单下方（数据源是磁盘记事本）。
 */
export function WorkspaceProjectAgentCreateDialog({
  open,
  onOpenChange,
  saving,
  loadingAgents,
  agents,
  editingAgent = null,
  onOpenAgent,
  onOpenNewChat,
  onRefresh,
  onCreate,
  onUpdate,
  workspacePath,
  workspaceIdentity,
  workspaceRemoteSessionId,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  saving: boolean;
  loadingAgents: boolean;
  agents: AgentSummary[];
  editingAgent?: AgentSummary | null;
  onOpenAgent: (agent: AgentSummary) => void;
  /** 「开新对话」（D3）：不续接历史、另起一段该员工的会话；缺席时不渲染入口。 */
  onOpenNewChat?: (agent: AgentSummary) => void;
  /** 打开对话框时刷新名册（设置页的增删不改这里的缓存，开门拿一次新账）。 */
  onRefresh?: () => void;
  onCreate: (draft: ProjectAgentDraft) => Promise<boolean>;
  onUpdate?: (agent: AgentSummary, draft: ProjectAgentDraft) => Promise<boolean>;
  /** 记忆区按目标工作区取数（编辑态才需要）；user 档案记事本在用户数据里，服务面自己解析。 */
  workspacePath?: string;
  workspaceIdentity?: string;
  workspaceRemoteSessionId?: string;
}) {
  const { intl } = useZCodeIntl();
  const [draft, setDraft] = useState<ProjectAgentDraft>({
    name: "",
    description: "",
    systemPrompt: "",
  });
  const [errors, setErrors] = useState<ProjectAgentDraftError[]>([]);
  // 模型选择（2026-09-29 用户需求：建档时指派用什么模型）。缺席 = 继承默认。
  const localHostServices = useBaseWorkspaceServices();
  const modelSelectionRead = useModelSelectionServiceView(localHostServices.modelSelectionService);
  const modelSelectionView =
    modelSelectionRead.state.status === "ready" ? modelSelectionRead.state.view : null;
  const modelGroups = useMemo((): ModelSelectGroup[] => {
    if (!modelSelectionView) return [];
    return buildRegistryModelSelectGroups(ZCODE_AGENT_PROVIDER, modelSelectionView, {
      startPlanBadgeLabel: intl.formatMessage({
        id: "settings.modelProvider.connectionMode.startPlanBadge",
      }),
      apiKeyLabel: intl.formatMessage({ id: "settings.modelProvider.apiKey" }),
      codingPlanLabel: intl.formatMessage({
        id: "settings.modelProvider.connectionMode.codingPlan",
      }),
    });
  }, [intl, modelSelectionView]);
  const isEditing = editingAgent !== null;
  // 打开即刷新名册（真机 2026-09-30）：设置页删了员工，这个对话框若还端着
  // 打开前的快照，就会出现「我都删掉了怎么他还在」。reload 是稳定 useCallback。
  useEffect(() => {
    if (open) {
      onRefresh?.();
    }
  }, [open, onRefresh]);
  // 模型选择器（2026-09-29 用户需求）：建档/编辑都指派"这个员工用什么模型"。
  // 清空 = 继承默认；reasoningLevel 取注册表默认档（与设置页同源），界面先不摆档位。
  const INHERIT_MODEL_VALUE = "inherit";
  const neverLocked = () => false;
  const modelValue = draft.modelSelection
    ? encodeCustomModelValue(draft.modelSelection.providerId, draft.modelSelection.modelId)
    : INHERIT_MODEL_VALUE;
  const modelValueAvailable = modelGroups.some((group) =>
    group.items.some((item) => item.value === modelValue),
  );
  const defaultModelLabel = intl.formatMessage({ id: "settings.subagents.model.defaultMain" });
  const modelTriggerLabel =
    modelValue === INHERIT_MODEL_VALUE
      ? defaultModelLabel
      : !modelValueAvailable
        ? intl.formatMessage({ id: "settings.subagents.model.select" })
        : (resolveModelDisplayName(modelGroups, modelValue) ?? modelValue);
  const modelFooterActions = useMemo<ModelSelectFooterAction[]>(
    () => [
      {
        key: "projectAgentModel:inherit",
        label: defaultModelLabel,
        onSelect: () => setDraft((prev) => ({ ...prev, modelSelection: undefined })),
        selected: draft.modelSelection === undefined,
      },
    ],
    [defaultModelLabel, draft.modelSelection],
  );
  const handleModelValueChange = (nextValue: string) => {
    if (nextValue === INHERIT_MODEL_VALUE) {
      setDraft((prev) => ({ ...prev, modelSelection: undefined }));
      return;
    }
    const parsed = parseModelPickerValue(nextValue);
    const withReasoning = modelSelectionView
      ? completeNewModelSelection(modelSelectionView, parsed)
      : undefined;
    setDraft((prev) => ({
      ...prev,
      modelSelection: {
        providerId: parsed.providerId,
        modelId: parsed.modelId,
        ...(withReasoning?.options?.reasoningLevel
          ? { options: { reasoningLevel: withReasoning.options.reasoningLevel } }
          : {}),
      },
    }));
  };

  useEffect(() => {
    if (open) {
      setDraft(
        editingAgent
          ? {
              name: editingAgent.name,
              description: editingAgent.description,
              systemPrompt: editingAgent.systemPrompt,
              ...(editingAgent.modelSelection
                ? { modelSelection: editingAgent.modelSelection }
                : {}),
            }
          : { name: "", description: "", systemPrompt: "" },
      );
      setErrors([]);
    }
  }, [open, editingAgent]);

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    const nextErrors = validateProjectAgentDraft(draft);
    if (nextErrors.length > 0) {
      setErrors(nextErrors);
      return;
    }
    const submitted = isEditing
      ? editingAgent && onUpdate
        ? await onUpdate(editingAgent, draft)
        : false
      : await onCreate(draft);
    if (submitted) {
      onOpenChange(false);
    }
  };

  const fieldError = (error: ProjectAgentDraftError) =>
    errors.includes(error) ? (
      <p className="text-ui-base text-destructive">{formatDraftError(intl, error)}</p>
    ) : null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="w-[min(520px,calc(100vw-2rem))] max-w-none max-h-[calc(100dvh-4rem)] overflow-x-hidden overflow-y-auto"
        data-testid={TID_PROJECT_AGENT_CREATE_DIALOG}
      >
        <DialogTitle className="text-ui-lg font-medium text-foreground">
          {intl.formatMessage({
            id: isEditing
              ? "workspaceSidebar.projectAgentEditTitle"
              : "workspaceSidebar.createProjectAgent",
          })}
        </DialogTitle>
        {isEditing ? null : (
          <div className="min-w-0 space-y-1.5">
            <p className="text-ui-base font-medium text-foreground-subtle">
              {intl.formatMessage({ id: "workspaceSidebar.projectAgents" })}
            </p>
            {loadingAgents ? (
              <p className="px-0.5 py-1 text-ui-sm text-foreground-subtle">
                {intl.formatMessage({ id: "workspaceSidebar.projectAgentsLoading" })}
              </p>
            ) : agents.length === 0 ? (
              <p className="px-0.5 py-1 text-ui-sm text-foreground-subtle">
                {intl.formatMessage({ id: "workspaceSidebar.projectAgentsEmpty" })}
              </p>
            ) : (
              <div className="space-y-0.5">
                <ul className="max-h-40 space-y-0.5 overflow-y-auto">
                  {agents.map((agent) => (
                    <li
                      key={agent.id}
                      className="flex items-center gap-1 rounded-md hover:bg-hover"
                    >
                      {/* 点员工 = 续接他最近一段对话（D3）；想开新话题走行尾「开新对话」。 */}
                      <button
                        type="button"
                        data-testid={testId(TID_PROJECT_AGENT_ROW, agent.name)}
                        className="flex min-w-0 flex-1 flex-col gap-0.5 rounded-md px-2.5 py-1 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring/30"
                        onClick={() => onOpenAgent(agent)}
                      >
                        <span className="flex min-w-0 items-center gap-1.5">
                          <span className="truncate text-ui-base text-foreground">
                            {agent.name}
                          </span>
                          {agent.source === "built-in" || agent.source === "plugin" ? (
                            <span className="shrink-0 rounded border border-border px-1 py-px text-[10px] leading-4 text-foreground-subtle">
                              {intl.formatMessage({
                                id:
                                  agent.source === "built-in"
                                    ? "workspaceSidebar.agentSource.builtIn"
                                    : "workspaceSidebar.agentSource.plugin",
                              })}
                            </span>
                          ) : null}
                        </span>
                        {agent.description ? (
                          <span className="truncate text-ui-sm text-foreground-subtle">
                            {agent.description}
                          </span>
                        ) : null}
                      </button>
                      {onOpenNewChat ? (
                        <button
                          type="button"
                          className="mr-1 shrink-0 rounded px-1.5 py-0.5 text-ui-sm text-foreground-subtle hover:bg-surface-hover hover:text-foreground"
                          onClick={() => onOpenNewChat(agent)}
                        >
                          {intl.formatMessage({ id: "workspaceSidebar.projectAgentNewChat" })}
                        </button>
                      ) : null}
                    </li>
                  ))}
                </ul>
                <p className="px-0.5 text-ui-sm text-foreground-subtle">
                  {intl.formatMessage({ id: "workspaceSidebar.projectAgentsResumeHint" })}
                </p>
              </div>
            )}
          </div>
        )}
        <form
          className={isEditing ? "min-w-0 space-y-3" : "min-w-0 space-y-3 border-t pt-3"}
          onSubmit={handleSubmit}
        >
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
              // 人设输入区定高：field-sizing-content 随内容无限长高，长人设会把
              // 对话框撑出一屏（真机 2026-09-30）；定高后超出部分框内自滚，
              // 对话框本体不滚动。
              className="h-32 [field-sizing:fixed] overflow-y-auto"
              value={draft.systemPrompt}
              onChange={(event) => setDraft({ ...draft, systemPrompt: event.target.value })}
              placeholder={intl.formatMessage({
                id: "settings.subagents.form.systemPrompt.placeholder",
              })}
            />
            {fieldError("promptRequired")}
          </div>
          <div className="space-y-1.5">
            <label className="block text-ui-base font-medium text-foreground-subtle">
              {intl.formatMessage({ id: "settings.subagents.form.model.label" })}
            </label>
            <ModelConfigSelect
              modelGroups={modelGroups}
              normalizedValue={modelValue}
              triggerLabel={modelTriggerLabel}
              showManageModelsAction={false}
              lockReasonMessage=""
              isItemLocked={neverLocked}
              onValueChange={handleModelValueChange}
              footerActions={modelFooterActions}
              manageModelsLabel={intl.formatMessage({
                id: "chat.toolbar.model.manageModels",
              })}
              contentSide="top"
              contentAlign="end"
              focusSelectorOnClose={null}
              labelVisibilityClassName="inline-flex min-w-0"
              triggerClassName="h-8 w-full max-w-none justify-between rounded-lg border border-input-border bg-input px-3 py-1.5 text-foreground hover:border-input-border-hover hover:bg-input focus-visible:border-input-border-focused focus-visible:bg-input-focused"
              triggerLabelClassName="inline-flex min-w-0 truncate text-left"
            />
          </div>
          {isEditing ? null : (
            <p className="text-ui-sm text-foreground-subtle">
              {intl.formatMessage({ id: "workspaceSidebar.projectAgentScopeHint" })}
            </p>
          )}
          {/* 保存/取消钉在滚动容器底部：表单再长也不用滚动找按钮（真机 2026-09-30）。
              钉底行与表单都必须 min-w-0：网格轨道被名单行的不可断行内容撑爆时，
              轨道横向溢出弹窗，右对齐的保存按钮会被 overflow-x-hidden 裁掉
              （真机 2026-10-02：弹窗 520 / 表单 617，保存按钮整个落在裁切盲区）。 */}
          <div className="sticky bottom-0 z-10 -mx-4 mt-2 flex justify-end gap-2 border-t border-border bg-popover px-4 pt-2">
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              {intl.formatMessage({ id: "common.cancel" })}
            </Button>
            <Button type="submit" disabled={saving}>
              {intl.formatMessage({ id: "common.save" })}
            </Button>
          </div>
        </form>
        {isEditing && editingAgent && workspacePath ? (
          <AgentMemorySection
            agent={editingAgent}
            workspacePath={workspacePath}
            {...(workspaceIdentity ? { workspaceIdentity } : {})}
            {...(workspaceRemoteSessionId ? { workspaceRemoteSessionId } : {})}
          />
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
