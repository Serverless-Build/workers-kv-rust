# Workers KV — read-heavy application configuration

Store versioned application configuration in Workers KV, read it from the edge, and evaluate a UI feature rule. This is a KV-shaped workload: small values, many reads, and infrequent operator writes.

## Pattern at a glance

| | |
|---|---|
| Runtime | Cloudflare Workers |
| Language | Rust |
| Storage | Workers KV |
| KV operations | `getWithMetadata()` on seeded, versioned keys |
| Public API | Read-only configuration and feature evaluation |

## Why KV fits

Applications often need the same configuration on many edge requests: UI settings, feature rollout rules, translations, or routing configuration. They can tolerate a short period of stale data during a rollout. KV caches frequently read values close to requests without requiring a relational query or coordinated actor for each lookup.

The seeds contain two independently addressable revisions (`app-config:v1`, `app-config:v2`). The v1 feature is available on selected plans and regions; v2 expands it. The application reads the chosen revision from **real KV**, then evaluates the stored rule against demo inputs.

This does not model user sessions, private per-visitor storage, atomic counters, or immediate global cache invalidation. A UI feature flag is not an authorization mechanism.

## Local setup

Install Node.js 22+ and Rust, then:

```sh
rustup target add wasm32-unknown-unknown
npm install
npm run check
npm run seed:local
npm run dev
```

```sh
curl 'http://localhost:8787/config?version=v1'
curl 'http://localhost:8787/evaluate?version=v1&plan=free&region=EU'
curl 'http://localhost:8787/evaluate?version=v2&plan=free&region=EU'
```

The evaluation changes from `false` to `true` because the stored rollout rule changed. Try Pro in APAC under v1, then v2, to see the region rule change too.

## Permanent deployment

```sh
npx wrangler login
npm run setup
npm run seed:remote
npm run deploy
```

`setup` creates a namespace in your account and updates the `KV` binding ID in your local Wrangler configuration. The checked-in configuration contains no shared account, namespace ID, or custom domain. Repeat the requests at the printed `workers.dev` URL. If using automatic namespace provisioning on first deployment instead, seed that namespace afterwards.

The current temporary-account API rejects namespace creation for this demo (verified HTTP 401, API code 10000). Use an authenticated account for the complete KV example; a binding-free temporary Worker would not demonstrate this pattern.

## Consistency and rollout

- KV is eventually consistent. Updating a key is not an instant global invalidation; different locations can temporarily see different values.
- `cacheTtl: 60` is an explicit KV read-cache policy. The response reports the policy, not an invented cache-hit count or propagation measurement.
- Publish a new immutable revision key rather than overwriting an existing revision. Reference a version deliberately; a mutable "current" pointer would itself be eventually consistent.
- The seed command is an operator operation. There is no public configuration-write endpoint and no caller-IP privacy claim.
- These are public demo settings. Do not put secrets in public configuration responses. Verify real user identity separately when feature choices control protected operations.

## Test the behavior

| Request | Expected result |
|---|---|
| `/config?version=v1` | Revision `v1`, metadata revision `v1`, theme `warm` |
| `/config?version=v2` | Revision `v2`, metadata revision `v2`, theme `cool` |
| `/evaluate?version=v1&plan=free&region=EU` | `fastSearchEnabled: false` |
| `/evaluate?version=v1&plan=pro&region=EU` | `fastSearchEnabled: true` |
| `/evaluate?version=v1&plan=pro&region=APAC` | `fastSearchEnabled: false` |
| `/evaluate?version=v2&plan=free&region=APAC` | `fastSearchEnabled: true` |
| `/config?version=v3` | HTTP 400 — the version is not allowlisted |
| `POST /config` | HTTP 405 — there is no public write operation |

Starting without seeds returns HTTP 503 from the config endpoints, with setup instructions. `/health` checks Worker liveness only. Successful config responses use `Cache-Control: no-store` so repeated requests reach the Worker; KV's internal read cache is a separate layer.

## References

- [How KV works and consistency](https://developers.cloudflare.com/kv/concepts/how-kv-works/)
- [Read values and metadata](https://developers.cloudflare.com/kv/api/read-key-value-pairs/)
- [Wrangler KV commands](https://developers.cloudflare.com/kv/reference/kv-commands/)

## Pattern and live demo

- [Pattern page](https://serverless.build/patterns/worker-kv)
- [Live deployment](https://workers-kv-config-rust.dwarven.workers.dev)
