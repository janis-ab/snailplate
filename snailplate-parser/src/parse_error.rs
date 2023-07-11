#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Component {
   Tokenizer,
   TokenBuf,
}



#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InstructionError {
   // This is unique "global" position in token stream for @instruction token
   // that has not been satisfied by required conditions.
   pub pos_zero: usize

   // TODO: add more fields
}



#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InternalError {
   pub component: Component,
   pub line: u32,
}



#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ParseError {
   /// This error is returned when memory could not be allocated. This is
   /// highly unlikeley to happen, since most probably in such a case something
   /// might have already called panic!
   NoMemory,

   /// This error is returned when there is some bug in code. Parser/tokenizer
   /// or any other component has reached a state that is not allowed. In such
   /// a case, it should be investigated and fixes should be applied to fix it.
   InternalError(InternalError),

   InstructionError(InstructionError),

   /// This error is returned, when instruction is opened, but not closed, i.e.
   /// "@include(".
   OpenInstruction(InstructionError),

   /// This error is returned when Iterator is built but no input was loaded
   /// for tokenizer.
   NoInput,

   /// Since we intend to store previous error in Tokenizer state, we need to
   /// have an initial value.
   None,
}