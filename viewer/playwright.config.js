// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./tests",
  projects: [
    { name: "chromium", use: { deviceScaleFactor: 1 } },
    { name: "chromium-hidpi", use: { deviceScaleFactor: 2 } },
  ],
  timeout: 60000,
  workers: 1,
  maxFailures: process.env.CI ? 1 : 0,
  use: {
    channel: process.env.SW_VIEWER_BROWSER_CHANNEL || undefined,
    baseURL: process.env.SW_VIEWER_BASE_URL || "http://127.0.0.1:8097/",
    viewport: { width: 1280, height: 850 },
    launchOptions: {
      args: [
        "--enable-unsafe-webgpu",
        "--enable-unsafe-swiftshader",
        ...(process.platform === "linux"
          ? ["--use-vulkan=swiftshader", "--use-webgpu-adapter=swiftshader"]
          : []),
        "--use-angle=swiftshader",
        "--enable-features=Vulkan",
        "--disable-vulkan-surface",
      ],
    },
  },
  webServer: process.env.SW_VIEWER_BASE_URL
    ? undefined
    : {
        command: "npm run serve",
        url: "http://127.0.0.1:8097",
        reuseExistingServer: true,
      },
});
