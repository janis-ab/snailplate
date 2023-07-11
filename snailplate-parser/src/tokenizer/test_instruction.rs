use crate::{
   tokenizer::Tokenizer,
   token::Token,
   tokenbody::TokenBody,
   span::Span,
   parse_error::{
      ParseError,
      InstructionError
   },
};

use super::tokenlist_match_or_fail;



// This tests if tokenizer can parse single @include() directive that starts
// exactly at current Tokenizer position. This is artificial test, but useful
// while developing. It should be that this test
// hits instruction_tokenize_correct_paren_now tokenization path.
// cargo test -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_01 -- --nocapture
#[test]
fn tokenizer_instruction_include_test_01() {
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      t.src_push(None, "@include(".into());
   }

   let list: Vec<Token> = [
      Token::Real(TokenBody::Include(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 8
      })),
      Token::Real(TokenBody::OpenParen(Span {
         index: 0, line: 0, pos_line: 8, pos_region: 8, pos_zero: 8, length: 1
      })),
      Token::Error(ParseError::OpenInstruction(InstructionError {
         pos_zero: 0,
      })),
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// This tests if tokenizer can parse single @include() directive that starts
// exactly at current Tokenizer position. This is artificial test, but useful
// while developing. It should be that this test
// hits instruction_tokenize_correct_paren_now tokenization path.
// cargo test -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_02 -- --nocapture
#[test]
fn tokenizer_instruction_include_test_02() {
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      t.src_push(None, "@include \t\n  \n\n    (".into());
   }

   let list: Vec<Token> = [
      Token::Real(TokenBody::Include(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 8
      })),
      Token::Real(TokenBody::WhiteSpace(Span {
         index: 0, line: 0, pos_line: 8, pos_region: 8, pos_zero: 8, length: 2
      })),
      Token::Real(TokenBody::Newline(Span {
         index: 0, line: 0, pos_line: 10, pos_region: 10, pos_zero: 10, length: 1
      })),
      Token::Real(TokenBody::WhiteSpace(Span {
         index: 0, line: 1, pos_line: 0, pos_region: 11, pos_zero: 11, length: 2
      })),
      Token::Real(TokenBody::Newline(Span {
         index: 0, line: 1, pos_line: 2, pos_region: 13, pos_zero: 13, length: 1
      })),
      Token::Real(TokenBody::Newline(Span {
         index: 0, line: 2, pos_line: 0, pos_region: 14, pos_zero: 14, length: 1
      })),
      Token::Real(TokenBody::WhiteSpace(Span {
         index: 0, line: 3, pos_line: 0, pos_region: 15, pos_zero: 15, length: 4
      })),
      Token::Real(TokenBody::OpenParen(Span {
         index: 0, line: 3, pos_line: 4, pos_region: 19, pos_zero: 19, length: 1
      })),
      Token::Error(ParseError::OpenInstruction(InstructionError {
         pos_zero: 0,
      })),
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// This tests if tokenizer can parse single @include() directive that starts
// exactly at current Tokenizer position. This is artificial test, but useful
// while developing. It should be that this test
// hits instruction_tokenize_correct_paren_defered tokenization path.
// cargo test -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_03 -- --nocapture
#[test]
fn tokenizer_instruction_include_test_03() {
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      t.src_push(None, "X Y@include(".into());
   }

   let list: Vec<Token> = [
      Token::Real(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 3
      })),
      Token::Real(TokenBody::Include(Span {
         index: 0, line: 0, pos_line: 3, pos_region: 3, pos_zero: 3, length: 8
      })),
      Token::Real(TokenBody::OpenParen(Span {
         index: 0, line: 0, pos_line: 11, pos_region: 11, pos_zero: 11, length: 1
      })),
      Token::Error(ParseError::OpenInstruction(InstructionError {
         pos_zero: 3,
      })),
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// This tests unfinished include instruction handling. This test should hit
// instruction_tokenize_unfinished case when tokenizing instruction.
//
// cargo test -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_04 -- --nocapture
#[test]
fn tokenizer_instruction_include_test_04() {
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      t.src_push(None, "@inclu".into());
   }


   let list: Vec<Token> = [
      Token::Real(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 6
      })),
      // TODO: in future we should test Phantom tokens as well, that have 
      // warning information with suggestions.
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// cargo test -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_05 -- --nocapture
#[test]
fn tokenizer_instruction_include_test_05() {
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      t.src_push(None, "@ include(".into());
   }

   let list: Vec<Token> = [
      Token::Real(TokenBody::UnescapedAt(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 1
      })),
      Token::Real(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 1, pos_region: 1, pos_zero: 1, length: 9
      })),
      // TODO: in future we should test Phantom tokens as well, that have
      // warning information with suggestions.
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// cargo test -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_06 -- --nocapture
#[test]
fn tokenizer_instruction_include_test_06() {
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      t.src_push(None, "@ include)(".into());
   }

   let list: Vec<Token> = [
      Token::Real(TokenBody::UnescapedAt(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 1
      })),
      Token::Error(ParseError::InstructionError(InstructionError {
            pos_zero: 0
      })),
      Token::Real(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 1, pos_region: 1, pos_zero: 1, length: 10
      })),
      // TODO: in future we should test Phantom tokens as well, that have
      // warning information with suggestions.
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// Tokenizer does not report anything about any parenthesis, but once an
// instruction that expects params has been matched, Tokenizer goes into special
// mode where where it counts open/closing parenthesis and when matched
// parenthesis is met, it returns it and switches back to ExpectDefered.
//
// Everything between parenthesis is returned as Token::Defered, unless
// different mode is used.
//
// cargo test -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_07 -- --nocapture
#[test]
fn tokenizer_instruction_include_test_07() {
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      t.src_push(None, "@include()".into());
   }

   // TODO: after @include, tokenizer has to go into state expect close paren

   let list: Vec<Token> = [
      Token::Real(TokenBody::Include(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 8
      })),
      Token::Real(TokenBody::OpenParen(Span {
         index: 0, line: 0, pos_line: 8, pos_region: 8, pos_zero: 8, length: 1
      })),
      Token::Real(TokenBody::CloseParen(Span {
         index: 0, line: 0, pos_line: 9, pos_region: 9, pos_zero: 9, length: 1
      })),
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// cargo test -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_08 -- --nocapture
#[test]
fn tokenizer_instruction_include_test_08() {
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      t.src_push(None, "@include(def(er)ed)".into());
   }

   // TODO: after @include, tokenizer has to go into state expect close paren

   let list: Vec<Token> = [
      Token::Real(TokenBody::Include(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 8
      })),
      Token::Real(TokenBody::OpenParen(Span {
         index: 0, line: 0, pos_line: 8, pos_region: 8, pos_zero: 8, length: 1
      })),
      Token::Real(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 9, pos_region: 9, pos_zero: 9, length: 9
      })),
      Token::Real(TokenBody::CloseParen(Span {
         index: 0, line: 0, pos_line: 18, pos_region: 18, pos_zero: 18, length: 1
      })),
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}




// cargo test -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_09 -- --nocapture
#[test]
fn tokenizer_instruction_include_test_09() {
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      t.src_push(None, "@include(defered)".into());
   }

   // TODO: after @include, tokenizer has to go into state expect close paren

   let list: Vec<Token> = [
      Token::Real(TokenBody::Include(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 8
      })),
      Token::Real(TokenBody::OpenParen(Span {
         index: 0, line: 0, pos_line: 8, pos_region: 8, pos_zero: 8, length: 1
      })),
      Token::Real(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 9, pos_region: 9, pos_zero: 9, length: 7
      })),
      Token::Real(TokenBody::CloseParen(Span {
         index: 0, line: 0, pos_line: 16, pos_region: 16, pos_zero: 16, length: 1
      })),
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// cargo test -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_10 -- --nocapture
#[test]
fn tokenizer_instruction_include_test_10() {
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      t.src_push(None, "@include(def\ne@red)".into());
   }

   let list: Vec<Token> = [
      Token::Real(TokenBody::Include(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 8
      })),
      Token::Real(TokenBody::OpenParen(Span {
         index: 0, line: 0, pos_line: 8, pos_region: 8, pos_zero: 8, length: 1
      })),

      Token::Real(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 9, pos_region: 9, pos_zero: 9, length: 3
      })),
      Token::Real(TokenBody::Newline(Span {
         index: 0, line: 0, pos_line: 12, pos_region: 12, pos_zero: 12, length: 1
      })),
      Token::Real(TokenBody::Defered(Span {
         index: 0, line: 1, pos_line: 0, pos_region: 13, pos_zero: 13, length: 5
      })),

      Token::Real(TokenBody::CloseParen(Span {
         index: 0, line: 1, pos_line: 5, pos_region: 18, pos_zero: 18, length: 1
      })),
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// cargo test -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_11 -- --nocapture
#[test]
fn tokenizer_instruction_include_test_11() {
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      t.src_push(None, "X Y@include(xxx".into());
   }

   let list: Vec<Token> = [
      Token::Real(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 3
      })),
      Token::Real(TokenBody::Include(Span {
         index: 0, line: 0, pos_line: 3, pos_region: 3, pos_zero: 3, length: 8
      })),
      Token::Real(TokenBody::OpenParen(Span {
         index: 0, line: 0, pos_line: 11, pos_region: 11, pos_zero: 11, length: 1
      })),
      Token::Real(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 12, pos_region: 12, pos_zero: 12, length: 3
      })),
      Token::Error(ParseError::OpenInstruction(InstructionError {
         pos_zero: 3,
      })),
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// cargo test -F future_passing_tests -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_101 -- --nocapture
#[test]
#[cfg(feature = "future_passing_tests")]
fn tokenizer_instruction_include_test_101() {
   println!("Starging iterator test 05");
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      t.src_push(None, "@include(xxx)".into());
   }

   let list: Vec<Token> = [
      Token::Real(TokenBody::Include(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 8
      })),
      Token::Real(TokenBody::OpenParen(Span {
         index: 0, line: 0, pos_line: 8, pos_region: 8, pos_zero: 8, length: 1
      })),
      Token::Real(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 9, pos_region: 9, pos_zero: 9, length: 3
      })),
      Token::Real(TokenBody::CloseParen(Span {
         index: 0, line: 0, pos_line: 12, pos_region: 12, pos_zero: 12, length: 1
      })),
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// cargo test -F future_passing_tests -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_102 -- --nocapture
#[test]
#[cfg(feature = "future_passing_tests")]
fn tokenizer_instruction_include_test_102() {
   println!("Starging iterator test 05");
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      t.src_push(None, "@if(prop == \")xx\")".into());
   }

   let list: Vec<Token> = [
      Token::Real(TokenBody::Include(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 8
      })),
      // TODO:
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// cargo test -F future_passing_tests -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose tokenizer::test_instruction::tokenizer_instruction_include_test_103 -- --nocapture
#[test]
#[cfg(feature = "future_passing_tests")]
fn tokenizer_instruction_include_test_103() {
   println!("Starging iterator test 05");
   let mut t = Tokenizer::new();

   // In this case Tokenizer should pass for the wrong reasons though, because
   // instruction contains matching parenthesis.
   #[allow(unused_must_use)] {
      t.src_push(None, "@if(/*(*/ prop == \")xx\")".into());
   }

   let list: Vec<Token> = [
      Token::Real(TokenBody::Include(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 8
      })),
      // TODO:
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list, true){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}


