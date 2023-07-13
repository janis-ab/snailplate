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



/// Structure that stores the location information for something that has been
/// returned.
///
/// Since we return error, warning, etc. Tokens, it is useful to be able to
/// find, what was the source that produced this Token.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Source {
   /// This is a unique "global" position in Token stream for given Component at 
   /// the moment when something was emited/returned. The use-case for value 
   /// varies a little bit between returned ParseError types.
   ///
   /// For example, InternalError returns actual position for given component
   /// that emited this error. For InstructionError or similar use cases this
   /// coul be the position for Token to which this ParseError is related to.
   /// Future will tell.
   pub pos_zero: usize,

   /// This describes from which component the error Token was created.
   pub component: Component,

   /// This is line in code where the error Token in given component was emited
   /// from.
   ///
   /// Usually file name and component name matches, thus it is easy to find
   /// exact location.
   pub line: u32,

   /// This is a unique per component error, warning, notice, etc. code.
   ///
   /// Since line numbers change between versions, there is no reliable way of
   /// how to identify specific error in specific function between versions.
   /// This number should solve that and every time when a new error is
   /// implemented, code should be increased by 1 relative to last used code
   /// value. When being lazy, set it to 0.
   pub code: u16,
}



#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ParseError {
   /// This error is returned when memory could not be allocated. This is
   /// highly unlikeley to happen, since most probably in such a case something
   /// might have already called panic!
   NoMemory(Source),

   /// This error is returned when there is some bug in code. Parser/tokenizer
   /// or any other component has reached a state that is not allowed. In such
   /// a case, it should be investigated and fixes should be applied to fix it.
   InternalError(Source),

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