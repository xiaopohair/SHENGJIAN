/**
 * 型号 → 产品正式名称（i18n key）。
 *
 * 名称以真机/厂商命名为准：ARN9 小鸟款与 RC003 2 Pro 是两款不同产品，
 * 但广播名都叫「小米蓝牙语音遥控器」，所以这里必须按型号分开写，
 * 不能沿用广播名。
 */
export function deviceProductNameKey(model?: string | null): string | null {
  if (model === "RC003") return "小米蓝牙语音遥控器 2 Pro";
  if (model === "ARN9") return "小米蓝牙语音遥控器";
  if (model === "RC001") return "小米蓝牙语音遥控器 2";
  return null;
}
