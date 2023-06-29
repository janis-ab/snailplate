use crate::{
   tokenizer::Tokenizer,
   token::Token,
};



impl Iterator for Tokenizer {
   type Item = Token;

   #[inline]
   fn next(&mut self) -> Option<Self::Item> {
      // We allow to consume tokenbuf even if Tokenizer is in failed state. This
      // is so that user can receive all warning/error tokens up to the point
      // where Tokenizer failed.
      if self.num_tokens > 0 {
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

      // TODO: implement source tokenization based on Tokenizer state
      None
   }
}