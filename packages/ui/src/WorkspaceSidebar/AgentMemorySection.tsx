/* eslint-disable max-lines -- 记忆区的取数/编辑/删除同属「看记事本」这一块状态面（D6 一个入口一个机制），
   拆文件会把同一套服务与状态链打散，先按 ProjectAgents 的先例整 file 托管。 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { AgentSummary } from "@zcode/shared";
import {
  TID_PROJECT_AGENT_MEMORY_EDITOR_SAVE,
  TID_PROJECT_AGENT_MEMORY_EMPTY,
  TID_PROJECT_AGENT_MEMORY_FILE,
  TID_PROJECT_AGENT_MEMORY_FILE_DELETE,
  TID_PROJECT_AGENT_MEMORY_SECTION,
  testId,
} from "@zcode/shared";
import { Alert, AlertDescription } from "@/components/ui/alert.js";
import { Button } from "@/components/ui/button.js";
import { Textarea } from "@/components/ui/textarea.js";
import { toast } from "@/components/ui/toast.js";
import { useConfirmDialog } from "@/hooks/useConfirmDialog.js";
import { useWorkspaceServicesResolution } from "@/hooks/useWorkspaceServices.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { formatMemoryUpdatedAt } from "@/settings/memoryUpdatedAt.js";

type MemorySectionState = "loading" | "ready" | "error";

interface MemoryFileRow {
  name: string;
  kind: "index" | "item";
  updatedAt: number;
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * 智能体编辑对话框里的「记忆」区（G3/D6）：直读 <ws>/.zcode/agent-memory(-local)/<key>/
 * 或 user 档案的用户数据记事本——数据源就是磁盘文件，没有影子存储。
 * 所见即所记：能看、能改、能删单条；改动（含智能体自己写的）下个会话才生效，
 * 因为会话开场只读一次 MEMORY.md（core context 初始化即缓存）。
 */
export function AgentMemorySection({
  agent,
  workspacePath,
  workspaceIdentity,
  workspaceRemoteSessionId,
}: {
  agent: AgentSummary;
  workspacePath: string;
  workspaceIdentity?: string;
  workspaceRemoteSessionId?: string;
}) {
  const { intl, locale } = useZCodeIntl();
  const confirmDialog = useConfirmDialog();
  const resolution = useWorkspaceServicesResolution(
    workspacePath,
    workspaceRemoteSessionId,
    workspaceIdentity,
  );
  const memoryService = resolution.services.memoryService;
  const requestIdRef = useRef(0);
  // 编辑器内容回填的请求号：切行后旧 read 不回填、旧失败不误报。
  const editorRequestRef = useRef(0);
  const [state, setState] = useState<MemorySectionState>("loading");
  const [error, setError] = useState<string | null>(null);
  const [rootDir, setRootDir] = useState<string | null>(null);
  const [files, setFiles] = useState<MemoryFileRow[]>([]);
  // 编辑器一次只开一条：内容按 fileName 惰性读取，写回即落盘（磁盘是唯一事实）。
  const [editingFile, setEditingFile] = useState<string | null>(null);
  const [editingContent, setEditingContent] = useState("");
  const [editingBusy, setEditingBusy] = useState(false);
  const [deletingFile, setDeletingFile] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const interval = window.setInterval(() => setNow(Date.now()), 60_000);
    return () => window.clearInterval(interval);
  }, []);

  const scope = agent.memory ?? "project";

  const refreshCatalog = useCallback(async () => {
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    setState((current) => (current === "ready" ? current : "loading"));
    try {
      const catalog = await memoryService.listAgentMemoryFiles({
        agentName: agent.name,
        scope,
        workspacePath,
      });
      if (requestIdRef.current !== requestId) {
        return;
      }
      setRootDir(catalog.rootDir);
      setFiles(catalog.files.map((file) => ({ name: file.name, kind: file.kind, updatedAt: file.updatedAt })));
      setError(null);
      setState("ready");
    } catch (caught) {
      if (requestIdRef.current !== requestId) {
        return;
      }
      setError(getErrorMessage(caught));
      setState("error");
    }
  }, [agent.name, memoryService, scope, workspacePath]);

  useEffect(() => {
    if (!resolution.rpcReady) {
      return;
    }
    void refreshCatalog();
  }, [refreshCatalog, resolution.rpcReady]);

  const openEditor = useCallback(
    async (fileName: string) => {
      if (editingBusy || deletingFile !== null) {
        return;
      }
      const requestId = editorRequestRef.current + 1;
      editorRequestRef.current = requestId;
      setEditingFile(fileName);
      setEditingContent("");
      try {
        const content = await memoryService.readAgentMemoryFile({
          agentName: agent.name,
          scope,
          workspacePath,
          fileName,
        });
        // 请求期间用户可能已切到另一条，内容不回填错行。
        if (editorRequestRef.current !== requestId) {
          return;
        }
        setEditingContent(content.content);
      } catch (caught) {
        if (editorRequestRef.current !== requestId) {
          return;
        }
        setEditingFile(null);
        toast(getErrorMessage(caught));
      }
    },
    [agent.name, deletingFile, editingBusy, memoryService, scope, workspacePath],
  );

  const saveEditor = useCallback(async () => {
    if (!editingFile || editingBusy) {
      return;
    }
    setEditingBusy(true);
    try {
      await memoryService.writeAgentMemoryFile({
        agentName: agent.name,
        scope,
        workspacePath,
        fileName: editingFile,
        content: editingContent,
      });
      setEditingFile(null);
      setEditingContent("");
      await refreshCatalog();
    } catch (caught) {
      toast(getErrorMessage(caught));
    } finally {
      setEditingBusy(false);
    }
  }, [agent.name, editingBusy, editingContent, editingFile, memoryService, refreshCatalog, scope, workspacePath]);

  const deleteMemoryFile = useCallback(
    async (fileName: string) => {
      const confirmed = await confirmDialog({
        title: intl.formatMessage({ id: "workspaceSidebar.agentMemory.deleteTitle" }),
        description: intl.formatMessage(
          { id: "workspaceSidebar.agentMemory.deleteDescription" },
          { name: fileName },
        ),
        confirmLabel: intl.formatMessage({ id: "common.delete" }),
        confirmVariant: "destructive",
      });
      if (!confirmed) {
        return;
      }
      setDeletingFile(fileName);
      try {
        await memoryService.deleteAgentMemoryFile({
          agentName: agent.name,
          scope,
          workspacePath,
          fileName,
        });
        if (editingFile === fileName) {
          setEditingFile(null);
          setEditingContent("");
        }
        await refreshCatalog();
      } catch (caught) {
        toast(getErrorMessage(caught));
      } finally {
        setDeletingFile(null);
      }
    },
    [agent.name, confirmDialog, editingFile, intl, memoryService, refreshCatalog, scope, workspacePath],
  );

  const memoryCount = useMemo(
    () => files.filter((file) => file.kind === "item").length,
    [files],
  );

  return (
    <section
      className="space-y-3 border-t pt-3"
      data-testid={TID_PROJECT_AGENT_MEMORY_SECTION}
    >
      <div className="flex min-w-0 flex-wrap items-center justify-between gap-2">
        <p className="text-ui-base font-medium text-foreground-subtle">
          {intl.formatMessage({ id: "workspaceSidebar.agentMemory.title" })}
        </p>
        {state === "ready" ? (
          <span className="text-ui-sm text-foreground-subtle">
            {intl.formatMessage(
              {
                id: `workspaceSidebar.agentMemory.fileCount.${memoryCount === 1 ? "one" : "other"}`,
              },
              { count: memoryCount },
            )}
          </span>
        ) : null}
      </div>
      <p className="text-ui-sm text-foreground-subtle">
        {intl.formatMessage({ id: "workspaceSidebar.agentMemory.hint" })}
      </p>
      {state === "error" ? (
        <Alert>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      ) : state === "loading" ? (
        <p className="px-0.5 py-1 text-ui-sm text-foreground-subtle">
          {intl.formatMessage({ id: "workspaceSidebar.agentMemory.loading" })}
        </p>
      ) : files.length === 0 ? (
        <div
          data-testid={TID_PROJECT_AGENT_MEMORY_EMPTY}
          className="rounded-xl border border-dashed border-border px-4 py-6 text-center text-ui-base text-foreground-subtle"
        >
          {intl.formatMessage({ id: "workspaceSidebar.agentMemory.empty" })}
        </div>
      ) : (
        <ul className="space-y-0.5">
          {files.map((file) => (
            <li key={file.name}>
              {editingFile === file.name ? (
                <div className="space-y-2 rounded-md border border-border px-2.5 py-2">
                  <Textarea
                    rows={8}
                    value={editingContent}
                    onChange={(event) => setEditingContent(event.target.value)}
                    aria-label={file.name}
                    disabled={editingBusy}
                  />
                  <div className="flex justify-end gap-2">
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      disabled={editingBusy}
                      onClick={() => {
                        setEditingFile(null);
                        setEditingContent("");
                      }}
                    >
                      {intl.formatMessage({ id: "common.cancel" })}
                    </Button>
                    <Button
                      type="button"
                      size="sm"
                      disabled={editingBusy}
                      data-testid={TID_PROJECT_AGENT_MEMORY_EDITOR_SAVE}
                      onClick={() => void saveEditor()}
                    >
                      {intl.formatMessage({ id: "common.save" })}
                    </Button>
                  </div>
                </div>
              ) : (
                <div
                  data-testid={testId(TID_PROJECT_AGENT_MEMORY_FILE, file.name)}
                  className="flex min-w-0 items-center gap-2 rounded-md px-2.5 py-1.5 hover:bg-hover"
                >
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-ui-base text-foreground">
                      {file.name}
                      {file.kind === "index" ? (
                        <span className="ml-1.5 text-ui-sm text-foreground-subtle">
                          {intl.formatMessage({ id: "workspaceSidebar.agentMemory.indexLabel" })}
                        </span>
                      ) : null}
                    </p>
                    <p className="truncate text-ui-sm text-foreground-subtle">
                      {formatMemoryUpdatedAt({
                        formatMessage: intl.formatMessage,
                        locale,
                        now,
                        updatedAt: file.updatedAt,
                      })}
                    </p>
                  </div>
                  <div className="flex shrink-0 items-center gap-1">
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      disabled={editingBusy || deletingFile !== null}
                      onClick={() => void openEditor(file.name)}
                    >
                      {intl.formatMessage({ id: "workspaceSidebar.agentMemory.editAction" })}
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="text-destructive hover:text-destructive"
                      disabled={editingBusy || deletingFile !== null}
                      data-testid={testId(TID_PROJECT_AGENT_MEMORY_FILE_DELETE, file.name)}
                      onClick={() => void deleteMemoryFile(file.name)}
                    >
                      {intl.formatMessage({ id: "common.delete" })}
                    </Button>
                  </div>
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
      {rootDir && state === "ready" ? (
        <p className="break-all text-ui-sm text-foreground-subtle">
          {intl.formatMessage({ id: "workspaceSidebar.agentMemory.location" }, { path: rootDir })}
        </p>
      ) : null}
    </section>
  );
}
