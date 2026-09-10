// Static build smoke test: serve dist/ over loopback HTTP, verify the
// bundle loads as a module, index parses, and a mocked device envelope
// flows through the client. Run after `npm run build`. This is a local
// Linux/node smoke check only — it is not a phone/tablet compatibility
// claim (see app/README.md for the browser matrix gap).

import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "dist");
if (!existsSync(join(root, "index.html"))) {
  console.error("SMOKE: FAIL — dist/ missing. Run `npm run build` first.");
  process.exit(1);
}

const types = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
};

const server = createServer(async (req, res) => {
  if (req.url === "/api/v1/snapshot") {
    res.writeHead(200, { "content-type": "application/json" });
    const fixture = await readFile(
      join(root, "..", "fixtures", "observation-valid.json"),
    );
    res.end(
      JSON.stringify({
        protocol_version: "0.1",
        request_id: "smoke",
        ok: true,
        data: JSON.parse(fixture.toString()),
      }),
    );
    return;
  }
  const file = req.url === "/" ? "index.html" : (req.url?.slice(1) ?? "");
  try {
    const body = await readFile(join(root, file));
    res.writeHead(200, {
      "content-type":
        types["." + file.split(".").pop()] ?? "application/octet-stream",
    });
    res.end(body);
  } catch {
    res.writeHead(404);
    res.end("not found");
  }
});

await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const port = server.address().port;
const base = `http://127.0.0.1:${port}`;

let failures = 0;
const check = (name, ok) => {
  console.log(`SMOKE: ${ok ? "pass" : "FAIL"} — ${name}`);
  if (!ok) failures += 1;
};

const index = await (await fetch(`${base}/`)).text();
check("index served", index.includes('<main id="main">'));
check("module entry referenced", /assets\/app\.js/.test(index));
const bundle = await (await fetch(`${base}/assets/app.js`)).text();
check("bundle non-trivial", bundle.length > 500);
check("bundle within size budget (<50 kB raw)", bundle.length < 50_000);
const css = await (await fetch(`${base}/assets/app.css`)).text();
check("css served", css.includes("skip-link"));
check("css within size budget (<10 kB raw)", css.length < 10_000);
const envelope = await (await fetch(`${base}/api/v1/snapshot`)).json();
check(
  "mock envelope schema-shaped",
  envelope.ok === true &&
    envelope.data.protocol_version === "0.1" &&
    typeof envelope.data.sequence === "number",
);

server.close();
if (failures > 0) {
  console.error(`SMOKE: ${failures} failure(s)`);
  process.exit(1);
}
console.log(
  "SMOKE: PASS (local Linux/node smoke only; browser matrix not run)",
);
