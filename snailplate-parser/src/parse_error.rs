use crate::span::Span;



#[derive(Debug, Clone)]
pub struct InstructionError {
   pub pos_at: usize

   // TODO: add more fields
}



#[derive(Debug, Clone)]
pub enum ParseError {
   /// This error is returned from Tokenizer when tokenbuf is not working as
   /// expected
   /// Some cases require to reset Tokenizers num_tokens and tokenbuf Vec. This
   /// is why num_tokens, tokenbuf.len() are stored in this returned error last
   /// usizes. If current Tokenizer state values differ from those in error,
   /// it means they have been reset to avoid infinite loops.
   TokenbufBroken(Span, String, usize, usize),

   /// This error is returned when memory could not be allocated. This is
   /// highly unlikeley to happen, since most probably in such a case something
   /// might have already called panic!
   NoMemory,

   /// This error is returned when there is some bug in code. Parser/tokenizer
   /// or any other component has reached a state that is not allowed. In such
   /// a case, it should be investigated and fixes should be applied to fix it.
   InternalError,

   InstructionError(InstructionError),

   /// This error is returned when Iterator is built but no input was loaded
   /// for tokenizer.
   NoInput,

   /// Since we intend to store previous error in Tokenizer state, we need to
   /// have an initial value.
   None,
}