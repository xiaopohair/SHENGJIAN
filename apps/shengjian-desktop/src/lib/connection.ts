export function findLastRemote<T extends { remoteId: string }>(
  remotes: T[],
  lastRemoteId: string,
): T | undefined {
  return remotes.find((remote) => remote.remoteId === lastRemoteId);
}

/** 连接页每一行设备按钮要表达的动作。 */
export type DeviceAction = "connect" | "disconnect" | "connecting" | "disconnecting";

/** 正在进行中的操作：落在哪台设备上、是连还是断。 */
export interface PendingDeviceOperation {
  remoteId: string;
  kind: "connect" | "disconnect";
}

/**
 * 某一行设备按钮该显示哪种动作文案。
 *
 * `connecting` 是「当前有连接操作在进行」的全局标记（同一时刻只允许一个操作），
 * 但它**不能**用来给每一行上文案：那样点一台设备时，列表里其他所有设备的按钮
 * 都会变成「连接中」（2026-11-12 现场 bug）。必须结合 `pending` 判断操作落在哪台。
 *
 * 动作类型也不能从 `isCurrent` 反推：连接成功的瞬间 `isCurrent` 就变 true 了，
 * 而 `connecting` 要到整个动作（含连接后刷新状态）结束才落，两者中间的窗口会把
 * 一次成功的连接错标成「断开中」。所以 `kind` 由发起方显式给出。
 */
export function deviceAction(input: {
  remoteId: string;
  isCurrent: boolean;
  connecting: boolean;
  pending: PendingDeviceOperation | null;
}): DeviceAction {
  if (input.connecting && input.pending?.remoteId === input.remoteId) {
    return input.pending.kind === "connect" ? "connecting" : "disconnecting";
  }
  return input.isCurrent ? "disconnect" : "connect";
}
