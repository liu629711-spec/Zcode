/**
 * 素材库验收 harness（临时页）：不接 Electron 桥，浏览器直接打开
 * /src/renderer/asset-library-harness.html 即可只挂 AssetLibrarySection。
 * 仅供备货后的真机视觉验收用；验收完可整页删除，不参与构建入口。
 */
import { createRoot } from "react-dom/client";
import { ZCodeIntlProvider } from "@zcode/ui";
import type { IPlatformService } from "@zcode/shared";
import { PlatformProvider } from "@/hooks/usePlatform.js";
import { AssetLibrarySection } from "@/asset-library/AssetLibrarySection.js";
import "@zcode/ui/styles.css";

// 验收页不接 Electron 桥：AssetDemoCard 只读模式下不触发 assetLibraryWriteFiles，
// 空 stub 走卡片内"平台不支持"回退分支即可。
const stubPlatform = {} as IPlatformService;

createRoot(document.getElementById("root")!).render(
  <PlatformProvider platform={stubPlatform}>
    <ZCodeIntlProvider initialLocale="zh-CN">
      <div style={{ height: "100vh", overflowY: "auto", padding: "16px" }}>
        <AssetLibrarySection
          workspacePath="D:\\demo-workspace"
          hasActiveChat={false}
          readOnly={true}
        />
      </div>
    </ZCodeIntlProvider>
  </PlatformProvider>,
);
