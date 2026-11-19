import { setButtonTestMode } from "./api";

/**
 * 按键测试模式的持有者计数。
 *
 * 设备测试面板和首次配置向导都可能进入测试模式；后端是一个布尔开关，
 * 这里用引用计数保证后关闭的一方不会把仍在使用中的测试模式提前解除。
 * 调用按顺序串行化，避免 enter/leave 快速切换时请求乱序。
 */
let holders = 0;
let queue: Promise<void> = Promise.resolve();

function enqueue(enabled: boolean): Promise<void> {
  queue = queue.then(() => setButtonTestMode(enabled)).catch(() => undefined);
  return queue;
}

export function enterButtonTestMode(): Promise<void> {
  holders += 1;
  return holders === 1 ? enqueue(true) : queue;
}

export function leaveButtonTestMode(): Promise<void> {
  if (holders === 0) return queue;
  holders -= 1;
  return holders === 0 ? enqueue(false) : queue;
}
