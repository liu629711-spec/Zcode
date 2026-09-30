import type { AssetCategory, AssetManifest } from "../types.js";

/**
 * UIverse（uiverse-io/galaxy）自动收录货的运行时外壳。
 *
 * 数据文件由 scripts/import-uiverse-galaxy.mjs 生成（3,800+ 件，MIT，逐字收录
 * 上游自包含 HTML），本模块把它们包成 AssetManifest：图纸 = 署名头 + 上游原文；
 * previewHtml 惰性生成（首读才算、按 id 缓存）——3,800 份预览文档不能在模块
 * 加载时全量拼出来，内存和首屏都遭不住。图纸保持上游逐字（含上游署名注释），
 * 外链清洗只发生在预览副本上（沙箱禁一切外链，见 catalogCheck.EXTERNAL_SRC）。
 */

export interface UiverseAutoMeta {
  id: string;
  title: string;
  titleEn: string;
  description: string;
  category: AssetCategory;
  tags: string[];
  prompt: string;
  /** 上游作者（UIverse.io 社区用户名） */
  author: string;
  /** 上游相对路径，如 Buttons/1osm_black-chicken-65.html */
  origin: string;
  /** 上游 HTML 片段原文（逐字，未清洗） */
  html: string;
}

/** 预览里命中外链图片一律换等比灰底占位；图纸不动。 */
const EXTERNAL_SRC = /src\s*=\s*(["'])https?:\/\/[^"']*\1/gi;
const EXTERNAL_CSS_URL = /url\(\s*(["']?)https?:\/\/[^)"']*\1\s*\)/gi;
const PLACEHOLDER =
  "data:image/svg+xml,%3Csvg%20xmlns='http://www.w3.org/2000/svg'%20width='160'%20height='90'%3E%3Crect%20width='100%25'%20height='100%25'%20fill='%23222b3a'/%3E%3C/svg%3E";

function sanitizeForSandbox(html: string): string {
  return html
    .replace(EXTERNAL_SRC, `src="${PLACEHOLDER}"`)
    .replace(EXTERNAL_CSS_URL, `url("${PLACEHOLDER}")`);
}

function buildPreview(meta: UiverseAutoMeta, blueprint: string): string {
  const credit = `UIverse · ${meta.author} · MIT`;
  return [
    "<!doctype html>",
    '<html lang="zh-CN">',
    "<head>",
    '<meta charset="utf-8">',
    '<meta name="viewport" content="width=device-width, initial-scale=1">',
    `<title>${meta.title}</title>`,
    "<style>",
    "* { box-sizing: border-box; margin: 0; }",
    "html, body { height: 100%; }",
    "body { display: grid; place-items: center; background: radial-gradient(120% 130% at 50% 0%, #10151f 0%, #0b0e15 60%, #080a10 100%); font-family: system-ui, 'PingFang SC', 'Microsoft YaHei', sans-serif; color: #cdd8ea; overflow: hidden; }",
    ".uv-stage { display: grid; place-items: center; gap: 14px; padding: 16px; max-width: 100%; }",
    ".uv-credit { font-size: 11px; letter-spacing: 0.06em; color: #5b6b8c; user-select: none; }",
    "</style>",
    "</head>",
    "<body>",
    '<div class="uv-stage">',
    sanitizeForSandbox(blueprint),
    `<p class="uv-credit">${credit}</p>`,
    "</div>",
    "</body>",
    "</html>",
  ].join("\n");
}

const previewCache = new Map<string, string>();

export function uiverseAutoAsset(meta: UiverseAutoMeta): AssetManifest {
  const blueprint = [
    "<!--",
    `  ${meta.title} · ${meta.id}.html`,
    "",
    `  来源：github.com/uiverse-io/galaxy · ${meta.origin}`,
    `  原址：https://github.com/uiverse-io/galaxy/blob/main/${meta.origin}`,
    `  作者：${meta.author}（UIverse.io 社区）`,
    "  许可：MIT（仓库 LICENSE）；按 UIverse.io 惯例署名原作者与 UIverse.io。",
    "",
    "  以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。",
    "-->",
    meta.html,
  ].join("\n");
  const manifest: AssetManifest = {
    id: meta.id,
    title: meta.title,
    titleEn: meta.titleEn,
    description: meta.description,
    category: meta.category,
    tags: meta.tags,
    get previewHtml() {
      let html = previewCache.get(meta.id);
      if (html === undefined) {
        html = buildPreview(meta, blueprint);
        previewCache.set(meta.id, html);
      }
      return html;
    },
    files: [{ name: `${meta.id}.html`, language: "html", content: blueprint }],
    prompt: meta.prompt,
    source: {
      site: "UIverse",
      url: `https://github.com/uiverse-io/galaxy/blob/main/${meta.origin}`,
      license: `MIT（github.com/uiverse-io/galaxy）· 作者 ${meta.author} · UIverse.io`,
    },
  };
  return manifest;
}
