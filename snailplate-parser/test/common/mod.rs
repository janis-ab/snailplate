use std::{
   collections::HashMap,
   fs::File,
   io::Read,
};

use snailplate_parser::{
   token::Token,
   tokenbody::TokenBody,
   span::Span,
};



// This trait must be implemented by all objects that want to act as token
// testers.
//
// When this trait is implemented, TokenIntegrationTester trait will be
// implemented automatically within this module.
pub trait ExpectedHashMap {
   // Function that returns HashMap that stores expected result tokenlists.
   //
   // Since traits do not allow to access properties of struct directly, we use
   // a wrapper functions that return expected property.
   fn expected(&self) -> &HashMap<String, Vec<Token>>;

   // Function that returns mutable HashMap where expected token-lists are
   // stored.
   fn expected_mut(&mut self) -> &mut HashMap<String, Vec<Token>>;

   // Function that returns name expected (this is filename for correct answer,
   // key within expected HashMap).
   fn name_expected(&self) -> &Option<String>;

   // Function that has to return iterable object, that returns Tokens. At the
   // moment there is only an implementation for Tokenizer, but the interface
   // is extensible/compatible for future components like Resolver, Parser, etc.
   fn tokenstream_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = Token> + 'a>;
}



// This is like a transparent trait, that ensures, that defined functions are
// implemented.
pub trait TokenIntegrationTester {
   fn expected_load(&mut self);
   fn token_test_run(&mut self);

   fn tokenlist_match_or_fail(&mut self, list: &[Token])
      -> Result<(), (Option<Token>, Option<Token>)>
   ;



   // Function that reads file as u8 buffer or panics if impossible.
   fn file_read(path: &str) -> Vec<u8> {
      // println!("file_read: {}", path);

      let error_msg = format!("Unable to open template file '{}'", path);
      let mut file = File::open(path).expect(&error_msg);

      let mut buf = Vec::new();
      let error_msg = format!("Unable to read file '{}'", path);
      file.read_to_end(&mut buf).expect(&error_msg);

      buf
   }



   fn template_dir_get() -> String {
      let dir_root = if let Some(dir) = option_env!("CARGO_MANIFEST_DIR") {
         dir
      }
      else {
         panic!("Build environment does not have defined CARGO_MANIFEST_DIR. Something is very wrong.");
      };

      let mut path = dir_root.to_owned();
      path.push_str("/test/fixture/template");

      path
   }



   // Function that returns path to test template source.
   //
   // It panics on error. This behavior is such, since it always should succeed
   // and this style allows us to write less code.
   fn filepath_get(filename: &str) -> String {
      let mut path = Self::template_dir_get();
      path.push_str("/");
      path.push_str(filename);
      path.push_str(".html");

      path
   }
}



// Auto-implement trait functions if struct has necessary methods available.
impl<T: ExpectedHashMap> TokenIntegrationTester for T {
   // Function that loads all expected answers.
   //
   // At the moment i think this is the easiest way how to achieve what we want.
   // Technically i could use some sort of #[derive] macro tricks, but they
   // require to create separate crate, since we would have to build source to
   // be compiled manually. At the moment test knows file name only at runtime,
   // but we want to reuse Rust code parsing ability.
   //
   // In future there are two ways how this could be improved:
   // 1) Change the expected output file syntax to something that can be easily
   //    parsed and expected structure is built at runtime. And compare to that.
   // 2) Create some sort of macro, that builds test source and expected list
   //    at compile time.
   //
   // For now we just load all allowed cases in HashMap which is inefficient,
   // i know. But i do not have enough time for this at the moment.
   fn expected_load(&mut self) {
      // To minimize repetetive code, we define a macro, that inserts test
      // sources and expected results into HasMap.
      macro_rules! register {
         ($key:expr) => {
            self.expected_mut().insert($key.into(),
               (include!(concat!("../fixture/expected/", $key, ".rs"))).to_vec()
            );
         }
      }

      register!("x_newline_y");
      register!("include_complete_defered");
   }



   // This is copy/paste of tokenizer::tokenlist_match_or_fail with minor
   // modifications. I could not find a way of how to reuse the same function
   // without violating Rust borrow checker rules.
   // One problem was with tokenbuf length, it was not possible to get it since
   // iterator already holds mutable reference. Tokenbuf length functionality is
   // not necessary for integration tests (how i think at the moment).
   // Other problem was with lifetimes, i could not pass Box<dyn..> to
   // tokenizer::tokenlist_match_or_fail as argument, due to problems with
   // lifetimes.
   fn tokenlist_match_or_fail(&mut self, list: &[Token])
      -> Result<(), (Option<Token>, Option<Token>)>
   {
      let mut idx = 0;

      // This index is out of bounds in relative measure to expected list.
      let idx_oob = list.len();

      let mut t = self.tokenstream_mut();

      // This is a tricky loop, because it must be able to detect if there are
      // enough items in buffer, if Token consumption is limited, then no more
      // items can be consumed than allowed.
      while let Some(token) = t.next() {
         // If tokenizer returns more items than are in expected item buffer,
         // we must error out. This must be done at iteration start.
         if idx >= idx_oob {
            return Err((None, Some(token)));
         }

         // We do not care if Tokenizer has changed state. We only care about
         // correct return tokens.
         if let Token::StateChange = token {
            continue;
         }

         // If there are expected items, compare if they match.
         if let Some(expect) = list.get(idx) {
            // println!("expected: {:?}, at idx: {}", expect, idx);
            if *expect != token {
               return Err((Some((*expect).clone()), Some(token)));
            }
         }
         else {
            return Err((None, Some(token)));
         }

         idx += 1;

         // Being here means that Token comparison succeeded.
      }

      // Tokenizer returned less Tokens than expected.
      if idx < idx_oob {
         if let Some(expect) = list.get(idx) {
            return Err((Some((*expect).clone()), None));
         }
      }

      Ok(())
   }



   // Load expected test answers and run Tokenizer iterator to comare them.
   // Panics if mismatched Tokens found.
   fn token_test_run(&mut self){
      if let None = self.name_expected() {
         panic!("There is no known expected result name for test.");
      }

      self.expected_load();

      let list = if let Some(expected) = self.name_expected() {
         let ex2 = expected.to_string();
         self.expected().get(&ex2).unwrap().clone()
      }
      else {
         panic!("There is no expected filename defined.");
      };

      if let Err((expect, got)) = self.tokenlist_match_or_fail(&list){
         panic!("Token mismatch. Expect: {:?} vs got: {:?}", expect, got);
      }
   }
}


