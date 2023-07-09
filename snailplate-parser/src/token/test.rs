use crate::{
   token::Token,
   tokenbody::TokenBody,
   span::Span,
   tokenbody::test::FormatTester,
   parse_error::{
      ParseError,
      InternalError,
      Component,
   },   
};


// To run this test:
// cargo test token::test::test_formatter_01
// cargo test token::test::test_formatter_01 -- --nocapture
#[test]
fn test_formatter_01(){
   let t = FormatTester::build("XXPASSZZ");

   let tok = Token::Real(TokenBody::Defered(Span {
      index: 0,
      length: 4,
      pos_line: 2,
      pos_region: 2,
      pos_zero: 2,
      line: 0,
   }));

   let out = format!("{:?}", tok.fmt(&t));
   let pass = "Real(Defered(Span { index: 0, length: 4, pos_line: 2, pos_region: 2, pos_zero: 2, line: 0, text: \"PASS\" }))";

   assert_eq!(out.as_str(), pass);
}



// This test is not so much a test when testing, but when developing, to see
// if formatter works as expected.
//
// cargo test token::test::test_formatter_internal_error -- --nocapture
#[test]
fn test_formatter_internal_error() {
   let t = FormatTester::build("XXPASSZZ");

   let tok = Token::Fatal(ParseError::InternalError(InternalError {
      component: Component::Tokenizer,
      line: 0,
   }));

   let out = format!("{:?}", tok.fmt(&t));
   println!("InternalError as text: '{}'", out);

   let pass = "Fatal(InternalError(InternalError { component: Tokenizer, line: 0 }))";

   assert_eq!(out.as_str(), pass);
}