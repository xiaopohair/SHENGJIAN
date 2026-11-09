export function deviceProductNameKey(model?: string | null): string | null {
  if (model === "RC003") return "小米蓝牙语音遥控器 2 Pro";
  if (model === "RC001") return "小米蓝牙语音遥控器 2";
  return null;
}
