// ============================================================
// D32 后门收口的可运行检查：Bash 调起 zcode CLI 跨会话操作 → alwaysAsk 守卫。
// 运行：npx tsx --test apps/zcode-cli/packages/core/src/tool/handlers/bash-cross-session-policy.test.ts
// （node 原生 --test 会在 @zcode/shared 的 .js→.ts 解析上报错，统一走 tsx。）
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { isCrossSessionCliInvocation } from "./bash-cross-session-policy.ts";
import { bashToolEntry } from "./bash.ts";

test("直呼 zcode --resume（真机事故命令形状）被守卫拦下", () => {
  assert.equal(
    isCrossSessionCliInvocation(
      'zcode --resume sess_51074972-a7eb-47d8-bec0-ebc175be21f9 --prompt "你好"',
    ),
    true,
  );
});

test("开发版 CLI 绝对路径 + node 调起（事故现场原命令，含管道）被守卫拦下", () => {
  assert.equal(
    isCrossSessionCliInvocation(
      'node "D:/ZCodeing/ZCode/apps/zcode-cli/packages/cli/dist/zcode.cjs" ' +
        '--resume sess_51074972-a7eb-47d8-bec0-ebc175be21f9 --prompt "你好，请给我回一句你好" 2>&1 | tail -40',
    ),
    true,
  );
});

test("--continue / -c / --resume= / npx / env 前缀都算跨会话操作", () => {
  assert.equal(isCrossSessionCliInvocation("zcode --continue"), true);
  assert.equal(isCrossSessionCliInvocation("zcode -c"), true);
  assert.equal(isCrossSessionCliInvocation("zcode --resume=sess_abc --prompt hi"), true);
  assert.equal(isCrossSessionCliInvocation("npx zcode --resume sess_abc"), true);
  assert.equal(isCrossSessionCliInvocation("env ZCODE_X=1 zcode -c"), true);
});

test("壳命令里的脚本字符串也递归守卫", () => {
  assert.equal(isCrossSessionCliInvocation("bash -c 'zcode --resume sess_abc'"), true);
  assert.equal(isCrossSessionCliInvocation('cmd /c "zcode --continue"'), true);
});

test("普通命令不误伤：无跨会话开关、或未提到 zcode 都放行", () => {
  assert.equal(isCrossSessionCliInvocation('zcode --prompt "帮我看看这段日志"'), false);
  assert.equal(isCrossSessionCliInvocation("grep -c zcode access.log"), false);
  assert.equal(isCrossSessionCliInvocation('echo "以后用 zcode --resume 接着聊"'), false);
  assert.equal(isCrossSessionCliInvocation("bash -c 'echo hi'"), false);
  assert.equal(isCrossSessionCliInvocation("git status"), false);
  assert.equal(isCrossSessionCliInvocation(""), false);
});

test("解析失败的命令按原文保守兜底（宁可多确认）", () => {
  assert.equal(isCrossSessionCliInvocation("zcode --resume 'sess_abc"), true);
  assert.equal(isCrossSessionCliInvocation("echo 'oops"), false);
});

test("经 bashToolEntry 的运行时能力解析，事故命令声明 alwaysAsk", () => {
  const capability = bashToolEntry.resolvePermissionCapability?.({
    command:
      'node "D:/ZCodeing/ZCode/apps/zcode-cli/packages/cli/dist/zcode.cjs" --resume sess_51074972-a7eb-47d8-bec0-ebc175be21f9 --prompt "你好"',
  });
  assert.deepEqual(capability, { permission: { alwaysAsk: true } });
  // 普通命令不受影响（只读降级逻辑照旧走自己的判定，这里只验证守卫不掺和）。
  const untouched = bashToolEntry.resolvePermissionCapability?.({ command: "git status" });
  assert.equal(untouched?.permission?.alwaysAsk, undefined);
});
