import { create } from "zustand";

import type { VoiceTranscribeProvider } from "@zcode/shared";

/**
 * 语音输入设置——localStorage 单键持久化（与皮肤库同款读写惯例）。
 * 渲染进程侧只管偏好；本地引擎的探测/调用都在主进程。
 *
 * ponytail: 云 API key 存 localStorage（明文落盘在用户本机）。真要上多设备同步
 * 或企业管控时，升级路径 = 宿主密钥库（对齐模型供应商 key 的存法）。
 */

export const VOICE_SETTINGS_STORAGE_KEY = "zcode-voice-settings-v1";

export interface VoiceSettings {
  provider: VoiceTranscribeProvider;
  /** 识别语言："auto" / "zh" / "en" / …（BCP-47 主子标签） */
  language: string;
  cloudBaseUrl: string;
  cloudApiKey: string;
  cloudModel: string;
}

const DEFAULT_VOICE_SETTINGS: VoiceSettings = {
  provider: "sensevoice",
  language: "zh",
  cloudBaseUrl: "",
  cloudApiKey: "",
  cloudModel: "whisper-1",
};

const PROVIDERS: readonly string[] = ["sensevoice", "whisper", "cloud"];

function readVoiceSettings(): VoiceSettings {
  try {
    if (typeof localStorage === "undefined") return DEFAULT_VOICE_SETTINGS;
    const raw = localStorage.getItem(VOICE_SETTINGS_STORAGE_KEY);
    if (!raw) return DEFAULT_VOICE_SETTINGS;
    const parsed = JSON.parse(raw) as Partial<VoiceSettings>;
    return {
      provider:
        typeof parsed.provider === "string" && PROVIDERS.includes(parsed.provider)
          ? parsed.provider
          : DEFAULT_VOICE_SETTINGS.provider,
      language: typeof parsed.language === "string" && parsed.language ? parsed.language : DEFAULT_VOICE_SETTINGS.language,
      cloudBaseUrl: typeof parsed.cloudBaseUrl === "string" ? parsed.cloudBaseUrl : "",
      cloudApiKey: typeof parsed.cloudApiKey === "string" ? parsed.cloudApiKey : "",
      cloudModel: typeof parsed.cloudModel === "string" && parsed.cloudModel ? parsed.cloudModel : DEFAULT_VOICE_SETTINGS.cloudModel,
    };
  } catch {
    return DEFAULT_VOICE_SETTINGS;
  }
}

interface VoiceSettingsStore extends VoiceSettings {
  update: (patch: Partial<VoiceSettings>) => void;
}

function persist(state: VoiceSettingsStore): void {
  try {
    localStorage.setItem(
      VOICE_SETTINGS_STORAGE_KEY,
      JSON.stringify({
        provider: state.provider,
        language: state.language,
        cloudBaseUrl: state.cloudBaseUrl,
        cloudApiKey: state.cloudApiKey,
        cloudModel: state.cloudModel,
      }),
    );
  } catch {
    // 偏好写失败不阻断功能：本次会话仍按内存值工作
  }
}

export const useVoiceSettingsStore = create<VoiceSettingsStore>((set) => ({
  ...readVoiceSettings(),
  update: (patch) => {
    set(patch);
    persist(useVoiceSettingsStore.getState());
  },
}));
