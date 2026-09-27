/**
 * 交互素材库展厅（技术设计 §4）：搜索 + 分类 chips + 卡片墙 + 详情弹层。
 *
 * 卡片墙用纯 CSS 静态缩影（分类色渐变+分类图标），不挂 iframe——否则一屏
 * 六件货就同屏六个沙箱；只有详情弹层挂真 AssetPreviewFrame（现挂现卸）。
 * 缩影素材在本组件内自持，不动 catalog 契约。
 */
import { useMemo, useState, type ComponentType } from "react";
import {
  LayoutGrid,
  MessageSquareQuote,
  Search,
  SearchX,
  ToggleLeft,
  Type,
  Wallpaper,
} from "lucide-react";
import { Button } from "@/components/ui/button.js";
import { Input } from "@/components/ui/input.js";
import type { CreateTaskRequest } from "@/app-shell/types.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import { ASSET_CATALOG } from "./catalog/index.js";
import type { AssetCategory, AssetManifest } from "./catalog/types.js";
import { filterAssets, type AssetFilterCategory } from "./assetFilter.js";
import { AssetDetailDialog } from "./AssetDetailDialog.js";

const CATEGORY_ORDER: readonly AssetCategory[] = [
  "text-animation",
  "background",
  "control",
  "block",
  "prompt",
];

/** 卡片墙静态缩影：每类一个渐变底+图标（纯 CSS，不挂 iframe）。 */
const CATEGORY_THUMBNAIL: Record<
  AssetCategory,
  { icon: ComponentType<{ className?: string }>; gradient: string }
> = {
  "text-animation": { icon: Type, gradient: "from-sky-500/25 to-indigo-500/15" },
  background: { icon: Wallpaper, gradient: "from-violet-500/25 to-fuchsia-500/15" },
  control: { icon: ToggleLeft, gradient: "from-emerald-500/25 to-teal-500/15" },
  block: { icon: LayoutGrid, gradient: "from-amber-500/25 to-orange-500/15" },
  prompt: { icon: MessageSquareQuote, gradient: "from-rose-500/25 to-pink-500/15" },
};

/** locale 展示名：En 字段缺失回退中文原字段（titleEn ?? title）。 */
function resolveDisplayTitle(manifest: AssetManifest, locale: string): string {
  return locale === "en-US" ? (manifest.titleEn ?? manifest.title) : manifest.title;
}

function AssetCard({
  manifest,
  locale,
  openLabel,
  onSelect,
}: {
  manifest: AssetManifest;
  locale: string;
  openLabel: string;
  onSelect: (manifest: AssetManifest) => void;
}) {
  const { intl } = useZCodeIntl();
  const title = resolveDisplayTitle(manifest, locale);
  const description =
    locale === "en-US" ? (manifest.descriptionEn ?? manifest.description) : manifest.description;
  const thumbnail = CATEGORY_THUMBNAIL[manifest.category];
  const ThumbnailIcon = thumbnail.icon;
  const license = manifest.source?.license ?? intl.formatMessage({ id: "assetLibrary.license.selfMade" });
  return (
    <Button
      type="button"
      variant="outline"
      data-testid="asset-library-card"
      data-asset-id={manifest.id}
      aria-label={openLabel}
      className="h-auto w-full shrink flex-col items-stretch gap-0 whitespace-normal rounded-xl p-0 text-left"
      onClick={() => onSelect(manifest)}
    >
      <span
        aria-hidden="true"
        className={cn(
          "flex h-24 w-full items-center justify-center gap-2 rounded-t-xl bg-gradient-to-br",
          thumbnail.gradient,
        )}
      >
        <ThumbnailIcon className="size-7 text-foreground-subtle" />
        <span className="text-ui-xl font-semibold text-foreground-subtle">{title.slice(0, 1)}</span>
      </span>
      <span className="flex w-full flex-1 flex-col gap-1.5 p-3">
        <span className="flex min-w-0 items-center justify-between gap-2">
          <span className="min-w-0 truncate text-ui-base font-semibold text-foreground">{title}</span>
          <span
            className="shrink-0 rounded-full bg-surface-hover px-1.5 py-0.5 text-ui-sm leading-none text-foreground-subtle"
            aria-label={intl.formatMessage(
              { id: "assetLibrary.license.aria" },
              { license },
            )}
          >
            {license}
          </span>
        </span>
        <span className="line-clamp-2 text-ui-sm leading-snug text-foreground-subtle">
          {description}
        </span>
        <span className="mt-auto flex flex-wrap gap-1 pt-1">
          {manifest.tags.map((tag) => (
            <span
              key={tag}
              className="rounded-full border border-border px-1.5 py-0.5 text-ui-sm leading-none text-foreground-subtle"
            >
              {tag}
            </span>
          ))}
        </span>
      </span>
    </Button>
  );
}

/** 递活接线（S4）：workspacePath/identity 供「发到当前会话」，onCreateTask 供「发到新会话」。 */
interface AssetLibrarySectionProps {
  workspacePath: string;
  workspaceIdentity?: string;
  /** 当前 workspace 有活动会话视图（ShellLayout 按 activeTaskId 判定）；无则隐藏「发到当前会话」。 */
  hasActiveChat: boolean;
  onCreateTask?: (request?: CreateTaskRequest) => void;
}

export function AssetLibrarySection({
  workspacePath,
  workspaceIdentity,
  hasActiveChat,
  onCreateTask,
}: AssetLibrarySectionProps) {
  const { intl, locale } = useZCodeIntl();
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<AssetFilterCategory>("all");
  const [detailAsset, setDetailAsset] = useState<AssetManifest | null>(null);
  const visibleAssets = useMemo(
    () => filterAssets(ASSET_CATALOG, { query, category }),
    [query, category],
  );

  return (
    <div className="flex flex-col gap-4">
      <div>
        <h1 className="text-ui-lg font-semibold text-foreground">
          {intl.formatMessage({ id: "assetLibrary.title" })}
        </h1>
        <p className="mt-1 text-ui-sm text-foreground-subtle">
          {intl.formatMessage({ id: "assetLibrary.description" })}
        </p>
      </div>

      <div className="flex flex-col gap-2.5">
        <div className="relative">
          <Search
            className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-foreground-subtlest"
            aria-hidden="true"
          />
          <Input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={intl.formatMessage({ id: "assetLibrary.search.placeholder" })}
            aria-label={intl.formatMessage({ id: "assetLibrary.search.label" })}
            className="pl-8"
            data-testid="asset-library-search"
          />
        </div>
        <div
          className="flex flex-wrap items-center gap-1.5"
          role="group"
          aria-label={intl.formatMessage({ id: "assetLibrary.category.ariaLabel" })}
          data-testid="asset-library-category-filter"
        >
          {(["all", ...CATEGORY_ORDER] as const).map((key) => (
            <button
              key={key}
              type="button"
              aria-pressed={category === key}
              className={cn(
                "rounded-full px-2.5 py-0.5 text-ui-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-input-border-focused",
                category === key
                  ? "bg-selected text-foreground"
                  : "text-foreground-subtle hover:bg-hover hover:text-foreground",
              )}
              onClick={() => setCategory(key)}
            >
              {intl.formatMessage({ id: `assetLibrary.category.${key}` })}
            </button>
          ))}
          <span
            className="ml-auto text-ui-sm text-foreground-subtlest"
            aria-live="polite"
            data-testid="asset-library-count"
          >
            {intl.formatMessage({ id: "assetLibrary.count" }, { count: visibleAssets.length })}
          </span>
        </div>
      </div>

      {visibleAssets.length === 0 ? (
        <div
          className="flex flex-col items-center gap-3 rounded-xl border border-dashed border-border px-6 py-12 text-center"
          data-testid="asset-library-empty"
        >
          <div className="flex size-12 items-center justify-center rounded-full bg-surface-hover">
            <SearchX className="size-6 text-foreground-subtle" aria-hidden="true" />
          </div>
          <p className="text-ui-base font-medium text-foreground">
            {intl.formatMessage({ id: "assetLibrary.empty.title" })}
          </p>
          <p className="text-ui-sm text-foreground-subtle">
            {intl.formatMessage({ id: "assetLibrary.empty.description" })}
          </p>
          <Button
            type="button"
            variant="secondary"
            size="sm"
            className="rounded-full"
            data-testid="asset-library-clear-search"
            onClick={() => {
              setQuery("");
              setCategory("all");
            }}
          >
            {intl.formatMessage({ id: "assetLibrary.empty.clearSearch" })}
          </Button>
        </div>
      ) : (
        <div
          className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3"
          data-testid="asset-library-grid"
        >
          {visibleAssets.map((manifest) => (
            <AssetCard
              key={manifest.id}
              manifest={manifest}
              locale={locale}
              openLabel={intl.formatMessage(
                { id: "assetLibrary.card.openLabel" },
                { title: resolveDisplayTitle(manifest, locale) },
              )}
              onSelect={setDetailAsset}
            />
          ))}
        </div>
      )}

      <AssetDetailDialog
        asset={detailAsset}
        onClose={() => setDetailAsset(null)}
        workspacePath={workspacePath}
        workspaceIdentity={workspaceIdentity}
        hasActiveChat={hasActiveChat}
        onCreateTask={onCreateTask}
      />
    </div>
  );
}
