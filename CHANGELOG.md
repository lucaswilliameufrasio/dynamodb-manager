# Changelog

All notable changes to DynamoDB Manager are documented here.

## [1.0.1] - 2026-09-26

### Bug Fixes

- *(release)* Checkout repo before publishing release

### Chores

- Upgrade Flutter to 3.47.5
## [1.0.0] - 2026-09-25

### Bug Fixes

- Upgrade fl_chart to 1.2.0 and fix item equality in DynamoItem
- Add dirty tracking and switch dialog to item editor

### CI / Build

- Limit release platforms to Flutter-supported targets

### Chores

- Add boilerplate
- Integrate flutter rust bridge
- Set rust version used
- Set flutter version used

### Documentation

- Update README with how to update Rust code and update flutter rust bridge integration
- Rewrite README and add architecture, auth, dev, troubleshooting, and security docs

### Features

- DynamoDB desktop app with Flutter UI + Rust backend
- Dev logs panel with Rust-side instrumentation
- Add dev commands (mise+make), metrics modal with fl_chart, and update docs
- *(dynamodb)* Add table and item management
- *(release)* Add Flutter desktop packages

### Performance

- *(dynamodb)* Reuse clients with bounded cache

### Testing

- *(perf)* Add local DynamoDB and Flutter baselines
- *(perf)* Validate DynamoDB calls against Floci
