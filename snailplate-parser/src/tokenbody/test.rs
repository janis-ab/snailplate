use std::fmt;
use crate::{
   tokenbody::TokenBody,
   span::{Span, SpanFormatter}
};



pub(crate) struct FormatTester {
   data: Vec<u8>,   
}



impl FormatTester {
   pub(crate) fn build(str: &str) -> Self {
      Self {
         data: str.into()
      }
   }
}



impl SpanFormatter for FormatTester {
   fn fmt_into(&self, 
      fmt: &mut std::fmt::Formatter, span: &Span
   ) -> fmt::Result {
      let slice = &self.data[span.pos_region..span.pos_region + span.length];

      let text = if let Ok(s) = std::str::from_utf8(slice) {
         Some(s.to_owned())
      }
      else {
         None
      };

      let mut r = fmt.debug_struct("Span");
      r.field("index", &span.index);
      r.field("length", &span.length);
      r.field("pos_line", &span.pos_line);
      r.field("pos_region", &span.pos_region);
      r.field("pos_zero", &span.pos_zero);
      r.field("line", &span.line);

      if let Some(text) = text {
         r.field("text", &text);
      }

      r.finish()
   }
}



// This tests if TokenBodyFormatWrapper works as expected. When TokenBody.fmt()
// is called it creates TokenBodyFormatWrapper which implements fmt::Debug
// trait.
//
// To run this test:
// cargo test tokenbody::test::test_formatter
// cargo test tokenbody::test::test_formatter -- --nocapture
#[test]
fn test_formatter(){
   let t = FormatTester::build("XXPASSZZ");

   let tb = TokenBody::Defered(Span {
      index: 0,
      length: 4,
      pos_line: 2,
      pos_region: 2,
      pos_zero: 2,
      line: 0,
   });

   let out = format!("{:?}", tb.fmt(&t));
   let pass = "Defered(Span { index: 0, length: 4, pos_line: 2, pos_region: 2, pos_zero: 2, line: 0, text: \"PASS\" })";

   assert_eq!(out.as_str(), pass);
}


