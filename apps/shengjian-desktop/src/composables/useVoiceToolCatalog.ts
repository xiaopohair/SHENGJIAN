/**
 * 语音输入工具目录（配置目录驱动）。
 *
 * 模块级缓存：设置页、连接页、向导、方案抽屉和顶部状态栏共用一次
 * `voice_tool_catalog` 请求。命令失败时保持空目录——各下拉会把已保存的旧 id
 * 作为临时 option 显示，不会有静默丢失。
 */
import { ref, shallowRef } from "vue";
import { getVoiceToolCatalog } from "../lib/api";
import type { VoiceToolInfo } from "../lib/types";

const catalog = shallowRef<VoiceToolInfo[]>([]);
const loaded = ref(false);
let request: Promise<void> | null = null;

export function useVoiceToolCatalog() {
  async function load(): Promise<void> {
    if (loaded.value) return;
    if (!request) {
      request = getVoiceToolCatalog()
        .then((tools) => {
          catalog.value = Array.isArray(tools) ? tools : [];
          loaded.value = true;
        })
        .catch(() => {
          // Command missing/failed: keep the dropdown usable through the
          // per-value fallback option instead of throwing on every page mount.
          catalog.value = [];
        })
        .finally(() => {
          request = null;
        });
    }
    return request;
  }

  return { catalog, loaded, load };
}

/** 测试辅助：清空模块级缓存，避免用例之间互相污染。 */
export function resetVoiceToolCatalog() {
  catalog.value = [];
  loaded.value = false;
  request = null;
}
