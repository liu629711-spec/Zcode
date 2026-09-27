/**
 * 递活管线纯函数（技术设计 §3）：把素材组装成发进会话的完整需求消息。
 *
 * 三种消费方共用同一输出：复制（兜底）、「发到新会话」（onCreateTask 预填）、
 * 「发到当前会话」（requestComposerTextInsert）。都是预填进输入框、用户按发送。
 */
import type { AssetFile, AssetManifest } from "./catalog/types.js";

/** 图纸内容里最长的连续反引号串（逐文件独立计算）。 */
function longestBacktickRun(content: string): number {
  let longest = 0;
  let current = 0;
  for (const char of content) {
    current = char === "`" ? current + 1 : 0;
    if (current > longest) {
      longest = current;
    }
  }
  return longest;
}

/** 单文件的图纸段：文件名在围栏前一行，围栏带语言标注。 */
function blueprintBlock(file: AssetFile): string {
  // 围栏必须比内容里最长的反引号串更长，否则图纸里的 ``` 会提前关掉代码块
  // （CommonMark 围栏规则；技术设计 §3，测试钉住）。
  const fence = "`".repeat(Math.max(3, longestBacktickRun(file.content) + 1));
  return `${file.name}\n${fence}${file.language}\n${file.content}\n${fence}`;
}

/** prompt 类货（files 为空）消息 = 口令原文 + 固定尾句；普通货 = 口令段 + 逐文件图纸段。 */
export function buildAssetTryPrompt(manifest: AssetManifest): string {
  if (manifest.files.length === 0) {
    return `${manifest.prompt}\n\n请照上面的口令直接干活，不需要另附代码。`;
  }
  const briefing = [
    `请把下面的「${manifest.title}」装进我的项目。`,
    `要求：${manifest.prompt}`,
    "注意：先看现有框架和风格，融入而不是覆盖；装完告诉我改了哪些文件。",
    "代码：",
  ].join("\n");
  return `${briefing}\n${manifest.files.map(blueprintBlock).join("\n")}`;
}
