use crate::{
   token::Token,
   tokenbody::TokenBody,
   span::Span,
   tokenbody::test::FormatTester
};


// To run this test:
// cargo test token::test::test_formatter
// cargo test token::test::test_formatter -- --nocapture
#[test]
fn test_formatter(){
   let t = FormatTester::build("XXPASSZZ");

   let tok = Token::Real(TokenBody::Defered(Span {
      index: 0,
      length: 4,
      pos_line: 2,
      pos_region: 2,
      pos_zero: 2,
   }));

   let out = format!("{:?}", tok.fmt(&t));
   let pass = "Real(Defered(Span { index: 0, length: 4, pos_line: 2, pos_region: 2, pos_zero: 2, text: \"PASS\" }))";

   assert_eq!(out.as_str(), pass);
}