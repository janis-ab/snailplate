use crate::tokenbody::TokenBody;
use crate::span::{Span, SpanFormatter};
use std::fmt;



/// This structure describes tokenized token. If content is corectly parsed 
/// only Real tokens are returned, but if there are errors or some tokens are
/// replaced by different content, then Pahntom tokens are returned.
///
/// This approach is used, because i want to implement an iterator over 
/// tokenizer that returns stream of tokens. This approach allows me to do that.
///
// I am not sure if this is the best approach; i was thinking a while should i
// use the same TokenBody for Real and Phantom tokens. It was a dilema, because
// most probably phantom tokens will be only for such a tokens like @include,
// errors and warnings, but real tokens will be for all tags, etc.
// I still decided to pick this approach, because it felt that like this the
// code can be reused more, especially for debugging purposes; another advantage
// that this gives is that any time in future any token can become a Phantom 
// token, conversion is easy.
#[derive(Debug, Clone, Copy)]
pub enum Token {
    /// These tokens are content related.
    Real(TokenBody),

    /// Phantom tokens should not be used to generate content. These tokens are
    /// used to return warnings, errors, consumed/expanded tokens. For example
    /// Resolver expands @include, but path has some errors, so Resolver returns
    /// Phantom(Include(..)) and then warning/error tokens to inform user about
    /// the situation. Token stream receiver can then do analysis on warning
    /// or error tokens and build useful message for user.
    Phantom(TokenBody),

    /// This token should be ignored by the outside code. This is necessary so that
    /// when some component changes state fro Tokenizer, Resolver, etc. the state
    /// is changed. This makes while loops/state changes easier to write. This
    /// is returned when sub-state changes as well.
    StateChange,    
}



impl Token {
    pub fn fmt<'a, F: SpanFormatter>(&'a self, bufowner: &'a F) -> TokenFormatWrapper<F> {
        TokenFormatWrapper(self, bufowner)
    }



    pub fn span_clone(&self) -> Option<Span> {
        use Token as T;

        match &self {
            T::Real(body) | T::Phantom(body) => Some(body.span_clone()),
            T::StateChange => None
        }
    }    
}



/// See tokenbody::TokenBodyFormatWrapper for idea explanation.
pub struct TokenFormatWrapper<'a, F: SpanFormatter> (&'a Token, &'a F);



impl<'a, F: SpanFormatter> std::fmt::Debug for TokenFormatWrapper<'a, F> {
   fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
      use Token as T;

      let (start, end, body) = match self.0 {
         T::Real(body) => (Some("Real("), Some(")"), Some(body)),
         T::Phantom(body) => (Some("Phantom("), Some(")"), Some(body)),
         T::StateChange => (Some("StateChange"), None, None),
      };

      let body = if let Some(body) = body {
         body
      }
      else {
         if let Some(start) = start {
            if let Err(e) = f.write_str(start) {
               return Err(e);
            }
         }

         if let Some(end) = end {
            if let Err(e) = f.write_str(end) {
               return Err(e);
            }
         }             

         // Being here means that start/end were formatted, and there is no
         // body, thus return Ok().

         return Ok(());
      };

      if let Some(start) = start {
         if let Err(e) = f.write_str(start) {
            return Err(e);
         }
      }

      // Here we use body.fmt() call to create TokenBodyFormatWrapper to get
      // Debug trait for that, not the derived one.
      if let Err(e) = fmt::Debug::fmt(&body.fmt(self.1), f){
         return Err(e);
      }    

      if let Some(end) = end {
         if let Err(e) = f.write_str(end) {
            return Err(e);
         }
      }        

      Ok(())
   }
}



#[cfg(test)]
mod test;