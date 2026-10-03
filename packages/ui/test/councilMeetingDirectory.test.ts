// ============================================================
// 会议室侧栏目录纯规则的可运行检查（node:test + node:assert/strict，无假件）：
//  1. 分组：进行中（running/proposed）vs 已收口（approved/rejected/deadlocked/
//     cancelled——收口含流会与全场派单失败的哑会）；
//  2. 排序：小节内按更新时间倒序（最近推进的会议排最上）；
//  3. 主文案：议题原话优先、退短标题、空白串不当正文。
//
// 本文件只依赖相对路径的纯函数模块（无 @/ 别名、无 shared 类型不进编译面）。
//
// 运行（本机 tsx 挂死，走 tsc 直编 + node:test）：
//   T=.zcode/workflow-drafts/gate-out && node node_modules/typescript/bin/tsc \
//     packages/ui/test/councilMeetingDirectory.test.ts \
//     --outDir "$T" --rootDir . --module nodenext --moduleResolution nodenext \
//     --target es2023 --skipLibCheck --strict --types node && \
//   node --test "$T/packages/ui/test/councilMeetingDirectory.test.js"
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  buildCouncilMeetingDirectory,
  councilMeetingEntrySubject,
  type CouncilMeetingSummaryLike,
} from "../src/v4/councilMeetingDirectory.js";

function summary(
  overrides: Partial<CouncilMeetingSummaryLike> & { councilId: string },
): CouncilMeetingSummaryLike {
  return {
    sessionId: `session-${overrides.councilId}`,
    kind: "plan",
    status: "running",
    timeUpdated: 1_000,
    ...overrides,
  };
}

test("会议室目录：进行中/已收口两小节，收口含流会与取消", () => {
  const directory = buildCouncilMeetingDirectory([
    summary({ councilId: "running-1" }),
    summary({ councilId: "deadlocked-1", status: "deadlocked" }),
    summary({ councilId: "approved-1", status: "approved" }),
    summary({ councilId: "rejected-1", status: "rejected" }),
    summary({ councilId: "cancelled-1", status: "cancelled" }),
  ]);
  assert.deepEqual(
    directory.running.map((entry) => entry.councilId),
    ["running-1"],
  );
  // 已收口保留原始 status（徽章分词），分组只决定小节。
  assert.deepEqual(
    directory.closed.map((entry) => entry.councilId).sort(),
    ["approved-1", "cancelled-1", "deadlocked-1", "rejected-1"],
  );
  const deadlocked = directory.closed.find(
    (entry) => entry.councilId === "deadlocked-1",
  );
  assert.equal(deadlocked?.group, "closed");
  assert.equal(deadlocked?.status, "deadlocked");
});

test("会议室目录：小节内按更新时间倒序", () => {
  const directory = buildCouncilMeetingDirectory([
    summary({ councilId: "old-running", timeUpdated: 100 }),
    summary({ councilId: "new-running", timeUpdated: 900 }),
    summary({ councilId: "mid-running", timeUpdated: 500 }),
    summary({ councilId: "new-closed", status: "approved", timeUpdated: 800 }),
    summary({ councilId: "old-closed", status: "rejected", timeUpdated: 200 }),
  ]);
  assert.deepEqual(
    directory.running.map((entry) => entry.councilId),
    ["new-running", "mid-running", "old-running"],
  );
  assert.deepEqual(
    directory.closed.map((entry) => entry.councilId),
    ["new-closed", "old-closed"],
  );
});

test("会议室目录条目主文案：议题优先、退标题、空白不当正文", () => {
  assert.equal(
    councilMeetingEntrySubject({ motion: " 登录页改版方案 ", title: "短标题" }),
    "登录页改版方案",
  );
  assert.equal(
    councilMeetingEntrySubject({ motion: "   ", title: "短标题" }),
    "短标题",
  );
  assert.equal(councilMeetingEntrySubject({}), undefined);
});
