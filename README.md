# deckwatch-plugin-fluent-bit

A [deckwatch](https://github.com/alexconrey/deckwatch) plugin that provisions a Fluent Bit
log-shipping sidecar for an application via the **Infrastructure tab** — no annotations required.

## Usage

1. Open the Application detail page in deckwatch.
2. Click the **Infrastructure** tab.
3. Click **+ Add Fluent Bit**.
4. Optionally set an output endpoint (e.g. Elasticsearch URL), then click **Provision**.

Once provisioned, deckwatch injects on the next reconcile:

- **Sidecar**: `fluent/fluent-bit:3.1` (50m CPU / 64Mi memory), named `fluent-bit`
  - `FLUENT_BIT_OUTPUT` is set as a sidecar env var when the output endpoint field is non-empty
- **Annotation**: `deckwatch.io/fluent-bit: "provisioned"` on all deployments

No application-level env vars are injected into the primary container (Fluent Bit reads logs
from shared volumes or stdout, not from env vars).

To remove the sidecar, click **Deprovision** on the Infrastructure tab. The sidecar is removed
from all deployments on the next reconcile via server-side apply.

## Configuration

### Resource fields (per-provisioned instance)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `FLUENT_BIT_IMAGE` | String | `fluent/fluent-bit:3.1` | Fluent Bit container image |
| `FLUENT_BIT_OUTPUT` | String | _(empty)_ | Log output destination URL (e.g. `http://es.example.com:9200`). Injected as `FLUENT_BIT_OUTPUT` into the sidecar. |

### Plugin config (global defaults)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `FLUENT_BIT_IMAGE` | String | `fluent/fluent-bit:3.1` | Default Fluent Bit image (overridable per-resource) |

Set global defaults in **Settings → Plugins → Fluent Bit → Configure**.

## Building

```bash
# Install the wasm32 target (once)
rustup target add wasm32-unknown-unknown

# Build the WASM artifact
make build          # → dist/plugin.wasm

# Run host-target unit tests
make test
```

## Marketplace

This plugin is available on the [deckwatch marketplace](http://market.deckwatch.io).

To install via the deckwatch UI: **Settings → Plugins → Browse Marketplace → Fluent Bit**.

### Submitting to the marketplace

To add or update this plugin in the marketplace catalog, open a PR against
[alexconrey/deckwatch-marketplace](https://github.com/alexconrey/deckwatch-marketplace)
adding the following entry to `catalog.json`:

```json
{
  "name": "Fluent Bit",
  "slug": "deckwatch-plugin-fluent-bit",
  "description": "Provisions a Fluent Bit log-shipping sidecar via the Application Infrastructure tab.",
  "author": "alexconrey",
  "homepage": "https://github.com/alexconrey/deckwatch-plugin-fluent-bit",
  "trust_level": "community",
  "tags": ["fluent-bit", "logging", "sidecar"],
  "latest_version": "0.1.0",
  "source": {
    "type": "github",
    "repo": "alexconrey/deckwatch-plugin-fluent-bit",
    "ref": "v0.1.0",
    "path": "plugin.wasm",
    "use_release": true
  },
  "allowed_hosts_hint": []
}
```

Also add `plugins/deckwatch-plugin-fluent-bit.md` following the template in the marketplace repo.

## License

Apache-2.0
