import { spawn } from "child_process";
import { createInterface } from "readline";

const proc = spawn("./target/debug/chromancy", ["wled-config.toml"], { cwd: process.cwd() });
const rl = createInterface({ input: proc.stdout });
let reqId = 1;
const pending = new Map();

function callTool(name, arguments_ = {}) {
  const id = ++reqId;
  const msg = JSON.stringify({ jsonrpc: "2.0", id, method: "tools/call", params: { name, arguments: arguments_ } });
  return new Promise((resolve) => {
    pending.set(id, resolve);
    console.log(`\n>>> ${name}(${JSON.stringify(arguments_)})`);
    proc.stdin.write(msg + "\n");
  });
}

rl.on("line", (line) => {
  try {
    const obj = JSON.parse(line);
    if (obj.id !== undefined && pending.has(obj.id)) {
      pending.get(obj.id)(obj);
      pending.delete(obj.id);
    }
  } catch {}
});

proc.stderr.on("data", (data) => {
  console.log(`  [stderr] ${data.toString().trimEnd()}`);
});

async function run() {
  await new Promise((r) => setTimeout(r, 500));
  proc.stdin.write(JSON.stringify({ jsonrpc: "2.0", id: 1, method: "initialize", params: { protocolVersion: "2024-11-05", capabilities: {}, clientInfo: { name: "test", version: "1.0" } } }) + "\n");
  await new Promise((r) => setTimeout(r, 200));
  proc.stdin.write(JSON.stringify({ jsonrpc: "2.0", id: 2, method: "tools/list" }) + "\n");
  await new Promise((r) => setTimeout(r, 200));

  const devices = ["wled-4", "wled-5", "wled-1"];
  for (const d of devices) {
    for (const tool of ["get_device_info", "get_device_state", "get_individual_state"]) {
      const resp = await callTool(tool, { device_name: d });
      const result = resp.result;
      if (result?.content) {
        for (const item of result.content) {
          if (item.type === "text") {
            try {
              const parsed = JSON.parse(item.text);
              if (parsed.error) console.log(`<<< ERROR: ${parsed.error}`);
              else console.log(`<<< OK (fields: ${Object.keys(parsed).join(", ")})`);
            } catch { console.log(`<<< ${item.text}`); }
          }
        }
      } else if (resp.error) {
        console.log(`<<< RPC ERROR: ${JSON.stringify(resp.error)}`);
      }
    }
  }

  proc.stdin.end();
  await new Promise((r) => proc.on("close", r));
  console.log("\n=== DONE ===");
}

run().catch((e) => { console.error(e); proc.kill(); process.exit(1); });
