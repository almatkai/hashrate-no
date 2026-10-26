# Verification and parent review

Both source commands are implemented. This clone is left unstaged/uncommitted
on `feat/native-v2-rust`, base `48f081909c2b07ab791af1949ad38325c0d1fe6c`.
No host/SDK/profile/sibling/ledger file, CI activation, catalog/acquisition entry,
release, PR, commit or extension installation was changed or created.

## Passed gates

| Gate | Evidence |
| --- | --- |
| V1 baseline | Original lock unchanged; 1 smoke test + typecheck passed in disposable copy. Actual locked API 1.104.25, utils 2.3.0, TypeScript 5.9.3, tsx 4.23.13. |
| Stable + MSRV | 7 Rust tests on each; fmt check and strict `clippy --all-targets --all-features -- -D warnings` passed on stable and 1.85.0. |
| Source oracles | 4 full TSX views; 35 argument, 48 currency, 14 media cases; both cache/request lifecycles, empty/error paths, source date instant/format and 5 escape checks. Actual TSX/hooks/locked utils in a closed VM with I/O producers stubbed first. |
| Process + TLS | 46 checks. Actual SDK JSONL OS pipes and actual reqwest against owned TLS. Production binary has OS sandbox `deny network*`; positive data is source-derived synthetic private cache. |
| Lifecycle | 140 cancelled opens without dispose; 128 subsequent sessions + explicit overflow; stale/cross-session IDs; cancelled/late HTTP response fencing; EOF cancellation; timeout/error + deliberate retry; cache restart and process-local serial ownership; credentials/prefs/arguments isolation; concurrent launch dedup. |
| Pinned renderer | 16 actual parser/component/event/reconciliation checks; A/B/A before debounce and late root reconciliation preserve local draft; hidden details adapted. No host patch needed. |
| Schema | 2 full pinned-schema positive manifests (source + packaged), 27 negative cases; AJV draft2020 + formats. |
| Empty build | New empty Cargo home + target, `--locked`, SDK fetched via immutable HTTPS revision. Production arm64 Mach-O build + protocol identity passed. |
| Dev archive | Two independent ZIP writes byte-identical; 335 closed members; complete manifest inventory; 9 extracted production-process checks, both commands. |
| Grants/provenance | 175 locked third-party crates (including SDK), Raycast upstream; 329 authentic license/notice/AUTHORS files. Registry archive SHA-256 checked against Cargo.lock; copied bytes compared to archives. Both r-efi AUTHORS grants included. |
| V1 preservation | All tracked V1 bytes unchanged except the exact short root README discovery suffix; inventory script enforces this. |

Stable: `rustc 1.98.0 (88d9e12ae 2026-08-18)`. MSRV: `rustc 1.85.0 (4d91de4e4 2025-02-17)`.

Production executable SHA-256:
`991f4e04831230d555e333b80f4312ac0ffc9f3c1abe5cfdd24efc9f5d38ed0c`

Development ARM64 ZIP SHA-256:
`de31ad3d974ba82af76c110854cb1f6d6f4025b58abc8eeb82e291ee1d651188`

Original icon: 14,134 bytes, independently fetched in memory from
raycast/extensions@6734d85992290066fd31a7f5bee3cca3d3545a1a and compared with V1.
SHA-256: `e20c76cba5a9ace3c952cc342da8d3ec177e9a8c073804700b240e5a31ac1bd2`.

## Remaining platform evidence

Windows GNU check and all-target strict clippy were each attempted once and
blocked in ring's C build by missing `x86_64-w64-mingw32-gcc` (exit 101).
The existing Rust GNU target was used; no compiler/target was installed. Windows
MSVC build and native Windows runtime/GUI acceptance are **not run**. Actual
Tezbar macOS GUI acceptance with a real account is also **not run**. Both manifest
platform verification records remain `not-run`; the source has no release URLs.
No real API key was read/requested, and no live product API query was performed.

## Bounds and finite differences

See [README](README.md#finite-differences-and-limits) for exact adaptations.
Responses: 512 KiB / 512 rows / 4096-byte scalar text, argument, search and key;
views and private cache file: 8 MiB; 16 response + 32 settings entries, with finite
FIFO eviction; 128 bounded action owners. No silent row/text truncation.
HTTPS: 4-second connect / 10-second total, fixed original service, at most three
same-origin HTTPS redirects; no environment proxy or endpoint override. Error
content is sanitized; old data remains available after failure. Cache identity
adds account separation, compared with V1's global UI metadata. Native filtering,
icons/layout and loading/error presentation use the existing host; date locale
is tested en-US M/D/YYYY with current local timezone formatting of cached instants.
Malformed schemas and unsupported JSON scalar shapes produce explicit errors.
No added mining calculation, advice, clipboard/browser effect, timer or background
lease exists. Completed checkpoints survive ordinary reopen/restart; fsync crash
durability and exactly-once API calls are not claimed.

## Search and review

Initial canonical XERJ index `ax-f993a25238cf` indexed all 6 V1 code files. The
single bounded final update aborted on frozen-schema evolution for new V2
Cargo.toml. No alternate prefix, server restart or schema change was attempted.
V2 XERJ coverage is incomplete. Full Git-object reads, full source reads, exhaustive
`rg` checks and independent byte hashes provide the final review basis.

All mutable QA state and generated binaries/archives remain ignored in this
clone. [REVIEW-INVENTORY.json](REVIEW-INVENTORY.json) covers every Git-visible V2
file + root README except itself. The index remains empty; the parent should
rehash staged blobs independently before committing. Clone is intact for review.
