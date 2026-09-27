// persona 快照的落盘编解码（G1 冷重启保身份的存储边界）。
//
// 惯例（对齐 T3）：坏 JSON/坏形状解析失败 → undefined + warn，不抛——
// persona_json 是历史行可缺席的新列，不能让一行脏数据阻断整条会话的读取。
// 校验复用 shared 的 zcodeSessionPersonaSchema，与 create 参数单一来源锁定；
// persona 形状从此只能加可选字段，不得改必填形状。

import {
  zcodeSessionPersonaSchema,
  type ZCodeSessionPersona,
} from "@zcode/shared";

/** 写侧：undefined/非法值写 NULL，不让半截快照落库。 */
export function encodeSessionPersonaJson(
  persona: ZCodeSessionPersona | undefined,
): string | null {
  if (!persona) return null;
  return zcodeSessionPersonaSchema.safeParse(persona).success
    ? JSON.stringify(persona)
    : null;
}

/** 读侧：NULL/坏 JSON/形状漂移 → undefined；解析失败留一行诊断。 */
export function decodeSessionPersonaJson(
  raw: string | null | undefined,
): ZCodeSessionPersona | undefined {
  if (!raw) return undefined;
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch (error) {
    console.warn(
      "[session-store] persona_json 解析失败，按无 persona 处理",
      error instanceof Error ? error.message : error,
    );
    return undefined;
  }
  const parsed = zcodeSessionPersonaSchema.safeParse(value);
  if (parsed.success) return parsed.data;
  console.warn(
    "[session-store] persona_json 形状非法，按无 persona 处理",
    parsed.error.issues
      .slice(0, 3)
      .map((issue) => `${issue.path.join(".")}: ${issue.message}`)
      .join("; "),
  );
  return undefined;
}
