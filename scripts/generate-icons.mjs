// Render tray + app icons from inline SVG via headless Chromium.
// Output:
//   src-tauri/icons/tray-{green,yellow,red}.png  44x44, color
//   src-tauri/icons/icon.png                     512x512, app icon
import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const ICONS_DIR = resolve(here, "..", "src-tauri", "icons");
mkdirSync(ICONS_DIR, { recursive: true });

// 3 colors used for both tray icons and the app icon.
const PALETTE = {
  green: { core: "#22dd66", glow: "#22dd66", deep: "#0a6a26" },
  yellow: { core: "#ffc81a", glow: "#ffb000", deep: "#7a5400" },
  red: { core: "#ff3030", glow: "#ff3030", deep: "#7a0808" },
};

function traySvg({ core, glow, deep }, size) {
  const r = size / 2 - 3;
  const cx = size / 2;
  const cy = size / 2;
  return `
<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}">
  <defs>
    <radialGradient id="g" cx="40%" cy="34%" r="62%">
      <stop offset="0%"   stop-color="#ffffff" stop-opacity="0.85"/>
      <stop offset="22%"  stop-color="${core}" stop-opacity="1"/>
      <stop offset="70%"  stop-color="${core}" stop-opacity="1"/>
      <stop offset="100%" stop-color="${deep}" stop-opacity="1"/>
    </radialGradient>
    <filter id="bloom" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="1.6" result="b"/>
      <feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
  </defs>
  <circle cx="${cx}" cy="${cy}" r="${r}" fill="${glow}" opacity="0.35"
          filter="url(#bloom)"/>
  <circle cx="${cx}" cy="${cy}" r="${r - 1}" fill="url(#g)"
          stroke="rgba(0,0,0,0.4)" stroke-width="0.8"/>
</svg>`;
}

function appIconSvg() {
  // Whole traffic light as the app icon. 512x512.
  return `
<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
  <defs>
    <linearGradient id="case" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#3a3e45"/>
      <stop offset="60%" stop-color="#1c1f24"/>
      <stop offset="100%" stop-color="#14171c"/>
    </linearGradient>
    <radialGradient id="r" cx="40%" cy="34%" r="60%">
      <stop offset="0%" stop-color="#ffd4d4"/>
      <stop offset="22%" stop-color="#ff5050"/>
      <stop offset="70%" stop-color="#d60000"/>
      <stop offset="100%" stop-color="#7a0000"/>
    </radialGradient>
    <radialGradient id="y" cx="40%" cy="34%" r="60%">
      <stop offset="0%" stop-color="#fff4c6"/>
      <stop offset="22%" stop-color="#ffc81a"/>
      <stop offset="70%" stop-color="#b87800"/>
      <stop offset="100%" stop-color="#6e4600"/>
    </radialGradient>
    <radialGradient id="g" cx="40%" cy="34%" r="60%">
      <stop offset="0%" stop-color="#d6ffe2"/>
      <stop offset="22%" stop-color="#22dd66"/>
      <stop offset="70%" stop-color="#0a8a30"/>
      <stop offset="100%" stop-color="#04501c"/>
    </radialGradient>
  </defs>
  <rect width="512" height="512" rx="96" ry="96" fill="#0b0d12"/>
  <rect x="146" y="42" width="220" height="428" rx="56" ry="56" fill="url(#case)"
        stroke="rgba(255,255,255,0.06)" stroke-width="2"/>
  <circle cx="256" cy="142" r="62" fill="url(#r)"/>
  <circle cx="256" cy="256" r="62" fill="url(#y)" opacity="0.95"/>
  <circle cx="256" cy="370" r="62" fill="url(#g)"/>
</svg>`;
}

function htmlFor(svg, size, transparent = true) {
  return `<!doctype html><html><head><meta charset="utf-8"><style>
    html,body{margin:0;padding:0;width:${size}px;height:${size}px;
      background:${transparent ? "transparent" : "#000"};}
  </style></head><body>${svg}</body></html>`;
}

const browser = await chromium.launch();
const ctx = await browser.newContext({ deviceScaleFactor: 1 });
const page = await ctx.newPage();

for (const name of ["green", "yellow", "red"]) {
  const size = 44;
  await page.setViewportSize({ width: size, height: size });
  await page.setContent(htmlFor(traySvg(PALETTE[name], size), size), {
    waitUntil: "load",
  });
  const buf = await page.screenshot({
    omitBackground: true,
    type: "png",
    clip: { x: 0, y: 0, width: size, height: size },
  });
  const path = resolve(ICONS_DIR, `tray-${name}.png`);
  writeFileSync(path, buf);
  console.log(`wrote ${path} (${buf.length} bytes)`);
}

{
  // App icon: keep transparent canvas so the PNG has an alpha channel (Tauri
  // bundler requires RGBA). The dark rounded background is drawn inside the
  // SVG so the visual still has a "card".
  const size = 512;
  await page.setViewportSize({ width: size, height: size });
  await page.setContent(htmlFor(appIconSvg(), size, true), { waitUntil: "load" });
  const buf = await page.screenshot({
    omitBackground: true,
    type: "png",
    clip: { x: 0, y: 0, width: size, height: size },
  });
  const path = resolve(ICONS_DIR, "icon.png");
  writeFileSync(path, buf);
  console.log(`wrote ${path} (${buf.length} bytes)`);
}

await browser.close();
