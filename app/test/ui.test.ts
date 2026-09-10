// Component/accessibility smoke tests over the real page markup.
// jsdom + @testing-library/dom. Checks keyboard-reachable controls,
// live-region announcements, non-color-only status text, and labelled
// forms. Browser compatibility claims remain scoped to this Linux
// environment (jsdom); real-device matrix runs are separate work.

import { test, expect, beforeEach } from "vitest";
import { JSDOM } from "jsdom";

function documentFromIndex(): Document {
  // Mirrors index.html; kept in sync via the smoke script build check.
  const dom = new JSDOM(
    `<!doctype html><html lang="en"><head><title>Spool Sentry</title></head>
     <body>
       <a class="skip-link" href="#main">Skip to main content</a>
       <header><h1>Spool Sentry</h1><p id="tagline"></p></header>
       <div id="status-region" role="status" aria-live="polite"></div>
       <main id="main"></main>
     </body></html>`,
    { url: "http://spool-sentry.local/" },
  );
  (globalThis as { document: Document }).document = dom.window.document;
  (globalThis as { window: unknown }).window = dom.window;
  return dom.window.document;
}

beforeEach(() => {
  documentFromIndex();
});

test("page has a landmark structure and skip link", async () => {
  const doc = globalThis.document;
  // Render the app UI into the empty document via main.render().
  const { render } = await import("../src/main.ts");
  render();
  expect(doc.querySelector("main#main")).not.toBeNull();
  expect(doc.querySelector(".skip-link")).not.toBeNull();
  const live = doc.getElementById("status-region");
  expect(live?.getAttribute("aria-live")).toBe("polite");
});

test("every button is a native focusable control with text", async () => {
  const doc = globalThis.document;
  const { render } = await import("../src/main.ts");
  render();
  const buttons = [...doc.querySelectorAll("button")];
  expect(buttons.length).toBeGreaterThan(0);
  for (const button of buttons) {
    expect(button.textContent?.trim().length).toBeGreaterThan(2);
  }
});

test("no-color-only status: stale state is spelled in text", async () => {
  const doc = globalThis.document;
  const { render } = await import("../src/main.ts");
  render();
  const text = doc.getElementById("main")?.textContent ?? "";
  // Offline initial state must say so in words, not just a red dot.
  expect(
    text + (doc.getElementById("status-region")?.textContent ?? ""),
  ).toMatch(/offline|no data|connect/i);
});

test("form controls have associated labels", async () => {
  const doc = globalThis.document;
  const { render } = await import("../src/main.ts");
  render();
  const inputs = [...doc.querySelectorAll("input, select")];
  for (const input of inputs) {
    const id = input.id;
    expect(id, "inputs need ids for labels").toBeTruthy();
    const label = doc.querySelector(`label[for="${id}"]`);
    expect(label, `label missing for ${id}`).not.toBeNull();
  }
});
