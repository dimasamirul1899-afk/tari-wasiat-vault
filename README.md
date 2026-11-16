# Wasiat — a digital inheritance vault on Tari Ootle

> *Wasiat* is Indonesian for "last will".

Crypto has no will. If the holder of a wallet disappears, the funds are locked forever — nobody can prove the owner is gone, and nobody can prove who should inherit. A will on paper names an heir, but it cannot move the coins, and it trusts whoever holds the keys.

**Wasiat puts the will on chain.** The owner locks XTM in a vault, names a beneficiary, and picks an inactivity period in epochs. As long as the owner is around, they send a `heartbeat` every so often and the deadline keeps moving. If the owner goes silent for the whole inactivity period, the beneficiary — and only the beneficiary — can `claim` the entire vault. The owner can change their mind at any time before a claim and `cancel` the vault, taking the funds back in full.

Entry for the **Tari November Build Contest: MVP** (cutoff 1 Dec 2026, 00:00 UTC).

## How it works

One Rust template, `WasiatVault` (`tari_template_lib` 0.40):

| Call | Who | Rule |
|---|---|---|
| `create(bucket, beneficiary, beneficiary_account, inactivity_epochs, owner_account)` | anyone (caller becomes the owner) | bucket must be TARI, amount positive, inactivity ≥ 1 epoch, beneficiary ≠ owner |
| `heartbeat()` | owner only | moves the claimable epoch to now + inactivity period |
| `claim()` | beneficiary only | only after the chain reaches the claimable epoch; pays the **whole** vault |
| `cancel()` | owner only | refunds the **whole** vault to the owner |
| `get_state()` / `get_details()` | anyone | read-only |

Rules enforced by the template itself, never just by the UI:

- Only the beneficiary can claim, and only after the inactivity deadline has passed without a heartbeat. An early claim is refused (`inactivity deadline has not been reached`).
- Only the owner can send a heartbeat or cancel.
- A claim or a cancel moves the whole vault: no partial amounts, no dust left behind.
- Once claimed or cancelled, the vault is finished: no second claim, no heartbeat, no cancel.
- Events for every step: `VaultCreated`, `Heartbeat`, `Claimed`, `Cancelled`.

A mainnet epoch is about 20 minutes, so pick the inactivity period accordingly — 100 epochs is about 33 hours (good for a demo), a real vault would use something like 6 months (~13,000 epochs).

## Demo

Live demo: **https://dimasamirul1899-afk.github.io/tari-wasiat-vault/web/index.html**

`web/` is a static, walletless simulation of the exact same state machine — no build step, no runtime dependencies. Open `web/index.html` in a browser (or serve the folder) and walk through create → heartbeat → claim, or the cancel path. Every action shows the Ootle call it maps to, and refusals use the template's exact messages.

The demo has a "Live (Sapient / Esmeralda)" tab. It is a clearly-labeled placeholder: the template has not been published to any network yet, and the tab says so. Nothing in the demo moves real funds, and nothing pretends to.

## Build and test

Requires Rust (stable) with the `wasm32-unknown-unknown` target:

```sh
rustup target add wasm32-unknown-unknown

cargo test            # engine tests (tari_template_test_tooling)
cargo build --release --target wasm32-unknown-unknown   # WASM template
```

The engine tests (`template/tests/wasiat_vault.rs`) cover: create locking funds, create guards (zero inactivity, self-beneficiary), heartbeat moving the deadline, heartbeat by a non-owner, early claim, claim by a non-beneficiary, claim after the deadline paying the exact vault amount, double-claim / heartbeat / cancel after a claim, cancel refunding the owner, and cancel by a non-owner.

WASM release build: `template/target/wasm32-unknown-unknown/release/wasiat_vault.wasm` — 129,397 bytes, sha256 `bea034ec9424a4c3f1e1ee54f88ac3b6610121dccf38cf742267f2ed91616742`.

## Publish to Esmeralda (you sign; this repo does not)

The template is built and tested, but it has not been published to Esmeralda or mainnet — publishing needs your own Tari wallet, and no key, seed, or payment address belongs in this repository. Steps:

1. Build the WASM above (or take it from a release of this repo).
2. Install the Ootle CLI (`cargo install tari-ootle-cli`, binary `tari`) or open the Tari wallet web UI (Sapient or Tari Universe) on Esmeralda.
3. Fund the wallet with tTARI (Esmeralda testnet: burn testnet Minotari and claim it in the wallet; the public faucet starts empty).
4. Publish the WASM from the wallet/CLI and note the `template_…` address.
5. Paste the template address into the demo's Live tab (and update this README).

Until then, the walletless demo is the honest path — several other November entries submitted exactly this way.

## Honest notes

- Not audited. The template is small and every transition is covered by engine tests, but do not lock meaningful amounts in it until it has been reviewed.
- Simulation-first: the web demo mirrors the template logic, but it is not the template. The template has not been published to Esmeralda or to mainnet yet, so there is no live path to try today.
- Built with an AI agent (officially allowed in this contest). MIT licensed.

## License

MIT — see [LICENSE](LICENSE).
