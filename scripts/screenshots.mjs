// Renders the README/Nexus screenshots and GIFs from the UI preview (fake data).
//   node scripts/screenshots.mjs            screenshots + reorder GIF
//   node scripts/screenshots.mjs --map      also the live map GIF (needs internet;
//                                           runs `cargo run --example mapserver` itself)
// Needs Google Chrome (or set CHROME_PATH) and ffmpeg on PATH.
import { spawn, execFileSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { homedir, platform, tmpdir } from "node:os";
import { join, resolve } from "node:path";
import puppeteer from "puppeteer-core";

const root = resolve(import.meta.dirname, "..");
const out = join(root, "docs", "media");
mkdirSync(out, { recursive: true });
const mapOnly = process.argv.includes("--map-only");
const withMap = mapOnly || process.argv.includes("--map");

const chrome =
  process.env.CHROME_PATH ||
  [
    "C:/Program Files/Google/Chrome/Application/chrome.exe",
    "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
  ].find(existsSync);
if (!chrome) throw new Error("Chrome not found – set CHROME_PATH");

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const W = 1320;
const H = 820;

async function waitFor(url, tries = 120) {
  for (let i = 0; i < tries; i++) {
    try {
      const r = await fetch(url);
      if (r.ok) return;
    } catch {}
    await sleep(500);
  }
  throw new Error(`${url} did not come up`);
}

const vite = spawn("npx vite --port 1420 --strictPort", { cwd: root, shell: true, stdio: "ignore" });
const children = [vite];
const cleanup = () => children.forEach((c) => { try { process.platform === "win32" ? execFileSync("taskkill", ["/pid", String(c.pid), "/T", "/F"], { stdio: "ignore" }) : c.kill(); } catch {} });
process.on("exit", cleanup);

await waitFor("http://localhost:1420/");
const browser = await puppeteer.launch({ executablePath: chrome, headless: true, args: ["--hide-scrollbars"] });
const page = await browser.newPage();
page.on("pageerror", (e) => console.error("PAGE ERROR:", e.message));
page.on("console", (m) => m.type() === "error" && console.error("CONSOLE:", m.text().slice(0, 300)));
await page.setViewport({ width: W, height: H, deviceScaleFactor: 1.5 });
await page.emulateMediaFeatures([{ name: "prefers-color-scheme", value: "dark" }]);

const open = async (query = "") => {
  await page.goto(`http://localhost:1420/?lang=en${query}`, { waitUntil: "networkidle0" });
  await sleep(600);
};
const click = (selector, text) =>
  page.evaluate(
    (sel, txt) => {
      const el = [...document.querySelectorAll(sel)].find((e) => e.textContent.trim().startsWith(txt));
      if (!el) throw new Error(`no ${sel} "${txt}"`);
      el.click();
    },
    selector,
    text,
  );
const nav = async (label) => {
  await click("nav button", label);
  await sleep(700);
};
const shot = async (name) => {
  await sleep(400);
  await page.screenshot({ path: join(out, `${name}.png`) });
  console.log("  ", name);
};

// ---------------------------------------------------------------- GIF helpers
function gif(framesDir, name, fps, width) {
  const target = join(out, `${name}.gif`);
  execFileSync("ffmpeg", [
    "-y", "-loglevel", "error", "-framerate", String(fps), "-i", join(framesDir, "%04d.png"),
    "-vf", `scale=${width}:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=160:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle`,
    target,
  ]);
  console.log("  ", `${name}.gif`);
}

if (!mapOnly) {
console.log("Screenshots:");
await open();
await click("main span", "Zone Survival Overhaul");
await shot("01-mods");

await click("main span", "Clean PDA HUD");
await sleep(300);
await click("aside button", "Options");
await page.evaluate(() => {
  const label = [...document.querySelectorAll("[role=dialog] label")].find((l) => l.textContent.includes("Minimal"));
  label?.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
});
await shot("02-options");

await page.keyboard.press("Escape");
await nav("Conflicts");
await page.evaluate(() => document.querySelector("main ul li button")?.click());
await shot("03-conflicts");

await nav("Key bindings");
await shot("04-keybindings");

await nav("Mod settings");
await shot("05-mod-settings");

await nav("Game tweaks");
await shot("06-game-tweaks");

await nav("Live map");
await shot("07-live-map-setup");

await nav("Mods");
await click("main button", "Auto-sort");
await sleep(500);
await shot("08-autosort");

await open("&welcome=1");
await shot("09-restore-vanilla");

console.log("GIFs:");
// Drag "Quick Actions" from position 7 up to position 3.
{
  const frames = join(tmpdir(), "s2mm-gif-reorder");
  rmSync(frames, { recursive: true, force: true });
  mkdirSync(frames);
  await page.setViewport({ width: 1100, height: 640, deviceScaleFactor: 1 });
  await open();
  let n = 0;
  const frame = async () => page.screenshot({ path: join(frames, String(n++).padStart(4, "0") + ".png") });
  const handle = async (name) =>
    page.evaluate((nm) => {
      const row = [...document.querySelectorAll("main [aria-label]")].find((b) => b.getAttribute("aria-label")?.startsWith(nm));
      const r = row.getBoundingClientRect();
      return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
    }, name);
  for (let i = 0; i < 6; i++) await frame();
  const from = await handle("Quick Actions");
  const to = await handle("Zone Survival Overhaul");
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  for (let i = 1; i <= 18; i++) {
    await page.mouse.move(from.x, from.y + ((to.y - 6 - from.y) * i) / 18);
    await frame();
  }
  await page.mouse.up();
  for (let i = 0; i < 14; i++) {
    await sleep(60);
    await frame();
  }
  gif(frames, "reorder", 10, 1100);
}

}

if (withMap) {
  // Real map from joric.github.io via the S2MM map server, with a walking player.
  const local = process.platform === "win32" ? process.env.LOCALAPPDATA : join(homedir(), ".s2mm-dev");
  const liveDir = join(local, "S2MM");
  mkdirSync(liveDir, { recursive: true });
  const write = (x, y, yaw) => writeFileSync(join(liveDir, "live.json"), JSON.stringify({ x, y, z: 0, yaw, t: 0 }));
  write(402400, 578997, 0);
  const server = spawn("cargo run -q --example mapserver", { cwd: join(root, "src-tauri"), shell: true, stdio: "ignore" });
  children.push(server);
  await waitFor("http://127.0.0.1:27421/s2mm/live", 600);

  const frames = join(tmpdir(), "s2mm-gif-map");
  rmSync(frames, { recursive: true, force: true });
  mkdirSync(frames);
  await page.setViewport({ width: 1100, height: 640, deviceScaleFactor: 1 });
  await page.goto("http://127.0.0.1:27421/", { waitUntil: "networkidle2", timeout: 180000 });
  await sleep(6000);
  // A short walk with turns; the map stays put so only the player moves (small GIF).
  const path = [];
  let x = 402400, y = 578997, yaw = 0;
  for (let i = 0; i < 36; i++) {
    yaw = i < 12 ? 20 : i < 24 ? 75 : 140;
    x += Math.cos((yaw * Math.PI) / 180) * 1300;
    y += Math.sin((yaw * Math.PI) / 180) * 1300;
    path.push([x, y, yaw]);
  }
  const xs = path.map((p) => p[0]), ys = path.map((p) => p[1]);
  await page.evaluate((cx, cy) => {
    document.querySelector("#s2mm-live button")?.click(); // stop following
    const m = window.__s2mmMap?.();
    m?.setCenter([cx, cy]);
    m?.setZoom(m.getZoom() + 1.5);
  }, (Math.min(...xs) + Math.max(...xs)) / 2, (Math.min(...ys) + Math.max(...ys)) / 2);
  await sleep(4000);
  let n = 0;
  for (const [px, py, pyaw] of path) {
    write(px, py, pyaw);
    await sleep(1100);
    await page.screenshot({ path: join(frames, String(n++).padStart(4, "0") + ".png") });
  }
  gif(frames, "live-map", 5, 760);
  await page.screenshot({ path: join(out, "10-live-map.png") });
  rmSync(join(liveDir, "live.json"), { force: true });
}

await browser.close();
cleanup();
console.log(`\nWritten to ${out}`);
process.exit(0);
