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
];
// pnpm stores one physical copy per version and symlinks it in. Following
// the symlink is what lets Metro see a workspace package's source at all.
config.resolver.unstable_enableSymlinks = true;

// `disableHierarchicalLookup` stays off, and that is the whole trick. It is
// the flag React Native docs reach for in an npm-style flat tree, where
// walking up parent directories only wastes stat calls. pnpm is the opposite
// shape: every package gets its own `node_modules` holding exactly what it
// declared, and the walk up from the requiring file is the only way to find
// it. Turning the walk off is what made `react-native` fail to resolve
// `scheduler` and `@expo/metro-runtime` fail to resolve `whatwg-fetch`,
// which both sit in their requirer's own folder.

module.exports = config;
