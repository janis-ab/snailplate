use super::Tokenizer;

use crate::{
   token::Token,
   tokenbody::TokenBody,
   span::Span,
   parse_error::{
      ParseError,
      InternalError,
      Component,
   },
};

use super::tokenlist_match_or_fail;

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

   assert_eq!(t.tokenbuf.num_tokens, t.tokenbuf.buf.len(), "Bad num_tokens count!");

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



// cargo test -F tokenbuf_push_guard tokenizer::test::tokenizer_src_push -- --nocapture
#[test]
fn tokenizer_src_push() {
   println!("Tokenizer src_push test");
   let mut t = Tokenizer::new();

   if let Err(e) = t.src_push(None, "test".into()){
      panic!("Expected Ok(None), got: Err({:?})", e);
   }

   if let Err(e) = t.src_push(None, "test".into()){
      panic!("Expected Ok(None), got: Err({:?})", e);
   }

   // TODO: it would be nice to implement some tests that exhaust tokenbuf region
   // Vec memory, so that we can test if correct error return code is returned.
}



// cargo test tokenizer::test::test_tokenizer_return_tokenized_01 -- --nocapture
#[test]
fn test_tokenizer_return_tokenized_01(){
   println!("Tokenizer src_push test");
   let mut t = Tokenizer::new();

   if let Err(e) = t.src_push(None, "AAABBB".into()){
      panic!("Expected Ok(None), got: Err({:?})", e);
   }

   //
   // Test if this token will update Tokenizers positions.
   //
   if let Some(token) = t.return_tokenized(Token::Real(TokenBody::Defered(Span {
      index: 0, line: 0, pos_region: 0, pos_zero: 0, pos_line: 0, length: 3
   }))) {
      if let Token::Real(..) = token {
         println!("token: {:?}", token);
         println!("tokenizer: {:?}", t);
      }
      else {
         panic!("Bad token returned from Tokenizer! Expected Token::Real, got {:?}", token);
      }
   }
   else {
      panic!("Token-1 was not returned!");
   }

   assert_eq!(t.index, 0);
   assert_eq!(t.pos_region, 3);
   assert_eq!(t.pos_line, 3);
   assert_eq!(t.pos_zero, 3);

   //
   // Test if Token without span, leaves Tokenizer positions as is.
   //
   if let Some(token) = t.return_tokenized(Token::StateChange) {
      if let Token::StateChange = token {
         println!("token: {:?}", token);
         println!("tokenizer: {:?}", t);
      }
      else {
         panic!("Bad token returned from Tokenizer! Expected Token::StateChange, got {:?}", token);
      }
   }
   else {
      panic!("Token-2 was not returned!");
   }
   
   assert_eq!(t.index, 0);
   assert_eq!(t.pos_region, 3);
   assert_eq!(t.pos_line, 3);
   assert_eq!(t.pos_zero, 3);

   //
   // Test if second token will update Tokenizers positions.
   //
   if let Some(token) = t.return_tokenized(Token::Real(TokenBody::Defered(Span {
      index: 0, line: 0, pos_region: 3, pos_zero: 3, pos_line: 3, length: 3
   }))) {
      if let Token::Real(..) = token {
         println!("token: {:?}", token);
         println!("tokenizer: {:?}", t);
      }
      else {
         panic!("Bad token returned from Tokenizer! Expected Token::Real, got {:?}", token);
      }
   }
   else {
      panic!("Token-1 was not returned!");
   }

   assert_eq!(t.index, 0);
   assert_eq!(t.pos_region, 6);
   assert_eq!(t.pos_line, 6);
   assert_eq!(t.pos_zero, 6);
}



// cargo test tokenizer::test::test_tokenizer_defered_tokenize_01 -- --nocapture
#[test]
fn test_tokenizer_defered_tokenize_01(){
   println!("Tokenizer src_push test");
   let mut t = Tokenizer::new();

   if let Err(e) = t.src_push(None, "CCC".into()){
      panic!("Expected Ok(None), got: Err({:?})", e);
   }

   if let Err(e) = t.src_push(None, "BBB".into()){
      panic!("Expected Ok(None), got: Err({:?})", e);
   }

   if let Err(e) = t.src_push(None, "AAA".into()){
      panic!("Expected Ok(None), got: Err({:?})", e);
   }

   // TODO: For now this is a rough test, but we should implement some easier
   // testing API, so that we provide list with exact tokens we want to receive
   // and compare them in single call.

   if let Some(token) = t.defered_tokenize() {
      let span = token.span_clone();
      if let Some(span) = span {
         assert_eq!(span.index, 2, 
            "Span-1 does not contain values as expected. Span: {:?}", span
         );
      }
      else {
         panic!("Token does not have span. Token: {:?}", token);
      }
   }
   else {
      panic!("Token-1 is None!");
   }


   if let Some(token) = t.defered_tokenize() {
      let span = token.span_clone();
      if let Some(span) = span {
         assert_eq!(span.index, 1, 
            "Span-1 does not contain values as expected. Span: {:?}", span
         );
      }
      else {
         panic!("Token does not have span. Token: {:?}", token);
      }
   }
   else {
      panic!("Token-1 is None!");
   }


   if let Some(token) = t.defered_tokenize() {
      let span = token.span_clone();
      if let Some(span) = span {
         assert_eq!(span.index, 0, 
            "Span-1 does not contain values as expected. Span: {:?}", span
         );
      }
      else {
         panic!("Token does not have span. Token: {:?}", token);
      }
   }
   else {
      panic!("Token-1 is None!");
   }
}



// Function that tests if whitespace_into_tokenbuf call behaves as expected.
//
// If you want this funciton to just print returned tokens, set feature flag
// tokenlist_match_or_fail_print_only when running test.
//
// This function implements the heavy lifting for testing whitespace
// tokenization when using function whitespace_into_tokenbuf only. It is
// intended for code reuse with different input scenarios.
//
// The function has artificial @include type behavior regarding source stack.
//
// This is designed for whitespace testing in such a manner, that if src is
// vec that contains multiple elements, each next element is pushed onto
// Tokenbuf stack after whitespace is parsed. This is like fake @include, to
// test if whitespacing works in stack as expected.
//
// Source string can consist of three parts:
//    - starting defered token (optional); for testing we usually use S(start)
//      and then add number, 1, 2, 3 for level. This gives easier understanding
//      about which token should be returned when;
//    - whitespace region, that should be " \t\n", this is mandatory, otherwise
//      that test is meaningless and tests this funciton instead;
//    - ending defered token (optional); this we use E(end) and then add number
//      with same as was used for starting.
//
// About expected tokenization order. To strings "S1 E1" and "S2 E2" should be
// tokenized like this: ["S1", " ", "S2", " ", "E2", "E1"]. After each space
// next string is pushed onto Tokenizer's stack, in a similar fashion @include
// instruction will do.
//
// # Arguments
//
// * `src` - Vec that holds tuples of test related data. Each tuple contains
//       four elements:
//          - source string,
//          - position for whitespace,
//          - line for first char,
//          - line after last char.
// * `expect` - Vector containing expected result tokens.
//
// # Notes
//
// * `2023-07-05`:
//    There are times when this test function fails with None expected but
//    Tokenizer returned some Token. At those times, the fault can be due to
//    this function, because it is tricky to implement and when bad parameters
//    are passed they mess up this functions logic, since it does not do
//    extensive checking on input parameters (we trust that test writer knows
//    what he is doing).
//
fn test_whitespace_into_tokenbuf(
   src: Vec<(&str, usize, usize, usize)>,
   expect: Vec<Token>
) {
   let mut t = Tokenizer::new();
   let mut reverse = Vec::new();

   let mut slice_base = 0;

   let len = src.len();
   let mut index = 0;
   while index < len {
      if let Some(item) = src.get(index) {
         if let Err(e) = t.src_push(None, item.0.into()){
            panic!("Expected Ok(None), got: Err({:?})", e);
         }
      }
      else {
         panic!("Could not get test input item at index: {}", index);
      }

      if let Some(item) = src.get(index) {
         let len_region = item.0.len();
         let pos_region = item.1;
         let line_start = item.2;
         let line_end = item.3;

         if pos_region > len_region {
            println!("item: {:?}", item);
            panic!("Incorrect test data, pos_region > than source size!");
         }

         let mut pos_defer = len_region - 1;
         let bytes = item.0.as_bytes();
         while pos_defer > 0 {
            match bytes[pos_defer] {
               0x0A | 0x20 | 0x09 => {
                  pos_defer += 1;
                  break;
               }
               _ch => {
                  // println!("ch: 0x{:02x}", _ch);
               }
            }
            pos_defer -= 1;
         }

         let len_whitespace = if pos_region > 0 {
            pos_defer - pos_region
         }
         else {
            pos_defer = len_region;
            len_region
         };
         let line = t.line;

         // There is a Defered token available before whitespace (at the start
         // of the string).
         if pos_region > 0 {
            let len_defered = pos_region;
            #[allow(unused_must_use)] {
               t.tokenbuf_push(Token::Real(TokenBody::Defered(Span {
                  index: index, pos_region: 0, pos_line: 0, length: len_defered,
                  line: line, pos_zero: t.pos_zero
               })));
            }
         }

         if let Some(error_token) = t.whitespace_into_tokenbuf(index,
            pos_region, len_whitespace, line_start, line_end,
         ) {
            panic!("Could not convert whitespace region into Tokens in tokenbuf. Error: {:?}", error_token);
         }

         // Since "whitespace" in this test works as @include instruction,
         // we must test already inserted tokens, otherwise internal state
         // will not be as expected for upcoming testing items/tuples.

         let num_tokens = t.tokenbuf.buf.len();
         if let Err((expect, got)) = tokenlist_match_or_fail(&mut t,
            &expect[slice_base..slice_base + num_tokens], false
         ){
            panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
         }

         // Here we can use pos_line form tokenizer, it should be a correct
         // value.
         let pos_line = t.pos_line;
         slice_base += num_tokens;

         if len_region > pos_defer {
            let len_post_defer = len_region - pos_defer;
            #[allow(unused_must_use)] {
               reverse.push(Token::Real(TokenBody::Defered(Span {
                  index: index, pos_region: pos_defer,
                  pos_line: pos_line,
                  length: len_post_defer,
                  line: t.line,
                  // NOTICE: pos_zero will be wrong value for all Defered tokens
                  // except the last one, but we do not know if this is the last
                  // one. The last one will be used to recalculate pos_zero for
                  // all otherlast tokens.
                  pos_zero: t.pos_zero
               })));

            }
         }
      }
      else {
         panic!("Could not get test input item for tokenization at index: {}.", 
            index
         );
      }

      index += 1;
   }

   let num_tokens = reverse.len();
   let slice_end = slice_base + num_tokens;

   if slice_end > expect.len() {
      // TODO: maybe we can print out unwanted items, expected None, returned: 
      // X style.

      panic!("Expected less items than there will be available! Did you provide\
       enough testing data!"
      );
   }

   // Defered ending tokens must be handled in reverse order, because this is
   // the behavior expected. And since pos_zero is correct only for the last
   // Token, we must adjust pos_zero for all other tokens.
   if num_tokens > 0 {
      let mut pos_zero = 0;
      for token in reverse.into_iter().rev() {
         let mut span_2 = if let Some(span) = token.span_clone() {
            span
         }
         else {
            panic!("Could not clone span!");
         };

         if pos_zero < span_2.pos_zero {
            pos_zero = span_2.pos_zero;
         }

         span_2.pos_zero = pos_zero;
         pos_zero += span_2.length;

         #[allow(unused_must_use)] {
            t.tokenbuf_push(Token::Real(TokenBody::Defered(span_2)));
         }
      }
   }

   if let Err((expect, got)) = tokenlist_match_or_fail(&mut t,
      &expect[slice_base..slice_end], true
   ){
      panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
   }
}



// This tests whitespace_into_tokenbuf ending with "\n" (without trailing
// whitespace).
// cargo test -F dbg_tokenizer_verbose -F dbg_tokenbuf_verbose tokenizer::test::test_whitespace_into_tokenbuf_01 -- --nocapture
#[test]
fn test_whitespace_into_tokenbuf_01(){
   println!("Tokenizer whitespace_into_tokenbuf test");

   test_whitespace_into_tokenbuf(
      [
         (" \t \n\n  \n", 0, 0, 3,)
      ].to_vec(),
      [
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 0, length: 3, pos_line: 0, pos_region: 0, pos_zero: 0
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 0, length: 1, pos_line: 3, pos_region: 3, pos_zero: 3
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 1, length: 1, pos_line: 0, pos_region: 4, pos_zero: 4
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 2, length: 2, pos_line: 0, pos_region: 5, pos_zero: 5
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 2, length: 1, pos_line: 2, pos_region: 7, pos_zero: 7
         })),
      ].to_vec()
   );
}



// This tests whitespace_into_tokenbuf with trailing whitespace
// cargo test -F dbg_tokenizer_verbose -F dbg_tokenbuf_verbose tokenizer::test::test_whitespace_into_tokenbuf_02 -- --nocapture
#[test]
fn test_whitespace_into_tokenbuf_02(){
   println!("Tokenizer test_whitespace_into_tokenbuf_02 test");

   test_whitespace_into_tokenbuf(
      [
         ("   \n\n  \n  \t  ", 0, 0, 3)
      ].to_vec(),
      [
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 0, length: 3, pos_line: 0, pos_region: 0, pos_zero: 0
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 0, length: 1, pos_line: 3, pos_region: 3, pos_zero: 3
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 1, length: 1, pos_line: 0, pos_region: 4, pos_zero: 4
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 2, length: 2, pos_line: 0, pos_region: 5, pos_zero: 5
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 2, length: 1, pos_line: 2, pos_region: 7, pos_zero: 7
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 3, length: 5, pos_line: 0, pos_region: 8, pos_zero: 8
         })),
      ].to_vec()
   );
}



// This tests whitespace_into_tokenbuf ending with "\n" (without trailing
// whitespace).
// cargo test -F dbg_tokenbuf_verbose tokenizer::test::test_whitespace_into_tokenbuf_03 -- --nocapture
#[test]
fn test_whitespace_into_tokenbuf_03(){
   println!("Tokenizer whitespace_into_tokenbuf test");

   test_whitespace_into_tokenbuf(
      [
         (" \t ", 0, 0, 0,)
      ].to_vec(),
      [
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 0, length: 3, pos_line: 0, pos_region: 0, pos_zero: 0
         })),
      ].to_vec()
   );
}



// cargo test -F dbg_tokenizer_verbose -F dbg_tokenbuf_verbose tokenizer::test::test_whitespace_into_tokenbuf_04 -- --nocapture
#[test]
fn test_whitespace_into_tokenbuf_04(){
   println!("Tokenizer test_whitespace_into_tokenbuf_04 test");

   test_whitespace_into_tokenbuf(
      [
         /* (&src, pos_whitespace, line_start, line_end) */
         ("S1 E1", 2, 0, 0),
         ("S2 E2", 2, 0, 0),
         ("S3 E3", 2, 0, 0),
      ].to_vec(),
      [
         // These are first tuple start tokens.
         Token::Real(TokenBody::Defered(Span {
            index: 0, line: 0, length: 2, pos_line: 0, pos_region: 0, pos_zero: 0
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 0, length: 1, pos_line: 2, pos_region: 2, pos_zero: 2
         })),

         // These are second tuple start tokens.
         Token::Real(TokenBody::Defered(Span {
            index: 1, line: 0, length: 2, pos_line: 0, pos_region: 0, pos_zero: 3
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 1, line: 0, length: 1, pos_line: 2, pos_region: 2, pos_zero: 5
         })),

         // These are third tuple start tokens.
         Token::Real(TokenBody::Defered(Span {
            index: 2, line: 0, length: 2, pos_line: 0, pos_region: 0, pos_zero: 6
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 2, line: 0, length: 1, pos_line: 2, pos_region: 2, pos_zero: 8
         })),

         // These are remaining Defered tokens, going backwards - third tuple,
         // second tuple, first tuple.
         Token::Real(TokenBody::Defered(Span {
            index: 2, line: 0, length: 2, pos_line: 3, pos_region: 3, pos_zero: 9
         })),
         Token::Real(TokenBody::Defered(Span {
            index: 1, line: 0, length: 2, pos_line: 3, pos_region: 3, pos_zero: 11
         })),
         Token::Real(TokenBody::Defered(Span {
            index: 0, line: 0, length: 2, pos_line: 3, pos_region: 3, pos_zero: 13
         })),
      ].to_vec()
   );
}



// This tests whitespace_into_tokenbuf with trailing whitespace
// cargo test -F dbg_tokenizer_verbose -F dbg_tokenbuf_verbose tokenizer::test::test_whitespace_into_tokenbuf_05 -- --nocapture
#[test]
fn test_whitespace_into_tokenbuf_05(){
   println!("Tokenizer test_whitespace_into_tokenbuf_05 test");

   test_whitespace_into_tokenbuf(
      [
         /* (&src, pos_whitespace, line_start, line_end) */
         ("S1   \n \n\n     E1", 2, 0, 3),
      ].to_vec(),
      [
         // These are first tuple start tokens.
         Token::Real(TokenBody::Defered(Span {
            index: 0, line: 0, length: 2, pos_line: 0, pos_region: 0, pos_zero: 0
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 0, length: 3, pos_line: 2, pos_region: 2, pos_zero: 2
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 0, length: 1, pos_line: 5, pos_region: 5, pos_zero: 5
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 1, length: 1, pos_line: 0, pos_region: 6, pos_zero: 6
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 1, length: 1, pos_line: 1, pos_region: 7, pos_zero: 7
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 2, length: 1, pos_line: 0, pos_region: 8, pos_zero: 8
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 3, length: 5, pos_line: 0, pos_region: 9, pos_zero: 9
         })),

         // These are remaining Defered tokens, going backwards - third tuple,
         // second tuple, first tuple.
         Token::Real(TokenBody::Defered(Span {
            index: 0, line: 3, length: 2, pos_line: 5, pos_region: 14, pos_zero: 14
         })),
      ].to_vec()
   );
}



// This tests whitespace_into_tokenbuf with trailing whitespace
// cargo test -F dbg_tokenizer_verbose -F dbg_tokenbuf_verbose tokenizer::test::test_whitespace_into_tokenbuf_06 -- --nocapture
#[test]
fn test_whitespace_into_tokenbuf_06(){
   println!("Tokenizer test_whitespace_into_tokenbuf_06 test");

   test_whitespace_into_tokenbuf(
      [
         /* (&src, pos_whitespace, line_start, line_end) */
         ("S1   \n \n\n     E1", 2, 0, 3),
         ("S2  E2", 2, 0, 0),
      ].to_vec(),
      [
         // These are the start tokens for first tuple.
         Token::Real(TokenBody::Defered(Span {
            index: 0, line: 0, length: 2, pos_line: 0, pos_region: 0, pos_zero: 0
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 0, length: 3, pos_line: 2, pos_region: 2, pos_zero: 2
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 0, length: 1, pos_line: 5, pos_region: 5, pos_zero: 5
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 1, length: 1, pos_line: 0, pos_region: 6, pos_zero: 6
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 1, length: 1, pos_line: 1, pos_region: 7, pos_zero: 7
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 2, length: 1, pos_line: 0, pos_region: 8, pos_zero: 8
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 3, length: 5, pos_line: 0, pos_region: 9, pos_zero: 9
         })),

         // These are the start tokens for second tuple.
         Token::Real(TokenBody::Defered(Span {
            index: 1, line: 0, length: 2, pos_line: 0, pos_region: 0, pos_zero: 14
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 1, line: 0, length: 2, pos_line: 2, pos_region: 2, pos_zero: 16
         })),

         // These are remaining Defered tokens, going backwards - third tuple,
         // second tuple, first tuple.
         Token::Real(TokenBody::Defered(Span {
            index: 1, line: 0, length: 2, pos_line: 4, pos_region: 4, pos_zero: 18
         })),
         Token::Real(TokenBody::Defered(Span {
            index: 0, line: 3, length: 2, pos_line: 5, pos_region: 14, pos_zero: 20
         })),
      ].to_vec()
   );
}



// This tests whitespace_into_tokenbuf with trailing whitespace
// cargo test -F dbg_tokenizer_verbose -F dbg_tokenbuf_verbose tokenizer::test::test_whitespace_into_tokenbuf_07 -- --nocapture
#[test]
fn test_whitespace_into_tokenbuf_07(){
   println!("Tokenizer test_whitespace_into_tokenbuf_07 test");

   test_whitespace_into_tokenbuf(
      [
         /* (&src, pos_whitespace, line_start, line_end) */
         ("S1   \n \n\n     E1", 2, 0, 3),
         ("S2  E2", 2, 0, 0),
         ("S3 \t\n \t \n \nE3", 2, 0, 3),
      ].to_vec(),
      [
         // These are the start tokens for first tuple.
         Token::Real(TokenBody::Defered(Span {
            index: 0, line: 0, length: 2, pos_line: 0, pos_region: 0, pos_zero: 0
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 0, length: 3, pos_line: 2, pos_region: 2, pos_zero: 2
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 0, length: 1, pos_line: 5, pos_region: 5, pos_zero: 5
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 1, length: 1, pos_line: 0, pos_region: 6, pos_zero: 6
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 1, length: 1, pos_line: 1, pos_region: 7, pos_zero: 7
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 2, length: 1, pos_line: 0, pos_region: 8, pos_zero: 8
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 3, length: 5, pos_line: 0, pos_region: 9, pos_zero: 9
         })),

         // These are the start tokens for second tuple.
         Token::Real(TokenBody::Defered(Span {
            index: 1, line: 0, length: 2, pos_line: 0, pos_region: 0, pos_zero: 14
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 1, line: 0, length: 2, pos_line: 2, pos_region: 2, pos_zero: 16
         })),

         // These are the start tokens for third tuple.
         Token::Real(TokenBody::Defered(Span {
            index: 2, line: 0, length: 2, pos_line: 0, pos_region: 0, pos_zero: 18
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 2, line: 0, length: 2, pos_line: 2, pos_region: 2, pos_zero: 20
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 2, line: 0, length: 1, pos_line: 4, pos_region: 4, pos_zero: 22
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 2, line: 1, length: 3, pos_line: 0, pos_region: 5, pos_zero: 23
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 2, line: 1, length: 1, pos_line: 3, pos_region: 8, pos_zero: 26
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 2, line: 2, length: 1, pos_line: 0, pos_region: 9, pos_zero: 27
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 2, line: 2, length: 1, pos_line: 1, pos_region: 10, pos_zero: 28
         })),

         // These are remaining Defered tokens, going backwards - third tuple,
         // second tuple, first tuple.
         Token::Real(TokenBody::Defered(Span {
            index: 2, line: 3, length: 2, pos_line: 0, pos_region: 11, pos_zero: 29
         })),
         Token::Real(TokenBody::Defered(Span {
            index: 1, line: 0, length: 2, pos_line: 4, pos_region: 4, pos_zero: 31
         })),
         Token::Real(TokenBody::Defered(Span {
            index: 0, line: 3, length: 2, pos_line: 5, pos_region: 14, pos_zero: 33
         })),
      ].to_vec()
   );
}



// This tests whitespace_into_tokenbuf with trailing whitespace
// cargo test -F dbg_tokenizer_verbose -F dbg_tokenbuf_verbose tokenizer::test::test_whitespace_into_tokenbuf_08 -- --nocapture
#[test]
fn test_whitespace_into_tokenbuf_08(){
   println!("Tokenizer test_whitespace_into_tokenbuf_08 test");

   test_whitespace_into_tokenbuf(
      [
         /* (&src, pos_whitespace, line_start, line_end) */
         ("S1   \n \n\n     ", 2, 0, 3),
      ].to_vec(),
      [
         // These are first tuple start tokens.
         Token::Real(TokenBody::Defered(Span {
            index: 0, line: 0, length: 2, pos_line: 0, pos_region: 0, pos_zero: 0
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 0, length: 3, pos_line: 2, pos_region: 2, pos_zero: 2
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 0, length: 1, pos_line: 5, pos_region: 5, pos_zero: 5
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 1, length: 1, pos_line: 0, pos_region: 6, pos_zero: 6
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 1, length: 1, pos_line: 1, pos_region: 7, pos_zero: 7
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 2, length: 1, pos_line: 0, pos_region: 8, pos_zero: 8
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 3, length: 5, pos_line: 0, pos_region: 9, pos_zero: 9
         })),
      ].to_vec()
   );
}



// This tests whitespace_into_tokenbuf with trailing whitespace
// cargo test -F dbg_tokenizer_verbose -F dbg_tokenbuf_verbose tokenizer::test::test_whitespace_into_tokenbuf_09 -- --nocapture
#[test]
fn test_whitespace_into_tokenbuf_09(){
   println!("Tokenizer test_whitespace_into_tokenbuf_09 test");

   test_whitespace_into_tokenbuf(
      [
         /* (&src, pos_whitespace, line_start, line_end) */
         ("S1   \n \n\n", 2, 0, 3),
      ].to_vec(),
      [
         // These are first tuple start tokens.
         Token::Real(TokenBody::Defered(Span {
            index: 0, line: 0, length: 2, pos_line: 0, pos_region: 0, pos_zero: 0
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 0, length: 3, pos_line: 2, pos_region: 2, pos_zero: 2
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 0, length: 1, pos_line: 5, pos_region: 5, pos_zero: 5
         })),
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 1, length: 1, pos_line: 0, pos_region: 6, pos_zero: 6
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 1, length: 1, pos_line: 1, pos_region: 7, pos_zero: 7
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 2, length: 1, pos_line: 0, pos_region: 8, pos_zero: 8
         })),
      ].to_vec()
   );
}



// cargo test -F dbg_tokenizer_verbose tokenizer::test::test_whitespace_into_tokenbuf_10 -- --nocapture
#[test]
fn test_whitespace_into_tokenbuf_10(){
   println!("Tokenizer test_whitespace_into_tokenbuf_10 test");

   test_whitespace_into_tokenbuf(
      [
         /* (&src, pos_whitespace, line_start, line_end) */
         ("\n", 0, 0, 1),
         ("\n", 0, 0, 1),
      ].to_vec(),
      [
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 0, length: 1, pos_line: 0, pos_region: 0, pos_zero: 0
         })),
         Token::Real(TokenBody::Newline(Span {
            index: 1, line: 0, length: 1, pos_line: 0, pos_region: 0, pos_zero: 1
         })),
      ].to_vec()
   );
}



// cargo test -F dbg_tokenizer_verbose tokenizer::test::test_whitespace_into_tokenbuf_11 -- --nocapture
#[test]
fn test_whitespace_into_tokenbuf_11(){
   println!("Tokenizer test_whitespace_into_tokenbuf_11 test");

   test_whitespace_into_tokenbuf(
      [
         /* (&src, pos_whitespace, line_start, line_end) */
         ("\n", 0, 0, 1),
      ].to_vec(),
      [
         Token::Real(TokenBody::Newline(Span {
            index: 0, line: 0, length: 1, pos_line: 0, pos_region: 0, pos_zero: 0
         })),
      ].to_vec()
   );
}



// cargo test -F dbg_tokenizer_verbose tokenizer::test::test_whitespace_into_tokenbuf_12 -- --nocapture
#[test]
#[should_panic]
fn test_whitespace_into_tokenbuf_12(){
   println!("Tokenizer test_whitespace_into_tokenbuf_12 test");

   // Actually panicking is not due to Tokenizer's function but due to
   // test_whitespace_into_tokenbuf, but i do not want to modify it, since in
   // other cases it has to panic. For this case  when Tokenizer returns 
   // Token::Fatal, we assume it is correct.

   test_whitespace_into_tokenbuf(
      [
         // Should panic because we provided bad testing info.
         (" ", 0, 0, 3),
      ].to_vec(),
      [
         Token::Fatal(ParseError::InternalError(InternalError {
            component: Component::Tokenizer,
            line: 0,
         })),
      ].to_vec()
   );
}



// cargo test -F dbg_tokenizer_verbose tokenizer::test::test_whitespace_into_tokenbuf_13 -- --nocapture
#[test]
fn test_whitespace_into_tokenbuf_13(){
   println!("Tokenizer test_whitespace_into_tokenbuf_13 test");

   test_whitespace_into_tokenbuf(
      [
         /* (&src, pos_whitespace, line_start, line_end) */
         (" ", 0, 0, 0),
      ].to_vec(),
      [
         Token::Real(TokenBody::WhiteSpace(Span {
            index: 0, line: 0, length: 1, pos_line: 0, pos_region: 0, pos_zero: 0
         })),
      ].to_vec()
   );
}



// ================== EOF: do not write below this ============================
