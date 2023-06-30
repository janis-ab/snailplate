use super::ident::*;



// cargo test -F dbg_tokenizer_verbose tokenizer::test_ident::tokenizer_ident_test_01 -- --nocapture
#[test]
fn tokenizer_ident_test_01() {
   println!("Tokenizer ident test");

   let buf = "include(filename)".as_bytes();
   let ident = ident_match(&buf, 0, 6);

   if let Ident::Include(start, end) = ident {
      assert_eq!(start, 0);
      assert_eq!(end, 6);
   }
   else {
      panic!("Bad ident returned. Expected Ident::Include, got: {:?}", ident);
   }
}


