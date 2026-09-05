#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const RETRY_EXIT = 75;
const CONFIG_EXIT = 78;
const CLIENT_NAME = "vampireotp-relay-bell";
const CLIENT_VERSION = "0.1.0";

function fail(message, code = RETRY_EXIT, details = {}) {
  log("failure", { message, ...details });
  process.exit(code);
}

function parseArgs(argv) {
  const values = {
    endpoint: "ws://127.0.0.1:45831",
    expectedPrompt: "scan the channel",
    logFile: null,
    mode: "deliver",
    stateFile: null,
    timeoutMs: 110_000,
  };
  const positional = [];
  for (let index = 0; index < argv.length; index += 1) {
    const item = argv[index];
    if (!item.startsWith("--")) {
      positional.push(...argv.slice(index));
      break;
    }
    const name = item.slice(2);
    if (!(name in values)) {
      throw new Error(`unknown option: ${item}`);
    }
    const value = argv[index + 1];
    if (value === undefined) {
      throw new Error(`missing value for ${item}`);
    }
    index += 1;
    values[name] = name === "timeoutMs" ? Number(value) : value;
  }
  if (!Number.isFinite(values.timeoutMs) || values.timeoutMs <= 0) {
    throw new Error("--timeoutMs must be a positive number");
  }
  return { ...values, positional };
}

let options;
try {
  options = parseArgs(process.argv.slice(2));
} catch (error) {
  process.stderr.write(`codex-app-server-bell: ${error.message}\n`);
  process.exit(CONFIG_EXIT);
}

function log(outcome, details = {}) {
  const row = JSON.stringify({
    timestamp: new Date().toISOString(),
    outcome,
    ...details,
  });
  if (options?.logFile) {
    fs.mkdirSync(path.dirname(options.logFile), { recursive: true });
    fs.appendFileSync(options.logFile, `${row}\n`, { encoding: "utf8" });
  }
  process.stderr.write(`${row}\n`);
}

if (!/^ws:\/\/127\.0\.0\.1:\d+$/.test(options.endpoint)) {
  fail("endpoint must be an explicit loopback WebSocket address", CONFIG_EXIT);
}
if (!['deliver', 'probe'].includes(options.mode)) {
  fail("--mode must be deliver or probe", CONFIG_EXIT);
}
if (options.mode === "deliver" && !options.stateFile) {
  fail("--stateFile is required", CONFIG_EXIT);
}
if (options.mode === "deliver" && (
  options.positional.length !== 4 ||
  options.positional[0] !== "exec" ||
  options.positional[1] !== "resume"
)) {
  fail("expected: exec resume <thread-id> <prompt>", CONFIG_EXIT);
}

const threadId = options.mode === "deliver" ? options.positional[2] : null;
const prompt = options.mode === "deliver" ? options.positional[3] : null;
if (options.mode === "deliver" && !/^[0-9a-f]{8}-[0-9a-f-]{27}$/.test(threadId)) {
  fail("invalid thread id", CONFIG_EXIT);
}
if (options.mode === "deliver" && prompt !== options.expectedPrompt) {
  fail("prompt does not match the fixed wake prompt", CONFIG_EXIT);
}

const promptSha256 = options.mode === "deliver"
  ? crypto.createHash("sha256").update(prompt).digest("hex")
  : null;
let socket;
let nextRequestId = 1;
const pendingRequests = new Map();
const completedTurns = new Map();

function atomicJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  const tempPath = `${filePath}.${process.pid}.tmp`;
  fs.writeFileSync(tempPath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  fs.renameSync(tempPath, filePath);
}

function loadState() {
  try {
    const parsed = JSON.parse(fs.readFileSync(options.stateFile, "utf8"));
    return parsed && typeof parsed === "object" ? parsed : null;
  } catch (error) {
    if (error.code === "ENOENT" || error instanceof SyntaxError) return null;
    throw error;
  }
}

function clearState() {
  try {
    fs.unlinkSync(options.stateFile);
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
}

function request(method, params) {
  const id = nextRequestId;
  nextRequestId += 1;
  return new Promise((resolve, reject) => {
    pendingRequests.set(id, { method, resolve, reject });
    socket.send(JSON.stringify({ id, method, params }));
  });
}

function activeTurn(thread) {
  const active = thread.turns.filter((turn) => turn.status === "inProgress");
  if (active.length !== 1) {
    throw new Error(
      `active thread must expose exactly one in-progress turn; found ${active.length}`,
    );
  }
  return active[0];
}

function waitForTurn(turnId) {
  if (completedTurns.has(turnId)) return Promise.resolve(completedTurns.get(turnId));
  return new Promise((resolve) => {
    const deadline = setTimeout(() => {
      log("accepted_turn_still_running", { threadId, turnId });
      process.exit(RETRY_EXIT);
    }, options.timeoutMs);
    const onCompleted = (event) => {
      const payload = event.detail;
      if (payload.threadId !== threadId || payload.turn.id !== turnId) return;
      clearTimeout(deadline);
      socket.removeEventListener("turn-completed", onCompleted);
      resolve(payload.turn);
    };
    socket.addEventListener("turn-completed", onCompleted);
  });
}

async function readThread() {
  return (await request("thread/read", { threadId, includeTurns: true })).thread;
}

async function ensureLoaded() {
  let thread = await readThread();
  if (thread.id !== threadId) throw new Error("thread/read returned the wrong thread");
  if (thread.status.type === "notLoaded") {
    const resumed = await request("thread/resume", { threadId });
    if (resumed.thread.id !== threadId) {
      throw new Error("thread/resume returned the wrong thread");
    }
    thread = await readThread();
  }
  return thread;
}

async function deliver() {
  let thread = await ensureLoaded();
  const remembered = loadState();
  if (
    remembered?.threadId === threadId &&
    remembered?.promptSha256 === promptSha256 &&
    typeof remembered?.turnId === "string"
  ) {
    if (thread.status.type === "active") {
      const turn = activeTurn(thread);
      if (turn.id === remembered.turnId) {
        log("awaiting_previously_accepted_turn", { threadId, turnId: turn.id });
        const completed = await waitForTurn(turn.id);
        clearState();
        log("previously_accepted_turn_completed", {
          threadId,
          turnId: turn.id,
          status: completed.status,
        });
        return;
      }
    }
    clearState();
    log("previous_delivery_already_settled", {
      threadId,
      turnId: remembered.turnId,
    });
    return;
  }

  if (thread.status.type === "systemError") {
    throw new Error("thread is in systemError state");
  }
  if (thread.status.type === "notLoaded") {
    throw new Error("thread remained notLoaded after resume");
  }

  let turnId;
  let delivery;
  if (thread.status.type === "idle") {
    const response = await request("turn/start", {
      threadId,
      clientUserMessageId: crypto.randomUUID(),
      input: [{ type: "text", text: prompt }],
    });
    turnId = response.turn.id;
    delivery = "turn_started";
  } else if (thread.status.type === "active") {
    if (thread.status.activeFlags.length > 0) {
      throw new Error(
        `active thread is waiting at an interaction boundary: ${thread.status.activeFlags.join(",")}`,
      );
    }
    const turn = activeTurn(thread);
    const response = await request("turn/steer", {
      threadId,
      expectedTurnId: turn.id,
      clientUserMessageId: crypto.randomUUID(),
      input: [{ type: "text", text: prompt }],
    });
    if (response.turnId !== turn.id) {
      throw new Error("turn/steer acknowledged an unexpected turn");
    }
    turnId = response.turnId;
    delivery = "turn_steered";
  } else {
    throw new Error(`unsupported thread state: ${thread.status.type}`);
  }

  atomicJson(options.stateFile, {
    schemaVersion: 1,
    threadId,
    turnId,
    promptSha256,
    acceptedAt: new Date().toISOString(),
  });
  log(delivery, { threadId, turnId });
  const completed = await waitForTurn(turnId);
  clearState();
  log("accepted_turn_completed", { threadId, turnId, status: completed.status });
}

try {
  socket = new WebSocket(options.endpoint);
} catch (error) {
  fail("failed to construct WebSocket", RETRY_EXIT, { error: error.message });
}

socket.addEventListener("message", (event) => {
  let message;
  try {
    message = JSON.parse(String(event.data));
  } catch (error) {
    fail("app-server emitted invalid JSON", RETRY_EXIT, { error: error.message });
  }
  if (message.id !== undefined && pendingRequests.has(message.id)) {
    const pending = pendingRequests.get(message.id);
    pendingRequests.delete(message.id);
    if (message.error) {
      pending.reject(
        new Error(`${pending.method} failed: ${JSON.stringify(message.error)}`),
      );
    } else {
      pending.resolve(message.result);
    }
    return;
  }
  if (message.method === "turn/completed" && message.params) {
    completedTurns.set(message.params.turn.id, message.params.turn);
    socket.dispatchEvent(
      new CustomEvent("turn-completed", { detail: message.params }),
    );
  }
});

socket.addEventListener("error", () => {
  fail("WebSocket connection error");
});

const connectionDeadline = setTimeout(() => {
  fail("timed out connecting to app-server");
}, 5_000);

socket.addEventListener("open", async () => {
  clearTimeout(connectionDeadline);
  try {
    await request("initialize", {
      clientInfo: {
        name: CLIENT_NAME,
        title: "VampireOTP Relay Bell",
        version: CLIENT_VERSION,
      },
      capabilities: {
        experimentalApi: true,
        optOutNotificationMethods: [
          "item/agentMessage/delta",
          "item/reasoning/summaryTextDelta",
          "item/reasoning/textDelta",
          "item/commandExecution/outputDelta",
        ],
      },
    });
    socket.send(JSON.stringify({ method: "initialized", params: {} }));
    if (options.mode === "probe") {
      const loaded = await request("thread/loaded/list", {});
      log("probe_ok", { loadedThreadIds: loaded.data });
      socket.close(1000, "probe complete");
      process.exit(0);
    }
    await deliver();
    socket.close(1000, "delivery settled");
    process.exit(0);
  } catch (error) {
    fail("delivery failed", RETRY_EXIT, { error: error.message });
  }
});
