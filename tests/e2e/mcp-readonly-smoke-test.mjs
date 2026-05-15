import { spawn } from "child_process";
import { createInterface } from "readline";

const proc = spawn("./target/debug/chromancy", ["wled-config.toml"], { cwd: process.cwd() });

const rl = createInterface({ input: proc.stdout });

let reqId = 1;
const pending = new Map();

function send(method, params = {}) {
  const id = ++reqId;
  const msg = JSON.stringify({ jsonrpc: "2.0", id, method, params });
  return new Promise((resolve) => {
    pending.set(id, resolve);
    proc.stdin.write(msg + "\n");
  });
}

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
    } else {
      console.log(`  [unmatched] ${line}`);
    }
  } catch (e) {
    console.log(`  [parse error] ${line}`);
  }
});

proc.stderr.on("data", (data) => {
  console.log(`  [stderr] ${data.toString().trimEnd()}`);
});

async function run() {
  await new Promise((r) => setTimeout(r, 500));

  console.log("=== Initialize ===");
  const init = await send("initialize", {
    protocolVersion: "2024-11-05",
    capabilities: {},
    clientInfo: { name: "test", version: "1.0" },
  });
  console.log(JSON.stringify(init, null, 2));

  console.log("\n=== tools/list ===");
  const toolsList = await send("tools/list");
  const tools = toolsList.result?.tools || [];
  console.log(`Found ${tools.length} tools:`);
  for (const t of tools) console.log(`  - ${t.name}`);

  const readOnly = [
    { name: "list_groups", args: {} },
    { name: "list_devices", args: {} },
    { name: "list_devices", args: { group_name: "main_house" } },
    { name: "list_devices", args: { group_name: "nonexistent" } },
    { name: "get_device_info", args: { device_name: "wled-4" } },
    { name: "get_device_info", args: { device_name: "wled-999" } },
    { name: "get_device_state", args: { device_name: "wled-4" } },
    { name: "get_device_state", args: { device_name: "wled-999" } },
    { name: "get_group_status", args: { group_name: "main_house" } },
    { name: "get_group_status", args: { group_name: "nonexistent" } },
    { name: "get_fleet_status", args: {} },
    { name: "list_presets", args: { group_name: "main_house" } },
    { name: "list_presets", args: { group_name: "nonexistent" } },
    { name: "check_sync_health", args: {} },
    { name: "check_sync_health", args: { group_name: "main_house" } },
    { name: "check_sync_health", args: { group_name: "nonexistent" } },
    { name: "get_individual_state", args: { device_name: "wled-4" } },
    { name: "get_individual_state", args: { device_name: "wled-999" } },
  ];

  for (const t of readOnly) {
    const resp = await callTool(t.name, t.args);
    const result = resp.result;
    if (result?.content) {
      for (const item of result.content) {
        if (item.type === "text") {
          try {
            const parsed = JSON.parse(item.text);
            console.log(`<<< ${JSON.stringify(parsed, null, 2)}`);
          } catch {
            console.log(`<<< ${item.text}`);
          }
        } else {
          console.log(`<<< ${JSON.stringify(item)}`);
        }
      }
    } else if (resp.error) {
      console.log(`<<< JSON-RPC ERROR: ${JSON.stringify(resp.error)}`);
    } else {
      console.log(`<<< ${JSON.stringify(resp, null, 2)}`);
    }
  }

  proc.stdin.end();
  await new Promise((r) => proc.on("close", r));
  console.log("\n=== DONE ===");
}

run().catch((e) => {
  console.error(e);
  proc.kill();
  process.exit(1);
});
