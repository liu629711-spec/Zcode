// ============================================================
// 班底进图纸（2026-10-01）：actor persona 的员工引用解析与身份拼装。
// 脚本作者在 agent() 的 persona 里点名员工（profile: "code-reviewer"），
// 引擎只透传字符串（app-free 纪律）；本模块在宿主侧把它展开成身份：
// 名册档案的 systemPrompt 成为 actor 的人设（脚本 system 变步骤附加要求），
// 档案模型插进模型优先级（workflow-actor-model.ts）。
// 名字匹配复用 resolveWorkOrderTarget（与派单点名同一套大小写/来源/歧义纪律）。
// ============================================================

import type { AgentProfile } from "@zcode/core";
import { resolveWorkOrderTarget } from "@zcode/core";
import type { PersonaSpec } from "@zcode/dynamic-workflow";

/** 点名了员工但解析失败时抛出；带人话原因（缺失给改法，歧义列撞名双方）。 */
export class WorkflowActorProfileError extends Error {
  readonly profileName: string;
  readonly kind: "not_found" | "ambiguous";
  readonly matchedNames?: readonly string[];

  constructor(
    kind: "not_found" | "ambiguous",
    profileName: string,
    matchedNames?: readonly string[],
  ) {
    super(
      kind === "not_found"
        ? `图纸里点名的员工「${profileName}」不在名册里（可能被改名或删除）。请改用名册里已有的员工名，或删掉这个 profile 引用改用普通 persona。`
        : `图纸里点名的员工「${profileName}」在名册里撞了名（${(matchedNames ?? []).join("、")}）。请改用唯一的员工名，或按工号点名。`,
    );
    this.name = "WorkflowActorProfileError";
    this.profileName = profileName;
    this.kind = kind;
    if (matchedNames !== undefined) this.matchedNames = matchedNames;
  }
}

/** persona 是否点了员工的将（引用为空串视同没点）。 */
export function hasWorkflowActorProfileRef(persona: PersonaSpec): boolean {
  const wanted = persona.profile?.trim();
  return wanted !== undefined && wanted.length > 0;
}

/**
 * persona 上的员工引用 → 名册档案。没点名返回 undefined；点名了但解析不出
 * （缺失/歧义）抛 {@link WorkflowActorProfileError}——**宁可 fail 整个 actor，
 * 也绝不悄悄退化成匿名工人**（那等于脚本作者点将点了个寂寞还不知道）。
 * 匹配口径与派单点名完全同源（大小写不敏感、限身份档三来源、歧义拒绝）。
 */
export function resolveWorkflowActorProfile(
  persona: PersonaSpec,
  profiles: readonly AgentProfile[],
): AgentProfile | undefined {
  const wanted = persona.profile?.trim();
  if (wanted === undefined || wanted.length === 0) return undefined;
  const resolution = resolveWorkOrderTarget(wanted, profiles);
  if (resolution.kind === "resolved") return resolution.profile;
  if (resolution.kind === "ambiguous") {
    throw new WorkflowActorProfileError("ambiguous", wanted, resolution.matchedNames);
  }
  throw new WorkflowActorProfileError("not_found", wanted);
}

/**
 * 有名员工 + 脚本人设 → actor 的 system 文本。员工档案的说明书是身份主体；
 * 脚本的 system（若给了）是**本步骤的附加要求**，追加在身份之后——图纸作者
 * 不该（也不能）改写员工的性格说明书，只能给这一步加要求。整体为空（无引用
 * 且脚本没给 system）返回 undefined，配置侧按「字段缺席」处理。
 * ponytail: 员工的记忆本（profile.memory）这一刀没接——actor 会话还没有
 * 记忆身份管道（projectAgentPersona + 记忆工具白名单联动），班底在图纸里
 * 暂时"有脸没记性"。升级路径 = 在 child runtime 配置上补记忆身份并让
 * persistent-memory 认它；观察到真机需求再接。
 */
export function composeWorkflowActorPersona(
  profile: AgentProfile,
  persona: PersonaSpec,
): string | undefined {
  const stepRequirements = persona.system?.trim();
  const base = profile.systemPrompt;
  const composed =
    stepRequirements === undefined || stepRequirements.length === 0
      ? base
      : [base, "## Step-specific requirements (from the workflow script)", "", stepRequirements].join(
          "\n",
        );
  return composed.trim().length > 0 ? composed : undefined;
}
