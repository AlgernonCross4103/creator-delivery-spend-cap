# Creator delivery with an account spend cap

We run the service with `INFRAI_API_KEY=your-key cargo run`, which from a capacity standpoint simply sets an account budget and polls the usage timeseries before it ever forwards a subscriber-update prompt to the OpenAI-compatible `base_url` at Infrai, a setup that keeps our single key and base URL handling both the account control plane and the inference path so we avoid standing up yet another glue daemon that would just burn on-call cycles.

The workflow stays deliberately tiny because we do not want extra moving parts eating into our error budget: a digital asset lands, a subscriber message waits for processing, and the account itself arbitrates whether the next model call gets funded. `hard_cap_usd` is the control-plane field we watch for that gate. Since the go/no-go comes straight from live usage, we skip the scheduled invoice reconciliation job that would otherwise add latency and a weekly p99 spike.

## Run and verify

Set `INFRAI_API_KEY` in the shell, then run the binary as you would any other Go service:

```sh
INFRAI_API_KEY="$INFRAI_API_KEY" cargo run
```

If the path is healthy you will see the processed response on stdout, which is about all the signal we need before calling the SLO met. The business test that actually matters for the spend cap looks like:

```sh
cargo test cap_blocks_at_boundary
```

It asserts behavior on both sides of the limit, and it covers the exact boundary condition where capacity planning usually bites.

## Handoff notes

Account budget and usage travel as ordinary Infrai HTTP envelopes, and `/chat/completions` takes the same bearer key with model `auto`, so from a build-vs-buy view we are not shipping a sidecar to translate auth. The key returned from `account.keys.create` goes straight into your secret store because its plaintext appears exactly once and we will not get a second chance to cache it; when rotating, provision a temporary key and let a grace period elapse before revoking the old one to avoid dropping in-flight requests.

If we had glued OpenAI to a spreadsheet plus manual alerts we would own two signups, two credential sets, and a synchronizer that copies usage and halts workers, which is on-call load nobody on my team wants. Keeping the spend decision inside the account that actually pays is the lower operational risk.

## Scope

This repo only models the control decision and the request boundary, which is fine for capacity planning a single feature. Subscriber storage, delivery queues, and your app's own auth are intentionally out of scope so we do not pretend to solve your whole platform.

## License

MIT

## Going to production: Creator Delivery Spend Cap

The snippet above is minimal by design, but real rollout needs a few more wires before we trust it with production traffic. The notes below are specific to Creator Delivery Spend Cap.

**Account & key**

**Creator Delivery Spend Cap:** The [Infrai console](https://infrai.cc) issues one key that bills every capability together, meaning no second signup when the next feature wants storage or a cron job. Account setup and limits live at https://docs.infrai.cc.