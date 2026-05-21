import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const electronRoot = path.resolve(__dirname, "..");
const workspaceRoot = path.resolve(electronRoot, "..", "..");
const hostRoot = path.join(workspaceRoot, "frameworks", "SpherePluginHost");
const debug = process.argv.includes("--debug");
const cargoArgs = ["build", ...(debug ? [] : ["--release"] )];

function run(command, args, options = {}) {
  return spawnSync(command, args, {
    cwd: hostRoot,
    stdio: "inherit",
    shell: false,
    ...options,
  });
}

function hasMsvcCompiler() {
  if (process.platform !== "win32") return true;
  const result = spawnSync("where", ["cl"], { stdio: "ignore", shell: false });
  return result.status === 0;
}

let result;
if (process.platform === "win32" && !hasMsvcCompiler()) {
  const powershell = "C:\\Windows\\SysWOW64\\WindowsPowerShell\\v1.0\\powershell.exe";
  const devShellModule = "C:\\Program Files\\Microsoft Visual Studio\\18\\Community\\Common7\\Tools\\Microsoft.VisualStudio.DevShell.dll";
  const cargoCommand = `cargo ${cargoArgs.join(" ")}`;
  const script = `&{Import-Module "${devShellModule}"; Enter-VsDevShell ca952b24; ${cargoCommand}; exit $LASTEXITCODE}`;
  console.log("[build-plugin-host] MSVC cl.exe not found; entering Visual Studio DevShell before cargo build.");
  result = spawnSync(powershell, ["-noe", "-c", script], {
    cwd: hostRoot,
    stdio: "inherit",
    shell: false,
  });
} else {
  result = run("cargo", cargoArgs);
}

process.exit(result.status ?? 1);
