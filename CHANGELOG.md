# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Updated tokenlist_match_or_fail, so that now it ignores Token::StateChange.
- New enum TokenizerState::ExpectInstructionClose.
- Tokenizer.tokenize_instruction_args with 4 tests.
- New enum ParseError::OpenInstruction.
- More information (component enum, line number) inside InternalError struct.
- Tokenizer ability to parse "@include(" into Token::Include, Token::OpenParen.
  Added token UnescapedAt. 6 tests for @include cases.
- Tokenizer.parse_error_prev.
- Implemented Tokenizer.return_tokenized.
- Renamed Tokenizer.fail to fail_token and refactred it so that it can be
  reused for different return contexts like Result, Option, raw Token. 
- Implemented Iterator that is capable to consume items from tokenbuf for 
  Tokenizer.
- Implemented SpanFormatter for Tokenizer
- Tokenizer.span_slice with 1 test
- 2 tests for Tokenizer.tokenbuf
- Tokenizer.tokenbuf push/consume methods
- 2 tests for fmt::Debug on Token and TokenBody
- fmt::Debug implementation for Token and TokenBody
- Token, TokenBody, Span structs
- Initial readme that describes goals for this project
- Project license, code of conduct

### Changed
- Tokenizer.return_tokenized now updates index and restores state from state_snap.
  Removed similad code part to return_tokenized from tokenbuf_consume, since now
  it can be handled by return_tokenized and there is no need to handle same cases
  in multiple places. Since this changed the behavior for tokenbuf, had to make
  some minor modifications to Iterator. Created test, that tokenizes whole
  input as defered tokens; this is to test if return_tokenized works as expected.