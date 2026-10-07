import { useEffect, useRef } from "react";
import { toast } from "@/components/ui/toast.js";
import { useZCodeIntl } from "@/i18n/index.js";
import { useConfirmDialogStore } from "@/store/confirmDialogStore.js";
import { useZCodeStore } from "@/store/StoreProvider.js";
import { consumeSkinShareFromLocation } from "./skinShare.js";
import { formatSkinValidationErrors } from "./skinSchema.js";

/**
 * 皮肤分享链接 intake——挂在 RootShell，启动时消费一次 location.hash 里的 #skin=...。
 * 校验通过→确认弹窗→入库+激活；取消则什么都不动。hash 随即清掉，刷新不会重复弹。
 */
export function SkinShareIntake() {
  const { intl } = useZCodeIntl();
  const importSkin = useZCodeStore((state) => state.importSkin);
  const setActiveSkinId = useZCodeStore((state) => state.setActiveSkinId);
  const requestConfirmation = useConfirmDialogStore((state) => state.requestConfirmation);
  const consumed = useRef(false);

  useEffect(() => {
    if (consumed.current) return;
    consumed.current = true;
    const result = consumeSkinShareFromLocation();
    if (!result) return;
    if (!result.ok) {
      toast(formatSkinValidationErrors(intl, result.errors));
      return;
    }
    void (async () => {
      const confirmed = await requestConfirmation({
        title: intl.formatMessage({ id: "skin.installConfirmTitle" }, { name: result.skin.name }),
        description: intl.formatMessage({ id: "skin.installConfirmLinkDescription" }),
        confirmLabel: intl.formatMessage({ id: "skin.install" }),
      });
      if (!confirmed) return;
      const imported = importSkin(result.skin);
      if (!imported.ok) {
        toast(formatSkinValidationErrors(intl, imported.errors));
        return;
      }
      setActiveSkinId(imported.skin.id);
      toast(intl.formatMessage({ id: "skin.imported" }, { name: imported.skin.name }));
    })();
    // 只在挂载时消费一次；intl/store action 是稳定引用，依赖留空
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return null;
}
