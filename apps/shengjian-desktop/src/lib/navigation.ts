import { ref } from "vue";

type LeaveGuard = () => boolean | Promise<boolean>;

const guards = new Set<LeaveGuard>();

export function registerLeaveGuard(guard: LeaveGuard): () => void {
  guards.add(guard);
  return () => {
    guards.delete(guard);
  };
}

export async function canLeavePage(): Promise<boolean> {
  for (const guard of [...guards]) {
    if (!(await guard())) return false;
  }
  return true;
}

/**
 * 跨页请求：「去设备设置」。
 *
 * 设置页里被设备覆盖的那几项会把用户送到「连接页 → 设备设置」抽屉。
 * 目标页此时还没挂载，所以不能用组件事件：请求先记成一次性标记，连接页挂载
 * （或已经挂载）时取走并打开抽屉。`deviceSettingsRequest` 计数器供已挂载的页面
 * 监听，避免"值一直是同一个"导致监听不触发。
 */
let pendingDeviceSettings = false;

export const deviceSettingsRequest = ref(0);

export function requestDeviceSettings() {
  pendingDeviceSettings = true;
  deviceSettingsRequest.value += 1;
}

/** 取走一次性请求；没有请求时返回 false。 */
export function takeDeviceSettingsRequest(): boolean {
  if (!pendingDeviceSettings) return false;
  pendingDeviceSettings = false;
  return true;
}

/**
 * 跨页请求：「去按键页」。
 *
 * 设置页装完全按键支持服务后，还要用户去「按键」页打开「全按键支持」开关，
 * 服务才会真的被拉起（计划任务没有触发器，只有主开关会 `schtasks /run`）。
 * 和上面的设备设置请求一样，设置页不直接改 `App.vue` 的 `active`，只投一个请求。
 */
export const buttonsPageRequest = ref(0);

export function requestButtonsPage() {
  buttonsPageRequest.value += 1;
}
