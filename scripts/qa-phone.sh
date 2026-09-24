#!/usr/bin/env bash
# The phone half of the first-day walk, automated (plan
# automated-qa-rounds-for-the-shop, round 1; the hand test of 2026-09-24
# paired a phone and rang a sale from it). Brings up a fresh shop, pairs
# Expo Go on the `dinar` emulator with a typed pairing code, signs in by PIN,
# rings a cash sale, then asks the API that exactly one ticket came of it
# with the right total. A green run is a paired phone selling for real; a
# red one leaves the clip and the logs behind.
#
# Runs beside Samir's dev stack, never on it: its own API (a copied binary,
# no cargo) on 4417 over .dev/qa.db, Metro on 8091, all in tmux `dz-qa`.
# Needs the emulator (started here if adb lists no device) and Maestro on
# the Windows side (scripts/maestro-windows.sh), as demo-phone.sh does.
#
# Knobs: DZPOS_API_BIN (default ~/.dz-night/bin/dzpos-api), QA_API_PORT,
# QA_METRO_PORT, QA_LANG (fr|ar: the phone's app language is not set here
# yet, the flow reads the English labels), QA_OUT (the clip).
set -euo pipefail
cd "$(dirname "$0")/.."
# A non-login shell on the WSL box may not carry the Windows PATH, and both
# the emulator start and scripts/maestro-windows.sh go through cmd.exe.
export PATH="$PATH:/mnt/c/Windows/System32"

bin="${DZPOS_API_BIN:-$HOME/.dz-night/bin/dzpos-api}"
api_port="${QA_API_PORT:-4417}"
metro_port="${QA_METRO_PORT:-8091}"
front_port=$((metro_port + 1))
out="${QA_OUT:-apps/desktop/e2e/.artifacts/qa-phone}"
sdk=/mnt/c/Users/Administrator/AppData/Local/Android/Sdk
adb="${DZPOS_ADB:-$sdk/platform-tools/adb.exe}"
maestro="${DZPOS_MAESTRO:-scripts/maestro-windows.sh}"
api="http://127.0.0.1:$api_port"
db=.dev/qa.db
owner=Samir
password=premier-jour-qa
pin=2468
mkdir -p .dev "$out"

# ---- the API, on an empty shop ----
tmux has-session -t dz-qa 2>/dev/null && tmux kill-session -t dz-qa
rm -f "$db" "$db-shm" "$db-wal"
umask 077
head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n' > .dev/qa-api-token
umask 022
launch="$(cat .dev/qa-api-token)"
tmux new-session -d -s dz-qa -n api \
  "DZPOS_API_TOKEN=$launch '$bin' --db $db --port $api_port --lan 2>&1 | tee .dev/qa-api.log"
for _ in $(seq 60); do curl -sf "$api/health" >/dev/null && break; sleep 1; done
curl -sf "$api/health" >/dev/null || { echo "the QA API never answered on $api" >&2; exit 1; }

call() { # method path [json]
  curl -sf -X "$1" "$api$2" -H "authorization: Bearer $launch" -H "x-dzpos-session: ${session:-}" \
    -H 'content-type: application/json' ${3:+-d "$3"}
}

# ---- the shop the hand test had by the time it paired the phone ----
session=""
session="$(call POST /auth/first-setup "$(jq -cn --arg n "$owner" --arg p "$password" '{name:$n,password:$p}')" | jq -r .token)"
[[ ${#session} -gt 20 ]] || { echo "first setup gave no session" >&2; exit 1; }
owner_id="$(call GET /auth/me | jq -r .user_id)"
call POST "/users/$owner_id/pin" "$(jq -cn --arg p "$pin" '{pin:$p}')" >/dev/null
product() { # name barcode cost sell stock_units rate_bps
  call POST /products "$(jq -cn --arg n "$1" --arg b "$2" --argjson c "$3" --argjson s "$4" --argjson q "$5" --argjson r "$6" \
    '{name:$n,barcode:$b,category_id:null,unit:"piece",cost_centimes:$c,selling_centimes:$s,wholesale_centimes:null,
      qty_on_hand_milli:($q*1000),low_stock_at_milli:0,rate_bps:$r,active:true}')" >/dev/null
}
product "Eau minérale 1,5L" 6131016000013 3000 4500 48 1900
product "Café moulu 250g" 6130008123457 32000 42000 20 900
product "Sucre en vrac" 2000010000012 9500 11000 50 0
call POST /till/shifts '{"opening_cash_centimes":500000}' >/dev/null

# ---- Metro, with the address and launch token baked in (findings T53, T54) ----
tmux new-window -t dz-qa -n metro \
  "cd apps/mobile && EXPO_PUBLIC_API_URL=$api EXPO_PUBLIC_API_TOKEN=$launch CI=1 pnpm exec expo start --port $metro_port 2>&1 | tee ../../.dev/qa-metro.log"
# adb reverse reaches Windows' 127.0.0.1; WSL mirrors Metro's dual-stack
# socket onto [::1] only, so an IPv4 front door sits before it.
tmux new-window -t dz-qa -n front "python3 scripts/ipv4-front.py $front_port $metro_port"
for _ in $(seq 120); do curl -sf "http://127.0.0.1:$metro_port/status" >/dev/null && break; sleep 1; done

# ---- the emulator ----
if ! "$adb" devices | grep -q 'device$'; then
  # Detached: `cmd.exe /c start` never returns through WSL interop.
  setsid nohup "$sdk/emulator/emulator.exe" -avd dinar -no-snapshot-save >/dev/null 2>&1 < /dev/null &
  "$adb" wait-for-device
  for _ in $(seq 120); do [[ "$("$adb" shell getprop sys.boot_completed | tr -d '\r')" == 1 ]] && break; sleep 2; done
fi
"$adb" reverse "tcp:$metro_port" "tcp:$front_port" >/dev/null
"$adb" reverse "tcp:$api_port" "tcp:$api_port" >/dev/null
"$adb" shell pm clear host.exp.exponent >/dev/null

flows=apps/mobile/maestro
win() { wslpath -w "$1"; }
$maestro test -e "PORT=$metro_port" "$(win $flows/qa-open.yaml)"

"$adb" shell rm -f /sdcard/qa-phone.mp4
"$adb" shell screenrecord --bit-rate 6000000 --time-limit 170 /sdcard/qa-phone.mp4 &
recorder=$!
sleep 2
set +e
$maestro test -e "API=$api" -e "LAUNCH=$launch" -e "SESSION=$session" -e "NAME=$owner" -e "PIN=$pin" "$(win $flows/demo.yaml)"
status=$?
set -e
"$adb" shell pkill -INT -x screenrecord || true
wait "$recorder" || true
sleep 2
"$adb" pull /sdcard/qa-phone.mp4 "$out/qa-phone.mp4" >/dev/null || true

# ---- what the shop file says, not what the phone showed ----
tickets="$(call GET '/sales?kind=ticket')"
count="$(jq 'length' <<<"$tickets")"
devices="$(call GET /pairing/devices | jq 'length')"
echo "paired devices: $devices; tickets: $count"
jq -c '.[] | {number: .printed_number, net: .totals.net_to_pay_centimes, mode: .payment_mode}' <<<"$tickets"
[[ $status -eq 0 ]] || { echo "maestro failed ($status); clip in $out" >&2; exit "$status"; }
[[ "$devices" == 1 ]] || { echo "expected one paired device, got $devices" >&2; exit 1; }
[[ "$count" == 1 ]] || { echo "expected exactly one ticket, got $count" >&2; exit 1; }
# The flow taps the first tile twice, 2 x Café moulu at 420,00 (9 %): HT
# 840,00, TVA 75,60, stamp 10,00 on a cash ticket, net 925,60.
net="$(jq '.[0].totals.net_to_pay_centimes' <<<"$tickets")"
[[ "$net" == 92560 ]] || { echo "expected net 92560 centimes, got $net" >&2; exit 1; }
echo "qa-phone: green, clip in $out/qa-phone.mp4"
