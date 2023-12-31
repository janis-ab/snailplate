//! Deque-like buffer for Token buffering.
//!
//! TokenBuf provides a simple dequeue-like API over Vec. Various tokenization 
//! components require a deque where to store Tokens, because in Iterator only
//! single Token is returned, while the Tokenizer itself has analyzed code and
//! generated multiple Tokens. The same applies when Tokenizer returns Error
//! tokens, it must store them somewhere since Iterator returns only one Token
//! at a time.
//!
//! TokenBuf is inteded to be used for crate internally, since it has some 
//! limitations that real deque implementations don't.
//!
//! It is not allowed to append and popleft Tokens from TokenBuf at the same 
//! time. Caller either appends items till it's done, or pops items until 
//! TokenBuf is empty (that's a limitation due to used design, but it is good
//! enough to satisfy Tokenizer requirements).
//!
//! TokenBuf is light-weight compared to reald deque, since it uses only Vec
//! for data storage.
//!
//! When we look at TokenBuf, we imagine that the end is on the right side, thus
//! when items are appended, they are appended on the right side. And items are
//! removed from the left side.

use crate::{
   token::Token,
   parse_error::{
      ParseError,
      Source,
      Component,
   },
};



#[derive(Debug)]
pub struct TokenBuf {
   // Number of tokens that are available inside tokenbuf. At the same time this
   // is how many items from the tokenbuf end has to be consumed before clearing
   // array empty.
   num_tokens: usize,

   // This buffer stores tokens temporarily. The idea is that while tokenizer is
   // consuming text, it can happen that it recognizes multiple tokens in one
   // go, but iterator interface requires us to return only single item. Thus
   // non returned items can be stored in buffer. This should make it easier to
   // write a tokenizer.
   buf: Vec<Token>,
}



impl TokenBuf {
   pub fn new() -> Self {
      Self {
         num_tokens: 0,
         buf: Vec::with_capacity(16)
      }
   }



   /// Append Token to the end of this TokenBuf.
   ///
   /// Memory is automatically allocated if necessary. In normal conditions this
   /// function should not fail at all. The returned error Source always has
   /// pos_zero set to 0, since TokenBuf does not have any knowledge of position
   /// within Token stream.
   ///
   /// If error is returned, caller is responsible to modify returned Token's
   /// Source.pos_zero accordingly. This i believe is a good performance 
   /// tradeoff because errors are expected to be from rare to none, and we do
   /// not want to pay with stack allocations in each call just to store 
   /// pos_zero value for those rare ocassions.
   ///
   /// # Arguments
   ///
   /// * `token` - Token that is going to be appended to TokenBuf.
   ///
   /// # Panics
   ///
   /// This function does not panic, unless there is a Bug in code.
   ///
   /// By design Vec.push can panic if there is not enough memory available when
   /// capacity has to be increased. This function handles that case by calling
   /// try_reserve, which does not panic, thus Vec capacity should always be 
   /// enough to store Token item and not panic.
   ///
   /// # Return
   ///
   /// * 'Ok(())' - success, Result is always an empty tuple since there is nothing
   ///   to return.
   ///
   /// * 'Err(Token)' - error, contains Token that can be forwarded to consumer
   ///   to inform about error that has occured.
   ///
   pub fn append(&mut self, token: Token) -> Result<(), Token> {
      let tb = &mut self.buf;

      // Ensure that there is enough memory in Vec. This is done because push 
      // will panic if there is not enough memory available, but we do not want
      // to panic in such cases. There is an experimental API function
      // push_within_capacity available, but i do not want to use experimental 
      // API either.
      let cap = tb.capacity();
      let len = tb.len();
      if cap < len + 1 {
         if let Err(..) = tb.try_reserve(16) {
            return Err(Token::Fatal(ParseError::NoMemory(Source {
               pos_zero: 0,
               component: Component::TokenBuf,
               line: line!(),
               code: 3,
            })));
         }
      }

      #[cfg(feature = "dbg_tokenbuf_verbose")] {
         println!("Tokenbuf push: {:?}", token);
      }

      // This feature is mostly intended to be used while developing to detect
      // some Tokenizer's state problems.
      //
      // In general, the idea is simple - Tokenizer should do single action
      // untill it is complete (regarding TokenBuf); i.e. if Tokenizer is 
      // pushing tokens into Tokenbuf, then it does only push action, if 
      // Tokenizer is consuming items from TokenBuf, then it consumes till 
      // TokenBuf is empty. It is not allowed to insert 5 items, consume 2, then
      // insert 1, and then consume 4 items. All-in or all-out, each action can
      // be iterated.
      //
      // This constraint exists, because i don't want to use real dequeue. It 
      // seems too heavy of a data structure for this task. We just need to 
      // buffer some tokens that were parsed and then return them through 
      // iterator till buffer is empty.
      #[cfg(feature = "tokenbuf_push_guard")] {
         if len != self.num_tokens {
            return Err(Token::Fatal(ParseError::InternalError(Source {
               pos_zero: 0,
               component: Component::TokenBuf,
               line: line!(),
               code: 3,
            })));
         }
      }

      self.buf.push(token);
      self.num_tokens += 1;

      Ok(())
   }



   /// Removes first Token out of TokenBuf and returns it.
   ///
   /// It is allowed to use TokenBuf in all-in/all-out manner only.
   ///
   /// # Return
   ///
   /// `Ok(None)` - if TokenBuf is empty, there is nothing to be returned.
   ///
   /// `Ok(Token)` - when TokenBuf is not empty, a Token is returned.
   ///
   /// `Err(Token)` - this should never happen if code works as intended, but in
   ///    case there is a bug in code, an error is returned containing Token, 
   ///    that envelops Source for error.
   ///
   pub fn popleft(&mut self) -> Result<Option<Token>, Token> {
      if self.num_tokens < 1 {
         return Ok(None);
      }

      let num_tokens = self.num_tokens;
      let num_in_buf = self.buf.len();

      // It is necessary to guard against access outside Vec buffer. We don't
      // want to panic due to such an error.
      if num_in_buf < num_tokens {
         // Unfortunateley we must alter tokenbuf state to avoid infinite loop.
         self.num_tokens = 0;
         self.buf.clear();

         return Err(Token::Fatal(ParseError::InternalError(Source {
            pos_zero: 0,
            component: Component::TokenBuf,
            line: line!(),
            code: 1,
         })));
      }

      let idx_item = num_in_buf - num_tokens;
      self.num_tokens -= 1;

      #[cfg(feature = "dbg_tokenbuf_verbose")] {
         println!("Tokenbuf: return item at index: {}", idx_item);
      }

      let num_tokens = self.num_tokens;

      let tok_ref = self.buf.get(idx_item);
      let token = if let Some(tok_ref) = tok_ref {
         let item = (*tok_ref).clone();
         #[cfg(feature = "dbg_tokenbuf_verbose")] {
            println!("Tokenbuf: return item at item: {:?}", item);
         }
         item
      }
      else {
         // This is really bad case; bug in code. The only way i see this
         // happening is that num_tokens is out of sync with tokenbuf.len(), but
         // never the less, we must handle that gracefully.

         // Unfortunateley we must alter tokenbuf state, because 
         // TokenBuf.popleft is allowed to be called even if Tokenizer is in
         // Failed state. I don't expect this to ever happen, but in case it
         // does happen, we will add extra debugging at that time.
         self.num_tokens = 0;
         self.buf.clear();

         return Err(Token::Fatal(ParseError::InternalError(Source {
            pos_zero: 0,
            component: Component::TokenBuf,
            line: line!(),
            code: 2,
         })));
      };

      #[cfg(feature = "dbg_tokenbuf_verbose")] {
         println!("Tokenbuf: array num items: {}, self.num_tokens: {}",
            num_in_buf, num_tokens
         );
      }

      // This was the last item that we cloned out, so we can clear Vec.
      if num_tokens < 1 {
         #[cfg(feature = "dbg_tokenbuf_verbose")] {
            println!("self.num_tokens: {}, must CLEAR BUFFER", self.num_tokens);
         }

         self.buf.clear();
      }

      Ok(Some(token))
   }



   /// Returns the number of slots reserved in buffer for Tokens.
   ///
   /// Since TokenBuf is used like dequeue, length of Vec does not necessarily
   /// represent the number of Tokens stored. It can be the number or Tokens
   /// stored available for reading from TokenBuf, but it can be more than
   /// available Tokens in cases when some Tokens are already consumed.
   pub fn buf_len(&self) -> usize {
      self.buf.len()
   }



   /// Returns the number of available tokens within TokenBuf for consumtion.
   pub fn num_tokens(&self) -> usize {
      self.num_tokens
   }
}



// ================== EOF: do not write below this ============================
