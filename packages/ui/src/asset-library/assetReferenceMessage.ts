/**
 * 递活引用化消息组装（技术设计 §10 V2-2）：普通货不再把全量代码怼进输入框，
 * 而是图纸先静默落盘到 `<workspace>/.zcode/asset-library/<id>/`，消息 = 口令 + 逐文件
 * 引用链接，智能体自己读文件装进项目。prompt 类货不落盘，仍走 buildAssetTryPrompt。
 */
import { buildFileMentionMarkdown } from "../mentions/mentionMarkdown.js";
import type { AssetManifest } from "./catalog/types.js";

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
