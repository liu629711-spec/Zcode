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

/**
 * 外链检测（audit 2026-10-01 加宽）：src/href/srcset/poster、CSS url()、@import
 * 里出现 http(s) 或协议相对 // 都算——src 之外的形态同样会发起请求或跳转。
 * xmlns 命名空间是标识符不是请求，不在词面上，天然不误伤。
 *
 * 复验加宽（2026-10-01 队列⑤）：当时记录的盲区一并关掉——meta refresh（外链
 * 跳转的第三条路）、form action 外链（提交即出站）、javascript: 伪协议 URL。
 * 三条都由 asset-library-reverify.mts 的全模式扫描背书：现行 4,076 件零命中，
 * 收进体检是防未来新增，不是修存量。
 */
const EXTERNAL_REF =
  /(?:\b(?:src|href|srcset|poster)\s*=\s*["']?(?:https?:)?\/\/)|(?:@import\s+(?:url\s*\(\s*)?["']?(?:https?:)?\/\/)|(?:url\(\s*["']?(?:https?:)?\/\/)/i;

/** 外链跳转的其它通道：meta refresh / form action 外链 / javascript: 伪协议。 */
const REDIRECT_AND_PROTOCOL_REF =
  /(?:<meta[^>]*http-equiv\s*=\s*["']?refresh)|(?:<form[^>]*\baction\s*=\s*["']?(?:https?:)?\/\/)|(?:\b(?:href|src|action)\s*=\s*["']?\s*javascript:)/i;

/**
 * 交付原文的**资源类**外链计数（复验拍板 2026-10-01）：复制图纸时的那一行提示用。
 * 台账口径是「图纸保持上游逐字，清洗只在预览副本」，所以不清洗、只数数——
 * 只数真正会发起外部**加载**的形态（src/srcset/poster、@import/url()、link href；
 * 含协议相对 //cdn…，与 EXTERNAL_REF 同宽，评审 B3）。署名的 <a href> 是超链接
 * 不是资源加载，离线渲染照常显示，不计——不然提示会说「离线看不到」看得见的
 * 东西（评审 B2）。xmlns 命名空间是标识符不是请求，词面上天然不命中。
 */
export function countDeliverableExternalRefs(content: string): number {
  return (
    content.match(
      /(?:\b(?:src|srcset|poster)\s*=\s*["']?(?:https?:)?\/\/)|(?:@import\s+(?:url\s*\(\s*)?["']?(?:https?:)?\/\/)|(?:url\(\s*["']?(?:https?:)?\/\/)|(?:<link[^>]+\bhref\s*=\s*["']?(?:https?:)?\/\/)/gi,
    )?.length ?? 0
  );
}

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

    // 瘦身拆分（2026-10-05）：bodyFrom 在场的货有两种形态——meta（正文未合并，
    // previewHtml 恒空串，允许）与 merged（loadAssetBody 之后真身已合并，非空）。
    // 非空要求只对"应该就地"的货（!lazyBody）提；**沙箱扫描对一切非空正文都做**
    // （meta 空串扫过等于没扫，merged 真身必须被扫到——校验语义不因拆分松掉）。
    const lazyBody = manifest.bodyFrom !== undefined;
    for (const field of ["title", "description", "prompt"] as const) {
      if (manifest[field].trim() === "") {
        errors.push(`${label}: 必填字段 ${field} 为空`);
      }
    }
    if (!lazyBody && manifest.previewHtml.trim() === "") {
      errors.push(`${label}: 必填字段 previewHtml 为空`);
    }
    if (manifest.previewHtml.trim() !== "") {
      if (EXTERNAL_REF.test(manifest.previewHtml)) {
        errors.push(`${label}: previewHtml 含外链（src="http…"），沙箱内禁一切外链`);
      }
      if (REDIRECT_AND_PROTOCOL_REF.test(manifest.previewHtml)) {
        errors.push(
          `${label}: previewHtml 含跳转/伪协议通道（meta refresh / form action 外链 / javascript:），沙箱内禁一切出站`,
        );
      }
      if (manifest.previewHtml.includes("localStorage")) {
        errors.push(`${label}: previewHtml 含 localStorage（沙箱无源环境会抛）`);
      }
    }
    if (!ASSET_CATEGORIES.includes(manifest.category)) {
      errors.push(`${label}: category 非法（${manifest.category}）`);
    }
    if (manifest.tags.length === 0) {
      errors.push(`${label}: tags 不能为空`);
    }

    const filesCount = manifest.filesCount ?? manifest.files.length;
    if (manifest.category !== "prompt" && filesCount === 0) {
      // 惰性 meta 货（bodyFrom 在场）真身下放，files 字段恒空、计数在 filesCount——
      // 报错文案统一沿用 files 口径，测试按它对号。
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
