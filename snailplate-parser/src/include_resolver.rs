// IncludeResolver is a component that transforms @include Real tokens to
// Phantom tokens and pushes included source file contents onto Tokenizers
// input buffer to generate real tokens.

use std::{
   fs::{self, File},
   io::Read,
};

use crate::{
   token::Token,
   tokenizer::{
      Tokenizer,
      TokenizerState,
   },
   tokenbody::TokenBody,
   tokenbuf::TokenBuf,
   span::Span,
   parse_error::{
      ParseError,
      Component,
      Source,
   }
};



mod iterator;



enum IncludeResolverState {
   // In this state, resolver returns tokens as-is from Tokenizer.
   Passthrough,

   // This state is activated when @include directive is met. In this state
   // IncludeResolver expects to have file path to be resolved, only allowed tokens are
   // OpenParen, CloseParen, Whitespace (will emit warning) and FilePath.
   // Other tokens will raise error.
   ResolveInclude,

   // Due to bugs in sub-components resolver can go into Failed state. This
   // should be rare.
   Failed,
}



// Each state might have more sub-states.
enum IncludeResolverSubState {
   // This is the default sub-state when nothing is set.
   Uninitialized,

   ExpectOpenParen,
   ExpectPath,
   ExpectCloseParen
}



// When receiving Tokens for complete batch, in each iteration it is necessary
// to decide if given batch is valid or Tokenizer returned a Token, that breaks
// whole chain thus batch becomes invalid.
enum IncludeResult {
   // This token is valid at this point in time, thus batch assembly must
   // continue.
   Progress(Token),

   // This token is valid, but it finalizes given batch. Batch handling routines
   // must be run.
   Finalized(Token),

   // This token is not valid, and it breaks the batch. IncludeResolver should
   // pass-through tokens as is with added warning/errors Tokens if necessary.
   Failed(Token),
}



pub struct IncludeResolver {
   pub tokenizer: Tokenizer,

   state: IncludeResolverState,
   substate: IncludeResolverSubState,

   // This buffer stores Tokens temporarily. The idea is that while Resolver is
   // consuming Tokens, it can happen that it must return multiple tokens when
   // only single Token is received.
   tokenbuf: TokenBuf,

   // Since IncludeResolver has to handle Tokens in batches, it must have a
   // temporary buffer for current batch. Not to allocate it on each iteration
   // have it here. It should be filled/emptied in single function call.
   batchbuf: TokenBuf,

   // This can contain Span for include/require file path.
   tokenspan_file: Option<Span>,

   // Path to directory where all template files should be searched for.
   root_dir: Option<String>,

   include_pos_zero: Option<usize>
}



impl IncludeResolver {
   pub fn new() -> Self {
      Self {
         state: IncludeResolverState::Passthrough,
         substate: IncludeResolverSubState::Uninitialized,
         tokenizer: Tokenizer::new(),
         tokenbuf: TokenBuf::new(),
         batchbuf: TokenBuf::new(),
         tokenspan_file: None,
         root_dir: None,
         include_pos_zero: None,
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
            op @ TokenBody::Include(span) => {
               #[cfg(feature = "dbg_include_resolver_verbose")] {
                  println!("Resolver: got include operation token: {:?}", op.fmt(t));
               }

               self.state = IncludeResolverState::ResolveInclude;
               self.substate = IncludeResolverSubState::ExpectOpenParen;

               self.include_pos_zero = Some(span.pos_zero);

               // Resolver takes care of @include token
               if let Err(etoken) = self.batchbuf.append(Token::Phantom(op)) {
                  self.state = IncludeResolverState::Failed;
                  return Some(etoken);
               }

               Some(Token::StateChange)
            }

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



   // Handle expected Token::OpenParen when in ResolveInclude state.
   //
   // This function exists just to split code in more manageable/readable
   // chunks.
   #[inline(always)]
   fn next_resolve_include_expect_open_paren(&mut self) -> IncludeResult {
      let t = &mut self.tokenizer;

      use Token as T;
      use TokenBody as Tb;
      use IncludeResolverSubState as SS;

      // This construct is used, so that we have less deep indentation.
      let token = if let Some(token) = t.next() { token }
      else {
         // @include instruction requires arguments, thus parenthesis must
         // exist, so we inform user about this.

         let tok = IncludeResult::Failed(Token::Error(
            ParseError::InstructionNotOpen(Source {
               pos_zero: self.include_pos_zero.unwrap_or(0),
               component: Component::IncludeResolver,
               line: line!(),
               code: 1,
            })
         ));

         return tok;
      };

      match token {
         T::Real(body) => match body {
            tok @ Tb::OpenParen(..) => {
               // TODO: here we could change Tokenizer state to "parse-path" or
               // something similar. At the moment we accept Defered token as
               // include file path.

               self.substate = SS::ExpectPath;

               // Outer code must still know that @include/require token was
               // tokenized, thus Token becomes a Phantom.
               IncludeResult::Progress(T::Real(tok))
            }

            tok @ Tb::WhiteSpace(..) => {
               // Based on DD-2023-07-14-01, IncludeResolver has transform
               // Real WhiteSpace to Phantom WhiteSpace when expecting
               // OpenParen.
               //
               // IncludeResolver does not have to emit any warning token,
               // because in this case it is Tokenizer's responsibility.
               IncludeResult::Progress(T::Real(tok))
            }

            tok => {
               // TODO: here we should actually return some error and warning
               // tokens as well. For now this would require to make an artificial
               // test/situation, since Tokenizer will not return @include
               // as a token if there is no open parenthesis (at the moment).

               // Since this was unexpected token, Resolver switches back to
               // pass-through state. @include instruction could not be
               // satisfied.
               self.state = IncludeResolverState::Passthrough;
               self.substate = IncludeResolverSubState::Uninitialized;

               IncludeResult::Failed(T::Real(tok))
            }
         }

         // At the moment IncludeResolver ignores any Phantom token.
         tok @ T::Phantom(..) => {IncludeResult::Progress(tok) }

         // StateChange is silently passed through, because it should not
         // influence IncludeResolver.
         tok @ T::StateChange => { IncludeResult::Progress(tok) }

         // If Tokenizer has returned Fatal, even if next() is called again,
         // it would return Fatal. Thus Resolver is allowed to be transparent.
         // But we are sure that instruction batch will not be collected.
         tok @ T::Fatal(..) => { IncludeResult::Failed(tok) }

         // TODO: There are some errors that can be useful for Resolver as well,
         // i.e. Tokenizer informs that instruction was not started correctly.
         // At the moment i will leave this for future to be fixed.
         // There should be no errors between instruction and parenthesis.
         tok @ T::Error(..) => { IncludeResult::Failed(tok) }

         tok @ T::Warning(..) => { IncludeResult::Progress(tok) }
      }
   }



   // Handle expected Token::Defered when in ResolveInclude/ExpectPath state.
   //
   // This function exists just to split code in more manageable/readable
   // chunks. The goal is to collect such a Token that describes allowed
   // file path to be included/required and cache it.
   #[inline(always)]
   fn next_resolve_include_expect_path(&mut self) -> IncludeResult {
      let t = &mut self.tokenizer;

      use Token as T;
      use TokenBody as Tb;
      use IncludeResolverSubState as SS;

      // This construct is used, so that we have less deep indentation.
      let token = if let Some(token) = t.next() { token }
      else {
         // There has to be path available within parenthesis.

         let tok = IncludeResult::Failed(Token::Error(
            ParseError::InstructionMissingArgs(Source {
               pos_zero: self.include_pos_zero.unwrap_or(0),
               component: Component::IncludeResolver,
               line: line!(),
               code: 2,
            })
         ));

         return tok;
      };

      match token {
         T::Real(body) => match body {
            // TODO: in future we should use special Token that describes
            // include path, instead of Defered. At the moment i do not want
            // to change Tokenizer's code.
            tbody @ Tb::Defered(span) => {
               self.substate = SS::ExpectCloseParen;
               self.tokenspan_file = Some(span);

               // TODO: if already one defered token exists, we can warn user
               // that filename is wrong, and we can even test if previous token
               // is valid file path, if is, then warn user about forgotten
               // close parenthesis.

               return IncludeResult::Progress(T::Real(tbody));
            }
            _ => {
               panic!("not impl")
            }
         }

         // At the moment IncludeResolver ignores any Phantom token.
         tok @ T::Phantom(..) => { IncludeResult::Progress(tok) }

         // StateChange is silently passed through, because it should not
         // influence IncludeResolver while it is collecting include path.
         tok @ T::StateChange => { IncludeResult::Progress(tok) }

         // If Tokenizer has returned Fatal, even if next() is called again,
         // it would return Fatal. Thus Resolver is allowed to be transparent.
         tok @ T::Fatal(..) => { IncludeResult::Failed(tok) }

         // TODO: There are some errors that can be useful for Resolver as well,
         // i.e. Tokenizer informs that instruction was not started/completed
         // correctly. At the moment i will leave this for future to be fixed.
         tok @ T::Error(..) => { IncludeResult::Failed(tok) }

         tok @ T::Warning(..) => { IncludeResult::Progress(tok) }
      }
   }



   // Handle expected Token::CloseParen when in ResolveInclude/ExpectCloseParen
   // state.
   //
   // This function exists just to split code in more manageable/readable
   // chunks.
   #[inline(always)]
   fn next_resolve_include_expect_close_paren(&mut self) -> IncludeResult {
      let t = &mut self.tokenizer;

      use Token as T;
      use TokenBody as Tb;
      use IncludeResolverSubState as SS;

      // This construct is used, so that we have less deep indentation.
      let token = if let Some(token) = t.next() { token }
      else {
         // There must exist closing parenthesis for us to allow instruction
         // to be resolved.

         let tok = IncludeResult::Failed(Token::Error(
            ParseError::OpenInstruction(Source {
               pos_zero: self.include_pos_zero.unwrap_or(0),
               component: Component::IncludeResolver,
               line: line!(),
               code: 3,
            })
         ));

         return tok;
      };

      match token {
         T::Real(body) => match body {
            tok @ Tb::CloseParen(..) => {
               #[cfg(feature = "dbg_include_resolver_verbose")] {
                  println!("Resolver: close paren received, can really include");
               }

               if let Some(span) = self.tokenspan_file {
                  self.tokenspan_file = None;
                  t.state_set(TokenizerState::ExpectDefered);
                  let slice = t.span_slice(&span);

                  #[cfg(feature = "dbg_include_resolver_verbose")] {
                     println!("Resolver: convert span to string filename");
                     println!("Resolver: filename slice: {:?}", slice);
                  }

                  if let Some(slice) = slice {
                        let fn_as_str = std::str::from_utf8(slice).unwrap().to_owned();
                        #[cfg(feature = "dbg_include_resolver_verbose")] {
                           println!("filename to include: {}", fn_as_str);
                        }

                        if let Err(..) = self.file_read(&fn_as_str) {
                           // TODO: here the challenge is that depending on
                           // instruction different action must be taken,
                           // @include returns warnings, @require returns
                           // errors.

                           println!("Error reading file"); // TODO:
                        }
                  }
                  else {
                        // This error should never happen unless there is a bug
                        // in code. Tokenizer should always be able to extract
                        // span slice for it's returned Tokens.

                        if let Err(etoken) = self.batchbuf.append(Token::Real(tok)) {
                           self.state = IncludeResolverState::Failed;

                           return IncludeResult::Failed(etoken);
                        }

                        let retok = IncludeResult::Failed(Token::Error(
                           ParseError::InternalError(Source {
                              pos_zero: self.include_pos_zero.unwrap_or(0),
                              component: Component::IncludeResolver,
                              line: line!(),
                              code: 4,
                           })
                        ));

                        return retok;
                  }

                  self.substate = SS::Uninitialized;
                  self.state = IncludeResolverState::Passthrough;

                  return IncludeResult::Finalized(Token::Real(tok));
               }
               else {
                  // TODO: return warning that include without file name is
                  // ignored and change state to parse defered?
                  println!("Resolver: not impl. return warning on empty include");

                  // Since Resolver could not fulfill @include instruction, it
                  // goes into pass-through state.
                  self.state = IncludeResolverState::Passthrough;
                  self.substate = IncludeResolverSubState::Uninitialized;

                  // We still return Phantom token here, since all parenthesis
                  // were matched and this would yield common behavior.
                  // Otherwise outer code would receive some Phantom tokens for
                  // '@include', '(' and Real token for ')'. That would seem
                  // weird.
                  // In this case it receives all tokens as Phantom. It is just
                  // that include did not happen, so it is replaced with
                  // nothing.

                  if let Err(etoken) = self.batchbuf.append(Token::Real(tok)) {
                     self.state = IncludeResolverState::Failed;

                     return IncludeResult::Failed(etoken);
                  }

                  let retok = IncludeResult::Failed(Token::Error(
                     ParseError::InstructionMissingArgs(Source {
                        pos_zero: self.include_pos_zero.unwrap_or(0),
                        component: Component::IncludeResolver,
                        line: line!(),
                        code: 5,
                     })
                  ));

                  return retok;
               }
            }

            tok => {
               // TODO: here we should actually return some error and warning
               // tokens as well.

               // Since this was unexpected token, Resolver switches back to
               // pass-through state. @include instruction could not be
               // satisfied.
               self.state = IncludeResolverState::Passthrough;
               self.substate = IncludeResolverSubState::Uninitialized;

               // TODO: think about correct behavior. At the moment '@include'
               // '(' and 'file-path' are returned as Phantom already. We got
               // unexpected Token instead of '('. Should we return it as Real
               // token, Phantom?

               IncludeResult::Failed(T::Real(tok))
            }
         }

         // At the moment IncludeResolver ignores any Phantom token.
         tok @ T::Phantom(..) => { IncludeResult::Progress(tok) }

         // StateChange is silently passed through, because it should not
         // influence IncludeResolver.
         tok @ T::StateChange => { IncludeResult::Progress(tok) }

         // If Tokenizer has returned Fatal, even if next() is called again,
         // it would return Fatal. Thus Resolver is allowed to be transparent.
         tok @ T::Fatal(..) => { IncludeResult::Failed(tok) }

         // TODO: There are some errors that can be useful for Resolver as well,
         // i.e. Tokenizer informs that instruction was not started correctly.
         // At the moment i will leave this for future to be fixed.
         tok @ T::Error(..) => { IncludeResult::Failed(tok) }

         tok @ T::Warning(..) => { IncludeResult::Progress(tok) }
      }
   }



   // Function that is called when next token is expected from Tokenizer. This
   // is just a placeholder for match statement so that code is in more
   // manageable chunks.
   #[inline(always)]
   fn next_resolve_include_by_substate(&mut self) -> IncludeResult {
      use IncludeResolverSubState as SS;

      match self.substate {
         SS::ExpectOpenParen => {
            self.next_resolve_include_expect_open_paren()
         }

         SS::ExpectPath => {
            self.next_resolve_include_expect_path()
         }

         SS::ExpectCloseParen => {
            self.next_resolve_include_expect_close_paren()
         }

         SS::Uninitialized => {
            panic!("Always should be initialized!");
         }
      }
   }



   // Function that is called when IncludeResolver has collected all @include
   // necessary tokens and moves translated Tokens from batchbuf to tokenbuf.
   //
   // Function returns first returnable token, normally it is OpenParen.
   #[inline(always)]
   fn next_resolve_include_finalized(&mut self) -> Option<Token> {
      // When returning list of tokens, we must hold first item for returning
      // and push all other items into tokenbuf, but maybe transformed
      let mut firstitem: Option<Token> = None;

      // Actually this function should not have been called if pos_zero is None.
      let pos_zero = self.include_pos_zero.unwrap_or(0);

      self.include_pos_zero = None;

      #[cfg(not(feature = "unguarded_include_resolver_integrity"))] {
         if self.batchbuf.buf_len() < 1 {
            self.state = IncludeResolverState::Failed;

            // In this case, there is nothing to be returned, batch buf is
            // emtpy, this function should not have been called at all.

            return Some(Token::Fatal(ParseError::InternalError(Source {
               pos_zero: pos_zero,
               component: Component::IncludeResolver,
               line: line!(),
               code: 7,
            })))
         }
      }

      let batchbuf = &mut self.batchbuf;
      loop {
         match batchbuf.popleft() {
            Ok(None) => {
               break;
            }

            Ok(Some(tok)) => {
               if let Some(..) = firstitem {
                  #[cfg(feature = "dbg_include_resolver_verbose")] {
                     println!("Resolver finalize nth Token: {:?}", tok);
                  }

                  match tok {
                     Token::Real(tbody) => {
                        let itoken = Token::Phantom(tbody);

                        if let Err(error_token) = self.tokenbuf.append(itoken) {
                           // This is really bad error, no memory or similar.
                           // There is nothing much we can do. Token is lost.
                           self.state = IncludeResolverState::Failed;
                           return Some(error_token);
                        }
                     }

                     p @ Token::Phantom(..)
                     | p @ Token::Error(..)
                     | p @ Token::Fatal(..)
                     | p @ Token::Warning(..)
                     => {
                        if let Err(error_token) = self.tokenbuf.append(p) {
                           self.state = IncludeResolverState::Failed;
                           return Some(error_token);
                        }
                     }

                     Token::StateChange => {}
                  }
               }
               else {
                  #[cfg(feature = "dbg_include_resolver_verbose")] {
                     println!("Resolver finalize first Token: {:?}", tok);
                  }

                  match tok {
                     Token::Real(tbody) => {
                        firstitem = Some(Token::Phantom(tbody));
                     }

                     p @ Token::Phantom(..)
                     | p @ Token::Error(..)
                     | p @ Token::Fatal(..)
                     | p @ Token::Warning(..)
                     => {
                        firstitem = Some(p)
                     }

                     Token::StateChange => {}
                  }
               }
            }

            Err(tok) => {
               self.state = IncludeResolverState::Failed;
               return Some(tok);
            }
         }
      }

      if let Some(tok) = firstitem {
         Some(tok)
      }
      else {
         // This case is impossible unless there is an error in code in this
         // function.

         self.state = IncludeResolverState::Failed;

         Some(Token::Fatal(ParseError::InternalError(Source {
            pos_zero: pos_zero,
            component: Component::IncludeResolver,
            line: line!(),
            code: 6,
         })))
      }
   }



   // Function that is called from Iterator when IncludeResolver is in state
   // ResolveInclude.
   //
   // The goal for this construct is to split code in readable chunks.
   //
   // Function tries to collect a batch of tokens from Tokenizer and if
   // collected batch satisfies @include, all tokens are transformed to Phantom
   // and returned (done by next_resolve_include_finalized).
   #[inline(always)]
   fn next_resolve_include(&mut self) -> Option<Token> {
      // IncludeResolver must transform Token::Real to Token::Phantom if it
      // resolves file inclusion, otherwise it should return Tokens as-is.
      //
      // To achieve that, IncludeResolver buffers tokens until whole instruction
      // is assembled and only then returns.

      #[cfg(not(feature = "unguarded_include_resolver_integrity"))] {
         if self.batchbuf.buf_len() != 1 {
            // TODO: do not panic, set state to failed and return error token?
            panic!("batchbuf should be empty!");
         }
      }

      enum BreakReason { Finalized, Failed }

      let break_reason = loop {
         match self.next_resolve_include_by_substate() {
            IncludeResult::Progress(token) => {
               #[cfg(feature = "dbg_include_resolver_verbose")] {
                  println!("Resolver collect Progress/Token: {:?}", token);
               }

               match token {
                  Token::StateChange => {}
                  _ => {
                     if let Err(e) = self.batchbuf.append(token) {
                        self.state = IncludeResolverState::Failed;
                        return Some(e);
                     }
                  }
               }
            }

            IncludeResult::Finalized(token) => {
               #[cfg(feature = "dbg_include_resolver_verbose")] {
                  println!("Resolver collect Finalized/Token: {:?}", token);
               }

               match token {
                  Token::StateChange => {}
                  _ => {
                     if let Err(e) = self.batchbuf.append(token) {
                        self.state = IncludeResolverState::Failed;
                        return Some(e);
                     }
                  }
               }

               break BreakReason::Finalized;
            }

            IncludeResult::Failed(token) => {
               #[cfg(feature = "dbg_include_resolver_verbose")] {
                  println!("Resolver collect Failed/Token: {:?}", token);
               }

               match token {
                  Token::StateChange => {}
                  _ => {
                     if let Err(e) = self.batchbuf.append(token) {
                        self.state = IncludeResolverState::Failed;
                        return Some(e);
                     }
                  }
               }

               break BreakReason::Failed;
            }
         }
      };

      match break_reason {
         BreakReason::Finalized => {
            self.next_resolve_include_finalized()
         }

         BreakReason::Failed => {
            // TODO: return Real tokens on error as-is.
            None
         }
      }
   }
}



// ================== EOF: do not write below this ============================
