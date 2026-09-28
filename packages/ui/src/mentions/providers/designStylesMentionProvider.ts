/**
 * 设计风格 @ 候选（V4.5 用户反馈：设计风格应该在输入框里 @ 选择）。
 *
 * 素材目录里的 design-style 货（DESIGN.md 规范文档）是纯本地静态数据，
 * 不需要异步 provider：query 过滤 + 截断全部同步推导。选中项是**文件类 mention**
 * （chip/markdown 与工作区文件引用同构），落盘由 MentionPlugin 的选中分支负责
 * （写 `.zcode/asset-library/<id>/` 后插入，智能体读盘应用风格）。
 */
import { useMemo } from "react";
import { ASSET_CATALOG } from "@/asset-library/catalog/index.js";
import { useOptionalPlatform } from "@/hooks/usePlatform.js";
import { buildFileMentionMarkdown } from "@/mentions/mentionMarkdown.js";
import {
  getMentionGroupLimitForQuery,
} from "@/mentions/mentionSearch.js";
import type { MentionCategoryResult, MentionItem } from "@/mentions/mentionTypes.js";

/** 有图纸的设计风格货（chip 指向落盘后的规范文档，没有图纸就没法落盘引用）。 */
const DESIGN_STYLE_ASSETS = ASSET_CATALOG.filter(
  (asset) => asset.category === "design-style" && asset.files.length > 0,
);

/** chip 上的展示名去掉固定的「· 设计风格」尾巴（分组标题已经说明了语境）。 */
function resolveDesignStyleLabel(title: string): string {
  return title.replace(/\s*·\s*设计风格$/, "");
}

function toDesignStyleMentionItem(asset: (typeof DESIGN_STYLE_ASSETS)[number]): MentionItem {
  const relativePath = `.zcode/asset-library/${asset.id}/${asset.files[0]!.name}`;
  const label = resolveDesignStyleLabel(asset.title);
  return {
    id: `asset:${asset.id}`,
    category: "files",
    label,
    description: asset.description,
    value: `./${relativePath}`,
    markdown: buildFileMentionMarkdown(`./${relativePath}`, label),
    keywords: [asset.id, asset.title, ...asset.tags],
    data: {
      kind: "file",
      relativePath,
    },
  };
}

function matchesDesignStyleQuery(asset: (typeof DESIGN_STYLE_ASSETS)[number], query: string): boolean {
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
  const normalizedQuery = query.trim().toLowerCase();
  const limit =
    getMentionGroupLimitForQuery(normalizedQuery, defaultPreviewLimit) ?? 8;
  const items = useMemo(() => {
    if (!enabled || !canWrite) {
      return [];
    }
    return DESIGN_STYLE_ASSETS.filter((asset) => matchesDesignStyleQuery(asset, normalizedQuery))
      .slice(0, limit)
      .map(toDesignStyleMentionItem);
  }, [canWrite, enabled, normalizedQuery, limit]);
  return {
    items,
    loading: false,
    error: null,
    emptyText,
    title,
  };
}
