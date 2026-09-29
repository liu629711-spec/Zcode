// ============================================================
// D32 后门收口的可运行检查：Bash 调起 zcode CLI 跨会话操作 → alwaysAsk 守卫。
// 运行：npx tsx --test apps/zcode-cli/packages/core/src/tool/handlers/bash-cross-session-policy.test.ts
// （node 原生 --test 会在 @zcode/shared 的 .js→.ts 解析上报错，统一走 tsx。）
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { resolveToolApproval } from "../executor/approval-gate.ts";
import { resolveRuntimePermissionCapability } from "../executor/permission-capability.ts";
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

test("合并短旗标的壳（-lc/-ic）同样递归，不漏网", () => {
  assert.equal(isCrossSessionCliInvocation("bash -lc 'zcode --resume sess_abc'"), true);
  assert.equal(isCrossSessionCliInvocation("bash -ic 'zcode --resume sess_abc'"), true);
  assert.equal(isCrossSessionCliInvocation("sh -lc 'zcode --continue'"), true);
});

test("命令替换/反引号体静态不可知，按原文兜底守卫", () => {
  assert.equal(isCrossSessionCliInvocation("echo $(zcode --resume sess_abc)"), true);
  assert.equal(isCrossSessionCliInvocation("echo `zcode --resume sess_abc`"), true);
  assert.equal(isCrossSessionCliInvocation('zcode --resume "$(get_session_id)"'), true);
  // 展开体里没有跨会话开关字面量的不误伤。
  assert.equal(isCrossSessionCliInvocation("grep zcode $(find . -name '*.log')"), false);
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

test("经 bashToolEntry 的运行时能力解析，事故命令声明 alwaysAsk + 会话作用域 askOptions", () => {
  const capability = bashToolEntry.resolvePermissionCapability?.({
    command:
      'node "D:/ZCodeing/ZCode/apps/zcode-cli/packages/cli/dist/zcode.cjs" --resume sess_51074972-a7eb-47d8-bec0-ebc175be21f9 --prompt "你好"',
  });
  assert.deepEqual(capability, {
    permission: { alwaysAsk: true, askOptions: { allowAlways: "session" } },
  });
  // 普通命令不受影响（不掺 alwaysAsk 也不掺 askOptions，普通 bash 弹窗逐字节不变）。
  const untouched = bashToolEntry.resolvePermissionCapability?.({ command: "git status" });
  assert.equal(untouched?.permission?.alwaysAsk, undefined);
  assert.equal(untouched?.permission?.askOptions, undefined);
});

test("审批 gate 认合并后能力的 askOptions：跨会话守卫弹会话作用域选项，普通命令不变", () => {
  const incident =
    'node "D:/ZCodeing/ZCode/apps/zcode-cli/packages/cli/dist/zcode.cjs" --resume sess_51074972-a7eb-47d8-bec0-ebc175be21f9 --prompt "你好"';
  const guarded = resolveRuntimePermissionCapability(
    bashToolEntry,
    { command: incident },
    {},
  );
  const guardedApproval = resolveToolApproval(
    {} as never,
    { name: "Bash", id: "call_guarded" } as never,
    bashToolEntry,
    guarded,
    { command: incident },
    { traceId: "t", spanId: "s" } as never,
  );
  assert.equal(guardedApproval.gate, "ask");
  // "session-always-allow" 让弹窗只提供会话作用域免确认（checkAlwaysAsk 认 sessionRules），
  // 不再提供永远压不过这道确认的持久项目规则假按钮。
  assert.equal(guardedApproval.optionsPolicy, "session-always-allow");

  const plain = resolveRuntimePermissionCapability(bashToolEntry, { command: "git status" }, {});
  const plainApproval = resolveToolApproval(
    {} as never,
    { name: "Bash", id: "call_plain" } as never,
    bashToolEntry,
    plain,
    { command: "git status" },
    { traceId: "t", spanId: "s" } as never,
  );
  assert.equal(plainApproval.gate, "ask");
  assert.equal(plainApproval.optionsPolicy, undefined);
});
