import { useRef } from "react";
import { ImagePlusIcon } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card.js";
import { Button } from "@/components/ui/button.js";
import { Switch } from "@/components/ui/switch.js";
import { toast } from "@/components/ui/toast.js";
import { useZCodeIntl } from "@/i18n/index.js";
import { useZCodeStore } from "@/store/StoreProvider.js";
import { DEFAULT_WALLPAPER_GLASS } from "@/skin-engine/skinApply.js";
import type { SkinPack } from "@/skin-engine/skinSchema.js";

/** 激活皮肤的实时调参面板：玻璃材质（融入度+三区域倍率）与壁纸（不透明度/模糊/暗化）。 */

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

  const patch = (partial: Partial<SkinPack>) => {
    const { quotaFull } = upsertSkin({ ...skin, ...partial });
    if (quotaFull) toast(intl.formatMessage({ id: "skin.quotaFull" }));
  };
  const patchGlass = (partial: Partial<NonNullable<SkinPack["glass"]>>) => {
    if (!glass) return;
    patch({ glass: { ...glass, ...partial } });
  };
  const patchWallpaper = (partial: Partial<NonNullable<SkinPack["wallpaper"]>>) => {
    if (!wallpaper) return;
    patch({ wallpaper: { ...wallpaper, ...partial } });
  };
  // 皮肤没写 glass 字段时开关也常驻：开启即落一份默认参数（融入度与三区域同起步）
  const handleGlassToggle = (enabled: boolean) => {
    if (glass) {
      patchGlass({ enabled });
      return;
    }
    if (enabled) {
      patch({ glass: { ...DEFAULT_WALLPAPER_GLASS, enabled } });
    }
  };

  const importWallpaperImage = async (file: File) => {
    const dataUrl = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result));
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
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
            checked={glass?.enabled ?? false}
            onCheckedChange={handleGlassToggle}
            aria-label={intl.formatMessage({ id: "skin.glassTitle" })}
          />
        </div>
        {glass?.enabled ? (
          <div className="space-y-2 px-4 pb-3">
            {(
              [
                ["skin.blend", glass.blend, (blend: number) => patchGlass({ blend })],
                ["skin.sidebar", glass.sidebar, (sidebar: number) => patchGlass({ sidebar })],
                ["skin.panel", glass.panel, (panel: number) => patchGlass({ panel })],
                ["skin.composer", glass.composer, (composer: number) => patchGlass({ composer })],
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
        {wallpaper ? (
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
              value={wallpaper.opacity}
              format={(v) => `${Math.round(v * 100)}%`}
              onChange={(opacity) => patchWallpaper({ opacity })}
            />
            <TuningSlider
              label={intl.formatMessage({ id: "skin.blur" })}
              min={0}
              max={40}
              step={1}
              value={wallpaper.blurPx}
              format={(v) => `${v}px`}
              onChange={(blurPx) => patchWallpaper({ blurPx })}
            />
            <TuningSlider
              label={intl.formatMessage({ id: "skin.dim" })}
              min={0}
              max={0.85}
              step={0.05}
              value={wallpaper.dim}
              format={(v) => `${Math.round(v * 100)}%`}
              onChange={(dim) => patchWallpaper({ dim })}
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
