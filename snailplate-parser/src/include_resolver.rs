// IncludeResolver is a component that transforms @include Real tokens to
// Phantom tokens and pushes included source file contents onto Tokenizers
// input buffer to generate real tokens.

use std::{
   fs::{self, File},
   io::Read,
};

use crate::{
   token::Token,
   tokenizer::Tokenizer,
};



mod iterator;



enum IncludeResolverState {
   // In this state, resolver returns tokens as-is from Tokenizer.
   Passthrough,
}



pub struct IncludeResolver {
   pub tokenizer: Tokenizer,

   state: IncludeResolverState,

   // Path to directory where all template files should be searched for.
   root_dir: Option<String>,
}



impl IncludeResolver {
   pub fn new() -> Self {
      Self {
         state: IncludeResolverState::Passthrough,
         tokenizer: Tokenizer::new(),
         root_dir: None,
      }
   }



   pub fn template_root_dir_set(&mut self, root_dir: &str) {
      // TODO: here we should check if provided directory path is absolute
      // or relative. If path is relative, get current working directory and
      // concatenate it with root_dir argument to make a full root_dir path.
      //
      // At the moment i can not think of a reason why anyone would like 
      // directory path to change automatically with CWD of process. Especially
      // since we are building statically compiled templataes.
      //
      // Resolving root_dir from relative to absolute at this stage would give
      // better stability, IMHO.
      //
      // Deny "../.." parts within path.

      self.root_dir = Some(root_dir.to_owned());
   }



   pub fn file_read(&mut self, filename: &str) -> Result<(), Token> {
      let root_dir = match &self.root_dir {
         None => panic!("must have root dir"),
         Some(dir) => dir
      };

      // TODO: actually here we would like to use OS independent code with
      // using path buffer and pushing items to it. For now this is a quick
      // prototype.

      let mut fn_path = root_dir.clone();
      fn_path.push('/');
      fn_path.push_str(filename);

      #[cfg(feature = "dbg_include_resolver_verbose")] {
         println!("file_read: {}", fn_path);
      }

      let file_size = fs::metadata(&fn_path).unwrap().len();

      let mut root = Vec::with_capacity(file_size.try_into().unwrap());
      let mut file = File::open(fn_path).expect("unable to open file");

      #[cfg(feature = "dbg_include_resolver_verbose")] {
         println!("template file size is: {}", file_size);
      }

      file.read_to_end(&mut root).expect("unable to read file");

      if let Err(token) = self.tokenizer.src_push(Some(filename), root) {
         return Err(token);
      }

      Ok(())
   }



   pub fn next_passthrough(&mut self) -> Option<Token> {
      let t = &mut self.tokenizer;

      let token = match t.next(){
         Some(token) => token,
         None => return None
      };

      match token {
         Token::Real(body) => match body {
            // TODO: here we should check if returned token is @include and
            // switch state to path parsing for Tokenizer.

            tok => {
               // Pass through any other token because for IncludeResolver it is
               // not significant.
               #[cfg(feature = "dbg_include_resolver_verbose")] {
                  println!("Resolver: got some token: {:?}", tok.fmt(&self.tokenizer));
               }
               Some(Token::Real(tok))
            }
         }

         phantom @ Token::Phantom(..) => Some(phantom),
         state_change @ Token::StateChange => Some(state_change),

         // TODO: handle error/waring tokens.
         // For now transparently pass through other tokens.
         tok => Some(tok),
      }
   }
}



// ================== EOF: do not write below this ============================
