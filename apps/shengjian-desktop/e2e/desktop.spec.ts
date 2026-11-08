import { chromium, expect, test, type Browser, type Page } from "@playwright/test";

let browser: Browser;
let page: Page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
  const context = browser.contexts()[0];
  page = context.pages()[0] ?? (await context.newPage());
  await page.waitForSelector(".topbar", { timeout: 30_000 });
  await page.evaluate(() => {
    localStorage.setItem("shengjian-onboarding-complete", "1");
  });
  await page.reload();
  await page.waitForSelector(".topbar", { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser?.close();
});

async function go(label: string, heading: string) {
  await page.locator(".nav-item").filter({ hasText: label }).first().click();
  await expect(page.locator("h1")).toHaveText(heading);
}

test("shell loads with brand, status and command search", async () => {
  await expect(page.locator(".sidebar-brand-name")).toHaveText("声键");
  await expect(page.locator(".topbar-status")).toBeVisible();
  await expect(page.locator(".topbar-search-toggle")).toBeVisible();
  await expect(page.locator(".titlebar")).toBeVisible();
});

test("navigates all seven desktop pages", async () => {
  await go("连接", "连接遥控器");
  await go("语音", "语音输入与测试");
  await go("按键", "按键与方案");
  await go("宏", "宏");
  await go("历史", "历史");
  await go("设置", "设置");
  await go("关于", "关于");
});

test("Ctrl+K searches and navigates to settings tabs", async () => {
  await go("连接", "连接遥控器");
  await page.keyboard.press("Control+k");
  const input = page.locator(".topbar-search-field input");
  await expect(input).toBeVisible();
  await input.fill("设置");
  await page.keyboard.press("Enter");
  await expect(page.locator("h1")).toHaveText("设置");

  await page.getByRole("tab", { name: "数据与诊断" }).click();
  await expect(page.locator('[role="tabpanel"]')).toHaveAttribute("id", "panel-diagnostics");
});

test("opens the profile editor with the full binding matrix", async () => {
  await go("按键", "按键与方案");
  await page.getByRole("button", { name: "新建", exact: true }).click();
  await expect(page.locator(".editor-card")).toBeVisible();
  await expect(page.locator(".binding-row")).toHaveCount(11);
  await page.getByRole("button", { name: "取消", exact: true }).last().click();
  await expect(page.locator(".editor-card")).toHaveCount(0);
});
