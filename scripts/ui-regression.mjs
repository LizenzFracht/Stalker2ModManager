// Exercises real React interactions against the browser preview's IPC boundary.
// Does not launch the installed application or touch game/user data.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync, mkdirSync } from "node:fs";
import { resolve, join } from "node:path";
import puppeteer from "puppeteer-core";

const root = resolve(import.meta.dirname, "..");
const chrome = process.env.CHROME_PATH || [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  "/usr/bin/google-chrome", "/usr/bin/chromium",
].find(existsSync);
assert.ok(chrome, "Set CHROME_PATH to a Chrome/Chromium executable");
const port = 1431;
const server = spawn(process.execPath, [join(root, "node_modules/vite/bin/vite.js"), "--host", "127.0.0.1", "--port", String(port), "--strictPort"], {
  cwd: root, stdio: ["ignore", "pipe", "pipe"], windowsHide: true,
});
let serverLog = "";
server.stdout.on("data", (chunk) => { serverLog = (serverLog + chunk).slice(-2000); });
server.stderr.on("data", (chunk) => { serverLog = (serverLog + chunk).slice(-2000); });
const delay = (ms) => new Promise((r) => setTimeout(r, ms));
let browser;
try {
  let ready = false;
  for (let i = 0; i < 80; i++) {
    if (server.exitCode !== null) throw new Error(`Preview server exited: ${server.exitCode}\n${serverLog}`);
    try { if ((await fetch(`http://127.0.0.1:${port}`)).ok) { ready = true; break; } } catch {}
    await delay(200);
  }
  assert.ok(ready, `Preview server did not start\n${serverLog}`);
  browser = await puppeteer.launch({ executablePath: chrome, headless: true });
  const page = await browser.newPage();
  await page.setViewport({ width: 1320, height: 840 });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const open = async (query) => {
    await page.goto(`http://127.0.0.1:${port}/?${query}`, { waitUntil: "networkidle0" });
    await page.waitForSelector("nav button");
  };
  const clickNav = async (label) => {
    const button = await page.evaluateHandle((label) => [...document.querySelectorAll("nav button")].find((b) => b.innerText.trim() === label), label);
    assert.ok(button.asElement(), `Navigation item not found: ${label}`);
    await button.asElement().click();
    await button.dispose();
  };
  const out = join(root, ".local/ui-qa");
  mkdirSync(out, { recursive: true });
  for (const [lang, nav, label] of [
    ["en", "Game tweaks", "Protect Engine.ini from being overwritten"],
    ["de", "Spiel-Tweaks", "Engine.ini vor Überschreiben schützen"],
  ]) {
    await open(`lang=${lang}`);
    await clickNav(nav);
    const selector = `[role="switch"][aria-label="${label}"]`;
    await page.waitForSelector(selector);
    const number = await page.$('main input:not([type])');
    assert.ok(number, "Expected an editable active tweak");
    await number.click({ clickCount: 3 });
    await number.type("1.3");
    assert.equal(await number.evaluate((e) => e.value), "1.3");
    await page.click(selector);
    await page.waitForFunction((s) => document.querySelector(s)?.getAttribute("aria-checked") === "false" && !document.querySelector(s)?.disabled, {}, selector);
    assert.equal(await number.evaluate((e) => e.value), "1.3", "Protection change discarded the unsaved draft");
    await page.click(selector);
    await page.waitForFunction((s) => document.querySelector(s)?.getAttribute("aria-checked") === "true" && !document.querySelector(s)?.disabled, {}, selector);
    assert.equal(await number.evaluate((e) => e.value), "1.3");
    await page.screenshot({ path: join(out, `tweaks-${lang}.png`) });
    console.log(`PASS ${lang}: protection toggle, settings refresh and draft preservation`);
  }
  await open("lang=en&failProtection=1");
  await clickNav("Game tweaks");
  const protection = '[role="switch"][aria-label="Protect Engine.ini from being overwritten"]';
  await page.waitForSelector(protection);
  await page.click(protection);
  await page.waitForFunction(() => document.body.innerText.includes("Could not change file permissions"));
  assert.equal(await page.$eval(protection, (e) => e.getAttribute("aria-checked")), "true", "Failed operation changed the displayed setting");
  console.log("PASS failed permission change leaves the setting unchanged and reports the error");
  await open("lang=en&failProtectionSave=1");
  await clickNav("Game tweaks");
  await page.waitForSelector(protection);
  await page.click(protection);
  await page.waitForFunction(() => document.body.innerText.includes("Could not save state"));
  await page.waitForFunction((s) => document.querySelector(s)?.getAttribute("aria-checked") === "false", {}, protection);
  assert.ok((await page.$eval("body", (e) => e.innerText)).includes("Could not save state"));
  console.log("PASS partial save failure refreshes the actual protection state while retaining the error");
  await open("lang=en&ue4ss=failed&recovery=1");
  await clickNav("Live map");
  await page.waitForFunction(() => document.body.innerText.includes("UE4SS failed to start on the last game launch."));
  const text = await page.$eval("body", (e) => e.innerText);
  assert.ok(text.includes("GUObjectArray"));
  assert.ok(text.includes("S2MM-Live\\live.json"));
  assert.ok(text.includes("Mod recovered from the library (disabled)."));
  assert.ok(!text.includes("Game connected"), "Fatal UE4SS error must not show connected status");
  await page.screenshot({ path: join(out, "livemap-error-en.png") });
  await page.setViewport({ width: 980, height: 620 });
  assert.ok(await page.$$eval('[role="alert"]', (alerts) => alerts.every((e) => e.scrollWidth <= e.clientWidth)), "Diagnostic text overflows at minimum window width");
  console.log("PASS fatal UE4SS diagnostics, recovery notice, corrected path and minimum width");
  assert.deepEqual(errors, [], "Unexpected page errors");
} finally {
  if (browser) await browser.close();
  server.kill();
}
