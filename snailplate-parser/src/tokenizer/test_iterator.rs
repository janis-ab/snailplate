use crate::{
   tokenizer::Tokenizer,
   token::Token,
   tokenbody::TokenBody,
   span::Span,
   parse_error::ParseError,
};

use super::tokenlist_match_or_fail;

// cargo test -F dbg_tokenizer_verbose tokenizer::test_iterator::tokenizer_iterator_test_01 -- --nocapture
#[test]
fn tokenizer_iterator_test_01() {
   println!("Tokenizer iterator test");

   let mut t = Tokenizer::new();

   if let Err(e) = t.src_push(None, "AAABBB".into()){
      panic!("Expected Ok(None), got: Err({:?})", e);
   }

   #[allow(unused_must_use)] {
      t.tokenbuf_push(Token::Real(TokenBody::Defered(Span {
         index: 0, line: 0, pos_region: 0, pos_zero: 0, pos_line: 0, length: 3
      })));
   }

   #[allow(unused_must_use)] {
      t.tokenbuf_push(Token::Real(TokenBody::Defered(Span {
         index: 0, line: 0, pos_region: 3, pos_zero: 3, pos_line: 3, length: 3
      })));
   }

   if let Some(token) = t.next() {
      if let Token::Real(body) = token {
         if let TokenBody::Defered(span) = body {
            assert_eq!(span.index, 0);
            assert_eq!(span.line, 0);
            assert_eq!(span.pos_region, 0);
            assert_eq!(span.pos_zero, 0);
            assert_eq!(span.pos_line, 0);
            assert_eq!(span.length, 3);
         }
         else {
            panic!("Token-1 type is not Defered!");
         }
      }
      else {
         panic!("Bad token-1 returned! Token: {:?}", token);
      }
   }
   else {
      panic!("Token-1 was not returned.");
   }

   if let Some(token) = t.next() {
      if let Token::Real(body) = token {
         if let TokenBody::Defered(span) = body {
            assert_eq!(span.index, 0);
            assert_eq!(span.line, 0);
            assert_eq!(span.pos_region, 3);
            assert_eq!(span.pos_zero, 3);
            assert_eq!(span.pos_line, 3);
            assert_eq!(span.length, 3);
         }
         else {
            panic!("Token-2 type is not Defered!");
         }
      }
      else {
         panic!("Bad token-2 returned!");
      }
   }
   else {
      panic!("Token-2 was not returned.");
   }

   if let Some(..) = t.next() {
      panic!("Received token, when None should be returned.");
   }
}



// This tests if tokenizer can parse multiple pushed sources as derefs. This is 
// artificial test, but useful while developing.
// cargo test -F dbg_tokenbuf_verbose tokenizer::test_iterator::tokenizer_iterator_test_02 -- --nocapture
#[test]
fn tokenizer_iterator_test_02() {
   println!("Starging iterator test04");
   let mut t = Tokenizer::new();

   #[allow(unused_must_use)] {
      // t.src_push(None, "deref1.1@include(filepath)deref1.2".into());
      t.src_push(None, "CCC".into());
      t.src_push(None, "BBB".into());
      t.src_push(None, "AAA".into());
   }


   // We expect tokens to be like this:
   // - length is 3 for all tokens,
   // - index decreases, because items are pulled from stack
   // - pos_zero always increases by parsed token size, thus 0, 3, 6.
   let list: Vec<Token> = [
      Token::Real(TokenBody::Defered(Span {
         index: 2, line: 0, length: 3, pos_line: 0, pos_region: 0, pos_zero: 0
      })),
      Token::Real(TokenBody::Defered(Span {
         index: 1, line: 0, length: 3, pos_line: 0, pos_region: 0, pos_zero: 3
      })),
      Token::Real(TokenBody::Defered(Span {
         index: 0, line: 0, length: 3, pos_line: 0, pos_region: 0, pos_zero: 6
      })),
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// This tests if tokenizer returns correctly buffered tokens before returning 
// error Token, since there is no input. This test in a way is unnatural when 
// compared to real life scenario, but while developing, this is what i have.
// cargo test tokenizer::test_iterator::tokenizer_iterator_test_03 -- --nocapture
#[test]
fn tokenizer_iterator_test_03() {
   println!("Starging iterator test01");
   let mut t = Tokenizer::new();

   // This is just a dummy buffer contents, so that Tokenizer does not panic.
   #[allow(unused_must_use)] {
      t.src_push(None, "XXXXXXX".into());
   }

   //
   // Artificially parse Tokenizers string buffer.
   //

   #[allow(unused_must_use)] {
      t.tokenbuf_push(Token::Phantom(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 1
      })));
   }

   #[allow(unused_must_use)] {
      t.tokenbuf_push(Token::Phantom(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 1, pos_region: 1, pos_zero: 1, length: 5
      })));
   }

   #[allow(unused_must_use)] {
      t.tokenbuf_push(Token::Phantom(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 6, pos_region: 6, pos_zero: 6, length: 1
      })));
   }

   //
   // Test tokenized list.
   //

   let list: Vec<Token> = [
      Token::Phantom(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 0, pos_region: 0, pos_zero: 0, length: 1
      })),
      Token::Phantom(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 1, pos_region: 1, pos_zero: 1, length: 5
      })),
      Token::Phantom(TokenBody::Defered(Span {
         index: 0, line: 0, pos_line: 6, pos_region: 6, pos_zero: 6, length: 1
      })),
   ].to_vec();

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t, &list){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// This tests if tokenizer returns correct error Token, if there is no input and
// iterator is built and called.
// cargo test -F dbg_tokenbuf_verbose tokenizer::test_iterator::tokenizer_iterator_test_04 -- --nocapture
#[test]
fn tokenizer_iterator_test_04() {
   println!("Starging iterator test01");
   let t = Tokenizer::new();

   let mut num_token = 0;

   for token in t {
      num_token += 1;
      if let Token::Error(parse_error) = token {
         if let ParseError::NoInput = parse_error {
            // This is good.
         }
         else {
            panic!("Expected ParseError::NoInput, got: {:?}", parse_error);
         }
      }
      else {
         panic!("Expected Token::Error, got: {:?}", token);
      }
   }

   assert_eq!(num_token, 1, "We expect to have exactly one token and it should be an error.");
}


