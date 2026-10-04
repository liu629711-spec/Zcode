import { useMemo, useRef } from "react";
import { DownloadIcon, Link2Icon, PaletteIcon, Trash2Icon, UploadIcon, WallpaperIcon } from "lucide-react";
import { Button } from "@/components/ui/button.js";
import { toast } from "@/components/ui/toast.js";
import { useZCodeIntl } from "@/i18n/index.js";
import { useConfirmDialogStore } from "@/store/confirmDialogStore.js";
import { useZCodeStore } from "@/store/StoreProvider.js";
import { DESIGN_STYLE_OPTIONS } from "@/settings/settingsPageConfig.js";
import { DESIGN_STYLE_RAMP_PREVIEW } from "@/themeStyles.js";
import { ActiveSkinTuning } from "@/settings/skinTuningPanel.js";
import {
  createSkinFromStyle,
  serializeSkin,
  validateSkin,
  type SkinPack,
  type SkinTokenSlot,
} from "@/skin-engine/skinSchema.js";
import { buildSkinShareUrl } from "@/skin-engine/skinShare.js";

/**
 * 皮肤工坊——外观设置里的皮肤中心：预设风格卡 + 我的皮肤库 + 激活皮肤的实时调参。
 * 安装/删除走全局确认弹窗（Promise 式）；导入失败逐条报错；分享链接仅纯配色。
 */

const SLOT_SWATCH_ORDER: SkinTokenSlot[] = [
  "background",
  "surface",
  "sidebar",
  "accent",
  "brand",
  "ink",
];

function downloadSkinFile(skin: SkinPack): void {
  const blob = new Blob([serializeSkin(skin)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `${skin.id}.zcode-skin.json`;
  anchor.click();
  URL.revokeObjectURL(url);
}

function SkinSwatches({ skin }: { skin: SkinPack }) {
  // 没写 override 的槽位回落到 baseStyle 的预设 ramp 色——卡片展示的是"这套皮肤实际长什么样"
  const ramp = DESIGN_STYLE_RAMP_PREVIEW[skin.baseStyle];
  const palette = { ...ramp.light, ...skin.light };
  const dark = { ...ramp.dark, ...skin.dark };
  return (
    <div className="flex h-8 overflow-hidden rounded-lg border border-border">
      {SLOT_SWATCH_ORDER.map((slot) => (
        <div key={slot} className="flex flex-1 flex-col">
          <span className="h-4" style={{ background: palette[slot] }} />
          <span className="h-4" style={{ background: dark[slot] }} />
        </div>
      ))}
    </div>
  );
}

function SkinCard({ skin, active, onActivate }: { skin: SkinPack; active: boolean; onActivate: () => void }) {
  const { intl } = useZCodeIntl();
  const removeSkin = useZCodeStore((state) => state.removeSkin);
  const requestConfirmation = useConfirmDialogStore((state) => state.requestConfirmation);
  const shareUrl = useMemo(() => buildSkinShareUrl(skin), [skin]);

  const handleDelete = async () => {
    const confirmed = await requestConfirmation({
      title: intl.formatMessage({ id: "skin.deleteConfirmTitle" }, { name: skin.name }),
      description: intl.formatMessage({ id: "skin.deleteConfirmDescription" }),
      confirmVariant: "destructive",
    });
    if (confirmed) removeSkin(skin.id);
  };

  return (
    <div
      className={`group relative rounded-xl border p-3 transition-colors ${
        active ? "border-brand/60 bg-brand/5" : "border-border bg-background/60 hover:border-border-hover"
      }`}
      data-testid={`skin-card-${skin.id}`}
      data-active={active || undefined}
    >
      <button type="button" onClick={onActivate} className="block w-full text-left" aria-pressed={active}>
        <SkinSwatches skin={skin} />
        <span className="mt-2 flex items-center gap-2">
          <span className="min-w-0 truncate text-ui-sm font-medium text-foreground">{skin.name}</span>
          {active ? (
            <span className="shrink-0 rounded-full bg-brand/15 px-1.5 py-0.5 text-ui-xs font-medium text-brand">
              {intl.formatMessage({ id: "skin.active" })}
            </span>
          ) : null}
        </span>
        <span className="mt-0.5 flex min-h-4 items-center gap-1.5 text-ui-xs text-foreground-subtle">
          {skin.author ? <span className="truncate">{skin.author}</span> : null}
          {skin.wallpaper ? (
            <span className="inline-flex shrink-0 items-center gap-0.5 rounded bg-foreground/5 px-1">
              <WallpaperIcon className="size-3" aria-hidden />
              {intl.formatMessage({ id: "skin.badge.wallpaper" })}
            </span>
          ) : null}
          {skin.glass?.enabled ? (
            <span className="shrink-0 rounded bg-foreground/5 px-1">
              {intl.formatMessage({ id: "skin.badge.glass" })}
            </span>
          ) : null}
        </span>
      </button>
      <div className="mt-2 flex items-center gap-1 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          disabled={shareUrl === null}
          title={
            shareUrl === null
              ? intl.formatMessage({ id: "skin.shareUnavailable" })
              : intl.formatMessage({ id: "skin.shareLink" })
          }
          aria-label={intl.formatMessage({ id: "skin.shareLink" })}
          onClick={async () => {
            if (shareUrl === null) return;
            await navigator.clipboard.writeText(shareUrl);
            toast(intl.formatMessage({ id: "skin.shareLinkCopied" }));
          }}
        >
          <Link2Icon className="size-3.5" aria-hidden />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          title={intl.formatMessage({ id: "skin.export" })}
          aria-label={intl.formatMessage({ id: "skin.export" })}
          onClick={() => downloadSkinFile(skin)}
        >
          <DownloadIcon className="size-3.5" aria-hidden />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          className="text-destructive hover:text-destructive"
          title={intl.formatMessage({ id: "skin.delete" })}
          aria-label={intl.formatMessage({ id: "skin.delete" })}
          onClick={() => void handleDelete()}
        >
          <Trash2Icon className="size-3.5" aria-hidden />
        </Button>
      </div>
    </div>
  );
}

export function SkinSettingsSection() {
  const { intl } = useZCodeIntl();
  const skinLibrary = useZCodeStore((state) => state.skinLibrary);
  const activeSkinId = useZCodeStore((state) => state.activeSkinId);
  const designStyle = useZCodeStore((state) => state.designStyle);
  const setDesignStyle = useZCodeStore((state) => state.setDesignStyle);
  const importSkin = useZCodeStore((state) => state.importSkin);
  const setActiveSkinId = useZCodeStore((state) => state.setActiveSkinId);
  const requestConfirmation = useConfirmDialogStore((state) => state.requestConfirmation);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const activeSkin = skinLibrary.find((pack) => pack.id === activeSkinId) ?? null;

  const createFromCurrentStyle = () => {
    const id = `my-${designStyle}-${Date.now().toString(36)}`;
    const result = importSkin(
      createSkinFromStyle(
        designStyle,
        id,
        intl.formatMessage({ id: "skin.defaultName" }, { count: skinLibrary.length + 1 }),
      ),
    );
    if (result.ok) {
      setActiveSkinId(id);
    } else {
      // 现实失败原因基本是存储配额满——不能静默无反应
      toast(result.errors.join("；"));
    }
  };

  const installPack = async (pack: SkinPack) => {
    // 第三方作品（文件导入/分享链接）：先亮明内容再装（拍板③）
    const overrides = Object.keys({ ...pack.light, ...pack.dark }).length;
    const contentParts = [
      pack.wallpaper && intl.formatMessage({ id: "skin.badge.wallpaper" }),
      pack.glass?.enabled && intl.formatMessage({ id: "skin.badge.glass" }),
      overrides > 0 && intl.formatMessage({ id: "skin.overrideCount" }, { count: overrides }),
    ].filter(Boolean);
    const confirmed = await requestConfirmation({
      title: intl.formatMessage({ id: "skin.installConfirmTitle" }, { name: pack.name }),
      description: intl.formatMessage(
        { id: contentParts.length > 0 ? "skin.installConfirmDescription" : "skin.installConfirmEmpty" },
        { content: contentParts.join("、") },
      ),
      confirmLabel: intl.formatMessage({ id: "skin.install" }),
    });
    if (!confirmed) return;
    const imported = importSkin(pack);
    if (!imported.ok) {
      toast(imported.errors.join("；"));
      return;
    }
    setActiveSkinId(imported.skin.id);
    toast(intl.formatMessage({ id: "skin.imported" }, { name: imported.skin.name }));
  };

  const handleImportFile = async (file: File) => {
    let raw: unknown;
    try {
      raw = JSON.parse(await file.text());
    } catch {
      toast(intl.formatMessage({ id: "skin.importBadJson" }));
      return;
    }
    const result = validateSkin(raw);
    if (!result.ok) {
      toast(result.errors.join("；"));
      return;
    }
    await installPack(result.skin);
  };

  return (
    <div className="min-w-0 space-y-3">
      <div>
        <h3 className="text-ui-lg font-semibold text-foreground">
          {intl.formatMessage({ id: "skin.title" })}
        </h3>
        <p className="mt-1 text-ui-base leading-6 text-foreground-subtle">
          {intl.formatMessage({ id: "skin.description" })}
        </p>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Button type="button" variant="outline" size="sm" onClick={createFromCurrentStyle}>
          <PaletteIcon className="size-3.5" aria-hidden />
          {intl.formatMessage({ id: "skin.createFromStyle" })}
        </Button>
        <Button type="button" variant="outline" size="sm" onClick={() => fileInputRef.current?.click()}>
          <UploadIcon className="size-3.5" aria-hidden />
          {intl.formatMessage({ id: "skin.import" })}
        </Button>
        <input
          ref={fileInputRef}
          type="file"
          accept=".json,application/json"
          className="hidden"
          onChange={(event) => {
            const file = event.currentTarget.files?.[0];
            event.currentTarget.value = "";
            if (file) void handleImportFile(file);
          }}
        />
      </div>

      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        {DESIGN_STYLE_OPTIONS.map((style) => {
          const active = activeSkinId === null && designStyle === style;
          return (
            <button
              key={style}
              type="button"
              onClick={() => {
                setActiveSkinId(null);
                setDesignStyle(style);
              }}
              aria-pressed={active}
              data-testid={`skin-preset-${style}`}
              className={`rounded-xl border p-3 text-left transition-colors ${
                active ? "border-brand/60 bg-brand/5" : "border-border bg-background/60 hover:border-border-hover"
              }`}
            >
              <SkinSwatches skin={createSkinFromStyle(style, "preview", "preview")} />
              <span className="mt-2 block truncate text-ui-sm font-medium text-foreground">
                {intl.formatMessage({ id: `settings.designStyle.${style}` })}
              </span>
            </button>
          );
        })}
      </div>

      <div className="space-y-2">
        <span className="text-ui-sm font-medium text-foreground">
          {intl.formatMessage({ id: "skin.libraryTitle" })}
        </span>
        {skinLibrary.length === 0 ? (
          <p className="rounded-xl border border-dashed border-border px-4 py-6 text-center text-ui-sm text-foreground-subtle">
            {intl.formatMessage({ id: "skin.emptyLibrary" })}
          </p>
        ) : (
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
            {skinLibrary.map((pack) => (
              <SkinCard
                key={pack.id}
                skin={pack}
                active={pack.id === activeSkinId}
                onActivate={() => setActiveSkinId(pack.id)}
              />
            ))}
          </div>
        )}
      </div>

      {activeSkin ? <ActiveSkinTuning skin={activeSkin} /> : null}
    </div>
  );
}
