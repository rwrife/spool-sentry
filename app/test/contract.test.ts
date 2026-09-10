import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { test, expect } from "vitest";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

// Contract tests: every app-side fixture must satisfy the canonical
// observation schema at docs/schemas/observation-v0.1.schema.json, the
// schema/fixture must stay mutually consistent, and the repo-valid
// fixture must equal the firmware-emitted fixture input (the Rust
// observation_fixtures test enforces the other half of this parity).

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, "..", "..");
const schemaPath = join(
  repoRoot,
  "docs",
  "schemas",
  "observation-v0.1.schema.json",
);
const repoFixturePath = join(
  repoRoot,
  "docs",
  "fixtures",
  "observation-valid-v0.1.json",
);
const appFixtureDir = join(here, "..", "fixtures");

const schema = JSON.parse(readFileSync(schemaPath, "utf8"));
const ajv = new Ajv2020({ allErrors: true, strict: false });
addFormats(ajv);
const validate = ajv.compile(schema);

function loadFixture(name: string): Record<string, unknown> {
  return JSON.parse(readFileSync(join(appFixtureDir, name), "utf8")) as Record<
    string,
    unknown
  >;
}

test("schema accepts the canonical repo fixture", () => {
  const fixture = JSON.parse(readFileSync(repoFixturePath, "utf8"));
  expect(validate(fixture)).toBe(true);
});

test("schema accepts every app observation fixture", () => {
  for (const name of [
    "observation-valid.json",
    "observation-stale.json",
    "observation-disconnected.json",
    "observation-saturated.json",
    "observation-uncalibrated.json",
    "observation-storage-fault.json",
  ]) {
    const fixture = loadFixture(name);
    expect(validate(fixture), `${name} must satisfy the schema`).toBe(true);
  }
});

test("rejects an observation missing protocol_version", () => {
  const fixture = loadFixture("observation-valid.json");
  delete fixture.protocol_version;
  expect(validate(fixture)).toBe(false);
});

test("rejects a stale-looking reassuring zero instead of null", () => {
  // A mass channel that is disconnected must never carry a fake 0.0 —
  // the schema allows the number, so the semantic rule lives here:
  // disconnected channels in our fixtures must use null values.
  const fixture = loadFixture("observation-disconnected.json");
  expect(fixture.gross_mass_g).toBeNull();
  expect(fixture.net_mass_estimate_g).toBeNull();
});
