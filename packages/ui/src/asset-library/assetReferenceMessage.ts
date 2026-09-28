/**
 * 递活消息组装（技术设计 §10 V2-2 → V3-2 引用 chip 化 → V4.4 纯 chip 化）：
 * 普通货不再把全量代码怼进输入框，图纸先静默落盘到 `<workspace>/.zcode/asset-library/<id>/`，
 * 点"发到会话"后 composer 里**只出现一个素材引用 chip**（像「1 条对话引用」那种块），
 * 不带口令/说明文字——需求描述由用户自己写（V4.4 真机反馈拍板）。
 */
import { buildFileMentionMarkdown } from "../mentions/mentionMarkdown.js";
import type { ComposerMentionPrefill } from "../store/zcodeSessionStoreTypes.js";
import type { AssetManifest } from "./catalog/types.js";

/** 引用 chip 的展示标签：用素材名，不用文件名/路径（用户看的是"这份素材"）。 */
function resolveChipLabel(asset: AssetManifest, locale = "zh-CN"): string {
  return locale === "en-US" ? (asset.titleEn ?? asset.title) : asset.title;
}

/**
 * 引用 chip：给 composer 的结构化 mention——
 * markdown 用**首个图纸文件的相对路径**做 canonical 链接目标（点 chip 能打开文件），
 * label 用素材名。composer 侧 setMention/appendFileMention 会渲染成 chip。
 * 多个文件时 chip 指向入口文件（index.html 优先），其余文件随目录落盘、智能体按需读取。
 */
export function buildAssetReferenceMention(
  asset: AssetManifest,
  writtenRelativePaths: string[],
  locale = "zh-CN",
): ComposerMentionPrefill | undefined {
  const entryIndex = Math.max(
    0,
    asset.files.findIndex((file) => file.name === "index.html"),
  );
  const entryPath = writtenRelativePaths[entryIndex];
  if (!entryPath) {
    return undefined;
  }
  const entryFile = asset.files[entryIndex];
  const label = resolveChipLabel(asset, locale);
  return {
    id: `asset:${asset.id}`,
    category: "files",
    label,
    value: entryPath,
    markdown: buildFileMentionMarkdown(entryPath, label),
    description: entryFile?.name,
    data: {
      kind: "file",
      relativePath: entryPath.replace(/^\.\//, ""),
    },
  };
}

/**
 * chip 化消息（V4.4）：正文**只有 chip 本身**（mention.markdown），零附加文字——
 * composer 用 startsWith 判定 chip 节点，剩余正文为空串即"纯引用块"。
 * 落盘失败等异常路径仍退回带预期文件清单的纯文本，保持不静默。
 */
export function buildAssetReferenceChipMessage(
  asset: AssetManifest,
  writtenRelativePaths: string[],
  locale = "zh-CN",
): { text: string; mention?: ComposerMentionPrefill } {
  const mention = buildAssetReferenceMention(asset, writtenRelativePaths, locale);
  if (!mention) {
    // 落盘返回空（不该发生）：退回纯文本，保持不静默——列出预期文件名供智能体自行查找。
    const expectedFiles = asset.files.map((file) => `.zcode/asset-library/${asset.id}/${file.name}`);
    return {
      text: [
        asset.prompt,
        "",
        `图纸应已写进项目（目录 .zcode/asset-library/${asset.id}/），请先读取以下文件再装进项目：`,
        ...expectedFiles,
      ].join("\n"),
    };
  }
  return { text: mention.markdown, mention };
}
