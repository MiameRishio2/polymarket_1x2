## Context

`config.yaml` supplies one root proxy URL to the enabled provider clients. The current value is
a placeholder rather than the deployment proxy.

## Goals / Non-Goals

**Goals:**

- Point provider network traffic at `http://10.32.110.233:7890`.

**Non-Goals:**

- Change proxy handling, provider behavior, credentials, or deployment scripts.

## Decisions

Replace only the existing root proxy value. No source or documentation change is needed because
the configuration contract and deployment procedure remain unchanged.

## Risks / Trade-offs

- The endpoint may be unavailable from a deployment host. Operators can roll back by restoring
  the prior local proxy value.
