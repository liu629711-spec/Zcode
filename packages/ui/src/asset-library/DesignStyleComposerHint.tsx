/**
 * 设计意图提醒条（V4.6 用户反馈：识别到用户要设计界面时，主动展示可选风格）。
 *
 * 触发：composer 草稿命中设计意图关键词（且未插过风格引用）时出现在输入框上方；
 * 内容：推荐位风格 chip（色板 + 风格名）+ 全量入口提示。点击 chip = 落盘规范 +
 * 经 composer 文本插入通道写入「风格引用 chip + 口令」，与 @ 选中、素材库发货同一条链。
 */
import { useMemo, useState } from "react";
import { LoaderIcon, PaletteIcon, XIcon } from "lucide-react";
import { toast } from "@/components/ui/toast.js";
import { cn } from "@/components/lib/utils.js";
import { useOptionalPlatform } from "@/hooks/usePlatform.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { useZCodeSessionStore } from "@/store/zcodeSessionStore.js";
import {
  buildAssetReferenceChipMessage,
  buildAssetReferenceMention,
} from "./assetReferenceMessage.js";
import {
  DESIGN_STYLE_ASSETS,
  extractDesignStylePalette,
  getFeaturedDesignStyleAssets,
  resolveDesignStyleDisplayName,
  resolveDesignStyleEntryPath,
} from "@/mentions/providers/designStylesMentionProvider.js";
import { writeAssetBlueprintFiles } from "./writeAssetFiles.js";

/** 设计意图识别：草稿命中即展示推荐条；已插入风格引用的草稿不再打扰。 */
export function matchesDesignStyleIntent(draftText: string): boolean {
  if (draftText.includes("](./.zcode/asset-library/")) {
    return false;
  }
  return /设计|界面|风格|美化|好看|页面|样式|海报|改版|配色|design|style|landing|redesign/i.test(
    draftText,
  );
}

export function DesignStyleComposerHint({
  workspacePath,
  workspaceIdentity,
  onDismiss,
}: {
  workspacePath: string;
  workspaceIdentity?: string;
  onDismiss: () => void;
}) {
  const { intl, locale } = useZCodeIntl();
  const platform = useOptionalPlatform();
  const [pendingId, setPendingId] = useState<string | null>(null);
  const featured = useMemo(() => getFeaturedDesignStyleAssets(4), []);

  const handlePick = async (assetId: string) => {
    if (pendingId) return;
    const asset = featured.find((candidate) => candidate.id === assetId);
    if (!asset) return;
    setPendingId(assetId);
    try {
      await writeAssetBlueprintFiles(platform, workspacePath, asset);
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error);
      toast(intl.formatMessage({ id: "assetLibrary.detail.writeFailedReason" }, { error: reason }));
      setPendingId(null);
      return;
    }
    const entryPath = resolveDesignStyleEntryPath(asset);
    const mention = buildAssetReferenceMention(asset, [entryPath], locale);
    const { text } = buildAssetReferenceChipMessage(asset, [entryPath], locale);
    useZCodeSessionStore
      .getState()
      .requestComposerTextInsert(workspacePath, text, workspaceIdentity, mention);
    onDismiss();
  };

  return (
    <div
      data-testid="composer-design-style-hint"
      className="mb-2 flex flex-wrap items-center gap-x-2 gap-y-1.5 rounded-lg border border-border bg-surface/60 px-2.5 py-1.5"
    >
      <span className="flex shrink-0 items-center gap-1.5 text-ui-xs font-medium text-foreground">
        <PaletteIcon className="size-3.5 text-foreground-subtle" aria-hidden="true" />
        {intl.formatMessage({ id: "chat.designStyle.hint.question" })}
      </span>
      {featured.map((asset) => {
        const palette = extractDesignStylePalette(asset);
        const pending = pendingId === asset.id;
        return (
          <button
            key={asset.id}
            type="button"
            disabled={pendingId !== null}
            data-testid={`composer-design-style-hint-${asset.id}`}
            onClick={() => void handlePick(asset.id)}
            className={cn(
              "flex shrink-0 items-center gap-1.5 rounded-full border border-border px-2 py-0.5 text-ui-xs text-foreground-subtle",
              "transition-colors hover:bg-hover hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-input-border-focused",
              pending && "opacity-60",
            )}
          >
            {pending ? (
              <LoaderIcon className="size-3 animate-spin" aria-hidden="true" />
            ) : (
              <span className="flex items-center gap-0.5" aria-hidden="true">
                {palette.slice(0, 3).map((color) => (
                  <span
                    key={color}
                    className="size-2 rounded-full border border-border"
                    style={{ backgroundColor: color }}
                  />
                ))}
              </span>
            )}
            {resolveDesignStyleDisplayName(asset, locale)}
          </button>
        );
      })}
      <span className="min-w-0 truncate text-ui-xs text-foreground-subtlest">
        {intl.formatMessage(
          { id: "chat.designStyle.hint.more" },
          { count: DESIGN_STYLE_ASSETS.length },
        )}
      </span>
      <button
        type="button"
        aria-label={intl.formatMessage({ id: "chat.designStyle.hint.dismiss" })}
        title={intl.formatMessage({ id: "chat.designStyle.hint.dismiss" })}
        onClick={onDismiss}
        className="ml-auto flex size-5 shrink-0 items-center justify-center rounded text-foreground-subtlest transition-colors hover:bg-hover hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-input-border-focused"
      >
        <XIcon className="size-3.5" aria-hidden="true" />
      </button>
    </div>
  );
}
