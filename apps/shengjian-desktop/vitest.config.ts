import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
    setupFiles: ["./src/test-setup.ts"],
    // Node 26 默认开启实验性 Web Storage：globalThis.localStorage 是返回 undefined
    // 的 getter，会遮蔽 jsdom 注入的 window.localStorage，依赖 localStorage 的测试
    // 会拿到 undefined。测试 worker 里关掉它，行为与 CI 使用的 Node 24 一致。
    poolOptions: {
      forks: { execArgv: ["--no-experimental-webstorage"] },
    },
  },
});
