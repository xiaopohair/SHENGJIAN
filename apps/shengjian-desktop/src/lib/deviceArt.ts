import rc001Line from "../assets/devices/xiaomi-rc001-line.png";
import rc003Line from "../assets/devices/xiaomi-rc003-line.png";
import arn9Line from "../assets/devices/xiaomi-arn9-line.png";

/**
 * 型号 → 系统现有的产品线稿（连接页用的是同一组资源）。
 *
 * 通用方案没有归属型号，沿用连接页的兜底：按 RC003 画。
 * 线稿是深色的，深色主题靠 `--line-art-invert` 反转，不要在组件里另写颜色。
 */
export function deviceLineArt(model?: string | null): string {
  switch ((model ?? "").trim().toUpperCase()) {
    case "ARN9":
      return arn9Line;
    case "RC001":
      return rc001Line;
    default:
      return rc003Line;
  }
}
