# Testnet deployment

> **Status: not yet deployed.** The table below is a template. Run
> [`scripts/deploy-testnet.sh`](../../scripts/deploy-testnet.sh) and fill it in
> with the real ids it prints. Do not invent contract ids.

## Deployed contracts

| Contract | Network | Contract ID | Stellar Expert |
| --- | --- | --- | --- |
| `PulseEscrow` | testnet | `TO_BE_FILLED` | [`TO_BE_FILLED`](https://stellar.expert/explorer/testnet/contract/TO_BE_FILLED) |
| `MockToken` | testnet | `TO_BE_FILLED` | [`TO_BE_FILLED`](https://stellar.expert/explorer/testnet/contract/TO_BE_FILLED) |

`MockToken` is test-only. On mainnet, pass a SEP-41 token or the Stellar Asset
Contract for the asset you settle in.

## Initialization

```bash
stellar contract invoke \
  --id "$PULSEESCROW_ID" --network testnet --source pulserun-deployer \
  -- init \
  --admin "$ADMIN_ADDRESS" \
  --dispute_window_secs 86400
```

`dispute_window_secs = 86400` gives a 24-hour challenge window. Lower it for
faster settlement in demos; raise it if requesters need more time to verify
outputs.

## Reproduce a deploy

```bash
./scripts/deploy-testnet.sh        # builds, deploys in dependency order, prints ids
```

The script is idempotent in spirit: it always builds fresh and prints the ids it
creates, so re-running produces a new deployment you can compare against.
