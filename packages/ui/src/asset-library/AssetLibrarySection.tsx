/**
 * 交互素材库展厅（技术设计 §10 V2-1，beautifului.dev 式画廊；V3-1 布局改版）：
 * 搜索 + 分类 chips + 计数 + 2 列网格演示卡流（2xl 屏 3 列、max-w-7xl 居中），
 * 原「点卡开详情弹层」入口退役——演示区真 iframe 直接内嵌进卡片。
 *
 * 工具栏（标题+搜索+chips+计数）sticky 吸附滚动容器顶（毛玻璃底），下滑全程可达；
 * 滚动超过 ~600px 右下角出现「回到顶部」（sticky bottom 悬浮条，平滑滚回）。
 * 滚动容器是 shell 层的 overflow-y-auto 主面板（WorkspaceShellLayout），
 * 这里向上找最近滚动祖先拿引用（监听 + 回顶），不在 shell 上加钩子。
 *
 * 每张卡的载体见 AssetDemoCard.tsx（序号+悬浮操作+内联代码面板+递活）；
 * iframe 首次进视口挂载后永久保留（出视口 display:none），见 AssetDemoCard。
 * 搜索无结果空态沿用；本组件不动 catalog 契约与 assetTryPrompt 语义。
 */
import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowUpIcon, SearchIcon, SearchXIcon } from "lucide-react";
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
  "design-style",
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

  // 回到顶部（V3-1）：滚动容器在 shell 层，向上找最近滚动祖先；
  // 找不到（jsdom / 其他宿主）就静默不启用，按钮不出现。
  const rootRef = useRef<HTMLDivElement | null>(null);
  const scrollerRef = useRef<HTMLElement | null>(null);
  const [showBackToTop, setShowBackToTop] = useState(false);
  useEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    let cursor: HTMLElement | null = root.parentElement;
    while (cursor) {
      const overflowY = window.getComputedStyle(cursor).overflowY;
      if (overflowY === "auto" || overflowY === "scroll") break;
      cursor = cursor.parentElement;
    }
    if (!cursor) return;
    const scroller = cursor;
    scrollerRef.current = scroller;
    const onScroll = () => setShowBackToTop(scroller.scrollTop > 600);
    onScroll();
    scroller.addEventListener("scroll", onScroll, { passive: true });
    return () => scroller.removeEventListener("scroll", onScroll);
  }, []);
  const scrollToTop = () => {
    scrollerRef.current?.scrollTo({ top: 0, behavior: "smooth" });
  };

  return (
    <div ref={rootRef} className="mx-auto flex w-full max-w-7xl flex-col gap-5">
      {/* sticky 工具栏：负 margin 抵消 shell 容器的 px/py，毛玻璃条全宽贴住滚动容器顶。 */}
      <div className="sticky top-0 z-10 -mx-4 -mt-4 bg-background/70 px-4 pt-4 pb-3 backdrop-blur-md md:-mx-6 md:-mt-6 md:px-6 md:pt-6">
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
        <div
          className="grid grid-cols-1 gap-4 lg:grid-cols-2 2xl:grid-cols-3"
          data-testid="asset-library-list"
        >
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

      {/* 回到顶部（sticky bottom 悬浮条）：滚过 600px 才出现，贴滚动容器右下角。 */}
      {showBackToTop ? (
        <div className="pointer-events-none sticky bottom-4 z-10 flex justify-end">
          <Button
            type="button"
            variant="secondary"
            size="sm"
            className="pointer-events-auto rounded-full shadow-lg"
            data-testid="asset-library-back-to-top"
            onClick={scrollToTop}
          >
            <ArrowUpIcon className="size-3.5" aria-hidden="true" data-icon="inline-start" />
            {intl.formatMessage({ id: "assetLibrary.backToTop" })}
          </Button>
        </div>
      ) : null}
    </div>
  );
}
