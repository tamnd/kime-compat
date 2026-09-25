// All three question types, text and JSON state, .withResponse() and models.list, with the checks
// a caller of the SDK relies on.
import assert from "node:assert/strict";
import { choice, noul, score, TypeSafeClient } from "@typesafe-ai/sdk";

const client = new TypeSafeClient();

const models = await client.models.list();
assert.ok(models.length > 0);
for (const m of models) {
  assert.equal(typeof m.name, "string");
  assert.equal(typeof m.release_date, "string");
}

const questions = {
  team: choice("Which team should handle this", { billing: "Payment issues", technical: "Bugs", other: null }),
  urgency: score("How urgent is it", ["not urgent", "this week", "today", "right now"]),
  churn: noul("The customer threatens to leave", { true: "they say they will cancel", false: null }),
};
for (const state of [
  "Hi, we were billed twice for March. Refund the duplicate today or we cancel.",
  { ticket: 4411, body: "The export button does nothing on Safari.", plan: "team" },
]) {
  const { data, response, requestId } = await client.systemOne({ state, questions }).withResponse();
  assert.equal(response.status, 200);
  assert.ok(requestId === undefined || typeof requestId === "string");
  assert.deepEqual(Object.keys(data.answers), ["team", "urgency", "churn"]);
  assert.ok(!data.model.endsWith("-latest"), `model ${data.model} is an alias`);
  const { team, urgency, churn } = data.answers;
  assert.deepEqual(Object.keys(team.probabilities), ["billing", "technical", "other"]);
  assert.equal(team.probabilities[team.choice], Math.max(...Object.values(team.probabilities)));
  assert.deepEqual(Object.keys(urgency.probabilities), ["0", "1", "2", "3"]);
  assert.deepEqual(urgency.legend, { 0: "not urgent", 1: "this week", 2: "today", 3: "right now" });
  assert.ok(urgency.score >= 0 && urgency.score <= 3);
  assert.ok(churn.noul >= 0 && churn.noul <= 1);
  assert.equal(data.usage.output_tokens, 0);
  console.log(`${data.model}: team ${team.choice}, urgency ${urgency.score}, churn ${churn.noul}`);
}
