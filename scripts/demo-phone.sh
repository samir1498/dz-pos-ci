#!/usr/bin/env bash
# The phone scene of the demo, recorded: mint a pairing code as the owner,
# start `adb shell screenrecord`, drive Expo Go with the Maestro flow, stop
# the recording and put the clip beside the desktop ones.
#
# Needs a running API (`just api 4317 .dev/dev.db "" lan`), Metro reachable
# from the device (on the WSL box: `adb reverse tcp:8081 tcp:8082` in front of
# the IPv4 forwarder in .dev/fwd4.py, see the demo loop page), and an emulator
# or phone `adb devices` lists. Every knob is an environment variable so the
# same script runs where Maestro is native and where it runs on the Windows
# side of a WSL box:
#   DZPOS_API          http://127.0.0.1:4317
#   DZPOS_ADB          adb            (adb.exe on the WSL box)
#   DZPOS_MAESTRO      maestro        (a wrapper that calls maestro.bat on Windows)
#   DZPOS_MAESTRO_WINDOWS  set to anything to hand Maestro a Windows path
#   DZPOS_OWNER_NAME / DZPOS_OWNER_PASSWORD / DZPOS_OWNER_PIN   the dev seed's owner
#   DZPOS_DEMO_OUT     where the mp4 goes (the Remotion project's recordings)
set -euo pipefail

api="${DZPOS_API:-http://127.0.0.1:4317}"
adb="${DZPOS_ADB:-adb}"
maestro="${DZPOS_MAESTRO:-maestro}"
name="${DZPOS_OWNER_NAME:-Propriétaire}"
password="${DZPOS_OWNER_PASSWORD:-developpement}"
pin="${DZPOS_OWNER_PIN:-1379}"
out="${DZPOS_DEMO_OUT:-$HOME/dinar-remotion/public/recordings}"
flow="apps/mobile/maestro/demo.yaml"
launch="$(cat .dev/api-token)"

# The owner's session. The pairing code itself is minted by the flow
# (apps/mobile/maestro/mint-pairing.js) at the moment it is typed: it lives
# sixty seconds, less than the flow takes to reach the field. DZPOS_API_FROM_MAESTRO
# is where the Maestro host reaches the API (on the WSL box, Windows reaches
# this box's 4317 on its own localhost).
session="$(curl -sf "$api/auth/login" -H "authorization: Bearer $launch" -H 'content-type: application/json' \
  -d "$(jq -cn --arg n "$name" --arg p "$password" '{name:$n,password:$p}')" | jq -r .token)"
[[ ${#session} -gt 20 ]] || { echo "no owner session came back" >&2; exit 1; }
api_from_maestro="${DZPOS_API_FROM_MAESTRO:-$api}"

# The phone starts unpaired every time: Expo Go's storage is where the app
# keeps its device token.
"$adb" shell pm clear host.exp.exponent >/dev/null

# Bring the app up first, off camera: a cold Expo Go spends about forty
# seconds fetching the bundle, and none of that is the product.
if [[ -n "${DZPOS_MAESTRO_WINDOWS:-}" ]]; then
  $maestro test "$(wslpath -w apps/mobile/maestro/demo-open.yaml)"
else
  $maestro test apps/mobile/maestro/demo-open.yaml
fi

# Record while the flow runs. screenrecord stops on SIGINT and finalises the
# file; a killed one leaves an mp4 nothing can open.
"$adb" shell rm -f /sdcard/dinar-demo.mp4
"$adb" shell screenrecord --bit-rate 8000000 --time-limit 170 /sdcard/dinar-demo.mp4 &
recorder=$!
sleep 2

if [[ -n "${DZPOS_MAESTRO_WINDOWS:-}" ]]; then flow="$(wslpath -w "$flow")"; fi
set +e
$maestro test -e "API=$api_from_maestro" -e "LAUNCH=$launch" -e "SESSION=$session" -e "PIN=$pin" -e "NAME=$name" "$flow"
status=$?
set -e

"$adb" shell pkill -INT -x screenrecord || true
wait "$recorder" || true
sleep 2
mkdir -p apps/desktop/demo-clips "$out"
"$adb" pull /sdcard/dinar-demo.mp4 apps/desktop/demo-clips/phone.mp4
ffmpeg -y -loglevel error -i apps/desktop/demo-clips/phone.mp4 -c:v libx264 -crf 18 -pix_fmt yuv420p -movflags +faststart "$out/phone.mp4"
echo "$out/phone.mp4"
exit "$status"
