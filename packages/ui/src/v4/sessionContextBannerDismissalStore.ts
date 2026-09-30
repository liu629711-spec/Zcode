import { useSyncExternalStore } from "react";
import {
  SESSION_CONTEXT_BANNER_REMIND_STEP_PERCENT,
  type SessionContextBannerState,
} from "@/v4/sessionContextBannerState.js";

/**
 * 换班横幅的会话内免打扰记忆（照 sessionQuotaBannerDismissalStore 先例：
 * Renderer 生命周期内、按会话隔离、不持久化——刷新后重新提示是可接受的）。
 * 忽略时记下当时的用量百分比；用量又涨了 REMIND_STEP 个百分点才重新可见，
 * 避免"每次用量更新都弹一次"的刷屏。
 */
interface SessionContextBannerDismissalStore {
  dismiss(sessionId: string, percentAtDismiss: number): void;
  isSuppressed(sessionId: string, state: SessionContextBannerState): boolean;
  subscribe(listener: () => void): () => void;
}

function createSessionContextBannerDismissalStore(): SessionContextBannerDismissalStore {
  const dismissedBySession = new Map<string, number>();
  const listeners = new Set<() => void>();
  let version = 0;

  const emit = (): void => {
    version += 1;
    for (const listener of listeners) listener();
  };

  return {
    dismiss(sessionId, percentAtDismiss) {
      const normalized = sessionId.trim();
      if (!normalized || !Number.isFinite(percentAtDismiss)) return;
      dismissedBySession.set(normalized, percentAtDismiss);
      emit();
    },
    isSuppressed(sessionId, state) {
      const dismissed = dismissedBySession.get(sessionId.trim());
      if (dismissed === undefined || state.percent === null) {
        return false;
      }
      return state.percent - dismissed < SESSION_CONTEXT_BANNER_REMIND_STEP_PERCENT;
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

export const sessionContextBannerDismissalStore =
  createSessionContextBannerDismissalStore();

export function useSessionContextBannerSuppressed(
  sessionId: string | null,
  state: SessionContextBannerState,
): boolean {
  return useSyncExternalStore(
    sessionContextBannerDismissalStore.subscribe,
    () =>
      sessionId
        ? sessionContextBannerDismissalStore.isSuppressed(sessionId, state)
        : false,
    () => false,
  );
}
