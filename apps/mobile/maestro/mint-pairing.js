// Mints the pairing code the demo flow types, at the moment it types it. A
// code lives sixty seconds and the flow spends most of that reaching the
// field (cold start of Expo Go, the bundle, the developer-menu card), so a
// code minted before the flow started was already dead. Runs on the Maestro
// host: API is where that host reaches the till's API, LAUNCH the launch
// token, SESSION an owner session `just demo-phone` opened.
const res = http.post(API + "/pairing/qr", {
  headers: {
    authorization: "Bearer " + LAUNCH,
    "x-dzpos-session": SESSION,
    "content-type": "application/json",
  },
  body: "{}",
});
if (!res.ok) {
  throw new Error("no pairing code: " + res.status + " " + res.body);
}
output.pairing = json(res.body).pairing_token;
