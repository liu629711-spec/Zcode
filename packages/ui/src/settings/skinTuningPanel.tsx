import { useEffect, useRef, useState } from "react";
import { ImagePlusIcon } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card.js";
import { Button } from "@/components/ui/button.js";
import { Switch } from "@/components/ui/switch.js";
import { toast } from "@/components/ui/toast.js";
import { useZCodeIntl } from "@/i18n/index.js";
import { useZCodeStore } from "@/store/StoreProvider.js";
import {
  applySkinToDocument,
  DEFAULT_WALLPAPER_GLASS,
  resolveEffectiveGlass,
} from "@/skin-engine/skinApply.js";
import {
  formatSkinValidationErrors,
  type SkinGlass,
  type SkinPack,
  type SkinWallpaper,
} from "@/skin-engine/skinSchema.js";

/** 激活皮肤的实时调参面板：玻璃材质（融入度+三区域倍率）与壁纸（不透明度/模糊/暗化）。 */

/** 拖动期间只走便宜的 document 预览，停手多久后一次性落库（换肤引擎批 2 滑杆防抖）。 */
const PERSIST_DEBOUNCE_MS = 250;

const GRADIENT_PRESETS = [
  { labelId: "skin.wallpaper.preset.aurora", value: "linear-gradient(160deg, #0f2027 0%, #203a43 50%, #2c5364 100%)" },
  { labelId: "skin.wallpaper.preset.sunset", value: "linear-gradient(160deg, #3a2b26 0%, #7a4a32 55%, #c96442 100%)" },
] as const;

function TuningSlider({
  label,
  value,
  min,
  max,
  step,
  format,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (value: number) => string;
  onChange: (value: number) => void;
}) {
  return (
    <label className="flex items-center gap-3 text-ui-sm text-foreground">
      <span className="w-20 shrink-0">{label}</span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(event) => onChange(Number(event.currentTarget.value))}
        className="h-1.5 min-w-0 flex-1 cursor-pointer accent-brand"
      />
      <span className="w-12 shrink-0 text-right tabular-nums text-ui-xs text-foreground-subtle">
        {format(value)}
      </span>
    </label>
  );
}

export function ActiveSkinTuning({ skin }: { skin: SkinPack }) {
  const { intl } = useZCodeIntl();
  const upsertSkin = useZCodeStore((state) => state.upsertSkin);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const glass = skin.glass;
  const wallpaper = skin.wallpaper;
  // 显示态跟着引擎的"生效态"走：有壁纸未写 glass 时引擎按默认融入度生效，
  // 开关必须显示为开，否则用户看到的效果与面板脱钩
  const effectiveGlass = resolveEffectiveGlass(skin);

  // 滑杆防抖（换肤引擎批 2）：拖动的每个 tick 原本都全量 upsertSkin——整库
  // localStorage 读写（壁纸 data URL 可达 4MB）+ 全店订阅者重渲染，拖一下卡一下。
  // 改成 tick 只更新草稿 + 立即 applySkinToDocument 预览（便宜路径，只重写覆盖层
  // <style>），停手 PERSIST_DEBOUNCE_MS 后一次性落库。
  const [draft, setDraft] = useState<{ glass?: SkinGlass; wallpaper?: SkinWallpaper }>({});
  const draftRef = useRef(draft);
  draftRef.current = draft;
  const pendingRef = useRef<Partial<SkinPack> | undefined>(undefined);
  const timerRef = useRef<number | undefined>(undefined);
  const patchRef = useRef<(partial: Partial<SkinPack>) => void>(() => {});

  const patch = (partial: Partial<SkinPack>) => {
    if (timerRef.current !== undefined) {
      window.clearTimeout(timerRef.current);
      timerRef.current = undefined;
      pendingRef.current = undefined;
    }
    setDraft({});
    const result = upsertSkin({ ...skin, ...partial });
    if (!result.ok) {
      if (result.quotaFull) toast(intl.formatMessage({ id: "skin.quotaFull" }));
      else toast(formatSkinValidationErrors(intl, result.errors));
    }
  };
  patchRef.current = patch;

  // 卸载时把没来得及落库的尾巴同步冲掉：拖完立刻关面板，最后一个值不丢。
  useEffect(
    () => () => {
      if (timerRef.current === undefined) return;
      window.clearTimeout(timerRef.current);
      timerRef.current = undefined;
      if (pendingRef.current) patchRef.current(pendingRef.current);
    },
    [],
  );

  const schedulePersist = (partial: Partial<SkinPack>) => {
    pendingRef.current = { ...pendingRef.current, ...partial };
    if (timerRef.current !== undefined) window.clearTimeout(timerRef.current);
    timerRef.current = window.setTimeout(() => {
      timerRef.current = undefined;
      const pending = pendingRef.current;
      pendingRef.current = undefined;
      if (pending) patchRef.current(pending);
    }, PERSIST_DEBOUNCE_MS);
  };

  const tune = (partial: { glass?: SkinGlass; wallpaper?: SkinWallpaper }) => {
    setDraft((previous) => ({ ...previous, ...partial }));
    applySkinToDocument({ ...skin, ...partial });
    schedulePersist(partial);
  };

  // glass 字段缺席时先落一份参数再改：enabled 跟随当前生效态（有壁纸=默认生效），
  // 否则拖滑杆会落 enabled:false 被引擎忽略，看起来像"拖了没反应"
  const glassBase = glass ?? {
    ...DEFAULT_WALLPAPER_GLASS,
    enabled: effectiveGlass?.enabled ?? false,
  };
  const draftGlass = draft.glass;
  const draftWallpaper = draft.wallpaper;
  const tuneGlass = (partial: Partial<SkinGlass>) => {
    tune({ glass: { ...glassBase, ...draftRef.current.glass, ...partial } });
  };
  const tuneWallpaper = (partial: Partial<SkinWallpaper>) => {
    if (!wallpaper) return;
    tune({ wallpaper: { ...wallpaper, ...draftRef.current.wallpaper, ...partial } });
  };
  const handleGlassToggle = (enabled: boolean) => {
    patch({ glass: { ...glassBase, ...draftRef.current.glass, enabled } });
  };

  const importWallpaperImage = async (file: File) => {
    let dataUrl: string;
    try {
      dataUrl = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(String(reader.result));
        reader.onerror = () => reject(reader.error);
        reader.readAsDataURL(file);
      });
    } catch {
      toast(intl.formatMessage({ id: "skin.wallpaperReadFailed" }));
      return;
    }
    patch({
      wallpaper: { kind: "image", imageDataUrl: dataUrl, opacity: 1, blurPx: 0, dim: 0.25 },
    });
  };

  return (
    <Card className="border border-border bg-card py-0 shadow-none">
      <CardContent className="space-y-3 px-0 py-1">
        <div className="flex items-center justify-between px-4 pt-2">
          <span className="text-ui-sm font-medium text-foreground">
            {intl.formatMessage({ id: "skin.glassTitle" })}
          </span>
          <Switch
            checked={effectiveGlass?.enabled ?? false}
            onCheckedChange={handleGlassToggle}
            aria-label={intl.formatMessage({ id: "skin.glassTitle" })}
          />
        </div>
        {effectiveGlass?.enabled ? (
          <div className="space-y-2 px-4 pb-3">
            {(
              [
                ["skin.blend", draftGlass?.blend ?? glassBase.blend, (blend: number) => tuneGlass({ blend })],
                ["skin.sidebar", draftGlass?.sidebar ?? glassBase.sidebar, (sidebar: number) => tuneGlass({ sidebar })],
                ["skin.panel", draftGlass?.panel ?? glassBase.panel, (panel: number) => tuneGlass({ panel })],
                ["skin.composer", draftGlass?.composer ?? glassBase.composer, (composer: number) => tuneGlass({ composer })],
              ] as const
            ).map(([labelId, value, onChange]) => (
              <TuningSlider
                key={labelId}
                label={intl.formatMessage({ id: labelId })}
                min={0}
                max={1}
                step={0.05}
                value={value}
                format={(v) => `${Math.round(v * 100)}%`}
                onChange={onChange}
              />
            ))}
          </div>
        ) : null}
        {wallpaper || draftWallpaper ? (
          <div className="space-y-2 border-t border-border px-4 py-3">
            <div className="flex items-center justify-between">
              <span className="text-ui-sm font-medium text-foreground">
                {intl.formatMessage({ id: "skin.wallpaperTitle" })}
              </span>
              <Button type="button" variant="ghost" size="sm" onClick={() => patch({ wallpaper: undefined })}>
                {intl.formatMessage({ id: "skin.removeWallpaper" })}
              </Button>
            </div>
            <TuningSlider
              label={intl.formatMessage({ id: "skin.opacity" })}
              min={0.15}
              max={1}
              step={0.05}
              value={draftWallpaper?.opacity ?? wallpaper?.opacity ?? 1}
              format={(v) => `${Math.round(v * 100)}%`}
              onChange={(opacity) => tuneWallpaper({ opacity })}
            />
            <TuningSlider
              label={intl.formatMessage({ id: "skin.blur" })}
              min={0}
              max={40}
              step={1}
              value={draftWallpaper?.blurPx ?? wallpaper?.blurPx ?? 0}
              format={(v) => `${v}px`}
              onChange={(blurPx) => tuneWallpaper({ blurPx })}
            />
            <TuningSlider
              label={intl.formatMessage({ id: "skin.dim" })}
              min={0}
              max={0.85}
              step={0.05}
              value={draftWallpaper?.dim ?? wallpaper?.dim ?? 0}
              format={(v) => `${Math.round(v * 100)}%`}
              onChange={(dim) => tuneWallpaper({ dim })}
            />
          </div>
        ) : (
          <div className="space-y-2 border-t border-border px-4 py-3">
            <span className="text-ui-sm font-medium text-foreground">
              {intl.formatMessage({ id: "skin.wallpaperTitle" })}
            </span>
            <div className="flex flex-wrap items-center gap-2">
              <Button type="button" variant="outline" size="sm" onClick={() => fileInputRef.current?.click()}>
                <ImagePlusIcon className="size-3.5" aria-hidden />
                {intl.formatMessage({ id: "skin.addWallpaperImage" })}
              </Button>
              {GRADIENT_PRESETS.map((preset) => (
                <Button
                  key={preset.value}
                  type="button"
                  variant="outline"
                  size="sm"
                  className="h-7 px-2"
                  title={intl.formatMessage({ id: preset.labelId })}
                  onClick={() =>
                    patch({ wallpaper: { kind: "gradient", gradient: preset.value, opacity: 1, blurPx: 0, dim: 0.2 } })
                  }
                >
                  <span
                    aria-hidden
                    className="size-4 rounded-full border border-border"
                    style={{ background: preset.value }}
                  />
                </Button>
              ))}
              <input
                ref={fileInputRef}
                type="file"
                accept="image/*"
                className="hidden"
                onChange={(event) => {
                  const file = event.currentTarget.files?.[0];
                  event.currentTarget.value = "";
                  if (file) void importWallpaperImage(file);
                }}
              />
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
