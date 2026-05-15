# Dependency vulnerability audit — `c:\code\zos`

Generated: 2026-05-15. Tools used:

- `cargo audit 0.22.1` (already installed, not re-installed) against the workspace `Cargo.lock` (741 crates scanned, RustSec advisory DB has 1090 advisories at scan time).
- `npm audit` against `interface/` (`npm` 11.x, 223 deps total: 30 prod, 194 dev, 29 optional).

Raw outputs are in this directory:

- `cargo-audit.json` / `cargo-audit.txt`
- `npm-audit.json` / `npm-audit.txt`
- `npm-audit-fix-dryrun.txt` (output of `npm audit fix --dry-run` in `interface/`)

## Headline counts

| Ecosystem | Critical | High | Medium / Moderate | Low | Unscored | Total vulns | Warnings (unmaintained / unsound) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Cargo (Rust) | 1 | 1 | 1 | 0 | 4 | **7** | 16 (14 unmaintained + 2 unsound) |
| npm (JS) | 0 | 1 | 1 | 0 | 0 | **2** | n/a |

The single critical is `hpke-rs` nonce reuse (CVSS 9.3); the single high is a related `hpke-rs` panic (CVSS 8.2). Both reach us transitively through the external path-dep `../../zero-sdk-10/crates/zero-crypto` (via `openmls_rust_crypto`), so they cannot be fixed by `cargo update -p hpke-rs` alone — see the **Path-dep trap** section.

---

## Cargo (Rust) — 7 advisories

All 7 advisories are pulled in via path-deps that live OUTSIDE this workspace (in the sibling `zero-sdk-10` checkout that `crates/zos-grid/Cargo.toml` references with `path = "../../../zero-sdk-10/crates/..."`). None of the four workspace members (`zero-desktop`, `zero-server`, `zos-auth`, `zos-grid`) declare any of these crates directly.

### Critical

| Advisory | Crate @ ver | Patched | Description | Path |
| --- | --- | --- | --- | --- |
| [RUSTSEC-2026-0071](https://rustsec.org/advisories/RUSTSEC-2026-0071) (CVSS 9.3) | `hpke-rs 0.2.0` | `>=0.6.0` | **Nonce Reuse in HPKE Context.** Crypto failure: two calls to seal/open with the same context can produce identical nonces, breaking AEAD security. | `hpke-rs` → `openmls_rust_crypto 0.3.0` → `zero-crypto` (path-dep in `zero-sdk-10`) → `zero-sdk` / `zero-messaging` → `zos-grid` → `zero-server` → `zero-desktop` |

### High

| Advisory | Crate @ ver | Patched | Description | Path |
| --- | --- | --- | --- | --- |
| [RUSTSEC-2026-0070](https://rustsec.org/advisories/RUSTSEC-2026-0070) (CVSS 8.2) | `hpke-rs 0.2.0` | `>=0.6.0` | **Panic When Opening or Sealing on Export-Only Context.** Remote-triggerable panic → DoS on availability. | same as above |

### Medium

| Advisory | Crate @ ver | Patched | Description | Path |
| --- | --- | --- | --- | --- |
| [RUSTSEC-2025-0144](https://github.com/RustCrypto/signatures/security/advisories/GHSA-hcp2-x6j4-29j7) (CVSS 6.4) | `ml-dsa 0.0.4` | `>=0.1.0-rc.3` | **Timing side-channel in ML-DSA decomposition.** Secret-dependent branching on adjacent-network attacker. | `ml-dsa` → `zid 0.2.0` → `zero-storage` / `zero-identity` / `grid-core` (path-deps in `zero-sdk-10`) |

### Unscored (severity not assigned, but treat as high — they're remotely reachable DoS / cryptographic-correctness bugs)

| Advisory | Crate @ ver | Patched | Description | Path |
| --- | --- | --- | --- | --- |
| [RUSTSEC-2026-0118](https://github.com/hickory-dns/hickory-dns/security/advisories/GHSA-3v94-mw7p-v465) | `hickory-proto 0.25.2` | **No fix yet** | NSEC3 closest-encloser proof validation enters an unbounded loop on cross-zone responses. DoS, unfixed upstream. | `hickory-proto` → `libp2p-mdns 0.48.0` / `libp2p-dns 0.44.0` → `libp2p 0.56.0` → `grid-net 0.2.0` (path-dep in `zero-sdk-10`) → `zero-network` → `zero-sdk` / `zero-messaging` |
| [RUSTSEC-2026-0119](https://github.com/hickory-dns/hickory-dns/security/advisories/GHSA-q2qq-hmj6-3wpp) | `hickory-proto 0.25.2` | `>=0.26.1` | O(n²) name compression → CPU exhaustion during message encoding. | same as above |
| [RUSTSEC-2026-0069](https://rustsec.org/advisories/RUSTSEC-2026-0069) | `hpke-rs 0.2.0` | `>=0.6.0` | Incorrect Length Encoding on KDF Export. | same as the other `hpke-rs` issues |
| [RUSTSEC-2026-0072](https://rustsec.org/advisories/RUSTSEC-2026-0072) | `hpke-rs-rust-crypto 0.2.0` | `>=0.6.0` | Missing check for all-zero X25519 shared secret → small-subgroup / invalid-curve attack. | `hpke-rs-rust-crypto` → `openmls_rust_crypto 0.3.0` → `zero-crypto` (same chain as `hpke-rs`) |

### Cargo warnings (unmaintained / unsound — informational, not vulns)

14 unmaintained + 2 unsound. None block deployment; flagged here for awareness.

| ID | Crate | Class | Source |
| --- | --- | --- | --- |
| RUSTSEC-2024-0411..0420, 0429 | `atk`, `atk-sys`, `gdk`, `gdk-sys`, `gdkwayland-sys`, `gdkx11`, `gdkx11-sys`, `gtk`, `gtk-sys`, `gtk3-macros`, `glib 0.18.5` (unsound) | gtk-rs GTK3 bindings deprecated | All via `wry` / `tao` / `webkit2gtk` (`zero-desktop` Tauri stack — Linux build only; benign on Windows/macOS) |
| RUSTSEC-2024-0436 | `paste 1.0.15` | unmaintained proc-macro | Pulled by `rav1e` (image codec), `pqcrypto-mldsa`, `netlink-packet-core` |
| RUSTSEC-2024-0370 | `proc-macro-error 1.0.4` | unmaintained | via `gtk3-macros` / `glib-macros` (Linux desktop) |
| RUSTSEC-2023-0089 | `atomic-polyfill 1.0.3` | unmaintained | via `heapless` → `postcard` (used by `zero-storage` / `zero-messaging`) |
| RUSTSEC-2025-0134 | `rustls-pemfile 1.0.4` | unmaintained | via `reqwest 0.11.27` → `zos-auth`, `zero-server` (direct dep we control — see Action Plan) |
| RUSTSEC-2026-0002 | `lru 0.12.5` | unsound (`IterMut` Stacked Borrows) | via `zero-storage` (path-dep in `zero-sdk-10`) |

### Path-dep trap (important)

The workspace at `c:\code\zos` declares its `zero-sdk` / `zero-identity` / `zero-messaging` deps as **filesystem path-deps** into a sibling checkout:

```toml
# crates/zos-grid/Cargo.toml
zero-sdk      = { path = "../../../zero-sdk-10/crates/zero-sdk" }
zero-identity = { path = "../../../zero-sdk-10/crates/zero-identity" }
zero-messaging= { path = "../../../zero-sdk-10/crates/zero-messaging" }
```

Everything in `zero-sdk-10` (notably `grid-net 0.2.0`, `grid-core 0.2.0`, `zero-crypto`, `zero-network`, `zero-storage`, `zid 0.2.0`) is in turn a path-dep, NOT a published crate. Cargo cannot replace path-dep crates from outside the workspace they live in. Practical consequences:

- `cargo update -p hpke-rs` will not pull in 0.6.0 because `openmls_rust_crypto 0.3.0` (in `zero-sdk-10/crates/zero-crypto`) pins `hpke-rs = "0.2"`. Upstream `zero-sdk-10` has to bump `openmls_rust_crypto` (which itself needs to gain `hpke-rs 0.6` support — that may not exist yet, in which case the fix is to swap implementations or wait).
- Same story for `hickory-proto`: needs `libp2p 0.57+` (or whichever lands the `hickory 0.26` bump) inside `zero-sdk-10/crates/grid-net`'s `Cargo.toml`.
- Same story for `ml-dsa`: needs `zid` (in `zero-sdk-10`) to move to `ml-dsa 0.1.0-rc.3+`.

The only escape hatches from THIS workspace are:

1. **Bump `zero-sdk-10` itself** (the proper fix; do that PR in the `zero-sdk-10` repo, then `cargo update` here picks it up).
2. **Add `[patch.crates-io]` overrides** in `c:\code\zos\Cargo.toml`, e.g. `hpke-rs = { git = "...", tag = "v0.6.1" }`. This is fragile because `openmls_rust_crypto 0.3.0` still has `hpke-rs = "^0.2"` in its `Cargo.toml`; the patch will only work if 0.6 is semver-compatible from `openmls_rust_crypto`'s POV (it isn't — major bump). So this is **not viable** for `hpke-rs` without also patching `openmls_rust_crypto`. It IS viable for `hickory-proto 0.25 → 0.26` only if `libp2p`'s `Cargo.toml` admits 0.26 in its semver range (it likely doesn't — `libp2p-mdns 0.48` pins `hickory-proto = "0.25"`).

Net: **none of the 7 Rust vulns can be fully fixed from inside `c:\code\zos`.** The fix has to land in `zero-sdk-10` (or upstream `libp2p`/`openmls`). Document this clearly to the parent.

---

## npm (JS) — 2 advisories, both auto-fixable

Both are in `interface/` and both are **dev-only** (vite is in `devDependencies`; postcss reaches us transitively through vite/lightningcss). They do not ship in the production bundle. `npm audit fix` resolves both — dry-run confirms it bumps `vite 8.0.3 → 8.0.13` and `postcss 8.5.8 → 8.5.14`, no breaking changes within `^8.0.0` / `^8.5.x`.

### High

| Advisory | Package @ ver | Patched | Description | Dep class |
| --- | --- | --- | --- | --- |
| [GHSA-v2wj-q39q-566r](https://github.com/advisories/GHSA-v2wj-q39q-566r) | `vite 8.0.0–8.0.4` | `>=8.0.5` | `server.fs.deny` bypassed with queries → arbitrary file read on dev server | dev (direct) |
| [GHSA-p9ff-h696-f583](https://github.com/advisories/GHSA-p9ff-h696-f583) | `vite 8.0.0–8.0.4` | `>=8.0.5` | Arbitrary file read via Vite dev-server WebSocket | dev (direct) |
| [GHSA-4w7w-66w2-5vf9](https://github.com/advisories/GHSA-4w7w-66w2-5vf9) | `vite 8.0.0–8.0.4` | `>=8.0.5` | Path traversal in optimised-deps `.map` handling | dev (direct) |

(npm collapses the three vite GHSAs under one "vite" entry with highest severity = high.)

### Moderate

| Advisory | Package @ ver | Patched | Description | Dep class |
| --- | --- | --- | --- | --- |
| [GHSA-qx2v-qp2m-jg93](https://github.com/advisories/GHSA-qx2v-qp2m-jg93) (CVSS 6.1) | `postcss <8.5.10` | `>=8.5.10` | XSS via unescaped `</style>` in CSS stringify output | dev (transitive via vite) |

Production deps (`react`, `react-dom`, `@tanstack/*`, `zustand`, `@dnd-kit/*`, `lucide-react`, `@fontsource-variable/inter`, `@cypher-asi/zui`): **0 advisories**.

---

## Action plan (priority order)

### 1. **Safe & immediate** — fix npm dev-deps now ✅

```bash
cd c:\code\zos\interface
npm audit fix
```

Equivalent surgical line changes in `interface/package.json` (no breaking changes — both are within the current caret range, so the manifest line doesn't strictly need to change; `npm audit fix` will update only `package-lock.json`):

- `"vite": "^8.0.0"` — leave the manifest, `package-lock.json` will be updated to 8.0.13. If you want to harden, change to `"vite": "^8.0.13"`.
- `postcss` is transitive (not in `package.json`); only `package-lock.json` changes.

Clears 1 high + 1 moderate.

### 2. **Required upstream PR** — bump `hpke-rs` chain in `zero-sdk-10` 🔴

Open a PR in `../../zero-sdk-10` that bumps `crates/zero-crypto/Cargo.toml`:

- `openmls_rust_crypto = "0.3"` → whatever current release brings in `hpke-rs >=0.6` (likely `openmls_rust_crypto 0.4+`; verify on crates.io). If no compatible `openmls_rust_crypto` release exists yet, the workaround is to vendor-patch or switch crypto provider — out of scope for this audit.

Once `zero-sdk-10` ships the bump, `cargo update -p hpke-rs` in `c:\code\zos` will pick up 0.6+ and clear:

- RUSTSEC-2026-0071 (critical, nonce reuse)
- RUSTSEC-2026-0070 (high, panic)
- RUSTSEC-2026-0069 (length encoding)
- RUSTSEC-2026-0072 (all-zero X25519)

**Cannot be done from `c:\code\zos` alone — major bump blocked by path-dep.**

### 3. **Required upstream PR** — bump `libp2p` / `hickory-proto` in `zero-sdk-10/crates/grid-net` 🟠

Bump `libp2p` (currently 0.56.0) in `grid-net/Cargo.toml` to a release that depends on `hickory-proto >=0.26.1`. As of writing, `libp2p-mdns 0.48.0` pins `hickory-proto 0.25`; need to wait for / push for a `libp2p-mdns 0.49+` release, then bump.

Clears RUSTSEC-2026-0119. Note RUSTSEC-2026-0118 has **no upstream fix yet** — track the hickory-dns issue and consider disabling NSEC3 validation or using a different DNS resolver in the meantime if mDNS is exposed to untrusted networks. (Defer — needs upstream coordination.)

### 4. **Required upstream PR** — bump `ml-dsa` in `zero-sdk-10/crates/zid` 🟡

Bump `zid`'s `ml-dsa = "0.0.4"` to `ml-dsa = "0.1.0-rc.3"` (or whatever is current). Clears RUSTSEC-2025-0144. Medium severity, **major bump** (pre-release version change), needs upstream API review.

### 5. **Optional cleanup** — modernise `reqwest 0.11 → 0.12` in this workspace 🟢

`apps/zero-server/Cargo.toml` line 27 (`reqwest = { version = "0.11", features = ["json"] }`) is the only dep we directly control that's flagged (it pulls unmaintained `rustls-pemfile 1.0.4`, RUSTSEC-2025-0134). Bumping to `0.12` upgrades to `rustls-pemfile 2.x`:

```toml
# apps/zero-server/Cargo.toml line 27
reqwest = { version = "0.12", features = ["json"] }
```

`reqwest 0.11 → 0.12` is a **minor breaking change** (TLS backend defaults shifted, builder API tweaks). Low priority — `rustls-pemfile` warning is "unmaintained", not an active CVE. Defer unless touching this code anyway.

### 6. **Deferred** — GTK3 unmaintained warnings 🟢

The 11 `gtk*`/`atk*`/`gdk*`/`glib`/`pango` unmaintained warnings all come from `wry 0.54.4` / `tao 0.34.8` (Tauri webview). They affect the Linux build only and require Tauri itself to migrate to GTK4 — wholly out of our hands. Mark as accepted risk; revisit when `tauri-apps/wry` ships a GTK4 line.

### What we are NOT bumping right now

| Item | Why deferred |
| --- | --- |
| `hpke-rs 0.2 → 0.6` via `[patch.crates-io]` | Major bump; `openmls_rust_crypto 0.3` declares `hpke-rs = "^0.2"`, so a patch would be rejected by semver resolution unless we ALSO patch `openmls_rust_crypto`. Upstream PR is the clean fix. |
| `hickory-proto 0.25 → 0.26` via `[patch.crates-io]` | `libp2p-mdns 0.48` declares `hickory-proto = "0.25"`; semver-incompatible. Upstream PR required. |
| `ml-dsa 0.0.4 → 0.1.0-rc.3` | Major + pre-release; needs `zid` API review. |
| Any `atk` / `gtk*` / `glib` bump | Blocked by `wry`/`tao`; we are not the gatekeeper. |
| `paste`, `proc-macro-error`, `atomic-polyfill` | Build-time / informational; no exploit, just abandoned proc-macros. |

---

## Top 3 actionable bumps the parent should apply first

1. **`npm audit fix` in `interface/`** — clears the only thing actionable from this workspace (1 high + 1 moderate, dev-only). Two-second command, no `package.json` edit strictly needed.
2. **File issue / open PR in `zero-sdk-10`** to bump `openmls_rust_crypto` so it pulls `hpke-rs >=0.6` — unblocks 1 critical + 1 high + 2 unscored crypto advisories. This is the highest-impact change but lives outside `c:\code\zos`.
3. **(Optional, in `c:\code\zos`)** Bump `reqwest = "0.11"` → `"0.12"` in `apps/zero-server/Cargo.toml` line 27 — clears the `rustls-pemfile 1.0.4` unmaintained warning, the only directly-controllable item.

## Fundamental blockers

- **Path-dep trap (Rust):** All 7 cargo CVEs flow through `crates/zos-grid/Cargo.toml`'s path-deps into `../../zero-sdk-10`. None can be patched via `cargo update -p <crate>` from inside this repo, and `[patch.crates-io]` is also blocked because the intermediate path-dep crates (`openmls_rust_crypto 0.3`, `libp2p-mdns 0.48`, `zid 0.2.0`) pin semver-incompatible major versions. **Real fix has to land in `zero-sdk-10` first.**
- **No active blockers from Windows / install side:** `cargo-audit` was already installed (0.22.1), `npm audit` ran cleanly, all four output files written. No tool failures.
- **One unfixable upstream:** `hickory-proto` RUSTSEC-2026-0118 has no upstream patch yet (advisory was published 2026-05-01). Track upstream; mitigate by network-layer scoping of mDNS exposure if that's a deployment concern.
