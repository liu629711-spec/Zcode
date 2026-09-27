import { mkdir, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import type { AssetLibraryWriteFilesRequest, AssetLibraryWriteFilesResult } from "@zcode/shared";
import { PlatformChannels } from "@zcode/shared";
import { ipcMain } from "electron";

// 白名单：relativeDir 只允许 `.zcode/asset-library/<kebab-id>`（素材 id 即 kebab-case），
// 杜绝 renderer 传 `..`、绝对路径或任意子目录把文件写出 workspace。
const ASSET_LIBRARY_DIR_PATTERN = /^\.zcode\/asset-library\/[a-z0-9]+(?:-[a-z0-9]+)*$/;
// 文件名只允许安全字符（图纸名如 MeteorBackground.tsx / index.html / style.css），
// 显式拒绝 "." / ".."，防止 join 后逃出白名单目录。
const ASSET_FILE_NAME_PATTERN = /^[A-Za-z0-9._-]+$/;

/**
 * 素材库 V2-2（技术设计 §10）：递活引用化的落盘通道。
 * 校验通过后 mkdir -p 并逐文件 utf8 写入；返回给智能体读的相对路径，
 * 形态 `./.zcode/asset-library/<id>/<name>`（与 mentions 的相对路径语法对齐）。
 */
export function registerDesktopAssetLibraryWriteFilesIpcHandler(logger: {
  warn: (...args: unknown[]) => void;
}) {
  ipcMain.handle(
    PlatformChannels.AssetLibraryWriteFiles,
    async (
      _event,
      payload: AssetLibraryWriteFilesRequest,
    ): Promise<AssetLibraryWriteFilesResult> => {
      const relativeDir =
        typeof payload?.relativeDir === "string" ? payload.relativeDir.trim() : "";
      const files = Array.isArray(payload?.files) ? payload.files : [];
      if (
        typeof payload?.workspacePath !== "string" ||
        payload.workspacePath.trim().length === 0 ||
        !ASSET_LIBRARY_DIR_PATTERN.test(relativeDir) ||
        files.length === 0 ||
        files.some(
          (file) =>
            !file ||
            typeof file.name !== "string" ||
            typeof file.content !== "string" ||
            !ASSET_FILE_NAME_PATTERN.test(file.name) ||
            file.name === "." ||
            file.name === "..",
        )
      ) {
        throw new Error("invalid_asset_library_payload");
      }

      // workspace 必须已存在：mkdir -p 只为建素材目录，不许把拼错的 workspace 路径整条造出来。
      const workspaceStat = await stat(payload.workspacePath).catch(() => null);
      if (!workspaceStat?.isDirectory()) {
        throw new Error("workspace_not_found");
      }

      const targetDir = join(payload.workspacePath, relativeDir);
      await mkdir(targetDir, { recursive: true });
      try {
        await Promise.all(
          files.map((file) => writeFile(join(targetDir, file.name), file.content, "utf8")),
        );
      } catch (error) {
        logger.warn(
          `[asset-library] 图纸写入失败 dir=${relativeDir} error=${
            error instanceof Error ? error.message : String(error)
          }`,
        );
        throw new Error("asset_library_write_failed");
      }
      return { writtenPaths: files.map((file) => `./${relativeDir}/${file.name}`) };
    },
  );
}
