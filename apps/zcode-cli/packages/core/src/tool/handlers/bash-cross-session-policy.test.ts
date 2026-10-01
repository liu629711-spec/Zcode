// ============================================================
// D32 后门收口的可运行检查：Bash 调起 zcode CLI 跨会话/无头建会话 → denied 硬墙。
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
  assert.equal(isCrossSessionCliInvocation("grep -c zcode access.log"), false);
  assert.equal(isCrossSessionCliInvocation('echo "以后用 zcode --resume 接着聊"'), false);
  assert.equal(isCrossSessionCliInvocation("bash -c 'echo hi'"), false);
  assert.equal(isCrossSessionCliInvocation("git status"), false);
  assert.equal(isCrossSessionCliInvocation(""), false);
});

test("无头建会话后门（真机事故 2026-09-30）：zcode --prompt/-p 直呼被守卫拦下", () => {
  // 事故原命令形状：node + 开发版绝对路径 + --prompt 新建野会话。
  assert.equal(
    isCrossSessionCliInvocation(
      'node "D:/ZCodeing/ZCode/apps/zcode-cli/packages/cli/dist/zcode.cjs" --prompt "请只回复一句话，内容要带帅哥二字"',
    ),
    true,
  );
  assert.equal(isCrossSessionCliInvocation("zcode --prompt 帮我看看日志"), true);
  assert.equal(isCrossSessionCliInvocation("zcode --prompt=帮我看看日志"), true);
  // 短旗标 -p 只在紧跟 zcode 时算（env 前缀也认）；sort -p 这类撞车不误伤。
  assert.equal(isCrossSessionCliInvocation("env ZCODE_MODEL=x zcode -p 你好"), true);
  assert.equal(isCrossSessionCliInvocation("sort -p zcode.txt"), false);
  // 壳命令递归同样覆盖。
  assert.equal(isCrossSessionCliInvocation("bash -c 'zcode --prompt hi'"), true);
});

test("解析失败的命令按原文保守兜底（宁可多确认）", () => {
  assert.equal(isCrossSessionCliInvocation("zcode --resume 'sess_abc"), true);
  assert.equal(isCrossSessionCliInvocation("echo 'oops"), false);
});

test("经 bashToolEntry 的运行时能力解析，事故命令声明 denied 硬墙 + 教学拒绝理由", () => {
  const capability = bashToolEntry.resolvePermissionCapability?.({
    command:
      'node "D:/ZCodeing/ZCode/apps/zcode-cli/packages/cli/dist/zcode.cjs" --resume sess_51074972-a7eb-47d8-bec0-ebc175be21f9 --prompt "你好"',
  });
  assert.equal(capability?.permission?.denied, true);
  assert.match(String(capability?.permission?.deniedReason), /AgentDispatch/);
  // 普通命令不受影响（不掺 denied 也不掺拒绝理由，普通 bash 弹窗逐字节不变）。
  const untouched = bashToolEntry.resolvePermissionCapability?.({ command: "git status" });
  assert.equal(untouched?.permission?.denied, undefined);
  assert.equal(untouched?.permission?.deniedReason, undefined);
});

test("权限服务认 denied 硬墙：压过 yolo 直通（真机事故：确认窗被放行后后门照走）", async () => {
  const { PermissionService } = await import("../../permission/service.js");
  const service = new PermissionService();
  const decision = service.checkPermission(
    { toolName: "Bash", mode: "yolo" } as never,
    { permission: { denied: true, deniedReason: "use AgentDispatch" } } as never,
  );
  assert.equal(decision.decision, "deny");
  assert.equal(decision.reason, "use AgentDispatch");
  // 没有 denied 的普通命令在 yolo 下照旧直通，行为不变。
  const plain = service.checkPermission(
    { toolName: "Bash", mode: "yolo" } as never,
    {} as never,
  );
  assert.equal(plain.decision, "allow");
});

test("动态词兜底也认短旗标（审计 P1-1）：$x -p 混淆不再漏网", () => {
  assert.equal(isCrossSessionCliInvocation("x=zcode; $x -p hi"), true);
  assert.equal(isCrossSessionCliInvocation("$(which zcode) -c"), true);
  // 长旗标路径不回归。
  assert.equal(isCrossSessionCliInvocation("z=zcode; $z --resume sess_1"), true);
});
