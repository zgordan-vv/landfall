import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const projects = [
  {
    manifest: "packages/protocol-ts/package.json",
    name: "@landfall/protocol",
    internalDependencies: [],
    tsconfig: "packages/protocol-ts/tsconfig.json",
  },
  {
    manifest: "packages/sdk-ts/package.json",
    name: "@landfall/sdk",
    internalDependencies: ["@landfall/protocol"],
    tsconfig: "packages/sdk-ts/tsconfig.json",
  },
  {
    manifest: "packages/api-client/package.json",
    name: "@landfall/api-client",
    internalDependencies: ["@landfall/protocol"],
    tsconfig: "packages/api-client/tsconfig.json",
  },
  {
    manifest: "apps/dashboard/package.json",
    name: "@landfall/dashboard",
    internalDependencies: ["@landfall/api-client"],
    tsconfig: "apps/dashboard/tsconfig.app.json",
  },
];

const dependencyFields = [
  "dependencies",
  "devDependencies",
  "optionalDependencies",
  "peerDependencies",
];
const manifestsByName = new Map();

async function readJson(relativePath) {
  const contents = await readFile(resolve(repositoryRoot, relativePath), "utf8");
  return JSON.parse(contents);
}

for (const project of projects) {
  const manifest = await readJson(project.manifest);

  if (manifest.name !== project.name) {
    throw new Error(`${project.manifest}: expected package name ${project.name}`);
  }

  manifestsByName.set(project.name, { ...project, manifest });
}

const internalPackageNames = new Set(manifestsByName.keys());

for (const project of projects) {
  const { manifest } = manifestsByName.get(project.name);
  const actualInternalDependencies = new Set();

  for (const field of dependencyFields) {
    for (const [dependencyName, version] of Object.entries(manifest[field] ?? {})) {
      if (!internalPackageNames.has(dependencyName)) {
        continue;
      }

      if (field !== "dependencies") {
        throw new Error(`${project.manifest}: ${dependencyName} must be a runtime dependency`);
      }

      if (version !== "workspace:*") {
        throw new Error(`${project.manifest}: ${dependencyName} must use workspace:*`);
      }

      actualInternalDependencies.add(dependencyName);
    }
  }

  const expected = [...project.internalDependencies].sort();
  const actual = [...actualInternalDependencies].sort();

  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(
      `${project.manifest}: expected internal dependencies [${expected}], found [${actual}]`,
    );
  }

  const tsconfig = await readJson(project.tsconfig);

  if (tsconfig.compilerOptions?.paths !== undefined) {
    throw new Error(`${project.tsconfig}: paths aliases may not bypass package exports`);
  }

  const referencedInternalPackages = [];

  for (const reference of tsconfig.references ?? []) {
    const referencedDirectory = resolve(repositoryRoot, dirname(project.tsconfig), reference.path);
    const referencedManifest = await readJson(
      `${referencedDirectory.slice(repositoryRoot.length + 1)}/package.json`,
    );

    if (internalPackageNames.has(referencedManifest.name)) {
      referencedInternalPackages.push(referencedManifest.name);
    }
  }

  if (JSON.stringify(referencedInternalPackages.sort()) !== JSON.stringify(expected)) {
    throw new Error(`${project.tsconfig}: project references must match package dependencies`);
  }
}

console.log("TypeScript package dependency direction is valid.");
