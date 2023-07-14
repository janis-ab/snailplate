use crate::{
   tokenbody::TokenBody,
   span::{Span, SpanFormatter},
   parse_error::ParseError,
};
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
#[derive(Debug, Clone, Eq, PartialEq)]
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

   // fatal errors are those that do not allow to continue parsing,
   Fatal(ParseError),

   // Error that allows parser to continue parsing template source, but 
   // fails the compilation of it. Sometimes this is a recoverable error.
   Error(ParseError),

   // Warnings are a Tokens that allow parsing to continue, but emit warnings 
   // messages at template compile time.
   Warning(ParseError),

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
      use ParseError as Pe;

      match &self {
         T::Real(body)
         | T::Phantom(body)
         => {
            Some(body.span_clone())
         }

         T::Fatal(parse_error)
         | T::Error(parse_error)
         | T::Warning(parse_error)
         => match parse_error {
            Pe::InstructionError(..) 
            | Pe::OpenInstruction(..)
            | Pe::InstructionNotOpen(..)
            | Pe::InstructionMissingArgs(..)
            | Pe::NoMemory(..)
            | Pe::InternalError(..)
            | Pe::NoInput(..)
            => {
               // TODO: In future maybe we can construct a meaningful Span object.
               None
            }
            Pe::None => None,
         }

         T::StateChange => None
      }
   }
}



/// See tokenbody::TokenBodyFormatWrapper for idea explanation.
pub struct TokenFormatWrapper<'a, F: SpanFormatter> (&'a Token, &'a F);



impl<'a, F: SpanFormatter> std::fmt::Debug for TokenFormatWrapper<'a, F> {
   fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
      use Token as T;
      use ParseError as Pe;

      // This macro is the least i could write to ease a little bit code
      // repetition. At first i wanted to write a macro that generates match
      // statement arms with body, but Rust does not suppot that, then i did
      // not want to pass $source as an argument, but Rust macro hygene forces
      // me to do so.
      macro_rules! error_tuple {
         ($case:tt, $parse_error:tt, $source:tt) => {(
            Some(concat!(stringify!($case), "(", stringify!($parse_error), "(")),
            Some(format!("{:?}", $source)), Some("))"), None
         )}
      }

      let (start, mid, end, body) = match self.0 {
         T::Real(body) => (Some("Real("), None, Some(")"), Some(body)),
         T::Phantom(body) => (Some("Phantom("), None, Some(")"), Some(body)),
         T::Fatal(parse_error) => match parse_error {
            Pe::OpenInstruction(source)
               => error_tuple!(Fatal, OpenInstruction, source),
            Pe::InstructionError(source)
               => error_tuple!(Fatal, InstructionError, source),
            Pe::InstructionNotOpen(source)
               => error_tuple!(Fatal, InstructionNotOpen, source),
            Pe::InstructionMissingArgs(source)
               => error_tuple!(Fatal, InstructionMissingArgs, source),
            Pe::NoMemory(source)
               => error_tuple!(Fatal, NoMemory, source),
            Pe::InternalError(source)
               => error_tuple!(Fatal, InternalError, source),
            Pe::NoInput(source)
               => error_tuple!(Fatal, NoInput, source),
            Pe::None => {
               (Some("Fatal(None"), None, Some(")"), None)
            }
         }

         T::Error(parse_error) => match parse_error {
            Pe::OpenInstruction(source)
               => error_tuple!(Error, OpenInstruction, source),
            Pe::InstructionError(source)
               => error_tuple!(Error, InstructionError, source),
            Pe::InstructionNotOpen(source)
               => error_tuple!(Error, InstructionNotOpen, source),
            Pe::InstructionMissingArgs(source)
               => error_tuple!(Error, InstructionMissingArgs, source),
            Pe::NoMemory(source)
               => error_tuple!(Error, NoMemory, source),
            Pe::InternalError(source)
               => error_tuple!(Error, InternalError, source),
            Pe::NoInput(source)
               => error_tuple!(Error, NoInput, source),
            Pe::None => {
               (Some("Error(None"), None, Some(")"), None)
            }
         }

         T::Warning(parse_error) => match parse_error {
            Pe::OpenInstruction(source)
               => error_tuple!(Warning, OpenInstruction, source),
            Pe::InstructionError(source)
               => error_tuple!(Warning, InstructionWarning, source),
            Pe::InstructionNotOpen(source)
               => error_tuple!(Warning, InstructionNotOpen, source),
            Pe::InstructionMissingArgs(source)
               => error_tuple!(Warning, InstructionMissingArgs, source),
            Pe::NoMemory(source)
               => error_tuple!(Warning, NoMemory, source),
            Pe::InternalError(source)
               => error_tuple!(Warning, InternalWarning, source),
            Pe::NoInput(source)
               => error_tuple!(Warning, NoInput, source),
            Pe::None => {
               (Some("Warning(None"), None, Some(")"), None)
            }
         }

         T::StateChange => (Some("StateChange"), None, None, None),
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

         if let Some(mid) = mid {
            if let Err(e) = f.write_str(&mid) {
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