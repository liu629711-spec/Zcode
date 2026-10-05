import { useCallback, useEffect, useState, type ReactNode } from "react";
import type { VoiceEngineStatus, VoiceTranscribeProvider } from "@zcode/shared";
import { FolderOpen } from "lucide-react";

import { Button } from "@/components/ui/button.js";
import { Input } from "@/components/ui/input.js";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select.js";
import { toast } from "@/components/ui/toast.js";
import { useOptionalPlatform } from "@/hooks/usePlatform.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import {
  useVoiceSettingsStore,
  type VoiceSettings,
} from "@/store/voiceSettingsStore.js";
import { SettingsGroupCard, SettingsRow } from "@/settings/SettingsPageParts.js";

const PROVIDER_OPTIONS: Array<{ value: VoiceTranscribeProvider; labelId: string }> = [
  { value: "sensevoice", labelId: "voice.settings.provider.sensevoice" },
  { value: "whisper", labelId: "voice.settings.provider.whisper" },
  { value: "cloud", labelId: "voice.settings.provider.cloud" },
];

const LANGUAGE_OPTIONS = ["auto", "zh", "en", "ja", "ko", "yue"] as const;

function EngineStatusRow(): ReactNode {
  const { intl } = useZCodeIntl();
  const platform = useOptionalPlatform();
  const [status, setStatus] = useState<VoiceEngineStatus | null>(null);

  useEffect(() => {
    if (!platform?.voiceGetEngineStatus) return;
    platform
      .voiceGetEngineStatus()
      .then(setStatus)
      .catch(() => setStatus(null));
  }, [platform]);

  if (!platform?.voiceGetEngineStatus) return null;

  const renderEngineLine = (labelId: string, engine: VoiceEngineStatus["whisper"]) => (
    <SettingsRow
      label={intl.formatMessage({ id: labelId })}
      description={
        engine.installed
          ? engine.modelPath
          : intl.formatMessage({ id: "voice.settings.engineMissing" })
      }
      control={
        <span
          className="text-ui-base font-medium"
          style={{ color: engine.installed ? undefined : "var(--text-warning, #b45309)" }}
        >
          {intl.formatMessage({
            id: engine.installed ? "voice.settings.engineReady" : "voice.settings.engineNotInstalled",
          })}
        </span>
      }
    />
  );

  return (
    <SettingsGroupCard>
      {renderEngineLine("voice.settings.engine.sensevoice", status?.sensevoice ?? { installed: false })}
      {renderEngineLine("voice.settings.engine.whisper", status?.whisper ?? { installed: false })}
      <SettingsRow
        label={intl.formatMessage({ id: "voice.settings.engineFolder" })}
        description={intl.formatMessage({ id: "voice.settings.engineFolderHint" })}
        control={
          <Button
            type="button"
            variant="secondary"
            size="sm"
            className="cursor-pointer"
            disabled={!status?.root}
            onClick={() => {
              if (!status?.root) return;
              void platform
                .openInFileManager?.(status.root)
                ?.catch(() =>
                  toast(intl.formatMessage({ id: "voice.settings.openFolderFailed" }), {
                    variant: "warning",
                  }),
                );
            }}
          >
            <FolderOpen className="size-4" />
            {intl.formatMessage({ id: "voice.settings.openFolder" })}
          </Button>
        }
      />
    </SettingsGroupCard>
  );
}

export function VoiceSettingsSection(): ReactNode {
  const { intl } = useZCodeIntl();
  const settings = useVoiceSettingsStore();
  const update = useCallback(
    (patch: Partial<VoiceSettings>) => settings.update(patch),
    [settings],
  );

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6 p-6">
      <SettingsGroupCard>
        <SettingsRow
          label={intl.formatMessage({ id: "voice.settings.provider" })}
          description={intl.formatMessage({ id: "voice.settings.providerHint" })}
          control={
            <Select
              value={settings.provider}
              onValueChange={(value) =>
                update({ provider: value as VoiceTranscribeProvider })
              }
            >
              <SelectTrigger className="w-48">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {PROVIDER_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {intl.formatMessage({ id: option.labelId })}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          }
        />
        <SettingsRow
          label={intl.formatMessage({ id: "voice.settings.language" })}
          description={intl.formatMessage({ id: "voice.settings.languageHint" })}
          control={
            <Select
              value={settings.language}
              onValueChange={(value) => update({ language: value })}
            >
              <SelectTrigger className="w-48">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {LANGUAGE_OPTIONS.map((language) => (
                  <SelectItem key={language} value={language}>
                    {intl.formatMessage({ id: `voice.settings.language.${language}` })}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          }
        />
      </SettingsGroupCard>

      {settings.provider === "cloud" ? (
        <SettingsGroupCard>
          <SettingsRow
            label={intl.formatMessage({ id: "voice.settings.cloud.baseUrl" })}
            description={intl.formatMessage({ id: "voice.settings.cloud.baseUrlHint" })}
            controlLayout="wide"
            control={
              <Input
                value={settings.cloudBaseUrl}
                placeholder="https://api.openai.com"
                onChange={(event) => update({ cloudBaseUrl: event.target.value })}
              />
            }
          />
          <SettingsRow
            label={intl.formatMessage({ id: "voice.settings.cloud.model" })}
            controlLayout="wide"
            control={
              <Input
                value={settings.cloudModel}
                placeholder="whisper-1"
                onChange={(event) => update({ cloudModel: event.target.value })}
              />
            }
          />
          <SettingsRow
            label={intl.formatMessage({ id: "voice.settings.cloud.apiKey" })}
            description={intl.formatMessage({ id: "voice.settings.cloud.apiKeyHint" })}
            controlLayout="wide"
            control={
              <Input
                type="password"
                value={settings.cloudApiKey}
                onChange={(event) => update({ cloudApiKey: event.target.value })}
              />
            }
          />
        </SettingsGroupCard>
      ) : null}

      <EngineStatusRow />

      <p className="px-1 text-ui-base leading-6 text-foreground-subtle">
        {intl.formatMessage({ id: "voice.settings.installHint" })}
      </p>
    </div>
  );
}
