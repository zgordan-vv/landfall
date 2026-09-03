import { execFileSync } from "node:child_process";

const allowedLicenses = new Set([
  "Apache-2.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "MIT",
  "MPL-2.0",
  "Unicode-3.0",
  "Zlib",
]);

const reportText = execFileSync("pnpm", ["licenses", "list", "--json", "--long"], {
  encoding: "utf8",
  env: { ...process.env, NO_COLOR: "1" },
  maxBuffer: 10 * 1024 * 1024,
});
const report = JSON.parse(reportText);
const rejected = Object.entries(report)
  .filter(([license]) => !allowedLicenses.has(license))
  .flatMap(([license, packages]) =>
    packages.map((dependency) => ({
      license,
      name: dependency.name,
      versions: dependency.versions.join(", "),
    })),
  );

if (rejected.length > 0) {
  console.error("Unapproved Node.js dependency licenses:");
  for (const dependency of rejected) {
    console.error(`- ${dependency.name}@${dependency.versions}: ${dependency.license}`);
  }
  process.exitCode = 1;
} else {
  const dependencyCount = Object.values(report).reduce(
    (count, dependencies) => count + dependencies.length,
    0,
  );
  console.log(`All ${dependencyCount} installed Node.js dependencies use approved licenses.`);
}
