use crate::{
   tokenizer::Tokenizer,
   token::Token,
   tokenbody::TokenBody,
   span::Span,
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


