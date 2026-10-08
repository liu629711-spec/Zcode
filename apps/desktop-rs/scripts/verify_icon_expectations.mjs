// 独立复核脚本：直接照抄 fileDisplayHelpers.ts:137-172 的 resolveIconName 语义，
// 用于交叉验证 Rust 侧 file_icons.rs 的断言是否与真源一致。
import fs from "fs";

const src = fs.readFileSync(
  "D:/ZCodeing/Zcode/packages/ui/src/lib/fileDisplayHelpers.ts",
  "utf8",
);

function grab(name) {
  const anchor = src.indexOf(`const ${name}: Record<string, string> = {`);
  if (anchor < 0) throw new Error(`未找到 ${name}`);
  // 锚点本身以 "= {" 结尾，其 { 即表内容的左括号。
  const open = anchor + `const ${name}: Record<string, string> = `.length;
  let d = 0,
    i = open;
  for (; i < src.length; i++) {
    if (src[i] === "{") d++;
    else if (src[i] === "}") {
      d--;
      if (!d) break;
    }
  }
  const m = new Map();
  // 真源文件是 CRLF，行尾的 \r 会让 $ 锚点失配——先归一化换行。
  for (const line of src.slice(open + 1, i).split(/\r?\n/)) {
    const r = line.match(/^\s*(?:"([^"]+)"|([A-Za-z0-9_$]+))\s*:\s*"([^"]+)",?\s*$/);
    if (r) m.set(r[1] ?? r[2], r[3]);
  }
  if (m.size === 0) throw new Error(`${name} 解析为空`);
  return m;
}

const FN = grab("FILE_NAME_ICON_ALIASES");
const EX = grab("EXTENSION_ICON_ALIASES");

function resolve(filePath) {
  const normalized = filePath.replace(/\\/g, "/");
  const ls = normalized.lastIndexOf("/");
  const leaf = ls === -1 ? normalized : normalized.slice(ls + 1);
  const nl = leaf.toLowerCase();
  const ld = leaf.lastIndexOf(".");
  const stem = ld === -1 ? nl : nl.slice(0, ld);
  const cands = new Set([nl, stem]);
  let s = stem;
  while (s.includes(".")) {
    s = s.slice(0, s.lastIndexOf("."));
    if (s) cands.add(s);
  }
  // Map 必须用 .get()，不能当普通对象索引（FN[c] 恒为 undefined）。
  for (const c of cands) {
    const hit = FN.get(c);
    if (hit) return hit;
  }
  if (ld === -1) return "document";
  return EX.get(nl.slice(ld + 1)) ?? nl.slice(ld + 1);
}

const cases = [
  "Dockerfile",
  ".randomdotfile",
  "LICENSE",
  ".npmrc",
  "Cargo.toml",
  "package-lock.json",
  "tsconfig.base.json",
  "src/main.rs",
  "a/b/c.tsx",
  "package.json",
  "README.md",
  ".gitignore",
  "vitest.config.ts",
  "a.unknownext",
  "src\\lib\\index.ts",
  "C:\\ws\\apps\\Cargo.toml",
];
for (const f of cases) console.log(JSON.stringify(f).padEnd(32), "->", resolve(f));