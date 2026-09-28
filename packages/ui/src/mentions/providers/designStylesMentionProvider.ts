/**
 * 设计风格 @ 候选（V4.5 用户反馈：在输入框 @ 选择设计风格；V4.6 行内色板区分）。
 *
 * 素材目录里的 design-style 货（DESIGN.md 规范文档）是纯本地静态数据，
 * 不需要异步 provider：query 过滤 + 截断全部同步推导。选中项是**文件类 mention**
 * （chip/markdown 与工作区文件引用同构），落盘由 MentionPlugin 的选中分支负责
 * （写 `.zcode/asset-library/<id>/` 后插入，智能体读盘应用风格）。
 *
 * 候选行带**色板**（从 DESIGN.md 提取的主色）——风格名多为英文，色板是
 * 语言无关的快速辨识特征（V4.6 真机反馈：纯文件名行完全无法区分）。
 */
import { useMemo } from "react";
import { ASSET_CATALOG } from "@/asset-library/catalog/index.js";
import { resolveDesignStyleZhName } from "@/asset-library/catalog/designStyleZh.js";
import type { AssetManifest } from "@/asset-library/catalog/types.js";
import { useOptionalPlatform } from "@/hooks/usePlatform.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import { buildFileMentionMarkdown } from "@/mentions/mentionMarkdown.js";
import { getMentionGroupLimitForQuery } from "@/mentions/mentionSearch.js";
import type { MentionCategoryResult, MentionItem } from "@/mentions/mentionTypes.js";

/** 有图纸的设计风格货（chip 指向落盘后的规范文档，没有图纸就没法落盘引用）。 */
export const DESIGN_STYLE_ASSETS: readonly AssetManifest[] = ASSET_CATALOG.filter(
  (asset) => asset.category === "design-style" && asset.files.length > 0,
);

/** chip 上的展示名去掉固定的「· 设计风格」尾巴（分组标题已经说明了语境）。 */
export function resolveDesignStyleLabel(title: string): string {
  return title.replace(/\s*·\s*设计风格$/, "");
}

/** 展示名（V4.6）：中文 locale 用中文名对照表，缺条目回落英文原名。 */
export function resolveDesignStyleDisplayName(
  asset: AssetManifest,
  locale: string,
): string {
  const english = resolveDesignStyleLabel(asset.title);
  return locale === "en-US" ? english : resolveDesignStyleZhName(asset.id, english);
}

/** 规范文档落盘后的确定性入口路径（单文件 DESIGN.md，先于落盘可知）。 */
export function resolveDesignStyleEntryPath(asset: AssetManifest): string {
  return `./.zcode/asset-library/${asset.id}/${asset.files[0]!.name}`;
}

/**
 * 从 DESIGN.md 提取风格主色（按出现顺序去重取前 4 个）。
 * 规范的 Color 章节用 `#RRGGBB` 标 token，顺序即 Primary/Secondary/... 优先级。
 */
export function extractDesignStylePalette(asset: AssetManifest): string[] {
  const palette: string[] = [];
  for (const match of asset.files[0]!.content.matchAll(/#[0-9a-fA-F]{6}\b/g)) {
    const color = match[0].toUpperCase();
    if (!palette.includes(color)) {
      palette.push(color);
    }
    if (palette.length >= 4) {
      break;
    }
  }
  return palette;
}

/** 推荐位风格：认知度高的品牌风优先；id 不在目录里自动跳过。 */
const FEATURED_DESIGN_STYLE_IDS = [
  "design-airbnb",
  "design-notion",
  "design-stripe",
  "design-spotify",
  "design-linear-app",
  "design-vercel",
];

export function getFeaturedDesignStyleAssets(count: number): AssetManifest[] {
  const byId = new Map(DESIGN_STYLE_ASSETS.map((asset) => [asset.id, asset]));
  const featured = FEATURED_DESIGN_STYLE_IDS.map((id) => byId.get(id)).filter(
    (asset): asset is AssetManifest => Boolean(asset),
  );
  if (featured.length >= count) {
    return featured.slice(0, count);
  }
  for (const asset of DESIGN_STYLE_ASSETS) {
    if (featured.length >= count) break;
    if (!featured.includes(asset)) featured.push(asset);
  }
  return featured;
}

function toDesignStyleMentionItem(asset: AssetManifest, locale: string): MentionItem {
  const relativePath = resolveDesignStyleEntryPath(asset).replace(/^\.\//, "");
  const label = resolveDesignStyleDisplayName(asset, locale);
  return {
    id: `asset:${asset.id}`,
    category: "files",
    label,
    description: asset.description,
    value: `./${relativePath}`,
    markdown: buildFileMentionMarkdown(`./${relativePath}`, label),
    keywords: [asset.id, asset.title, resolveDesignStyleZhName(asset.id, ""), ...asset.tags],
    data: {
      kind: "file",
      relativePath,
      designStyle: true,
      assetId: asset.id,
      palette: extractDesignStylePalette(asset),
    },
  };
}

function matchesDesignStyleQuery(asset: AssetManifest, query: string): boolean {
  if (!query) {
    return true;
  }
  const haystack = `${asset.title} ${asset.id} ${asset.tags.join(" ")} ${asset.description}`.toLowerCase();
  return haystack.includes(query);
}

export function useDesignStyleMentionProvider(
  query: string,
  enabled: boolean,
  emptyText: string,
  title: string,
  defaultPreviewLimit?: number,
): MentionCategoryResult {
  // 没有平台通道（写盘不可能成功）的宿主里直接不出候选，避免给出必败选项。
  const canWrite = useOptionalPlatform()?.assetLibraryWriteFiles != null;
  const { locale } = useZCodeIntl();
  const normalizedQuery = query.trim().toLowerCase();
  const limit = getMentionGroupLimitForQuery(normalizedQuery, defaultPreviewLimit) ?? 8;
  const items = useMemo(() => {
    if (!enabled || !canWrite) {
      return [];
    }
    return DESIGN_STYLE_ASSETS.filter((asset) => matchesDesignStyleQuery(asset, normalizedQuery))
      .slice(0, limit)
      .map((asset) => toDesignStyleMentionItem(asset, locale));
  }, [canWrite, enabled, locale, normalizedQuery, limit]);
  return {
    items,
    loading: false,
    error: null,
    emptyText,
    title,
  };
}
