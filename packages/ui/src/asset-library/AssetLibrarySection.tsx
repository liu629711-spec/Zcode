/**
 * 交互素材库展厅（技术设计 §10 V2-1，beautifului.dev 式画廊）：
 * 搜索 + 分类 chips + 计数 + 单列大演示卡流（max-w-4xl 居中、间距舒展），
 * 原「点卡开详情弹层」入口退役——演示区真 iframe 直接内嵌进卡片。
 *
 * 每张卡的载体见 AssetDemoCard.tsx（序号+悬浮操作+内联代码面板+递活）；
 * iframe 挂载数由 useInView 红线约束：任何时刻 = 视口内卡片数（±200px）。
 * 搜索无结果空态沿用；本组件不动 catalog 契约与 assetTryPrompt 语义。
 */
import { useMemo, useState } from "react";
import { SearchIcon, SearchXIcon } from "lucide-react";
import { Button } from "@/components/ui/button.js";
import { Input } from "@/components/ui/input.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { cn } from "@/components/lib/utils.js";
import { ASSET_CATALOG } from "./catalog/index.js";
import type { AssetCategory } from "./catalog/types.js";
import { filterAssets, type AssetFilterCategory } from "./assetFilter.js";
import { AssetDemoCard, type AssetCardActions } from "./AssetDemoCard.js";

const CATEGORY_ORDER: readonly AssetCategory[] = [
  "text-animation",
  "background",
  "control",
  "block",
  "prompt",
];

/** 递活接线（S4）：workspacePath/identity 供「发到当前会话」，onCreateTask 供「发到新会话」。 */
type AssetLibrarySectionProps = AssetCardActions;

export function AssetLibrarySection({
  workspacePath,
  workspaceIdentity,
  hasActiveChat,
  readOnly = false,
  onOpenChat,
  onCreateTask,
}: AssetLibrarySectionProps) {
  const { intl, locale } = useZCodeIntl();
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<AssetFilterCategory>("all");
  const visibleAssets = useMemo(
    () => filterAssets(ASSET_CATALOG, { query, category }),
    [query, category],
  );

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
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
          <SearchIcon
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
            <SearchXIcon className="size-6 text-foreground-subtle" aria-hidden="true" />
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
        <div className="flex flex-col gap-6" data-testid="asset-library-list">
          {visibleAssets.map((manifest, index) => (
            <AssetDemoCard
              key={manifest.id}
              manifest={manifest}
              index={index}
              locale={locale}
              workspacePath={workspacePath}
              workspaceIdentity={workspaceIdentity}
              hasActiveChat={hasActiveChat}
              readOnly={readOnly}
              onOpenChat={onOpenChat}
              onCreateTask={onCreateTask}
            />
          ))}
        </div>
      )}
    </div>
  );
}
