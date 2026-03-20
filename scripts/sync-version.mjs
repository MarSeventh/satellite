import { existsSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageJsonPath = path.join(rootDir, "package.json");
const packageLockPath = path.join(rootDir, "package-lock.json");
const tauriConfigPath = path.join(rootDir, "src-tauri", "tauri.conf.json");
const cargoTomlPath = path.join(rootDir, "src-tauri", "Cargo.toml");

const changedFiles = [];

function readJson(filePath) {
  return JSON.parse(readFileSync(filePath, "utf8"));
}

function writeJson(filePath, value) {
  const next = `${JSON.stringify(value, null, 2)}\n`;
  const current = readFileSync(filePath, "utf8");

  if (current !== next) {
    writeFileSync(filePath, next, "utf8");
    changedFiles.push(path.relative(rootDir, filePath));
  }
}

function syncCargoVersion(filePath, version) {
  const current = readFileSync(filePath, "utf8");
  const next = current.replace(
    /(\[package\][\s\S]*?^version = ")([^"]+)(")/m,
    `$1${version}$3`
  );

  if (current !== next) {
    writeFileSync(filePath, next, "utf8");
    changedFiles.push(path.relative(rootDir, filePath));
  }
}

const packageJson = readJson(packageJsonPath);
const version = packageJson.version;

if (!version) {
  throw new Error("package.json version is missing");
}

const tauriConfig = readJson(tauriConfigPath);
tauriConfig.version = version;
writeJson(tauriConfigPath, tauriConfig);

if (existsSync(packageLockPath)) {
  const packageLock = readJson(packageLockPath);
  packageLock.version = version;
  packageLock.packages ??= {};
  packageLock.packages[""] ??= {};
  packageLock.packages[""].version = version;
  writeJson(packageLockPath, packageLock);
}

syncCargoVersion(cargoTomlPath, version);

if (changedFiles.length === 0) {
  console.log(`Version ${version} already synced`);
} else {
  console.log(`Synced version ${version} -> ${changedFiles.join(", ")}`);
}
