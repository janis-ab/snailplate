use std::{
   collections::HashMap,
};

mod common;
use common::*;

use snailplate_parser::{
   include_resolver::IncludeResolver,
   token::Token,
};



struct ResolverTester {
   resolver: IncludeResolver,
   expected: HashMap<String, Vec<Token>>,
   name_expected: Option<String>,
}



impl ExpectedHashMap for ResolverTester {
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
      Box::new(&mut self.resolver)
   }
}



impl ResolverTester {
   // src - list of filenames relative to ./test/fixture/template/
   pub fn new(src: &[&str]) -> Self {
      let mut r = IncludeResolver::new();

      let dir_template = Self::template_dir_get();
      // println!("dir_template: {}", dir_template);
      r.template_root_dir_set(&dir_template);

      let mut name_expected: Option<String> = None;

      for fn_src in src.iter() {
         if let Some(..) = name_expected { }
         else {
            name_expected = Some(fn_src.to_string());
         }

         let mut filename = fn_src.to_string();
         filename.push_str(".html");

         if let Err(token) = r.file_read(&filename) {
            panic!("Resolver failed with file reading. Return token: {:?}", token);
         }
      }

      Self {
         resolver: r,
         expected: HashMap::new(),
         name_expected: name_expected,
      }
   }
}



// cargo test -F dbg_include_resolver_verbose -F dbg_tokenbuf_verbose -F dbg_tokenizer_verbose --test include_resolver_test resolver_passthrough_test_01 -- --nocapture
#[test]
fn resolver_passthrough_test_01() {
   let mut tt = ResolverTester::new(&["x_newline_y"]);
   tt.token_test_run();
}


