/**
 * 素材/设计风格引用 chip 的悬停预览（V4.6 用户反馈：鼠标放在引用块上可以预览效果）。
 *
 * 宿主是 Lexical 装饰层（非 React 树内），所以用单例 portal + createRoot 承载预览卡：
 * chip mouseenter 延时弹出、mouseleave 延时收起（移入卡片会取消收起），预览用
 * AssetPreviewFrame 的缩放舞台，与素材库看到的效果一致。监听随 chip DOM 移除自动失效
 * （WeakMap 记账 AbortController，重挂装饰先 abort 旧的，防重复监听）。
 */
import { useEffect, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { PaletteIcon } from "lucide-react";
import { ASSET_CATALOG } from "@/asset-library/catalog/index.js";
import { loadAssetBody } from "@/asset-library/catalog/assetBodies.js";
import { resolveDesignStyleZhName } from "@/asset-library/catalog/designStyleZh.js";
import { AssetPreviewFrame } from "@/asset-library/AssetPreviewFrame.js";

const SHOW_DELAY_MS = 350;
const HIDE_DELAY_MS = 220;
const CARD_WIDTH = 272;

const attachedControllers = new WeakMap<HTMLElement, AbortController>();
let portalHost: HTMLDivElement | null = null;
let portalRoot: Root | null = null;
let showTimer: ReturnType<typeof setTimeout> | undefined;
let hideTimer: ReturnType<typeof setTimeout> | undefined;
let anchorRect: DOMRect | null = null;

function clearTimers() {
  if (showTimer !== undefined) clearTimeout(showTimer);
  if (hideTimer !== undefined) clearTimeout(hideTimer);
  showTimer = undefined;
  hideTimer = undefined;
}

function ensurePortal(): { host: HTMLDivElement; root: Root } {
  if (!portalHost) {
    portalHost = document.createElement("div");
    portalHost.setAttribute("data-asset-mention-preview-portal", "true");
    document.body.append(portalHost);
    portalRoot = createRoot(portalHost);
  }
  return { host: portalHost, root: portalRoot! };
}

function scheduleHide() {
  clearTimers();
  hideTimer = setTimeout(hideNow, HIDE_DELAY_MS);
}

function hideNow() {
  clearTimers();
  anchorRect = null;
  portalRoot?.render(null);
}

function AssetPreviewPopoverCard({ assetId }: { assetId: string }) {
  const asset = ASSET_CATALOG.find((candidate) => candidate.id === assetId);
  // 瘦身拆分（2026-10-05）：lazy 货（bodyFrom 在场）预览 HTML 按需拉——悬停
  // 弹出才触发，live goods 不受影响。
  const [lazyPreviewHtml, setLazyPreviewHtml] = useState<string | null>(null);
  useEffect(() => {
    if (!asset?.bodyFrom) return;
    let alive = true;
    loadAssetBody(asset).then(
      (body) => {
        if (alive) setLazyPreviewHtml(body.previewHtml);
      },
      (error: unknown) => {
        console.warn("[asset-library] 悬停预览正文加载失败:", asset.id, error);
      },
    );
    return () => {
      alive = false;
    };
  }, [asset]);
  const previewHtml = asset?.bodyFrom ? lazyPreviewHtml : (asset?.previewHtml ?? null);
  if (!asset || !previewHtml || !anchorRect) {
    return null;
  }
  const isDesignStyle = asset.category === "design-style";
  const title = isDesignStyle ? resolveDesignStyleZhName(asset.id, asset.title) : asset.title;
  const viewportWidth = document.documentElement.clientWidth;
  const left = Math.max(
    8,
    Math.min(anchorRect.left, viewportWidth - CARD_WIDTH - 8),
  );
  // 卡高 ≈ 标题行 + 8:5 预览 + 内边距；空间不足时翻到 chip 下方。
  const estimatedCardHeight = CARD_WIDTH * (500 / 800) + 48;
  const top =
    anchorRect.top - estimatedCardHeight - 8 >= 8
      ? anchorRect.top - estimatedCardHeight - 8
      : anchorRect.bottom + 8;
  return (
    <div
      style={{ position: "fixed", top, left, width: CARD_WIDTH, zIndex: 60 }}
      className="rounded-xl border border-border bg-card p-2 shadow-xl"
      onMouseEnter={() => {
        if (hideTimer !== undefined) clearTimeout(hideTimer);
        hideTimer = undefined;
      }}
      onMouseLeave={scheduleHide}
    >
      <div className="mb-1 flex min-w-0 items-center gap-1.5 px-0.5 text-ui-xs font-medium text-foreground">
        <PaletteIcon className="size-3 shrink-0 text-foreground-subtle" aria-hidden="true" />
        <span className="truncate">{title}</span>
        <span className="ml-auto shrink-0 text-foreground-subtlest">素材预览</span>
      </div>
      <div
        className="overflow-hidden rounded-lg border border-border"
        style={{ aspectRatio: "800 / 500" }}
      >
        <AssetPreviewFrame previewHtml={previewHtml} title={title} />
      </div>
    </div>
  );
}

function showAssetMentionPreview(assetId: string, anchor: HTMLElement) {
  clearTimers();
  showTimer = setTimeout(() => {
    anchorRect = anchor.getBoundingClientRect();
    if (anchorRect.width === 0) return;
    const { root } = ensurePortal();
    root.render(<AssetPreviewPopoverCard assetId={assetId} />);
  }, SHOW_DELAY_MS);
}

/**
 * 给素材/设计风格引用 chip 挂悬停预览。装饰可能随编辑器重算重复执行：
 * 同一 dom 的旧监听先 abort（WeakMap 记账），chip DOM 移除时监听随之失效。
 */
export function attachAssetMentionPreview(dom: HTMLElement, assetId: string): void {
  attachedControllers.get(dom)?.abort();
  const controller = new AbortController();
  attachedControllers.set(dom, controller);
  const options = { signal: controller.signal };
  dom.addEventListener(
    "mouseenter",
    () => showAssetMentionPreview(assetId, dom),
    options,
  );
  dom.addEventListener("mouseleave", scheduleHide, options);
  dom.addEventListener("remove", hideNow, options);
}
