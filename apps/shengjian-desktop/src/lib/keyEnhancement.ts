/**
 * 按键增强（RC003 helper 的增强捕获）适用机型。
 *
 * 三款遥控器都是小米的，返回键/音量键在三个型号上都走同一条增强捕获通道，
 * 所以服务与开关对所有机型一视同仁；这里集中一处判断，避免各页面各写一遍
 * `model === "RC003"` 把另外两款漏掉。
 */
export const KEY_ENHANCEMENT_MODELS = ["RC003", "RC001", "ARN9"] as const;

/** 该型号是否支持按键增强；型号未知（未连接/识别不出）时返回 false。 */
export function supportsKeyEnhancement(model?: string | null): boolean {
  const normalized = (model ?? "").trim().toUpperCase();
  return (KEY_ENHANCEMENT_MODELS as readonly string[]).includes(normalized);
}
