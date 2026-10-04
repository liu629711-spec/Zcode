/**
 * 素材正文按需装载钩子（地基清理·展厅瘦身 2026-10-05）。
 *
 * lazy 货（bodyFrom 在场）在 needed（卡片进缓冲带/开代码面板/递活）为真时才
 * 拉 body chunk；正文就地的货直接回 inline 值（useMemo 定住身份，避免下游
 * effect 因引用变化空转）。装载失败只留痕（控制台），卡片停在骨架态——
 * bodyCache 已在失败时清槽，下一次 needed 触发会重试。
 */
import { useEffect, useMemo, useState } from "react";
import { loadAssetBody, type AssetBody } from "./catalog/assetBodies.js";
import type { AssetManifest } from "./catalog/types.js";

export function useAssetBody(manifest: AssetManifest, needed: boolean): AssetBody | null {
  const lazy = manifest.bodyFrom !== undefined;
  const inline = useMemo<AssetBody | null>(
    () => (lazy ? null : { previewHtml: manifest.previewHtml, files: manifest.files }),
    [lazy, manifest],
  );
  const [loaded, setLoaded] = useState<AssetBody | null>(null);
  useEffect(() => {
    if (!lazy || !needed || loaded) return;
    let alive = true;
    loadAssetBody(manifest).then(
      (body) => {
        if (alive) setLoaded(body);
      },
      (error: unknown) => {
        console.warn("[asset-library] 素材正文加载失败:", manifest.id, error);
      },
    );
    return () => {
      alive = false;
    };
  }, [lazy, needed, loaded, manifest]);
  return lazy ? loaded : inline;
}
