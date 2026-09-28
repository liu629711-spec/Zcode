import type { AssetCategory, AssetManifest } from "./types.js";

/**
 * 货架校验器（技术设计 §6 可运行检查）。
 * 构建期/测试期拦坏货：id 唯一且 kebab-case、必填字段非空、非 prompt 类 files
 * 必须非空、previewHtml 无外链、无 localStorage；沙箱常量恒等由 assertSandboxSafe 钉。
 */

/**
 * iframe sandbox 的唯一合法值（定义在本模块、由 AssetPreviewFrame 再导出：
 * 常量与守门活在同一处，测试运行器跑纯 .ts 链也稳）。
 */
export const ASSET_SANDBOX = "allow-scripts";

const KEBAB_CASE = /^[a-z0-9]+(-[a-z0-9]+)*$/;

const ASSET_CATEGORIES: readonly AssetCategory[] = [
  "text-animation",
  "background",
  "control",
  "block",
  "prompt",
  "design-style",
];

/** 外链检测：src 指向 http(s)（双引号/单引号都算）。相对引用与 data: 不拦。 */
const EXTERNAL_SRC = /src\s*=\s*["']https?:/i;

/**
 * 对整份目录逐件体检，返回全部问题（人可读，含 id 定位）；空数组 = 全绿。
 */
export function validateCatalog(manifests: readonly AssetManifest[]): string[] {
  const errors: string[] = [];
  const seenIds = new Set<string>();
  for (const manifest of manifests) {
    const label = manifest.id.trim() === "" ? "<无 id>" : manifest.id;
    if (!KEBAB_CASE.test(manifest.id)) {
      errors.push(`${label}: id 必须是 kebab-case（小写字母/数字/连字符）`);
    }
    if (seenIds.has(manifest.id)) {
      errors.push(`${label}: id 全局重复`);
    }
    seenIds.add(manifest.id);

    for (const field of ["title", "description", "previewHtml", "prompt"] as const) {
      if (manifest[field].trim() === "") {
        errors.push(`${label}: 必填字段 ${field} 为空`);
      }
    }
    if (!ASSET_CATEGORIES.includes(manifest.category)) {
      errors.push(`${label}: category 非法（${manifest.category}）`);
    }
    if (manifest.tags.length === 0) {
      errors.push(`${label}: tags 不能为空`);
    }

    if (manifest.category !== "prompt" && manifest.files.length === 0) {
      errors.push(`${label}: 非 prompt 类货 files 不能为空（图纸是递活的实体）`);
    }
    // V2-4 台账：标了出处的货，site/url/license 三字段必须全填，缺一即坏货
    if (manifest.source) {
      for (const field of ["site", "url", "license"] as const) {
        if (manifest.source[field].trim() === "") {
          errors.push(`${label}: source.${field} 为空（台账三字段必填）`);
        }
      }
    }
    if (EXTERNAL_SRC.test(manifest.previewHtml)) {
      errors.push(`${label}: previewHtml 含外链（src="http…"），沙箱内禁一切外链`);
    }
    if (manifest.previewHtml.includes("localStorage")) {
      errors.push(`${label}: previewHtml 含 localStorage（沙箱无源环境会抛）`);
    }
  }
  return errors;
}

/**
 * 沙箱铁律守门：值必须恒等于 ASSET_SANDBOX（"allow-scripts"）。
 * 任何多出来的 token（allow-same-origin / allow-popups / allow-top-navigation）
 * 都不等于它，一律 throw——不加白名单，不给例外。
 */
export function assertSandboxSafe(value: string): void {
  if (value !== ASSET_SANDBOX) {
    throw new Error(
      `沙箱铁律被破坏：sandbox 必须恒等于 "${ASSET_SANDBOX}"，实际是 "${value}"`,
    );
  }
}
