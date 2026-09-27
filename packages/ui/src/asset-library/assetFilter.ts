import type { AssetCategory, AssetManifest } from "./catalog/types.js";

/** 展厅分类过滤取值："all" = 不限分类。 */
export type AssetFilterCategory = AssetCategory | "all";

export interface AssetFilterQuery {
  /** 关键词；空串/纯空白 = 不过滤 */
  query: string;
  category: AssetFilterCategory;
}

/**
 * 展厅搜索+分类过滤（技术设计 §4/§6 纯函数）。
 *
 * 匹配字段：title/titleEn/tags/description/descriptionEn，大小写不敏感；
 * En 字段缺失时回退中文原字段（titleEn ?? title）。不过感知 locale——
 * 两套字段都进 haystack，任何 locale 下命中即算命中。
 */
export function filterAssets(
  manifests: readonly AssetManifest[],
  { query, category }: AssetFilterQuery,
): AssetManifest[] {
  const keyword = query.trim().toLowerCase();
  return manifests.filter((manifest) => {
    if (category !== "all" && manifest.category !== category) {
      return false;
    }
    if (!keyword) {
      return true;
    }
    const haystack = [
      manifest.title,
      manifest.titleEn ?? manifest.title,
      manifest.description,
      manifest.descriptionEn ?? manifest.description,
      ...manifest.tags,
    ];
    return haystack.some((field) => field.toLowerCase().includes(keyword));
  });
}
