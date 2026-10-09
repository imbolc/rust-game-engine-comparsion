// Run after serving target/wasm-size on port 8765 and starting Chromium with
// --remote-debugging-port=9222. Uses Node 22's native fetch and WebSocket.
// For mobile, use a second Chromium with --force-device-scale-factor=2 and
// pass its debugging URL through --mobile-devtools=http://127.0.0.1:9223.
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";
import { inflateSync } from "node:zlib";

const devtoolsOption = process.argv.find((arg) => arg.startsWith("--devtools="));
const devtools = devtoolsOption?.slice("--devtools=".length) || process.env.DEVTOOLS_URL || "http://127.0.0.1:9222";
const mobileOption = process.argv.find((arg) => arg.startsWith("--mobile-devtools="));
const mobileDevtools = mobileOption?.slice("--mobile-devtools=".length) || process.env.MOBILE_DEVTOOLS_URL || devtools;
const site = process.env.GAME_URL || "http://127.0.0.1:8765";
const engines = process.argv.slice(2).filter((arg) => !arg.startsWith("--devtools=") && !arg.startsWith("--mobile-devtools="));
const output = fileURLToPath(new URL("../target/wasm-size/", import.meta.url));
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const results = [];

function bottomCells(png) {
  const width = png.readUInt32BE(16), height = png.readUInt32BE(20), color = png[25];
  if (png[24] !== 8 || ![2, 6].includes(color) || png[28] !== 0) throw new Error("Unsupported Chromium PNG format");
  const chunks = [];
  for (let offset = 8; offset < png.length;) {
    const length = png.readUInt32BE(offset);
    if (png.toString("ascii", offset + 4, offset + 8) === "IDAT") chunks.push(png.subarray(offset + 8, offset + 8 + length));
    offset += length + 12;
  }
  const channels = color === 6 ? 4 : 3, stride = width * channels;
  const raw = inflateSync(Buffer.concat(chunks)), pixels = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    for (let x = 0; x < stride; x++) {
      const index = y * stride + x;
      const a = x >= channels ? pixels[index - channels] : 0;
      const b = y ? pixels[index - stride] : 0;
      const c = y && x >= channels ? pixels[index - stride - channels] : 0;
      const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
      const predictor = [0, a, b, Math.floor((a + b) / 2), pa <= pb && pa <= pc ? a : pb <= pc ? b : c][filter];
      if (predictor === undefined) throw new Error(`Unsupported PNG filter ${filter}`);
      pixels[index] = (raw[y * (stride + 1) + x + 1] + predictor) & 255;
    }
  }
  const scale = width / 400;
  let occupied = 0;
  for (let row = 16; row < 20; row++) for (let column = 0; column < 10; column++) {
    const index = (Math.round((102 + 28 * row) * scale) * width + Math.round((74 + 28 * column) * scale)) * channels;
    if ([26, 31, 43].some((empty, channel) => Math.abs(pixels[index + channel] - empty) > 8)) occupied++;
  }
  return occupied;
}

async function connect(url, errors, mode) {
  const socket = new WebSocket(url);
  const pending = new Map();
  let sequence = 0;
  socket.addEventListener("message", ({ data }) => {
    const message = JSON.parse(data);
    if (message.id) {
      const task = pending.get(message.id);
      if (!task) return;
      pending.delete(message.id);
      clearTimeout(task.timer);
      message.error ? task.reject(new Error(JSON.stringify(message.error))) : task.resolve(message.result);
    } else if (message.method === "Runtime.exceptionThrown") {
      const detail = message.params.exceptionDetails;
      errors.push({ kind: "exception", mode, text: detail.exception?.description || detail.text });
    } else if (message.method === "Runtime.consoleAPICalled" && message.params.type === "error") {
      errors.push({ kind: "console", mode, text: message.params.args.map((arg) => arg.value ?? arg.description).join(" ") });
    } else if (message.method === "Log.entryAdded" && message.params.entry.level === "error") {
      const entry = message.params.entry;
      if (entry.url && new URL(entry.url, site).pathname === "/favicon.ico") return;
      errors.push({ kind: "browser", mode, text: entry.text, url: entry.url });
    }
  });
  await new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
  return {
    socket,
    call(method, params = {}) {
      return new Promise((resolve, reject) => {
        const id = ++sequence;
        const timer = setTimeout(() => { pending.delete(id); reject(new Error(`${method} timed out`)); }, 20000);
        pending.set(id, { resolve, reject, timer });
        socket.send(JSON.stringify({ id, method, params }));
      });
    },
  };
}

async function smoke(engine) {
  const result = { engine, errors: [], viewports: [], screenshots: [] };
  results.push(result);
  const directory = `${output}/${engine}`;
  await mkdir(directory, { recursive: true });
  let call;

  async function screenshot(name, expectedBottomCells) {
    const { data } = await call("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
    const bytes = Buffer.from(data, "base64");
    await writeFile(`${directory}/${name}.png`, bytes);
    const occupied = bottomCells(bytes);
    result.screenshots.push({ file: `${engine}/${name}.png`, sha256: createHash("sha256").update(bytes).digest("hex"), bottomCells: occupied });
    if (occupied !== expectedBottomCells) throw new Error(`${engine} ${name}: expected ${expectedBottomCells} occupied bottom cells, observed ${occupied}`);
  }

  async function key(key, code, windowsVirtualKeyCode) {
    await call("Input.dispatchKeyEvent", { type: "keyDown", key, code, windowsVirtualKeyCode });
    await pause(80);
    await call("Input.dispatchKeyEvent", { type: "keyUp", key, code, windowsVirtualKeyCode });
    await pause(100);
  }

  async function boot(mobile) {
    await call("Emulation.setDeviceMetricsOverride", { width: 400, height: 800, deviceScaleFactor: mobile ? 2 : 1, mobile });
    await call("Emulation.setTouchEmulationEnabled", { enabled: mobile, maxTouchPoints: 1 });
    await call("Page.navigate", { url: `${site}/${engine}/` });
    const deadline = Date.now() + 20000;
    while (Date.now() < deadline) {
      const { result: ready } = await call("Runtime.evaluate", {
        expression: `(() => { const c = document.querySelector('canvas'); return !!c && c.width >= 400 && c.height >= 800 && performance.getEntriesByType('resource').some(r => r.name.includes('.wasm') && r.responseEnd > 0); })()`,
        returnByValue: true,
      });
      if (ready.value) {
        await pause(1200);
        const { result: viewport } = await call("Runtime.evaluate", {
          expression: `(() => { const c = document.querySelector('canvas'); const r = c.getBoundingClientRect(); return { width: innerWidth, height: innerHeight, dpr: devicePixelRatio, canvasWidth: c.width, canvasHeight: c.height, cssWidth: r.width, cssHeight: r.height }; })()`,
          returnByValue: true,
        });
        result.viewports.push({ mobile, devtools: mobile ? mobileDevtools : devtools, ...viewport.value });
        const v = viewport.value;
        if (v.width !== 400 || v.height !== 800 || v.dpr !== (mobile ? 2 : 1)) {
          throw new Error(`${engine}: expected 400x800 at DPR ${mobile ? 2 : 1}, got ${v.width}x${v.height} at DPR ${v.dpr}`);
        }
        if (v.canvasWidth !== Math.round(v.cssWidth * v.dpr) || v.canvasHeight !== Math.round(v.cssHeight * v.dpr)) {
          throw new Error(`${engine}: canvas backing ${v.canvasWidth}x${v.canvasHeight} does not match CSS ${v.cssWidth}x${v.cssHeight} at DPR ${v.dpr}; use Chromium --force-device-scale-factor=${mobile ? 2 : 1}`);
        }
        return;
      }
      await pause(200);
    }
    throw new Error(`${engine} ${mobile ? "mobile" : "desktop"} WASM/canvas did not initialize within 20 seconds`);
  }

  for (const mobile of [false, true]) {
    const mode = mobile ? "mobile" : "desktop";
    const browser = mobile ? mobileDevtools : devtools;
    let target, socket;
    try {
      const response = await fetch(`${browser}/json/new?about:blank`, { method: "PUT" });
      if (!response.ok) throw new Error(`Cannot create Chromium target: ${response.status}`);
      target = await response.json();
      ({ socket, call } = await connect(target.webSocketDebuggerUrl, result.errors, mode));
      await call("Page.enable");
      await call("Runtime.enable");
      await call("Log.enable");
      console.log(`${engine}: ${mode} 400x800, DPR ${mobile ? 2 : 1}, ${browser}`);
      await boot(mobile);
      if (!mobile) await key(" ", "Space", 32);
      await screenshot(`${mode}-before`, 0);
      if (!mobile) {
        await key("ArrowLeft", "ArrowLeft", 37);
        await key("ArrowUp", "ArrowUp", 38);
      }
      await key("ArrowDown", "ArrowDown", 40);
      await screenshot(`${mode}-after-drop`, 4);
      if (mobile) {
        await call("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: 200, y: 734, id: 1 }] });
        await pause(80);
        await call("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
        await pause(150);
        await screenshot("mobile-after-touch-restart", 0);
      } else {
        await key("r", "KeyR", 82);
        await screenshot("desktop-after-key-restart", 0);
        await key("ArrowDown", "ArrowDown", 40);
        await screenshot("desktop-before-pointer-restart", 4);
        await call("Input.dispatchMouseEvent", { type: "mousePressed", x: 200, y: 734, button: "left", clickCount: 1 });
        await pause(80);
        await call("Input.dispatchMouseEvent", { type: "mouseReleased", x: 200, y: 734, button: "left", clickCount: 1 });
        await pause(100);
        await screenshot("desktop-after-pointer-restart", 0);
      }
    } catch (error) {
      result.errors.push({ kind: "smoke", mode, text: String(error) });
    } finally {
      socket?.close();
      if (target) await fetch(`${browser}/json/close/${target.id}`);
    }
  }
  await writeFile(`${directory}/smoke.json`, JSON.stringify(result, null, 2) + "\n");
  console.log(`${engine}: ${result.screenshots.length} screenshots, ${result.errors.length} errors`);
}

try {
  for (const engine of engines.length ? engines : ["bevy", "fyrox", "macroquad"]) await smoke(engine);
  await writeFile(`${output}/smoke.json`, JSON.stringify(results, null, 2) + "\n");
  if (results.some((result) => result.errors.length)) process.exitCode = 1;
} catch (error) {
  console.error(error);
  process.exitCode = 1;
}
