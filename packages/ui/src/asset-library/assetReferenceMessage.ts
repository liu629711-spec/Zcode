/**
 * 递活消息组装（技术设计 §10 V2-2 + V3-2 引用 chip 化）：
 * 普通货不再把全量代码怼进输入框，图纸先静默落盘到 `<workspace>/.zcode/asset-library/<id>/`，
 * 消息 = 口令一句 + **结构化文件引用 chip**（不是代码原文），智能体自己读文件装进项目。
 *
 * V3-2 真机反馈：用户要把素材引用做成插件商店那种"一个 chip"的体验（像「1 条对话引用」），
 * 所以这里除了 canonical 文本，还产出 ComposerMentionPrefill——composer 会把它渲染成
 * 可点击的 chip 节点，而不是把路径/代码铺满输入框。
 */
import { buildFileMentionMarkdown } from "../mentions/mentionMarkdown.js";
import type { ComposerMentionPrefill } from "../store/zcodeSessionStoreTypes.js";
import type { AssetManifest } from "./catalog/types.js";

/** 引用 chip 的展示标签：用素材名，不用文件名/路径（用户看的是"这份素材"）。 */
function resolveChipLabel(asset: AssetManifest, locale = "zh-CN"): string {
  return locale === "en-US" ? (asset.titleEn ?? asset.title) : asset.title;
}

/**
 * canonical 消息 = 口令 + 落盘指引 + 逐文件相对路径引用。
 * 这是"真正发给智能体"的文本（含文件路径信息，供它读盘），不是给用户看的代码。
 */
export function buildAssetReferenceMessage(asset: AssetManifest, writtenRelativePaths: string[]): string {
  return [
    asset.prompt,
    "",
    `图纸已写进项目（${writtenRelativePaths.length} 个文件），直接读这些文件按口令装进项目，不要在回复里重复贴代码：`,
    // writtenPaths 与 asset.files 同序（main 侧按 files 顺序返回）
    ...writtenRelativePaths.map((writtenPath, index) =>
      buildFileMentionMarkdown(writtenPath, asset.files[index]!.name),
    ),
  ].join("\n");
}

/**
 * 引用 chip（V3-2）：给 composer 的结构化 mention——
 * markdown 用**首个图纸文件的相对路径**做 canonical 链接目标（点 chip 能打开文件），
 * label 用素材名。composer 侧 setMention/appendFileMention 会渲染成 chip。
 * 多个文件时 chip 指向入口文件（index.html 优先），其余文件路径留在正文里。
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
 * 引用 chip 化消息：正文=口令 + 落盘指引（**不含文件路径列表**，路径由 chip 承载），
 * mention.markdown 必须与正文 canonical 前缀一致（composer 用 startsWith 判定 chip 节点）。
 * 组装顺序遵循 composer 既有约定：`[chip] 正文`——chip 在消息开头（同插件商店试用）。
 */
export function buildAssetReferenceChipMessage(
  asset: AssetManifest,
  writtenRelativePaths: string[],
  locale = "zh-CN",
): { text: string; mention?: ComposerMentionPrefill } {
  const mention = buildAssetReferenceMention(asset, writtenRelativePaths, locale);
  const guidance = `图纸已写进项目（${writtenRelativePaths.length} 个文件），直接读这些文件按口令装进项目，不要在回复里重复贴代码。`;
  if (!mention) {
    // 落盘返回空（不该发生）：退回纯文本，保持不静默——列出预期文件名供智能体自行查找。
    const expectedFiles = asset.files.map((file) => `.zcode/asset-library/${asset.id}/${file.name}`);
    return {
      text: [
        asset.prompt,
        "",
        `图纸应已写进项目（目录 .zcode/asset-library/${asset.id}/），请先读取以下文件再按口令装进项目：`,
        ...expectedFiles,
      ].join("\n"),
    };
  }
  // chip 开头 + 空行 + 口令 + 落盘指引；mention.markdown 是正文的 canonical 前缀。
  const text = `${mention.markdown}\n\n${asset.prompt}\n\n${guidance}`;
  return { text, mention };
}
