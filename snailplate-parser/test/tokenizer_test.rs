use std::{
   collections::HashMap,
};

mod common;
use common::*;

use snailplate_parser::{
   tokenizer::{
      Tokenizer,
   },
   token::Token,
};



struct TokenizerTester {
   tokenizer: Tokenizer,
   expected: HashMap<String, Vec<Token>>,
   name_expected: Option<String>,
}



impl ExpectedHashMap for TokenizerTester {
   fn expected_mut(&mut self) -> &mut HashMap<String, Vec<Token>> {
      &mut self.expected
   }

   fn expected(&self) -> &HashMap<String, Vec<Token>> {
      &self.expected
   }

   fn name_expected(&self) -> &Option<String> {
      &self.name_expected
   }

   fn tokenstream_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = Token> + 'a> {
      Box::new(&mut self.tokenizer)
   }
}



impl TokenizerTester {
   // src - list of filenames relative to ./test/fixture/template/
   pub fn new(src: &[&str]) -> Self {
      let mut t = Tokenizer::new();

      let mut name_expected: Option<String> = None;

      for fn_src in src.iter() {
         if let Some(..) = name_expected { }
         else {
            name_expected = Some(fn_src.to_string());
         }

         let path = Self::filepath_get(fn_src);
         let contents = Self::file_read(&path);

         t.src_push(Some(&path), contents)
            .expect("Could not push contents into Tokenizer.")
         ;
      }

      Self {
         tokenizer: t,
         expected: HashMap::new(),
         name_expected: name_expected,
      }
   }
}



// cargo test -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose --test tokenizer_test tokenizer_defered_newlines_test_01 -- --nocapture
#[test]
fn tokenizer_defered_newlines_test_01() {
   let mut tt = TokenizerTester::new(&["x_newline_y"]);
   tt.token_test_run();
}



// cargo test -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose --test tokenizer_test tokenizer_instruction_include_test_101 -- --nocapture
#[test]
fn tokenizer_instruction_include_test_101() {
   let mut tt = TokenizerTester::new(&["include_complete_defered"]);
   tt.token_test_run();
}


