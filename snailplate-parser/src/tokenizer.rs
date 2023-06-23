use crate::{
   token::Token,
   span::Span,
   ParseError,
};


// Tokenizer states.
pub enum TokenizerState {
   /// This is the initial state for Tokenizer. In this state user is not
   /// allowed to invoke iterator::next, since there is no source to tokenize.
   ExpectInput,

   /// This state is active when Tokenizer has got into unrecoverable
   /// tokenization error. This can happen due to various reasons, like, bug in
   /// code, bad input, etc. Once Tokenizer is in this sate it will not recover
   /// from encountered error. It still allows to consume buffered tokens, but
   /// nothing more.
   Failed,
}



/// Tokenizer struct stores internal state for Tokenizer. Each time a new byte
/// is read, it increases pos_*, line values, once Token is recognized, those
/// values are copied into Span token that is wrapped with returned Token.
pub struct Tokenizer {
   /// This is the index for current active region string. This will be cloned
   /// into Span when token is recognized.
   index: usize,

   /// This is "global" parsing position increased per each byte. When next
   /// source region is pushed, pos_zero keeps growing. Copied into Span.
   pos_zero: usize,

   /// This is source region relative position. It resets to 0 each time when
   /// new source chunk is pushed. Copied into Span.
   pos_region: usize,

   /// This is a line position in current region. Copied into Span.
   pos_line: usize,

   /// This is a line number in current region. Copied into Span.
   line: usize,

   /// Tokenization state
   state: TokenizerState,

   /// Number of tokens that are available inside tokenbuf. At the same time this
   /// is how many items from the tokenbuf end has to be consumed before clearing
   /// array empty.
   num_tokens: usize,

   /// This buffer stores tokens temporarily. The idea is that while tokenizer is
   /// consuming text, it can happen that it recognizes multiple tokens in one
   /// go, but iterator interface requires us to return only single item. Thus
   /// non returned items can be stored in buffer. This should make it easier to
   /// write a tokenizer.
   tokenbuf: Vec<Token>,
}



impl Tokenizer {
   pub fn new() -> Self {
      Self {
         state: TokenizerState::ExpectInput,
         index: 0,
         pos_zero: 0,
         pos_region: 0,
         pos_line: 0,
         line: 0,
         num_tokens: 0,
         tokenbuf: Vec::with_capacity(16),
      }
   }



   // Each time when fatal error is returned, it is necessary to set Tokenizer
   // state to Failed, but i already keep forgetting to do that too often, thus
   // create function to resolve that and forget this forever.
   #[inline(always)]
   fn fail(&mut self, return_token: Token) -> Result<Option<Token>, Token> {
      self.state = TokenizerState::Failed;
      Err(return_token)
   }



   // Function that allows us to push token into tokenbuf.
   // It is allowed to use tokenbuf in all-in/all-out manner only.
   #[inline(always)]
   fn tokenbuf_push(&mut self, tok: Token) -> Result<Option<Token>, Token> {
      let tb = &mut self.tokenbuf;

      // Ensure that there is enough memory in Vec. This is because push will
      // panic if there is not enough capacity, but we do not want to panic on
      // that. There is an experimental API push_within_capacity, but i do not
      // want to use experimental API either.
      let cap = tb.capacity();
      let len = tb.len();
      if cap < len + 1 {
         if let Err(..) = tb.try_reserve(16) {
            return self.fail(Token::Fatal(ParseError::NoMemory));
         }
      }

      #[cfg(feature = "dbg_tokenbuf_verbose")] {
         println!("Tokenbuf push: {:?}", tok);
      }

      // This feature is mostly intended to be used while developing to detect
      // some Tokenizers state problems.
      //
      // In general, the idea is simple - Tokenizer should do single action
      // regarding tokenbufer untill it is complete; i.e. if pushing tokens to
      // tokenbuf, then only push, if consuming items from tokenbuf, then
      // consume till tokenbuf is empty. It is not allowed to insert 5 items,
      // remove 2, then insert 1, etc. All-in or all-out, each action can be
      // iterated.
      //
      // This is like so, because i don't want to use dequeue. It seems too
      // heavy data structure for this task. We just need to buffer some tokens
      // that were parsed and then return them in iterator till buffer is empty.
      #[cfg(feature = "tokenbuf_push_guard")] {
         if len != self.num_tokens {
            return self.fail(Token::Fatal(ParseError::InternalError));
         }
      }

      self.tokenbuf.push(tok);
      self.num_tokens += 1;

      Ok(None)
   }



   // Function that takes one token out of tokenbuf.
   // It is allowed to use tokenbuf in all-in/all-out manner only.
   #[inline(always)]
   fn tokenbuf_consume(&mut self) -> Result<Option<Token>, Token> {
      #[cfg(feature = "dbg_tokenbuf_verbose")] {
         println!("Tokenbuf: consume index: {}, pos_local: {}",
            self.index, self.pos_local
         );
      }

      if self.num_tokens < 1 {
         return Ok(None);
      }

      let num_tokens = self.num_tokens;
      let num_in_buf = self.tokenbuf.len();

      // It is necessary to guard against access outside Vec buffer. We don't
      // want to panic due to such an error.
      if num_in_buf < num_tokens {
         // Unfortunateley we must alter tokenbuf state to avoid infinite loop.
         let num_tokens_prev = self.num_tokens;
         let len_prev = self.tokenbuf.len();
         self.num_tokens = 0;
         self.tokenbuf.clear();

         let msg = format!("Accessed index is negative (out of range). Current \
         num_tokens: {}, num_in_buf: {}", num_tokens, num_in_buf
         );

         return self.fail(Token::Fatal(ParseError::TokenbufBroken(Span {
               index: self.index, pos_region: self.pos_region,
               pos_line: self.pos_line, pos_zero:self.pos_zero,
               line: self.line, length: 0
            },
            msg, num_tokens_prev, len_prev
         )));
      }

      let idx_item = num_in_buf - num_tokens;
      self.num_tokens -= 1;

      #[cfg(feature = "dbg_tokenbuf_verbose")] {
         println!("Tokenbuf: return item at index: {}", idx_item);
      }

      let num_tokens = self.num_tokens;

      let tok_ref = self.tokenbuf.get(idx_item);
      let token = if let Some(tok_ref) = tok_ref {
         (*tok_ref).clone()
      }
      else {
         // This is really bad case; bug in code. The only way i see this
         // happening is that num_tokens is out of sync with tokenbuf.len(), but
         // never the less, we must handle that gracefully.

         // Unfortunateley we must alter tokenbuf state, because tokenbuf_consume
         // is allowed even if Tokenizer is in Failed state. I don't expect this
         // to happen, so in case it does, will add extra debugging at that time.
         let num_tokens_prev = self.num_tokens;
         let len_prev = self.tokenbuf.len();
         self.num_tokens = 0;
         self.tokenbuf.clear();

         let msg = format!(
            "There was no item in accessed index: {}.", idx_item
         );

         return self.fail(Token::Fatal(ParseError::TokenbufBroken(Span {
               index: self.index, pos_region: self.pos_region,
               pos_line: self.pos_line, pos_zero:self.pos_zero,
               line: self.line, length: 0
            },
            msg, num_tokens_prev, len_prev
         )));
      };

      #[cfg(feature = "dbg_tokenbuf_verbose")] {
         println!("Tokenbuf: array num items: {}, self.num_tokens: {}",
            num_in_buf, num_tokens
         );
      }

      // This was the last item that we cloned out, so we can flush array
      if num_tokens < 1 {
         #[cfg(feature = "dbg_tokenbuf_verbose")] {
            println!("self.num_tokens: {}, must CLEAR BUFFER", self.num_tokens);
         }

         self.tokenbuf.clear();
      }

      // Tokens that have a span, change Tokenizers position. Non-span tokens
      // are just informative tokens that do not change Tokenizers position.
      if let Some(span) = token.span_clone() {
         // At first i thought that maybe this should be a feature flag, but
         // then i realized that one little if statement is far better than
         // wrong parse results.
         if self.pos_region != span.pos_region {
            // Unfortunateley we must alter tokenbuf state.
            let num_tokens_prev = self.num_tokens;
            let len_prev = self.tokenbuf.len();
            self.num_tokens = 0;
            self.tokenbuf.clear();

            let msg = format!(
               "Tokenizer.pos_region != Span.pos_region. Token: {:?}", token
            );

            // It is not possible to return whole token, since that would
            // require ParseError to be able to store Token, which is not
            // possible. But anyways this should give us enough information
            // to detect error.
            return self.fail(Token::Fatal(ParseError::TokenbufBroken(Span {
                  index: self.index, pos_region: self.pos_region,
                  pos_line: self.pos_line, pos_zero:self.pos_zero,
                  line: self.line, length: 0
               },
               msg, num_tokens_prev, len_prev,
            )));
         }

         // At this point returned token is oficially recognized, so we
         // update current position acordingly.
         self.pos_region += span.length;
         self.pos_zero += span.length;
      }

      Ok(Some(token))
   }
}



#[cfg(test)]
mod test;