use crate::tokenizer::Tokenizer;
use crate::span::Span;

// cargo test tokenizer::test_span::tokenizer_slice_test_01 -- --nocapture
#[test]
fn tokenizer_slice_test_01() {
   println!("Tokenizer span_slice test");
   let mut t = Tokenizer::new();

   if let Err(e) = t.src_push(None, "XXPASS0XX".into()){
      panic!("Expected Ok(None), got: Err({:?})", e);
   }

   if let Err(e) = t.src_push(None, "YYYPASS1YYY".into()){
      panic!("Expected Ok(None), got: Err({:?})", e);
   }

   let span1 = Span {
      index: 0, length: 5, pos_region: 2, pos_line: 2, pos_zero: 2, line: 0
   };

   if let Some(slice) = t.span_slice(&span1) {
      let slicestr = String::from_utf8(slice.to_vec()).expect("Invalid utf-8 string.");
      assert_eq!(slicestr, "PASS0");
   }
   else {
      panic!("Could not create slice1 from bytes.");
   }

   let span2 = Span {
      index: 1, length: 5, pos_region: 3, pos_line: 3, pos_zero: 5, line: 0
   };

   if let Some(slice) = t.span_slice(&span2) {
      let slicestr = String::from_utf8(slice.to_vec()).expect("Invalid utf-8 string.");
      assert_eq!(slicestr, "PASS1");
   }
   else {
      panic!("Could not create slice1 from bytes.");
   }
}