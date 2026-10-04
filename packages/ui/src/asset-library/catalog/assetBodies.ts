/**
 * 素材正文按需解析（地基清理·展厅瘦身 2026-10-05）。
 *
 * bodyFrom 在场的货（galaxy 大袋 3,793 件 + 3 件静态 react 货）的
 * previewHtml/files 真身不在 meta 里，按需从独立 chunk 拉：
 *  - "galaxy:<源分类>" → generated/galaxy-bodies/<源分类>.ts（~0.3-8MB/块，
 *    进门不拉；卡片滚进缓冲带/开代码面板/递活才拉对应分类）；
 *  - "preview-html" → preview-html.ts（3.96MB 惰性 map，此前静态吸进
 *    ASSET_CATALOG 的每个消费方）。
 * 解析结果按 id 模块级缓存（Promise 级，并发去重）；失败清缓存可重试。
 * meta + body 合并成完整 AssetManifest 的动作在消费方（AssetDemoCard 等）完成。
 */
import type { AssetFile, AssetManifest } from "./types.js";

export interface AssetBody {
  previewHtml: string;
  files: AssetFile[];
}

/** bodyFrom 真身在独立 chunk 的 galaxy 源分类 → 该分类 body chunk 的装载器。 */
const GALAXY_BODY_CHUNKS: Record<string, () => Promise<Record<string, AssetBody>>> = {
  buttons: () => import("./generated/galaxy-bodies/buttons.js").then((m) => m.GALAXY_BODIES),
  cards: () => import("./generated/galaxy-bodies/cards.js").then((m) => m.GALAXY_BODIES),
  checkbox: () => import("./generated/galaxy-bodies/checkbox.js").then((m) => m.GALAXY_BODIES),
  forms: () => import("./generated/galaxy-bodies/forms.js").then((m) => m.GALAXY_BODIES),
  inputs: () => import("./generated/galaxy-bodies/inputs.js").then((m) => m.GALAXY_BODIES),
  loaders: () => import("./generated/galaxy-bodies/loaders.js").then((m) => m.GALAXY_BODIES),
  notifications: () =>
    import("./generated/galaxy-bodies/notifications.js").then((m) => m.GALAXY_BODIES),
  patterns: () => import("./generated/galaxy-bodies/patterns.js").then((m) => m.GALAXY_BODIES),
  "radio-button": () =>
    import("./generated/galaxy-bodies/radio-button.js").then((m) => m.GALAXY_BODIES),
  "toggle-switch": () =>
    import("./generated/galaxy-bodies/toggle-switch.js").then((m) => m.GALAXY_BODIES),
  tooltips: () => import("./generated/galaxy-bodies/tooltips.js").then((m) => m.GALAXY_BODIES),
};

function inlineBody(manifest: AssetManifest): AssetBody | null {
  if (manifest.bodyFrom !== undefined) return null;
  return { previewHtml: manifest.previewHtml, files: manifest.files };
}

const bodyCache = new Map<string, Promise<AssetBody>>();

/** 按 manifest 的 bodyFrom 把正文真身解析出来（幂等；失败可重试）。 */
export function loadAssetBody(manifest: AssetManifest): Promise<AssetBody> {
  const inline = inlineBody(manifest);
  if (inline) return Promise.resolve(inline);
  const cached = bodyCache.get(manifest.id);
  if (cached) return cached;
  const pending =
    manifest.bodyFrom === "preview-html"
      ? import("./preview-html.js").then(
          (module) =>
            ({
              previewHtml: module.REACT_PREVIEW_HTML[manifest.id] ?? "",
              files: manifest.files,
            }) satisfies AssetBody,
        )
      : loadGalaxyBody(manifest);
  const guarded = pending.catch((error: unknown) => {
    bodyCache.delete(manifest.id);
    throw error;
  });
  bodyCache.set(manifest.id, guarded);
  return guarded;
}

async function loadGalaxyBody(manifest: AssetManifest): Promise<AssetBody> {
  const bodyFrom = manifest.bodyFrom ?? "";
  const chunkName = bodyFrom.startsWith("galaxy:") ? bodyFrom.slice("galaxy:".length) : "";
  const loader = GALAXY_BODY_CHUNKS[chunkName];
  if (!loader) {
    throw new Error(`asset body chunk 不存在：${bodyFrom}（id=${manifest.id}）`);
  }
  const bodies = await loader();
  const body = bodies[manifest.id];
  if (!body) {
    throw new Error(`asset body 缺失：chunk=${chunkName} id=${manifest.id}`);
  }
  return body;
}

/** meta（正文下放）+ body 合并成完整 manifest；body 未到时保持 meta 形状。 */
export function mergeAssetBody(manifest: AssetManifest, body: AssetBody | null): AssetManifest {
  if (!body) return manifest;
  return { ...manifest, previewHtml: body.previewHtml, files: body.files };
}
