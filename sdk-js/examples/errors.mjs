// What the SDK throws when kime-serve refuses a request, and that the error classes line up.
import assert from "node:assert/strict";
import { NotFoundError, noul, TypeSafeClient, TypeSafeError, UnprocessableEntityError } from "@typesafe-ai/sdk";

const client = new TypeSafeClient({ retry: { maxRetries: 0 } });

assert.throws(() => client.systemOne({ state: "x", questions: {} }), TypeSafeError);

const missing = await client.systemOne({ state: "x", questions: { a: noul() }, model: "no-such-model" }).catch((e) => e);
assert.ok(missing instanceof NotFoundError, `got ${missing}`);
assert.equal(missing.status, 404);
console.log(`unknown model: ${missing.message}`);

const bad = await client.systemOne({ state: "x", questions: { a: { type: "choice", criteria: {} } } }).catch((e) => e);
assert.ok(bad instanceof UnprocessableEntityError, `got ${bad}`);
assert.equal(bad.status, 422);
console.log(`empty choice: ${bad.message}`);
