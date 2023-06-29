# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Implemented Iterator that is capable to consume items from tokenbuf for Tokenizer.
- Implemented SpanFormatter for Tokenizer
- Tokenizer.span_slice with 1 test
- 2 tests for Tokenizer.tokenbuf
- Tokenizer.tokenbuf push/consume methods
- 2 tests for fmt::Debug on Token and TokenBody
- fmt::Debug implementation for Token and TokenBody
- Token, TokenBody, Span structs
- Initial readme that describes goals for this project
- Project license, code of conduct