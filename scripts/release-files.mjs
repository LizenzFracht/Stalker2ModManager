import { readdirSync } from "node:fs";
import { join } from "node:path";

export function findBundle(dir, version, suffix) {
  const matches = readdirSync(dir).filter((name) => name.includes(`_${version}_`) && name.endsWith(suffix));
  if (matches.length !== 1) throw new Error(`Expected one ${version} ${suffix} bundle in ${dir}; found ${matches.length}`);
  return join(dir, matches[0]);
}
