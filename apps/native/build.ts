// ─────────────────────────────────────────────────────────────────────────────
//  Mochi DAW · native build driver  (apps/native/build.ts)
//
//  Mirrors frameworks/SphereEngine/build.ts so the two projects feel the same
//  on the CLI.  Drives CMake configure + build for the MochiDAW executable
//  and forwards the framework's prebuilt / fetch flags down to the
//  SphereEngine sub-build that this directory pulls in via add_subdirectory.
//
//  Usage (from this directory):
//
//      bun run build                  # configure (if needed) + build MochiDAW
//      bun run build configure        # force reconfigure
//      bun run build --debug          # build the Debug config
//      bun run build --target <t>     # build a different CMake target
//      bun run build --framework      # build only the SphereKit shared libs
//      bun run build --run            # build then launch the exe
//
//  Honoured environment variables (all optional, picked up from this folder's
//  .env first, then from the SphereEngine .env, then from the parent shell):
//      CMAKE_BUILD_TYPE      Debug | Release          (default Release)
//      CMAKE_GENERATOR       Ninja | "Visual Studio …" (default Ninja)
//      CMAKE_PATH            override cmake exe name
//      NATIVE_BUILD_DIR      relative build folder    (default ./build)
//      USE_PREBUILT          0/1                      (default 1)
//      FETCH_PREBUILT        0/1                      (default 1)
//      PREBUILT_DIR          override SPHERE_PREBUILT_DIR
//      SKIA_DIR  V8_DIR      override prebuilt roots
//      SKIA_URL  V8_URL      override prebuilt download URLs
//      VS_PATH               Visual Studio install path (for vcvars64.bat)
// ─────────────────────────────────────────────────────────────────────────────

import { existsSync, readFileSync } from "node:fs";
import { cpus } from "node:os";
import { isAbsolute, join, resolve } from "node:path";

// ── Self-contained env helpers ──────────────────────────────────────────────
// Kept inline so this script has no cross-dir TS imports and can run from a
// checkout that doesn't yet have the framework's `scripts/` folder ready.
function loadDotEnv(root: string): void {
  const envPath = join(root, ".env");
  if (!existsSync(envPath)) return;
  const text = readFileSync(envPath, "utf8");
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;
    const eq = line.indexOf("=");
    if (eq <= 0) continue;
    const key = line.slice(0, eq).trim();
    let value = line.slice(eq + 1).trim();
    if ((value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))) {
      value = value.slice(1, -1);
    }
    if (process.env[key] === undefined) process.env[key] = value;
  }
}

function envFlag(name: string, fallback: boolean): boolean {
  const value = process.env[name];
  if (value === undefined || value === "") return fallback;
  return ["1", "true", "yes", "on"].includes(value.toLowerCase());
}

function cmakeBool(value: boolean): string {
  return value ? "ON" : "OFF";
}

// ── Paths & defaults ────────────────────────────────────────────────────────
const root       = import.meta.dir;
const sphereRoot = resolve(root, "..", "..", "frameworks", "SphereEngine");

// Load env files in the order the framework expects: local overrides win,
// but the framework's prebuilt URLs / paths fill the gaps.
loadDotEnv(root);
loadDotEnv(sphereRoot);

const buildDir = join(root, process.env.NATIVE_BUILD_DIR ?? "build");
const config   = process.env.CMAKE_BUILD_TYPE ?? (hasArg("--debug") ? "Debug" : "Release");
const cmake    = process.env.NATIVE_CMAKE ?? process.env.CMAKE_PATH ?? "cmake";

// Single-target project for now — every example in the framework gets its
// own target, but the native app is just `MochiDAW`.  Framework libs can
// still be built via --framework for incremental rebuilds.
const nativeTargets = ["MochiDAW"];

const frameworkTargets = [
  "SphereKit_Foundation",
  "SphereKit_GraphicInterface",
  "SphereKit_GraphicComponent",
  ...(process.platform === "win32" ? ["SphereKit_DirectAudioEngine"] : []),
];

// ── Argv helpers ────────────────────────────────────────────────────────────
function args(): string[] { return Bun.argv.slice(2); }

function hasArg(...names: string[]): boolean {
  return args().some((arg) => names.includes(arg));
}

function jobs(): string {
  return process.env.NUMBER_OF_PROCESSORS ?? String(cpus().length || 4);
}

async function run(command: string[], options: { cwd?: string; env?: Record<string, string> } = {}): Promise<void> {
  console.log(`> ${command.join(" ")}`);
  const proc = Bun.spawn(command, {
    cwd: options.cwd ?? root,
    env: { ...process.env, ...options.env },
    stdout: "inherit",
    stderr: "inherit",
    stdin: "inherit",
  });
  const code = await proc.exited;
  if (code !== 0) {
    throw new Error(`${command[0]} exited with code ${code}`);
  }
}

// ── MSVC environment ────────────────────────────────────────────────────────
// Same VS-discovery list the framework uses, so a developer who has VS 2022
// installed in a default location doesn't need to launch a Developer Prompt
// before invoking `bun run build`.
async function importMsvcEnvironment(): Promise<void> {
  if (process.platform !== "win32" || process.env.VSCMD_ARG_TGT_ARCH) return;

  const configuredVsPath = process.env.VS_PATH;
  const candidates = [
    ...(configuredVsPath
      ? [
          configuredVsPath.endsWith(".bat")
            ? configuredVsPath
            : join(configuredVsPath, "VC", "Auxiliary", "Build", "vcvars64.bat"),
        ]
      : []),
    "C:\\Program Files\\Microsoft Visual Studio\\18\\Community\\VC\\Auxiliary\\Build\\vcvars64.bat",
    "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Auxiliary\\Build\\vcvars64.bat",
    "C:\\Program Files\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Auxiliary\\Build\\vcvars64.bat",
    "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Auxiliary\\Build\\vcvars64.bat",
    "C:\\Program Files\\Microsoft Visual Studio\\2022\\Professional\\VC\\Auxiliary\\Build\\vcvars64.bat",
  ];
  const vcvars = candidates.find(existsSync);
  if (!vcvars) return;

  const psCommand = `$envLines = cmd /c '"${vcvars}" >nul && set'; $envLines`;
  const proc = Bun.spawn(["powershell.exe", "-NoProfile", "-Command", psCommand], {
    stdout: "pipe",
    stderr: "inherit",
  });
  const output = await new Response(proc.stdout).text();
  const code = await proc.exited;
  if (code !== 0) {
    throw new Error("vcvars64.bat failed");
  }

  for (const line of output.split(/\r?\n/)) {
    const eq = line.indexOf("=");
    if (eq > 0) process.env[line.slice(0, eq)] = line.slice(eq + 1);
  }
}

// ── Configure ───────────────────────────────────────────────────────────────
async function configure(): Promise<void> {
  await importMsvcEnvironment();
  const generator = process.env.CMAKE_GENERATOR ?? "Ninja";

  // Forward framework knobs straight through.  The native CMakeLists is a
  // thin wrapper around add_subdirectory(SphereEngine), so any `-D` we set
  // here flows into the framework's own configure.
  const cmakeDefinitions = [
    `-DSPHERE_USE_PREBUILT=${cmakeBool(envFlag("USE_PREBUILT", true))}`,
    `-DSPHERE_FETCH_PREBUILT=${cmakeBool(envFlag("FETCH_PREBUILT", true))}`,
    `-DMOCHI_DAW_SPHERE_ROOT=${sphereRoot}`,
  ];

  const envToCmake: Record<string, string> = {
    PREBUILT_DIR: "SPHERE_PREBUILT_DIR",
    SKIA_DIR:     "SKIA_DIR",
    V8_DIR:       "V8_DIR",
    SKIA_URL:     "SKIA_URL",
    V8_URL:       "V8_URL",
  };
  for (const [envName, cmakeName] of Object.entries(envToCmake)) {
    let value = process.env[envName];
    if (!value) continue;
    // *_DIR overrides are resolved against the framework root so users can
    // drop relative paths in .env exactly like the framework expects.
    if (envName.endsWith("_DIR") && !isAbsolute(value)) {
      value = join(sphereRoot, value);
    }
    cmakeDefinitions.push(`-D${cmakeName}=${value}`);
  }

  await run([
    cmake,
    "-S", root,
    "-B", buildDir,
    "-G", generator,
    `-DCMAKE_BUILD_TYPE=${config}`,
    ...cmakeDefinitions,
  ]);
}

async function ensureConfigured(): Promise<void> {
  if (hasArg("--configure") || !existsSync(join(buildDir, "CMakeCache.txt"))) {
    await configure();
  } else {
    await importMsvcEnvironment();
  }
}

async function buildTargets(targets: string[]): Promise<void> {
  await ensureConfigured();
  for (const target of targets) {
    await run([cmake, "--build", buildDir, "--target", target, "--config", config, "-j", jobs()]);
  }
}

function requestedTargets(): string[] {
  const targetIndex = args().findIndex((arg) => arg === "--target" || arg === "-t");
  if (targetIndex >= 0 && args()[targetIndex + 1]) {
    return [args()[targetIndex + 1]];
  }
  if (hasArg("--framework") || hasArg("framework")) return frameworkTargets;
  if (hasArg("--all") || hasArg("all")) return [...frameworkTargets, ...nativeTargets];
  return nativeTargets;
}

async function runExecutable(): Promise<void> {
  const exeName = process.platform === "win32" ? "MochiDAW.exe" : "MochiDAW";
  // Ninja drops the exe directly under build/, but multi-config generators
  // (Visual Studio) stage it under build/<Config>/.  CMake's POST_BUILD also
  // copies it to build/ so this single path resolves either way.
  const exe = join(buildDir, exeName);
  if (!existsSync(exe)) {
    throw new Error(`Executable not found at ${exe} — did the build succeed?`);
  }
  await run([exe]);
}

// ── Entry ───────────────────────────────────────────────────────────────────
async function main(): Promise<void> {
  if (hasArg("configure")) {
    await configure();
    return;
  }

  await buildTargets(requestedTargets());

  if (hasArg("--run", "run")) {
    await runExecutable();
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
});
