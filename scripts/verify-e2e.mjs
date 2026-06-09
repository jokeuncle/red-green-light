// End-to-end verification:
// 1. opens the real frontend in headless Chromium (subscribes to SSE),
// 2. sends curl-equivalent POSTs to /events,
// 3. screenshots the page after each transition,
// 4. asserts the visible bulb matches the expected color.
//
// Run with the Tauri dev server already up (or just `pnpm dev`).
import { chromium } from "playwright";
import { mkdirSync } from "node:fs";

const OUT = "/tmp/rgl-e2e";
mkdirSync(OUT, { recursive: true });

async function postEvent(payload) {
  const r = await fetch("http://127.0.0.1:7878/events", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!r.ok) throw new Error(`POST /events failed: ${r.status}`);
}

const browser = await chromium.launch();
const ctx = await browser.newContext({
  viewport: { width: 280, height: 620 },
  deviceScaleFactor: 2,
});
const page = await ctx.newPage();
// `networkidle` never resolves because the SSE connection stays open.
await page.goto("http://localhost:5180/?preview=1", { waitUntil: "domcontentloaded" });
await page.waitForSelector(".tl-root", { timeout: 5000 });

// Reset to a clean slate.
await postEvent({ source: "claude-code", session_id: "verify-1", hook_event_name: "SessionEnd" });

const steps = [
  { label: "01-idle", expect: "green", pre: null },
  {
    label: "02-working",
    expect: "yellow",
    pre: {
      source: "claude-code",
      session_id: "verify-1",
      hook_event_name: "PreToolUse",
      tool_name: "Bash",
    },
  },
  {
    label: "03-waiting",
    expect: "red",
    pre: {
      source: "claude-code",
      session_id: "verify-1",
      hook_event_name: "Notification",
    },
  },
  {
    label: "04-idle-after-stop",
    expect: "green",
    pre: {
      source: "claude-code",
      session_id: "verify-1",
      hook_event_name: "Stop",
    },
  },
];

let pass = 0;
for (const s of steps) {
  if (s.pre) await postEvent(s.pre);
  // wait for the SSE-driven DOM mutation
  await page.waitForFunction(
    (expected) => {
      const el = document.querySelector(".tl-root");
      return !!el && el.getAttribute("data-active") === expected;
    },
    s.expect,
    { timeout: 3000 },
  );
  await page.waitForTimeout(350);
  const active = await page.getAttribute(".tl-root", "data-active");
  const ok = active === s.expect;
  const path = `${OUT}/${s.label}.png`;
  await page.screenshot({ path });
  console.log(
    `${ok ? "PASS" : "FAIL"}  ${s.label}: expect=${s.expect} got=${active}  shot=${path}`,
  );
  if (ok) pass++;
}

// Multi-session aggregation: open two sessions in different states, verify red wins.
await postEvent({ source: "claude-code", session_id: "agg-A", hook_event_name: "PreToolUse" });
await postEvent({ source: "claude-code", session_id: "agg-B", hook_event_name: "Notification" });
await page.waitForFunction(
  () => document.querySelector(".tl-root")?.getAttribute("data-active") === "red",
  null,
  { timeout: 3000 },
);
await page.waitForTimeout(350);
await page.screenshot({ path: `${OUT}/05-multi-aggregate.png` });
const aggBadge = await page.textContent(".session-badge").catch(() => "");
console.log(`PASS  multi-session aggregation → red, badge="${aggBadge}"`);
pass++;

// Cleanup
await postEvent({ source: "claude-code", session_id: "agg-A", hook_event_name: "SessionEnd" });
await postEvent({ source: "claude-code", session_id: "agg-B", hook_event_name: "SessionEnd" });
await postEvent({ source: "claude-code", session_id: "verify-1", hook_event_name: "SessionEnd" });

await browser.close();
console.log(`\n${pass}/${steps.length + 1} checks passed.`);
if (pass !== steps.length + 1) process.exit(1);
