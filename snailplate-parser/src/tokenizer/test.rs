use super::Tokenizer;

use crate::{
   token::Token,
   tokenbody::TokenBody,
   span::Span,
};

// cargo test tokenizer::test::tokenizer_test_buf_generic -- --nocapture
// cargo test -F tokenbuf_push_guard tokenizer::test::tokenizer_test_buf_generic -- --nocapture
// This tests generic functionality for tokenbuf. Do inserts happen at all,
// can items be retrieved in expected order, aren't no extra items returned
// when they should not.
#[test]
fn tokenizer_test_buf_generic() {
   let mut t = Tokenizer::new();

   for i in 0..3 {
      let tok = Token::Real(TokenBody::Defered(Span{
         index: 0,
         pos_region: i,
         pos_line: i,
         pos_zero: i,
         line: 0,
         // In this case token length has to be 1, since for each inserted
         // token in this loop we increase position by 1.
         length: 1
      }));

      if let Err(..) = t.tokenbuf_push(tok) {
         panic!("could not push token into tokenbuf!");
      }
   }

   assert_eq!(t.num_tokens, t.tokenbuf.len(), "Bad num_tokens count!");

   for i in 0..3 {
      match t.tokenbuf_consume() {
         Ok(Some(token)) => match token {
            // receiving some token is fine. Now check if it has expected index.
            Token::Real(body) => match body {
               // Here actually it would be less code if we extracted span by
               // using token.span_clone. But let's be masochists and test for
               // unexpected cases as well.
               TokenBody::Defered(span) => {
                  assert_eq!(span.pos_region, i, "Wrong order for consumed token.");
                  assert_eq!(span.pos_line, i, "Wrong order for consumed token.");
                  assert_eq!(span.pos_zero, i, "Wrong order for consumed token.");
               }
               tokbody => {
                  panic!("Received unexpected token body: {:?}", tokbody);
               }
            }
            // This is for crazy bug. How can we receive different kind of
            // token?
            tok => {
               panic!("Received unexpected token: {:?}", tok);
            }
         }
         Ok(None) => {
            panic!("Received none, although expected token.");
         }
         Err(etok) => {
            panic!("Could not get token out: Error: {:?}", etok);
         }
      }
   }

   // All tokens should have been consumed.
   match t.tokenbuf_consume() {
      Ok(Some(tok)) => {
         panic!("Received some token when none should be available: {:?}", tok);
      }
      Ok(None) => {
         /* This is how it should be. */
      }
      Err(etok) => {
         panic!("Could not get token out: Error: {:?}", etok);
      }
   }
}



// cargo test tokenizer::test::tokenizer_test_buf_mixed_in_out -- --nocapture
// cargo test -F tokenbuf_push_guard tokenizer::test::tokenizer_test_buf_mixed_in_out -- --nocapture
// This test tests if tokenbuf allows mixed in and out operations. In general
// i don't think there will be problems other than increased memory consumption
// if tokenbuf does not work as intended in this regard, but at this point in
// time i want to be sure that in/out policy is enforced.
// This test should be run only if tokenbuf push guard is enabled. Otherwise
// there is no detection code compiled in.
#[test]
#[cfg(feature = "tokenbuf_push_guard")]
fn tokenizer_test_buf_mixed_in_out() {
   let mut t = Tokenizer::new();

   let tok = Token::Real(TokenBody::Defered(Span{
      index: 0,
      pos_region: 0,
      pos_line: 0,
      pos_zero: 0,
      line: 0,
      length: 1
   }));

   if let Err(..) = t.tokenbuf_push(tok) {
      panic!("could not push token into tokenbuf!");
   }

   let tok = Token::Real(TokenBody::Defered(Span{
      index: 0,
      pos_region: 1,
      pos_line: 1,
      pos_zero: 1,
      line: 0,
      length: 1
   }));

   if let Err(..) = t.tokenbuf_push(tok) {
      panic!("could not push second token into tokenbuf!");
   }

   match t.tokenbuf_consume() {
      Ok(Some(..)) => { /* receiving some token is fine. */ }
      Ok(None) => {
         panic!("Received none, although expected one token.");
      }
      Err(etok) => {
         panic!("Could not get token out: Error: {:?}", etok);
      }
   }

   // Now try to push in half-consumed tokenbuf. This should fail by desing.
   let tok = Token::Real(TokenBody::Defered(Span{
      index: 0,
      pos_region: 1,
      pos_line: 1,
      pos_zero: 1,
      line: 0,
      length: 1
   }));

   if let Ok(None) = t.tokenbuf_push(tok) {
      panic!("Tokenbuf allowed to do push when not being fully consumed before!");
   }

   // If error was returned before, test has passed.
}