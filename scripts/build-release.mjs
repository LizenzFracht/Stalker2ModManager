// Local release build: same packages as CI, without local paths embedded in the binary.
//   npm run release            Windows: setup + portable zip, Linux: .deb + AppImage
// Output goes to ./release/.
import { execSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { homedir, platform } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const version = JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8")).version;
const win = platform() === "win32";

// Rust embeds source paths (e.g. ~/.cargo/registry/...) for panic messages; map them away.
const remaps = [
  `--remap-path-prefix=${homedir()}=~`,
  `--remap-path-prefix=${root}=s2mm`,
];
const env = { ...process.env, RUSTFLAGS: [process.env.RUSTFLAGS, ...remaps].filter(Boolean).join(" ") };

const run = (cmd) => execSync(cmd, { cwd: root, stdio: "inherit", env });

run("node scripts/third-party-notices.mjs");
run(`npx tauri build --bundles ${win ? "nsis" : "deb,appimage"}`);

const out = join(root, "release");
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
const bundle = join(root, "src-tauri/target/release/bundle");
const first = (dir, ext) => join(dir, readdirSync(dir).find((f) => f.endsWith(ext)));

if (win) {
  copyFileSync(first(join(bundle, "nsis"), "-setup.exe"), join(out, `S2MM_${version}_windows_x64_setup.exe`));
  const portable = join(out, "portable");
  mkdirSync(portable);
  copyFileSync(join(root, "src-tauri/target/release/stalker2mm.exe"), join(portable, "S2MM.exe"));
  writeFileSync(join(portable, "portable.txt"), "");
  for (const f of ["LICENSE", "THIRD_PARTY_NOTICES.md"]) copyFileSync(join(root, f), join(portable, f));
  execSync(
    `powershell -NoProfile -Command "Compress-Archive -Path '${portable}\\*' -DestinationPath '${join(out, `S2MM_${version}_windows_x64_portable.zip`)}'"`,
    { stdio: "inherit" },
  );
  rmSync(portable, { recursive: true });
} else {
  copyFileSync(first(join(bundle, "deb"), ".deb"), join(out, `S2MM_${version}_linux_amd64_setup.deb`));
  copyFileSync(first(join(bundle, "appimage"), ".AppImage"), join(out, `S2MM_${version}_linux_x86_64_portable.AppImage`));
}
console.log(`\nPackages in ${out}`);
