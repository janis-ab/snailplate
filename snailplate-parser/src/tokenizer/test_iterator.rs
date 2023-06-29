use crate::{
   tokenizer::Tokenizer,
   token::Token,
   tokenbody::TokenBody,
   span::Span,
};

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