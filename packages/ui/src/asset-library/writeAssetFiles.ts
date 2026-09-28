/**
 * 素材图纸落盘（V4.6）：素材库"发到会话"、@ 设计风格选中、composer 风格推荐条
 * 三个入口共用的同一条写盘通道（IPC 白名单见 desktop 侧 handler）。
 * 失败抛错（含 not_supported=主进程旧版），由调用方决定 toast 与兜底。
 */
import type { IPlatformService } from "@zcode/shared";
import type { AssetManifest } from "./catalog/types.js";

export async function writeAssetBlueprintFiles(
  platform: Pick<IPlatformService, "assetLibraryWriteFiles"> | null | undefined,
  workspacePath: string,
  asset: AssetManifest,
): Promise<void> {
  const result = await platform?.assetLibraryWriteFiles?.({
    workspacePath,
    relativeDir: `.zcode/asset-library/${asset.id}`,
    files: asset.files.map((file) => ({ name: file.name, content: file.content })),
  });
  if (!result) {
    throw new Error("not_supported");
  }
}
