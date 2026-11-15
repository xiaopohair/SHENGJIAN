import { tr } from "./i18n";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/**
 * 相对时间文案：刚刚 / N 分钟前 / N 小时前 / N 天前 / N 个月前 / N 年前。
 *
 * 上限只做到"年"，列表用途不需要更细；未来时间（时钟回拨）按"刚刚"处理。
 */
export function formatRelativeTime(at: number, now: number = Date.now()): string {
  const elapsed = Math.max(0, now - at);
  if (elapsed < MINUTE) return tr("刚刚");
  if (elapsed < HOUR) return tr("{minutes} 分钟前", { minutes: Math.floor(elapsed / MINUTE) });
  if (elapsed < DAY) return tr("{hours} 小时前", { hours: Math.floor(elapsed / HOUR) });
  const days = Math.floor(elapsed / DAY);
  if (days < 30) return tr("{days} 天前", { days });
  const months = Math.floor(days / 30);
  if (months < 12) return tr("{months} 个月前", { months });
  return tr("{years} 年前", { years: Math.floor(months / 12) });
}
