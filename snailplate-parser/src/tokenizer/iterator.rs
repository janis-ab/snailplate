use crate::{
   tokenizer::{
      Tokenizer,
      TokenizerState,
   },
   token::Token,
   parse_error::ParseError,
};



impl Iterator for Tokenizer {
   type Item = Token;

   #[inline]
   fn next(&mut self) -> Option<Self::Item> {
      use TokenizerState as Ts;

      // We allow to consume tokenbuf even if Tokenizer is in failed state. This
      // is so that user can receive all warning/error tokens up to the point
      // where Tokenizer failed.
      if self.tokenbuf.num_tokens() > 0 {
         match self.tokenbuf_consume() {
            Ok(tok) => {
               if let Some(tok) = tok {
                  // If we have consumed correct buffered token, just return it
                  // to the caller.
                  #[cfg(feature = "dbg_tokenbuf_verbose")] {
                     println!("Tokenizer: return buffered token: {:?}", tok);
                  }
                  return self.return_tokenized(tok);
               }
               else {
                  #[cfg(feature = "dbg_tokenbuf_verbose")] {
                     println!("Tokenizer(WARN): buffered token not found!");
                  }

                  /* That's okay. Tokenbuf has been completeley consumed. */
               }
            }

            Err(etok) => {
               // It is expected that tokenbuf_consume, if there is an error,
               // leaves Tokenizer in such a state, that even if it is used in
               // Iterator/loop, None is returned on next call. Thus we should
               // not worry about infinite looping here.
               return Some(etok);
            }
         }
      }

      match self.state {
         Ts::ExpectDefered => {
            self.defered_tokenize()
         }
         Ts::ExpectInstructionClose => {
            self.tokenize_instruction_args()
         }
         Ts::Failed => {
            None
         }
         Ts::ExpectInput => {
            if let ParseError::NoInput = self.parse_error_prev {
               None
            }
            else {
               self.parse_error_prev = ParseError::NoInput;

               // TODO: I don't know if in this case tokenizer state should be
               // set to "failed". Because this is a wrong way to use this
               // tokenizer (call without input), but it does not do too much
               // damage to anything, since tokenizer can stay at almost initial
               // state. If we did set state to failed here, then we would have
               // to write special recovery code in src_push function. Maybe that
               // is the right way to do.
               Some(Token::Error(ParseError::NoInput))
            }
         }
      }
   }
}


