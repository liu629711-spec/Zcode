// ============================================================
// Bash 跨会话 CLI 守卫（D32 后门收口）
// 模型用 Bash 直呼 `zcode --resume/--continue …` 可以绕开产品逻辑直接操作其他会话
// （跑马灯会话、绕过 persona/派单/审批链）。这里识别"调起 zcode CLI 并携带跨会话
// 开关"的命令，供 Bash 工具的 resolvePermissionCapability 声明 alwaysAsk：
// 确认窗在任何权限模式下都成立（照 ToolPermissionSpec.alwaysAsk 的契约语义，
// 压过 yolo 直通与项目 allow 规则，只让位于硬禁用与项目 deny），用户仍可逐次放行。
// ============================================================

import { analyzeBashCommand } from "./bash-command-parser.js";
import { executableBasename } from "./bash-command-permission-policy.js";

/** zcode CLI 的可执行名（bin 名与 Windows/打包常见后缀；basename 精确匹配）。 */
const ZCODE_CLI_BASENAMES = new Set([
  "zcode",
  "zcode.exe",
  "zcode.cmd",
  "zcode.ps1",
  "zcode.cjs",
  "zcode.mjs",
  "zcode.js",
  // beta 渠道的兄弟 bin（ZCODE_STORAGE_DIR 会指到 ~/.zcode-beta，同一个后门）。
  "zcode-beta",
  "zcode-beta.exe",
  "zcode-beta.cmd",
  "zcode-beta.cjs",
]);

/**
 * 跨会话长开关：--resume 直呼指定会话，--continue 唤醒最近会话。
 * --fork 目前只是 TUI 命令没有 CLI 开关，先收进来防它将来落成开关时漏守。
 */
const CROSS_SESSION_LONG_FLAGS = new Set(["--resume", "--continue", "--fork"]);

/** 短开关 -c = --continue。只在 argv[0] 本身就是 zcode CLI 时认它，避免 grep -c 之类误伤。 */
const CROSS_SESSION_SHORT_FLAGS = new Set(["-c"]);

/**
 * 无头建会话开关（真机事故 2026-09-30 补收）：`zcode --prompt/-p …` 另起一个
 * 不进产品名册的野会话，与 --resume 同属绕开派单/审批链的后门——模型已用它
 * 绕过后门还谎报"新会话开好了"。与跨会话开关同一守卫（alwaysAsk 压过 yolo）。
 */
const HEADLESS_RUN_LONG_FLAGS = new Set(["--prompt"]);
const HEADLESS_RUN_SHORT_FLAGS = new Set(["-p"]);

/** 把脚本字符串再递归交给守卫的壳命令：`bash -c 'zcode --resume x'` 也算跨会话操作。 */
const SHELL_WRAPPERS = new Set([
  "bash",
  "sh",
  "zsh",
  "fish",
  "ksh",
  "cmd",
  "cmd.exe",
  "powershell",
  "powershell.exe",
  "pwsh",
  "pwsh.exe",
]);

/** 递归进壳命令脚本字符串的层数上限（bash -c "bash -c …" 这类套娃不无限跟）。 */
const MAX_WRAPPER_DEPTH = 3;

/**
 * 该 Bash 命令是否在调起 zcode CLI 做跨会话操作。
 * 纯函数：供 resolvePermissionCapability 声明 alwaysAsk，也供测试直接断言。
 */
export function isCrossSessionCliInvocation(command: string, depth = 0): boolean {
  const trimmed = command.trim();
  if (trimmed.length === 0) return false;
  const analysis = analyzeBashCommand(trimmed);
  if (analysis.hasParseErrors || analysis.hasDynamicWords) {
    // 解析失败，或带命令替换/参数展开（$(…)、反引号、$var）：argv 里的结构静态不可信——
    // $(…) 里真正执行什么静态不可知，与解析失败同一认识论处境。保守起见对原文做兜底判定，
    // 宁可多弹一次确认也不放走一条跨会话直呼。
    return rawTextGuard(trimmed);
  }
  return analysis.commands.some((invocation) => isGuardedInvocation(invocation.argv, depth));
}

/** 原文兜底：同时提到 zcode 系可执行与跨会话/无头开关字面量才算（误伤面极窄，方向是多确认）。 */
function rawTextGuard(text: string): boolean {
  return (
    /\bzcode(?:\.(?:exe|cmd|ps1|cjs|mjs|js))?\b/i.test(text) &&
    // 短旗标 -p/-c 也要认（审计 2026-10-01 P1-1）：动态词命令（$x -p hi）走不到
    // argv 结构化路径，只看原文——漏短旗标等于漏掉无头建会话的主要拼法。
    // 误伤（如 grep -c zcode 恰好带动态词）方向是多弹确认，可接受。
    /--resume|--continue|--fork|--prompt/.test(text) || /(^|\s)-[pc]\b/.test(text)
  );
}

function isGuardedInvocation(argv: readonly string[], depth: number): boolean {
  if (argv.length === 0) return false;
  const head = executableBasename(argv[0]!);
  if (depth < MAX_WRAPPER_DEPTH && SHELL_WRAPPERS.has(head)) {
    const script = extractWrapperScript(argv);
    if (script !== undefined && isCrossSessionCliInvocation(script, depth + 1)) return true;
  }
  // "提到 zcode"按 basename 精确匹配，覆盖 zcode 直呼、node …/zcode.cjs、npx zcode；
  // `bash -c` 内的脚本字符串走上面的递归，不在这里当普通 token 扫。
  if (!argv.some((token) => ZCODE_CLI_BASENAMES.has(executableBasename(token)))) return false;
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index]!;
    if (CROSS_SESSION_LONG_FLAGS.has(token) || token.startsWith("--resume=")) return true;
    if (HEADLESS_RUN_LONG_FLAGS.has(token) || token.startsWith("--prompt=")) return true;
    // -c/-p 只在紧跟 zcode 位置上才算它的开关（`zcode -c`、`env X=1 zcode -p hi`），
    // `grep -c zcode`、`sort -p` 这类撞车不算。
    if (
      (CROSS_SESSION_SHORT_FLAGS.has(token) || HEADLESS_RUN_SHORT_FLAGS.has(token)) &&
      index > 0 &&
      ZCODE_CLI_BASENAMES.has(executableBasename(argv[index - 1]!))
    ) {
      return true;
    }
  }
  return false;
}

/** 取壳命令引出的脚本字符串（`bash -c/-lc/-ic <script>` / `cmd /c <script>` / `pwsh -Command <script>`）。 */
function extractWrapperScript(argv: readonly string[]): string | undefined {
  for (let index = 1; index < argv.length - 1; index += 1) {
    const flag = argv[index]!.toLowerCase();
    // -lc/-ic 这类合并短旗标同样把脚本字符串带在下一个 token（bash -lc 'zcode --resume x' 是模型日常拼写）。
    if (/^-[a-z]*c$/i.test(flag) || flag === "/c" || flag === "/k" || flag === "-command") {
      return argv[index + 1];
    }
  }
  return undefined;
}
