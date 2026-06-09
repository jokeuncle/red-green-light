// Screenshot helper for visual iteration on the TrafficLight component.
// Usage: node scripts/snap.mjs <color|all> [--out=path]
import { chromium } from "playwright";
import { mkdirSync } from "node:fs";

const colorArg = process.argv[2] || "all";
const outDir = "/tmp/rgl-shots";
mkdirSync(outDir, { recursive: true });

const colors = colorArg === "all" ? ["red", "yellow", "green"] : [colorArg];

const browser = await chromium.launch();
const ctx = await browser.newContext({
  viewport: { width: 280, height: 620 },
  deviceScaleFactor: 2,
});
const page = await ctx.newPage();

for (const c of colors) {
  await page.goto(`http://localhost:5180/?color=${c}`, {
    waitUntil: "networkidle",
  });
  // wait for any css transitions
  await page.waitForTimeout(500);
  const path = `${outDir}/tl-${c}.png`;
  await page.screenshot({ path, fullPage: false });
  console.log(`saved ${path}`);
}

await browser.close();
