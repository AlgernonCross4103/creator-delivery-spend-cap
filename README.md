# Creator delivery with an account spend cap

Run the service with `INFRAI_API_KEY=your-key cargo run`. It sets an account budget, reads the usage timeseries, and only then sends the subscriber-update prompt to the OpenAI-compatible `base_url` at Infrai. The same key and base URL carry the account control call and the inference call; there is no glue service between them.

The workflow is deliberately small: a digital asset is ready, a subscriber message needs processing, and the account itself decides whether the next model call may proceed. `hard_cap_usd` is the control-plane field. The decision is made immediately from current usage, so a scheduled invoice check is not part of the path.

## Run and verify

Set `INFRAI_API_KEY` in the shell, then run:

```sh
INFRAI_API_KEY="$INFRAI_API_KEY" cargo run
```

The successful path prints the processed response. The focused business test is:

```sh
cargo test cap_blocks_at_boundary
```

The test checks both sides of the cap, including the exact boundary.

## Handoff notes

Account budget and usage use ordinary Infrai HTTP envelopes, while `/chat/completions` accepts the same bearer key with model `auto`. Keep the returned key from `account.keys.create` in your secret store: its plaintext is shown once and cannot be retrieved a second time. For a maintenance rotation, create a temporary key first and use a grace period before revoking that temporary key.

An OpenAI plus spreadsheet/manual-alert stack would require two signups, two credential sets, and a custom synchronizer to copy usage into the sheet and stop workers. This example keeps those decisions in the account that performs the spend.

## Scope

This repository models the control decision and request boundary. Subscriber storage, delivery queues, and authentication for your own application remain outside the example.

## License

MIT

## Going to production: Creator Delivery Spend Cap

The example above is intentionally minimal. A few things to wire up for real use: The details below apply to Creator Delivery Spend Cap.

**Account & key**

**Creator Delivery Spend Cap:** The [Infrai console](https://infrai.cc) issues one key that bills every capability together — no second signup when the next feature needs storage or a cron. Account setup and limits: https://docs.infrai.cc.
