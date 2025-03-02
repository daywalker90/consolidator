# Changelog

## [0.2.2] Unreleased

### Added

- option `consolidator-persist` with default `False`, you can turn this on for `consolidate-below` to be persistent between plugin/node restarts

## [0.2.1] 2024-12-10

### Changed

- upgrade dependencies

## [0.2.0] 2024-11-10

### Changed

- Instead of CLN's *opening* feerate (basically blockcount:12 feerate) ``consolidator`` now uses the blockcount:6 feerate for ``consolidate-below``. Because we already wait for the desired feerate in the mempool before consolidating we want the tx confirmed fast/reliable at that point.

### Added

- option ``consolidator-feemulti`` to multiply the fee for the actual tx from ``consolidate-below``. The check is stil made against CLN's blockcount:6 feerate but the tx will have this multiplier applied. Set this higher if you want an even better chance to get the tx confirmed in a timely manner. Keep in mind that CLN likes to naturally overshoot the actual feerate a little. Defaults to ``1.1`` (10%)
- some sanity checks for all option values

## [0.1.1] 2024-11-07

### Changed

- improve logging

## [0.1.0] 2024-11-07

### Added

- initial release