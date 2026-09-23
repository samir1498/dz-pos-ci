// Dynamic in place of the static `app.json` it replaces, so the one thing
// C7 (`the-first-clinic-module-patients-queue-appointments`) needs to
// change per build — which directory Expo Router discovers routes in —
// can be computed from `DINAR_MOBILE_MODULES` (default "retail", read once
// here at config-evaluation time; see `modules.config.js` for why the
// router root, not a per-file list, is the knob this framework exposes).
// Everything else below is `app.json`'s fixed content, moved over
// unchanged.

const { resolveModules, routerRootFor } = require("./modules.config");

module.exports = () => {
  const active = resolveModules(process.env.DINAR_MOBILE_MODULES);

  return {
    expo: {
      name: "Dinar",
      slug: "dinar-mobile",
      version: "0.1.0",
      orientation: "portrait",
      scheme: "dinar",
      platforms: ["ios", "android", "web"],
      userInterfaceStyle: "automatic",
      newArchEnabled: true,
      experiments: {
        typedRoutes: true,
        reactCompiler: true,
      },
      plugins: [
        ["expo-router", { root: routerRootFor(active) }],
        [
          "expo-camera",
          {
            cameraPermission:
              "Dinar uses the camera once, to read the pairing code off the till computer's screen.",
          },
        ],
        [
          "expo-splash-screen",
          {
            image: "./assets/splash.png",
            imageWidth: 160,
            resizeMode: "contain",
            backgroundColor: "#f8f7f4",
            dark: {
              image: "./assets/splash.png",
              backgroundColor: "#09090b",
            },
          },
        ],
      ],
      android: {
        package: "com.dinar.app",
        adaptiveIcon: {
          foregroundImage: "./assets/adaptive-icon.png",
          backgroundColor: "#f8f7f4",
        },
      },
      ios: {
        bundleIdentifier: "com.dinar.app",
        supportsTablet: true,
      },
      icon: "./assets/icon.png",
      web: {
        favicon: "./assets/favicon.png",
      },
    },
  };
};
