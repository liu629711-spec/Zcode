// 圆桌控场 hook（2026-10-03 刀3）：council/pause + council/interject。
// 一次一条串行发（busy 期间控件再禁用），失败原样上抛给调用方提示。

import { useCallback, useState } from "react";
import { useBaseWorkspaceServices } from "@/hooks/useWorkspaceServices.js";
import { useRemoteWorkspaceSessionStore } from "@/store/remoteWorkspaceSessionStore.js";
import { resolveWorkspaceServices } from "@/lib/workspaceServiceResolver.js";

export interface CouncilControls {
  busy: boolean;
  pause: (params: { sessionId: string; councilId: string; paused: boolean }) => Promise<void>;
  interject: (params: { sessionId: string; councilId: string; text: string }) => Promise<void>;
}

export function useCouncilControls(params: {
  workspacePath: string;
  workspaceIdentity?: string;
}): CouncilControls {
  const baseServices = useBaseWorkspaceServices();
  const sessionsById = useRemoteWorkspaceSessionStore((state) => state.sessionsById);
  const sessionIdByWorkspaceIdentity = useRemoteWorkspaceSessionStore(
    (state) => state.sessionIdByWorkspaceIdentity,
  );
  const sessionIdByWorkspacePath = useRemoteWorkspaceSessionStore(
    (state) => state.sessionIdByWorkspacePath,
  );
  const [busy, setBusy] = useState(false);

  const call = useCallback(
    async (
      op: (
        service: typeof baseServices.zcodeAgentService,
        target: { workspacePath: string; workspaceIdentity?: string },
      ) => Promise<unknown>,
    ) => {
      const resolved = resolveWorkspaceServices(
        {
          workspacePath: params.workspacePath,
          ...(params.workspaceIdentity ? { workspaceIdentity: params.workspaceIdentity } : {}),
        },
        baseServices,
        {
          sessionsById,
          sessionIdByWorkspaceIdentity,
          sessionIdByWorkspacePath,
        },
      );
      if (!resolved) throw new Error("workspace_not_connected");
      setBusy(true);
      try {
        await op(resolved.services.zcodeAgentService, {
          workspacePath: params.workspacePath,
          ...(params.workspaceIdentity ? { workspaceIdentity: params.workspaceIdentity } : {}),
        });
      } finally {
        setBusy(false);
      }
    },
    [
      baseServices,
      params.workspaceIdentity,
      params.workspacePath,
      sessionIdByWorkspaceIdentity,
      sessionIdByWorkspacePath,
      sessionsById,
    ],
  );

  const pause = useCallback(
    async (control: { sessionId: string; councilId: string; paused: boolean }) => {
      await call((service, target) =>
        service.pauseCouncilMeeting({
          ...target,
          sessionId: control.sessionId,
          councilId: control.councilId,
          paused: control.paused,
        }),
      );
    },
    [call],
  );

  const interject = useCallback(
    async (control: { sessionId: string; councilId: string; text: string }) => {
      await call((service, target) =>
        service.interjectCouncilMeeting({
          ...target,
          sessionId: control.sessionId,
          councilId: control.councilId,
          text: control.text,
        }),
      );
    },
    [call],
  );

  return { busy, pause, interject };
}
