## Why

The committed runtime configuration still contains a proxy placeholder, so provider HTTP
requests cannot use the required reachable proxy endpoint.

## What Changes

- Set the root proxy URL to `http://10.32.110.233:7890`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None.

## Impact

Only the proxy value in `config.yaml` changes. Both provider clients continue to consume the
same root proxy setting.
