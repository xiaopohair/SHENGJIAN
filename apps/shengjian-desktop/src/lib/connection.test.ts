import { describe, expect, it } from "vitest";
import { deviceAction, findLastRemote } from "./connection";

describe("findLastRemote", () => {
  it("returns the matching remote", () => {
    const remote = findLastRemote([{ remoteId: "a" }, { remoteId: "b" }], "b");
    expect(remote?.remoteId).toBe("b");
  });

  it("returns undefined when the last remote is absent", () => {
    expect(findLastRemote([{ remoteId: "a" }], "missing")).toBeUndefined();
  });
});

describe("deviceAction", () => {
  const idle = { connecting: false, pending: null };

  it("shows connect/disconnect when nothing is happening", () => {
    expect(deviceAction({ remoteId: "a", isCurrent: false, ...idle })).toBe("connect");
    expect(deviceAction({ remoteId: "a", isCurrent: true, ...idle })).toBe("disconnect");
  });

  it("only marks the row being connected, not every row", () => {
    // 曾出现的 bug：点一台设备时，列表里其他所有设备都显示「连接中」。
    const state = {
      connecting: true,
      pending: { remoteId: "a", kind: "connect" as const },
    };
    expect(deviceAction({ remoteId: "a", isCurrent: false, ...state })).toBe("connecting");
    expect(deviceAction({ remoteId: "b", isCurrent: false, ...state })).toBe("connect");
    expect(deviceAction({ remoteId: "c", isCurrent: false, ...state })).toBe("connect");
  });

  it("keeps a successful connect labelled connecting even after it becomes current", () => {
    // 连接成功的瞬间 isCurrent 已经是 true，但整个动作还没结束；
    // 这里不能因为 isCurrent 就翻成「断开中」。
    const state = {
      connecting: true,
      pending: { remoteId: "a", kind: "connect" as const },
    };
    expect(deviceAction({ remoteId: "a", isCurrent: true, ...state })).toBe("connecting");
  });

  it("marks a disconnect on the current row", () => {
    const state = {
      connecting: true,
      pending: { remoteId: "a", kind: "disconnect" as const },
    };
    expect(deviceAction({ remoteId: "a", isCurrent: true, ...state })).toBe("disconnecting");
    expect(deviceAction({ remoteId: "b", isCurrent: false, ...state })).toBe("connect");
  });

  it("does not leak a stale pending operation after it finishes", () => {
    expect(
      deviceAction({
        remoteId: "a",
        isCurrent: false,
        connecting: false,
        pending: { remoteId: "a", kind: "connect" },
      }),
    ).toBe("connect");
  });
});
