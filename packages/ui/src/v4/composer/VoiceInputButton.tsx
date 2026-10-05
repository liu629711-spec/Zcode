import { useCallback, useEffect, type ReactNode } from "react";
import { MicIcon, SquareIcon, XIcon } from "lucide-react";

import type { LexicalChatInputHandle } from "@/LexicalChatInput.js";
import { TID_V4_VOICE } from "@zcode/shared";
import { Button } from "@/components/ui/button.js";
import { Spinner } from "@/components/ui/spinner.js";
import { toast } from "@/components/ui/toast.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { useOptionalPlatform } from "@/hooks/usePlatform.js";
import { useVoiceSettingsStore } from "@/store/voiceSettingsStore.js";
import { useVoiceCapture } from "@/v4/composer/useVoiceCapture.js";

const MILLISECONDS_PER_SECOND = 1_000;

function formatElapsed(elapsedMs: number): string {
  const totalSeconds = Math.floor(elapsedMs / MILLISECONDS_PER_SECOND);
  return `${Math.floor(totalSeconds / 60)}:${String(totalSeconds % 60).padStart(2, "0")}`;
}

/**
 * 输入区语音输入按钮（交互抄 ai-elements SpeechInput / ekko VoiceDialogueControls）：
 * 点击录音（红方块 + 计时 + 电平脉冲）→ 再点停止并转写 → 文字 appendText 进草稿，
 * 不自动发送。录音中提供独立取消键。
 */
export function VoiceInputButton({
  disabled,
  inputApiRef,
}: {
  disabled?: boolean;
  inputApiRef: React.MutableRefObject<LexicalChatInputHandle | null>;
}): ReactNode {
  const { intl } = useZCodeIntl();
  const platform = useOptionalPlatform();
  const unavailable = !platform?.voiceTranscribe;

  const insertTranscript = useCallback(
    (text: string) => {
      const inputApi = inputApiRef.current;
      if (!inputApi || !text.trim()) return;
      inputApi.appendText(text.trim());
      inputApi.focus();
    },
    [inputApiRef],
  );

  const handleWavReady = useCallback(
    async (wav: ArrayBuffer) => {
      if (!platform?.voiceTranscribe) return;
      const { provider, language, cloudBaseUrl, cloudApiKey, cloudModel } =
        useVoiceSettingsStore.getState();
      if (provider === "cloud" && (!cloudBaseUrl.trim() || !cloudModel.trim())) {
        throw new Error(intl.formatMessage({ id: "voice.cloud.missing" }));
      }
      const result = await platform.voiceTranscribe({
        provider,
        audio: wav,
        language: language === "auto" ? undefined : language,
        cloud: provider === "cloud" ? { baseUrl: cloudBaseUrl, apiKey: cloudApiKey, model: cloudModel } : undefined,
      });
      insertTranscript(result.text);
    },
    [platform, intl, insertTranscript],
  );

  const capture = useVoiceCapture({ onWavReady: handleWavReady });

  useEffect(() => {
    if (!capture.error) return;
    toast(intl.formatMessage({ id: capture.error }), { variant: "warning" });
    capture.clearError();
  }, [capture, intl]);

  // 转写失败的错误经 handleWavReady 抛出，hook 不会捕获——这里补一个 toast 兜底
  const handleStopClick = useCallback(() => {
    capture.stop("commit").catch(() => {
      toast(intl.formatMessage({ id: "voice.transcribe.failed" }), { variant: "warning" });
    });
  }, [capture, intl]);

  const handleClick = useCallback(() => {
    if (capture.status === "idle") {
      void capture.start();
      return;
    }
    if (capture.status === "recording") {
      handleStopClick();
    }
  }, [capture, handleStopClick]);

  if (unavailable) return null;

  let content: ReactNode;
  let labelId: string;
  if (capture.status === "recording") {
    labelId = "voice.stop";
    content = (
      <>
        <SquareIcon className="size-3 fill-current" />
        <span className="max-w-10 font-mono text-[11px] tabular-nums">
          {formatElapsed(capture.elapsedMs)}
        </span>
      </>
    );
  } else if (capture.status === "transcribing" || capture.status === "requesting") {
    labelId = "voice.transcribing";
    content = <Spinner className="size-4" />;
  } else {
    labelId = "voice.record";
    content = (
      <>
        <MicIcon className="size-4" />
        <span
          aria-hidden
          className="pointer-events-none absolute inset-0 rounded-lg bg-brand transition-opacity"
          style={{ opacity: Math.min(0.35, capture.level * 0.7) }}
        />
      </>
    );
  }

  return (
    <span className="relative inline-flex items-center gap-1">
      <Button
        type="button"
        variant="secondary"
        size="icon-md"
        className={
          capture.status === "recording"
            ? "relative cursor-pointer gap-1 overflow-visible rounded-lg bg-destructive/10 text-destructive hover:bg-destructive/20"
            : "relative cursor-pointer overflow-visible rounded-lg"
        }
        style={capture.status === "recording" ? { minWidth: 56 } : undefined}
        disabled={disabled}
        onClick={handleClick}
        data-testid={TID_V4_VOICE}
        data-state={capture.status}
        aria-pressed={capture.status === "recording"}
        aria-label={intl.formatMessage({ id: labelId })}
      >
        {content}
        <span className="sr-only">{intl.formatMessage({ id: labelId })}</span>
      </Button>
      {capture.status === "recording" ? (
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          className="cursor-pointer text-foreground-subtle"
          onClick={() => void capture.stop("cancel")}
          aria-label={intl.formatMessage({ id: "voice.cancel" })}
        >
          <XIcon className="size-3.5" />
        </Button>
      ) : null}
    </span>
  );
}
