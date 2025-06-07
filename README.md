# Cinnabar

A CI/CD system built in Rust, mainly intended as a learning project, not a serious attempt at creating a fully-featured product.

## Features

- Multi-step pipelines based on docker containers
- Pipeline triggers based on conditions (e.g. only trigger pipelines for pull-requests, or pushes to the main branch)
- JSON or [Jsonnet](https://jsonnet.org/) for configuration
- Basic caching of build artifacts for subsequent runs (to be improved)

## Missing features

- Configurable dependencies between steps (right now steps run sequentially)
- Persistence of pipeline runs/logs
- Distributed runners (routing pipelines to different machines based on tags)
- Autoscaling runners (spinning up and destroying machines dynamically based on load)
- Cron-based triggers for recurring pipelines
- etc.
