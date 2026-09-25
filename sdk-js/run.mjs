// Runs every example under examples/ against the server at $KIME_COMPAT_URL, in the runtime this
// script runs in, and exits non-zero if any of them fails. The SDK reads its base URL and key from
// the TypeSafe environment variables, so the examples are exactly as a TypeSafe user writes them.
import { spawnSync } from "node:child_process";
import { readdirSync } from "node:fs";

const url = process.env.KIME_COMPAT_URL ?? "http://127.0.0.1:8000";
const runtime = process.execPath;
const name = typeof Bun !== "undefined" ? `bun ${Bun.version}` : `node ${process.version}`;
const env = { ...process.env, TYPESAFE_BASE_URL: url, TYPESAFE_API_KEY: "kime-compat" };

let failed = 0;
const examples = readdirSync(new URL("examples/", import.meta.url)).filter((f) => f.endsWith(".mjs")).sort();
for (const file of examples) {
  const started = performance.now();
  const r = spawnSync(runtime, [`examples/${file}`], { cwd: import.meta.dirname, env, encoding: "utf8" });
  const ms = (performance.now() - started).toFixed(0);
  const ok = r.status === 0;
  failed += ok ? 0 : 1;
  console.log(`${ok ? "pass" : "FAIL"} ${file} on ${name} in ${ms} ms`);
  const out = (ok ? r.stdout : r.stdout + r.stderr).trim();
  if (out) console.log(out.replace(/^/gm, "  "));
}
console.log(`${examples.length - failed} of ${examples.length} examples pass against ${url}`);
process.exit(failed ? 1 : 0);
