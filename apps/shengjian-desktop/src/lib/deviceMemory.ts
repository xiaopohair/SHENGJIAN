/**
 * 连接过的设备档案（设备令牌 → 型号 + 最近连接时间）。
 *
 * 扫描只拿得到广播名；通用名「小米蓝牙语音遥控器」推不出型号，型号真值要连上后读
 * GATT `0x2A24`。断开后 snapshot 里的 remote 变 null，这两份知识就丢了，于是列表里
 * 几支同名设备全是「型号=未知」、也看不出哪支多久没连过。这里把连接成功时看到的
 * 型号与时间戳按**设备令牌**（扫描行的 `remoteId`，由蓝牙地址派生，重连/重启都不变）
 * 落盘，扫描后回填。
 *
 * 只做锦上添花：读写失败（隐私模式、配额满、存储被清）时安静退化成当前行为。
 * 存储格式兼容旧版「令牌 → 型号字符串」。
 */
const STORAGE_KEY = "shengjian.device-models";

export interface DeviceRecord {
  /** 最近一次连接时读到的型号（GATT 0x2A24）。 */
  model?: string;
  /** 最近一次连接成功的时间（epoch ms）。 */
  lastConnectedAt?: number;
}

export type DeviceRecords = Record<string, DeviceRecord>;

function storage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

/** 读取全部设备档案；内容非法时当作空表，不抛错。 */
export function loadDeviceRecords(): DeviceRecords {
  const store = storage();
  if (!store) return {};
  try {
    const raw = store.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    const records: DeviceRecords = {};
    for (const [rawKey, rawValue] of Object.entries(parsed as Record<string, unknown>)) {
      const token = rawKey.trim();
      if (!token) continue;
      // 旧格式：直接是型号字符串。
      if (typeof rawValue === "string") {
        const model = rawValue.trim();
        if (model) records[token] = { model };
        continue;
      }
      if (!rawValue || typeof rawValue !== "object") continue;
      const record: DeviceRecord = {};
      const model = (rawValue as DeviceRecord).model;
      if (typeof model === "string" && model.trim()) record.model = model.trim();
      const at = (rawValue as DeviceRecord).lastConnectedAt;
      if (typeof at === "number" && Number.isFinite(at) && at > 0) {
        record.lastConnectedAt = Math.round(at);
      }
      if (record.model || record.lastConnectedAt) records[token] = record;
    }
    return records;
  } catch {
    return {};
  }
}

function saveDeviceRecords(records: DeviceRecords): void {
  try {
    storage()?.setItem(STORAGE_KEY, JSON.stringify(records));
  } catch {
    // 缓存只是锦上添花，写不进去不影响连接与识别。
  }
}

function upsert(token: string | null | undefined, patch: DeviceRecord): DeviceRecords {
  const key = (token ?? "").trim();
  if (!key) return loadDeviceRecords();
  const records = loadDeviceRecords();
  const next: DeviceRecord = { ...records[key], ...patch };
  records[key] = next;
  saveDeviceRecords(records);
  return records;
}

/** 连接成功：记下型号与时间（型号未知时只记时间）。 */
export function markDeviceConnected(
  token: string | null | undefined,
  model?: string | null,
  at: number = Date.now(),
): DeviceRecords {
  const patch: DeviceRecord = { lastConnectedAt: at };
  const value = (model ?? "").trim();
  if (value) patch.model = value;
  return upsert(token, patch);
}

/** 只补型号（时间戳保持不变），用于扫描后发现的型号。 */
export function rememberDeviceModel(
  token: string | null | undefined,
  model: string | null | undefined,
): DeviceRecords {
  const value = (model ?? "").trim();
  if (!value) return loadDeviceRecords();
  return upsert(token, { model: value });
}

/** 忘掉一支设备的本地档案（型号 + 连接时间）。 */
export function forgetDeviceRecord(token: string | null | undefined): DeviceRecords {
  const key = (token ?? "").trim();
  if (!key) return loadDeviceRecords();
  const records = loadDeviceRecords();
  if (!(key in records)) return records;
  delete records[key];
  saveDeviceRecords(records);
  return records;
}

/** 某支设备上次连接的时间；从未连接过返回 null。 */
export function lastConnectedAt(
  records: DeviceRecords,
  token: string | null | undefined,
): number | null {
  const key = (token ?? "").trim();
  if (!key) return null;
  return records[key]?.lastConnectedAt ?? null;
}

/** 扫描结果回填：只补空型号，不覆盖扫描自己给出的型号。 */
export function applyRememberedModels<T extends { remoteId: string; model: string | null }>(
  remotes: readonly T[],
  records: DeviceRecords,
): T[] {
  return remotes.map((remote) => {
    if (remote.model) return remote;
    const model = records[remote.remoteId]?.model;
    return model ? { ...remote, model } : remote;
  });
}
