// Metro in a pnpm workspace. The phone imports two workspace packages
// (`@dzpos/design` for the tokens, `@dzpos/shared` for the money rules and
// the generated DTOs), and both live outside this folder with their real
// files behind pnpm's symlinked store. Metro does not follow either by
// default, so it is told where the repo root is and to resolve modules from
// both node_modules trees.
const { getDefaultConfig } = require("expo/metro-config");
const path = require("node:path");

const projectRoot = __dirname;
const workspaceRoot = path.resolve(projectRoot, "../..");

const config = getDefaultConfig(projectRoot);

config.watchFolders = [workspaceRoot];
config.resolver.nodeModulesPaths = [
  path.resolve(projectRoot, "node_modules"),
  path.resolve(workspaceRoot, "node_modules"),
  // pnpm's hoisted store (the repo runs `node-linker=hoisted`, see the root
  // .npmrc). A handful of Expo's own packages import things they never
  // declared — `@expo/metro-runtime` imports `whatwg-fetch` — and under a
  // strict tree those resolve from nowhere. This is the one directory that
  // has every installed package flat, so it is the honest last resort
  // rather than adding somebody else's forgotten dependency to ours.
  path.resolve(workspaceRoot, "node_modules/.pnpm/node_modules"),
];
// pnpm stores one physical copy per version and symlinks it in. Following
// the symlink is what lets Metro see a workspace package's source at all.
config.resolver.unstable_enableSymlinks = true;
config.resolver.disableHierarchicalLookup = true;

module.exports = config;
