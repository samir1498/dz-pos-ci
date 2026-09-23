// Expo's preset is the whole build: it carries the JSX runtime, expo-router's
// transform and the React Compiler (switched on in app.config.js under
// `experiments.reactCompiler`, not here).
//
// No Reanimated worklet plugin, because there is no Reanimated: expo-router's
// Stack animates through react-native-screens' native stack, and Reanimated 4
// would drag in react-native-worklets for transitions a till never sees.
module.exports = function (api) {
  api.cache(true);
  return {
    presets: ["babel-preset-expo"],
  };
};
