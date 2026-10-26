// Local release build: same packages as CI, without local paths embedded in the binary.
//   npm run release            Windows: setup + portable zip, Linux: .deb + AppImage
// Output goes to ./release/.
import { execSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { homedir, platform } from "node:os";
import { join, resolve } from "node:path";
import { findBundle } from "./release-files.mjs";

const root = resolve(import.meta.dirname, "..");
const version = JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8")).version;
const win = platform() === "win32";

// Rust embeds source paths (e.g. ~/.cargo/registry/...) for panic messages; map them away.
const remaps = [
  `--remap-path-prefix=${homedir()}=~`,
  `--remap-path-prefix=${root}=s2mm`,
];
// Native compression libraries also embed __FILE__ in debug assertions. Rust's
// path remapping does not cover C/C++; use normal release-mode assertions there.
const nativeReleaseFlag = win ? "/DNDEBUG" : "-DNDEBUG";
const env = {
  ...process.env,
  RUSTFLAGS: [process.env.RUSTFLAGS, ...remaps].filter(Boolean).join(" "),
  CFLAGS: [process.env.CFLAGS, nativeReleaseFlag].filter(Boolean).join(" "),
  CXXFLAGS: [process.env.CXXFLAGS, nativeReleaseFlag].filter(Boolean).join(" "),
};

const run = (cmd) => execSync(cmd, { cwd: root, stdio: "inherit", env });

run("node scripts/third-party-notices.mjs");
run(`npx tauri build --bundles ${win ? "nsis" : "deb,appimage"}`);

const out = join(root, "release");
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
const bundle = join(root, "src-tauri/target/release/bundle");

if (win) {
  copyFileSync(findBundle(join(bundle, "nsis"), version, "-setup.exe"), join(out, `S2MM_${version}_windows_x64_setup.exe`));
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
  copyFileSync(findBundle(join(bundle, "deb"), version, ".deb"), join(out, `S2MM_${version}_linux_amd64_setup.deb`));
  copyFileSync(findBundle(join(bundle, "appimage"), version, ".AppImage"), join(out, `S2MM_${version}_linux_x86_64_portable.AppImage`));
}
console.log(`\nPackages in ${out}`);
