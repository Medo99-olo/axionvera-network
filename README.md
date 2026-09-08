<div align="center">

# Axionvera Network

**The smart contract and network foundation for transparent vaults, rewards, and community payouts.**

Axionvera Network powers the on-chain layer for Axionvera, including vault initialization, deposits, withdrawals, reward claims, accounting, lifecycle events, and SDK-facing contract methods.

</div>

---

## Overview

Axionvera Network is the blockchain foundation for Axionvera.

It is designed to support communities, builders, and project teams that need transparent fund management, contributor rewards, and reliable payout infrastructure.

The current codebase focuses on a clean, tested Soroban foundation before adding more advanced features.

---

## Current Focus

The restarted-codebase campaign is focused on:

- clean Soroban vault contracts
- reliable deposit and withdrawal accounting
- reward calculation and claim flows
- owner/admin initialization safety
- stable lifecycle events
- SDK-to-contract interface alignment
- network-node configuration and health checks
- local Husky quality checks
- GitHub Actions pipeline checks

---

## Repository Structure

```text
axionvera-network/
├── contracts/
│   ├── vault-contract/
│   │   └── Soroban vault contract
│   │
│   └── rewards/
│       └── Reward calculation helpers
│
├── network-node/
│   └── Network configuration and health helpers
│
├── docs/
│   └── Contract and SDK integration documentation
│
├── .github/
│   └── GitHub Actions workflows
│
└── .husky/
    └── Local pre-commit checks
```

---

## Packages

### `contracts/vault-contract`

The main Soroban vault contract.

Current capabilities include:

- vault initialization
- owner/admin state
- deposit accounting
- withdrawal accounting
- reward claim flow
- user balance queries
- total deposit queries
- lifecycle events
- initialization protection
- authorization checks
- edge-case tests

### `contracts/rewards`

Reward calculation helper crate.

Current capabilities include:

- proportional reward calculation
- pending reward calculation
- zero-value handling
- overflow-safe behavior
- large-value edge-case tests

### `network-node`

Network support crate.

Current capabilities include:

- default network configuration
- config validation
- structured health status
- environment checks
- serialization tests

#### Configuration

The network-node crate uses environment variables for configuration. See `.env.example` for the supported configuration fields:

- `AXIONVERA_NETWORK_NAME` - Target network (`local`, `testnet`, `mainnet`, `futurenet`)
- `AXIONVERA_RPC_URL` - Soroban RPC endpoint URL
- `AXIONVERA_ENVIRONMENT` - Deployment environment (`development`, `staging`, `production`)

To set up local configuration:

```bash
cp .env.example .env
# Edit .env with your values
```

---

## Quality Standard

Every new function or implementation must include unit tests.

Tests should cover:

- happy path
- invalid input
- edge cases
- expected failure behavior
- authorization behavior where applicable
- state consistency where applicable

This rule applies to contract logic, reward helpers, network-node helpers, and SDK-facing behavior.

---

## Local Development

Run the full local quality check:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

Or run the checks individually:

```bash
cargo fmt --all -- --check
```

```bash
cargo check --workspace --all-targets
```

```bash
cargo test --workspace --all-targets
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Local Commit Checks

This repository uses Husky pre-commit checks.

Before a commit is accepted locally, the project should pass:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

This helps keep commits clean before they reach GitHub.

---

## CI Pipeline

GitHub Actions runs on every pull request and every push to `main`.

The pipeline purpose is to make sure that no formatting issues, compilation errors, broken tests, or Clippy warnings are merged into the main branch.

The pipeline runs four checks in order:

| Check | Command | What fails it |
|---|---|---|
| Formatting | `cargo fmt --all -- --check` | Any unformatted Rust file |
| Workspace | `cargo check --workspace --all-targets` | Compilation errors |
| Tests | `cargo test --workspace --all-targets` | Failing assertions |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Any lint warning |

All four checks must pass before a PR can be merged.

Common reasons a PR fails CI:

- Code was not formatted before pushing. Run `cargo fmt --all` and commit the result.
- A Clippy warning was introduced. Run `cargo clippy --workspace --all-targets -- -D warnings` and address every warning.
- A test was broken by a change. Run `cargo test --workspace --all-targets` locally and fix all failures.
- A compilation error was introduced. Run `cargo check --workspace --all-targets` and fix all errors.

For full details on each check, how to reproduce failures locally, and how to fix them, see [docs/ci-and-local-checks.md](./docs/ci-and-local-checks.md).

---

## Building the Vault Contract

The vault contract can be built as a WASM binary for deployment using the provided build script.

### Build Script

A repeatable build script is provided at `scripts/build-vault-wasm.sh`.

To build the vault contract WASM:

```bash
./scripts/build-vault-wasm.sh
```

The script will:
- Check that the wasm32-unknown-unknown target is installed (add it if missing)
- Build the vault contract for the wasm32 target in release mode
- Output the WASM file location on success

The built WASM file will be located at:
```
contracts/vault-contract/target/wasm32-unknown-unknown/release/axionvera_vault_contract.wasm
```

If the contract directory is missing or the build fails, the script will exit with a clear error message.

---

## Testnet Deployment

Before deploying the vault contract to Stellar testnet, follow the [Vault Contract Testnet Deployment Checklist](./docs/testnet-deployment-checklist.md).

The checklist covers local quality checks, the WASM build, explicit testnet network selection, contract ID recording, initialization, and post-deployment validation. It is for testnet preparation only and does not indicate production readiness.

---

## Contract Design Goals

Axionvera Network aims to keep the vault layer:

- simple
- testable
- predictable
- SDK-friendly
- event-driven
- safe by default
- easy to document
- easy to extend

The current implementation intentionally prioritizes a strong foundation over unnecessary complexity.

---

## SDK Alignment

Axionvera Network is designed to work with the Axionvera SDK.

The SDK should be able to map cleanly to the vault contract methods for:

- reading vault information
- reading user balances
- reading pending rewards
- submitting deposits
- submitting withdrawals
- claiming rewards
- tracking emitted events

Contract method names, argument order, return values, and event behavior should remain stable once documented.
See the [SDK-to-Contract Interface Documentation](./docs/sdk-contract-interface.md) for full integration details.

---

## Contributing

Contributions are welcome through assigned issues.

Before opening a pull request:

- make sure the issue is assigned to you
- keep the PR focused
- add or update unit tests
- run all local checks
- include a clear PR summary
- reference the issue number

See [CONTRIBUTING.md](./CONTRIBUTING.md) for full contribution guidance.

---

## Security

Axionvera Network is under active development and has not yet completed a formal security audit.

Do not treat the current codebase as production-audited.

For security guidance, see [SECURITY.md](./SECURITY.md).

---

## License

This project is licensed under the MIT License.

See [LICENSE](./LICENSE).

---

<div align="center">

**Axionvera Network: clean contracts, tested logic, transparent rewards.**

</div>
