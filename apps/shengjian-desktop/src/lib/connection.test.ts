import { describe, expect, it } from "vitest";
import { findLastRemote } from "./connection";

describe("findLastRemote", () => {
  it("returns the matching remote", () => {
    const remote = findLastRemote([{ remoteId: "a" }, { remoteId: "b" }], "b");
    expect(remote?.remoteId).toBe("b");
  });

  it("returns undefined when the last remote is absent", () => {
    expect(findLastRemote([{ remoteId: "a" }], "missing")).toBeUndefined();
  });
});
