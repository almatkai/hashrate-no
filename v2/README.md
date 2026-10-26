# Hashrate native V2

Standalone Rust process port of **both** V1 commands: `coins` (Coins) and
`gpu_estimates` (GPU Estimates). Canonical ID `com.tezbar.native.hashrate-no`;
replaces exactly `com.tezbar.hashrate-no` and `raycast.hashrate-no`.
Schema 3, protocol 1, SDK 0.1.1, Rust MSRV 1.85.0. No Tauri adapter, app registry
entry, host/SDK modification, root CI activation, published artifact or installer.
The development manifest deliberately has `artifacts: {}`.

The required password preference remains `api_key`. GPU Estimates retains its
required **text** argument `power_cost`, placeholder `POWER_COST (0-1.00)`.
The original source, V1 lock, assets and license remain unchanged; only the root
README gains a discovery link. The original icon is copied without modification.

## Original behavior and native adaptation

| Area | Behavior |
| --- | --- |
| Service | Fixed `https://api.hashrate.no/v1/coins` and `/v1/gpuEstimates`; GET only. |
| Authentication | Original query parameter `apiKey`; GPU adds `powerCost` with the **original argument string**, not a reformatted number. No auth header substitution. |
| Power validation | ECMAScript `Number` semantics. `0`, `-0`, whitespace and NaN fail with “Power Cost must be a number!”. Nonzero values outside 0–1 fail with the original range message. Positive subnormal, signed/whitespace decimal, exponent and unsigned radix forms are preserved. The source's rejection of zero is intentional parity. |
| Coins | Original array order; Coin/Name title changes with detail visibility; Algo subtitle; raw `$` + `usdPrice` accessory; all 24 labels and 3 separators in source order. Price strings retain all original digits. |
| GPU Estimates | Original `Object.entries` order, including integer-key ordering; device/brand and all four Highest Profit/Revenue blocks, current and 24H. No new calculation or recommendation. |
| Currency | Source `Intl.NumberFormat("en", {currency:"USD",style:"currency"})`, using JS shortest number representation, decimal half-expand, grouping, negative zero, NaN/infinity and scalar coercions. Rust projection is compared against actual TSX formatter output. |
| Actions | Toggle Details and Refresh. Defaults: Coins hidden, GPUs visible. Explicit Refresh requests once and issues new action IDs. No clipboard, paste, shell, selection, dialog, settings or window RPC. |
| Cache | No TTL, as in `execute:false` + source mount effect. Nonempty Coins are reused; empty results refetch on the next launch. GPU caches include exact raw argument identity. The source first warm GPU relaunch can refresh again because initial empty fetch does not set `lastPowerCost`; this quirk is retained. |
| Cache privacy | SHA-256 credential identity plus command/argument fingerprints, private host data directory, no API key or request URL in persisted records. Accounts cannot reuse each other's data/settings. Dates/details/last power identity are scoped per account and command. No V1 profile migration or real profile access. |
| Failure | Completed failures retain prior data and expose a sanitized message plus deliberate Refresh. No automatic retry, polling, background actor, lease or timer. API title/detail and raw HTTP/library/JSON errors are suppressed to prevent credential/URL disclosure. |

Pinned ListRuntime infers detail visibility from whether a row contains a detail
node; the bridge omits that node while details are hidden and retains its data in
session state. Its debounced local search callback expects a view, so search and
stale action replies return a complete, effect-free view. They never echo a
`searchText` prop: the renderer owns the draft immediately, including A/B/A input
before debounce. Search does not fetch. Existing global action descriptors remain
available for empty/error recovery. No host patch is required at the pinned host.

Action consumption lives outside the SDK's staged JSON state. Once consumed, an
old/queued ID cannot request again, even when cancellation rolls back the staged
view. A stale ID can return a current view; it has no network/storage/toggle
effect. A cancelled action can obtain fresh IDs through that effect-free response
or passive refresh, then require a new deliberate activation. Completed network
results are checkpointed before subsequent awaits; concurrent launches of the same
request share the completed result. Deduplication serials are process-local and
never trusted across restart. Cancellation can still consume an API call at the
service; neither exactly-once delivery nor crash durability is claimed.

## Finite differences and limits

The native host uses substring filtering of title/subtitle/section, its own icons,
selection and layout, rather than Raycast's proprietary filtering/visuals. Cached update instants are formatted again in the current local timezone. Section
`last_update` uses local calendar time with `M/D/YYYY` (the tested default `en-US`);
OS-specific `toLocaleDateString()` localization is not exposed by the pinned SDK.
An initial uncached launch awaits its bounded request in the host opening flow,
instead of rendering intermediate React loading frames. Invalid GPU input returns
the source error as a native no-view message rather than a toast + `popToRoot` RPC.
Preferences/launch arguments are the SDK session snapshot; changed preferences or
power input take effect on a new launch. No editable form or dropdown is invented.

Bounds are explicit errors, not truncated service rows or text: 512 KiB response,
512 rows, 4096 bytes per text/key/argument/search input, 8 MiB rendered view, 8 MiB
private cache file, 16 FIFO cached responses and 32 settings entries. Cache
entries may be evicted to honor these finite bounds and then refetched on an
explicit launch. Response schemas reject malformed root/record/nested/text
shapes; missing optional consumed scalar fields retain source blank/undefined or
NaN behavior. JSON numbers must be representable by serde_json; arbitrary JSON
objects/arrays in typed scalar fields are rejected. Secret-echoing consumed data
is rejected rather than persisted/displayed. Unknown service fields and unused
GPU logo URLs are discarded; service content is never executed or used as a file
or browser target. Invalid/unsafe/oversized private cache files report an error
instead of being silently overwritten. A failed cache write retains the result
in memory with a clear message. No durable fsync/crash guarantee is made.

HTTPS uses WebPKI roots, no environment proxy or endpoint override, 4-second
connect and 10-second total timeout. Up to three same-service HTTPS redirects are
allowed; downgrade, other origin and URL credentials are rejected. Media parsing
matches the locked utils JSON media parser. The SDK retains its immutable 16 MiB
frame, 128-session/handler/RPC and bounded queue limits. Adapter action owners are
bounded to 128; cancelled opens are pruned without requiring dispose. No detached
helper thread, timer, task or background lease is added by the port.

## Evidence and reproduction

All mutable QA state, source copies, dependency downloads, HOME/TMP/XDG/npm/Cargo
caches, certificates and output live under this clone's `v2/.qa`, `v2/target` or
`v2/dist`. Scripts derive their own absolute root. The existing stable/1.85.0
Rust toolchains are read without installation/update. No real profile, API key,
clipboard, selection, dialog or product API is used. The `qa` feature adds a
separate owned-loopback TLS fixture binary; default/release production builds
cannot enable a transport override via CLI or environment. QA binaries are not
packaged.

Prerequisites already present for these checks: Node/npm, Python 3, Cargo with
stable + 1.85.0, their fmt/clippy components, and `/usr/bin/openssl`. QA tooling is
locked in `qa/tooling/package-lock.json`; it is separate from the untouched V1
lock. No system dependency installation is performed by the scripts.

From any working directory:

```sh
/absolute/path/to/hashrate-no/v2/scripts/bootstrap.sh
/absolute/path/to/hashrate-no/v2/scripts/verify.sh
/absolute/path/to/hashrate-no/v2/scripts/fresh_build.sh
/absolute/path/to/hashrate-no/v2/scripts/package.sh
/absolute/path/to/hashrate-no/v2/scripts/inventory.sh
```

`bootstrap.sh` creates disposable source/tool copies and downloads locked
build/test dependencies inside `.qa`; it never modifies the V1 files. `verify.sh`
runs the source oracle, renderer and process/TLS tests, full pinned schema with
negative cases, and stable + MSRV fmt/tests/strict all-target clippy. Windows GNU
check/clippy are separately attempted by `windows_qa.sh` and reported truthfully;
no missing compiler is installed. `fresh_build.sh` uses a new **empty** Cargo home
and target, fetching the pinned SDK by HTTPS with `--locked`.

`package.sh` builds a sorted, fixed-timestamp/mode ARM64 development ZIP from that
fresh production executable. Every file except the self manifest is hash-bound
in `files`; a second independent ZIP write must be byte-identical. It verifies the
closed extracted member set and runs both commands through the extracted
production process. This is a development review artifact, not a published or
installed release. All executable/archive output stays ignored.

- [QA results](VALIDATION.md), [source + host + SDK hashes](evidence/inputs.json).
- [Actual TSX/locked hook oracle](tests/source-oracle.json), [process/TLS checks](evidence/process-qa.json), [pinned renderer checks](evidence/renderer-qa.json).
- [Archive inventory and SHA-256](evidence/archive.json), [fresh HTTPS build](evidence/fresh-build.json), [schema checks](evidence/schema-qa.json).
- [All locked dependency provenance and authentic grants](resources/locked-dependencies.json). Both `r-efi` versions retain their actual AUTHORS grants.
- [Full Git-visible review inventory](REVIEW-INVENTORY.json). Excludes itself; includes every V2 file and the root README, including fixtures/evidence/notices. Parent should rehash staged blobs independently before committing.

Original source pin: `48f081909c2b07ab791af1949ad38325c0d1fe6c`.
SDK pin: `e955911f0bf5b558f9d3eb5183df0adc95c22af2`.
Host read-only pin: `ea100cee3f69711de12485ba061950c5fc65258f`.
Icon: `e20c76cba5a9ace3c952cc342da8d3ec177e9a8c073804700b240e5a31ac1bd2`.
