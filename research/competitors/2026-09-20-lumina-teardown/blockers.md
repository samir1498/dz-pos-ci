# What this pass could not do, and what it would take

## Android: everything past account creation

**What happened, this pass.** The tooling problem from the earlier pass was
fixed: a Maestro flow (`maestro/01-owner-account.yaml`) drives the owner-
account form with `inputText`/`hideKeyboard` instead of raw `adb shell input`,
and the flow's own log shows every step running, through the final field and
a save action. Shortly after, the emulator stopped responding to input at all
— Android itself reported "Lumina isn't responding", then "Digital Wellbeing
isn't responding", then "Process system isn't responding", one after another.
Host load average was 7.4–9.5 at the time. **The actual cause is host CPU
contention on this shared box**, almost certainly a concurrent cargo build —
this is the exact hazard `context/processes/20260908-machines-and-heavy-jobs.md`
already documents for this machine, not a Lumina defect and not a Maestro
defect. Tapping "Wait" repeatedly did not recover the emulator inside this
pass's time budget, so Android work was stopped there and desktop roles work
was picked up instead, per the coordinator's explicit instruction to put roles
first.

**What it would take to finish.** Nothing new — the Maestro path works. The
one thing worth fixing in `01-owner-account.yaml` before trusting it as a
reusable flow: it taps by screen-percentage coordinates, which is fragile
once the on-screen keyboard scrolls the form (this is what caused the
password-mismatch symptom in the pass before this one). Rewriting the taps as
label-relative selectors (`tapOn: { below: { text: "..." } }`) would make it
survive that scroll. One flow file per area, as originally planned, was not
reached — only the owner-account flow exists. Re-run once the box is not
contended; check `loadavg`/`just disk`-style host state before starting.

Furthest point actually reached on the phone this pass, after account setup:
a "الاتصال بلومينا" (connect to Lumina) screen offering three connection
modes — manual IP, LAN auto-search, and QR-via-cloud — reached via an
unplanned path rather than a deliberate scripted one. See the pairing section
below for what that screen implies.

## Android: pairing with the desktop

**Attempted this pass, partially.** The desktop's mobile-pairing toggle is
real, is off by default, and was switched on, then switched back off when
done — see findings.md's pairing section for the screenshots. Turning it on
reveals a QR code. The phone's own "ربط عبر QR" option pairs against that QR
over the internet by its own labelling — **it needs Lumina's cloud relay, not
just LAN reachability between the emulator and the Windows host.** This
narrows the blocker written here before this pass: it is not that the two
sides can't reach each other on the LAN (they can, and that fact stands), it
is that the QR path Lumina actually built routes through their servers, which
is off-limits under the hard rule against touching their cloud.

**Not attempted, and the one avenue actually worth trying next:** the phone's
"بحث تلقائي" (auto-search, local network) option is worded as LAN discovery
rather than internet relay, and might complete without touching their cloud
at all. It was found only after the emulator had already become unresponsive
from the CPU contention above, so tapping "بدء البحث" (start search) was
attempted but not confirmed — the input landed inconsistently while the
emulator was starved and no result screen was captured. This is the
concrete next step, not the QR path.

## Anything behind activation

The trial runs; activation does not. It needs a code that a human at the vendor
sends back after the in-app request form, which asks for a name, a phone, a
wilaya and a commune. We are not going to ask them for one. So the trial caps
(100 desktop sales, 50 phone invoices and 10 phone customers) are the ceiling on
how much can ever be driven, and nothing gated behind a paid licence — if
anything is — can be seen at all.

## Cloud, multi-branch and the AI invoice scan

All three need their server and an account with them. The cloud join code is a
prefixed four-by-four format issued by the vendor. The AI invoice scan is
metered against a quota endpoint on their side. Nothing here can be exercised
without being their customer, and that is a commercial decision, not a technical
one: it would mean buying a licence at 12 000 DZD and giving them a phone
number and a wilaya.

## Roles

**Done this pass.** A second user was created with the Seller role, logged in
as, and driven directly against the app's own in-renderer functions and IPC
bridge. Full write-up, including a demonstrated privilege-escalation path and
the comparison against `crates/api/src/gates.rs`, is in findings.md. Not done:
Cashier, Accountant and Assistant were not individually created or probed, so
whether the sidebar hides different things per role (versus one blanket
non-Admin hiding) is unconfirmed for those three specifically.

## Workflows not exercised

Driven: shop-level settings, a product with stock and prices, a customer with
all four fiscal identifiers, a cart, and one cash sale end to end.

Not driven, and each is reachable — they only need time, not permission:
a credit sale, a discounted sale, a return or credit note, a payment against a
customer's debt with the balance checked before and after, a purchase from a
supplier, a stock adjustment, an inventory count, a printed or previewed
document of any kind, and the eight report tabs opened one by one with an export
saved from each.

The printed document is the most valuable of those and should be first next
time: the invoice template picker, the print-language selector and the eleven
layouts are the gap our own invoice-layouts plan is aimed at, and this pass saw
the settings for them without ever producing a page.

## Two environment facts worth keeping

**WSL cannot reach a Windows-side loopback port.** The Windows firewall blocks
it and there is no admin password on this machine. This is already written down
for adb in `scripts/maestro-windows.sh`; it applies equally to a remote
debugging port. The way round it is to run the driver on the Windows side —
there is a Node install there — and talk to the port from that side. That is how
the desktop app got driven at all.

**A file copied to the Windows filesystem is not executable until it is
chmod +x.** The desktop build failed to start with a permission error until the
bit was set on the copy. Easy to misread as Windows blocking the binary.
