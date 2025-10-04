# Changelog

All notable changes to the Honorary Fee Position program will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-10-04

### Added
- Initial release of honorary fee position program
- `initialize_honorary_position` instruction for creating program-owned LP positions
- `crank_distribute` instruction for permissionless fee distribution
- Quote-only fee accrual enforcement with deterministic validation
- Streamflow integration for reading still-locked amounts
- Pro-rata distribution based on locked amounts with f_locked calculation
- Daily cap and minimum payout enforcement
- Dust handling and carry-forward mechanism
- Pagination support for large investor sets
- Idempotency and resumable distribution
- Comprehensive error codes and events
- Full test suite with local validator testing
- Mock Streamflow adapter for testing
- Scripts for development and testing
- Complete documentation in README.md

### Security
- All math operations use checked arithmetic
- PDA seeds are deterministic and properly validated
- Position ownership enforced through PDAs
- Base fee detection with deterministic failure
