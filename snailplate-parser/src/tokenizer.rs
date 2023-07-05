use crate::{
   token::Token,
   tokenbody::TokenBody,
   span::Span,
   parse_error::ParseError,
};

mod formatter;
mod iterator;
mod ident;


// Tokenizer states.
#[derive(Debug)]
pub enum TokenizerState {
   /// This is the initial state for Tokenizer. In this state user is not
   /// allowed to invoke iterator::next, since there is no source to tokenize.
   ExpectInput,

   /// All unrecognized text by Tokenizer is left for later parsing. Thus we
   /// call those tokens Defered. Tokenizer calling code can split/transform
   /// Defered tokens into any other tokens if necessary.
   ExpectDefered,

   /// This state is active when Tokenizer has got into unrecoverable
   /// tokenization error. This can happen due to various reasons, like, bug in
   /// code, bad input, etc. Once Tokenizer is in this sate it will not recover
   /// from encountered error. It still allows to consume buffered tokens, but
   /// nothing more.
   Failed,
}



// On each @include instruction Tokenizer calling code is expected to push
// new template sources into region stack, Tokenizer must remember significant
// position values, so that when current region (stack item) is tokenized and
// Tokenizer pops back to region which had @include instruction, position values
// can be restored back so that Tokenizer can continue from the location where
// it was when @include instruction was tokenized.
//
// For example: pos_region stores position in region that was active before
// new source was pushed and had @include instruciton in the middle of string,
// tokenized string. Value of pos_region stores the position for last tokenized
// instruction and it's last tokenized char. After when include contents are
// tokenized and Tokenizer pops back, this is where Tokenizer can continue to
// scan previous file from stored location.
//
// We call this - a state snapshot, that is restored when included source region
// was parsed.
//
// Fields in general have the same meaning as for Tokenizer struct.
#[derive(Debug)]
struct StateSnap {
   pos_region: usize,
   pos_line: usize,
   line: usize,

   // this stores the index from where given origin was pushed from
   // (@include/@require). It is necessary when deeper origin has been
   // tokenized and Tokenizer must pop virtual stack.
   index: usize,
}



// Each time when some source is pushed in region Vec, we store some information
// that is useful to make errors/warnings more verbose.
// We will need to read this only in Parser code, so for now ignore warnings.
#[derive(Debug)]
#[allow(dead_code)]
struct SrcRegionMeta {
   // index for region from which this region was included from
   index: usize,

   pos_zero: usize,

   // Position to @include statement in file from which this item was included
   // It is current file relative not global offset relative.
   pos_region: usize,

   pos_line: usize,
   line: usize,

   // filename that describes this region, relative to template directory root
   // It can be None, when contents are not read from file, for example when
   // testing or generating template string for parsing.
   filename: Option<String>,
}



#[derive(Debug)]
struct TokenBuf {
   /// Number of tokens that are available inside tokenbuf. At the same time this
   /// is how many items from the tokenbuf end has to be consumed before clearing
   /// array empty.
   num_tokens: usize,

   /// This buffer stores tokens temporarily. The idea is that while tokenizer is
   /// consuming text, it can happen that it recognizes multiple tokens in one
   /// go, but iterator interface requires us to return only single item. Thus
   /// non returned items can be stored in buffer. This should make it easier to
   /// write a tokenizer.
   buf: Vec<Token>,
}


/// Tokenizer struct stores internal state for Tokenizer. Each time a new byte
/// is read, it increases pos_*, line values, once Token is recognized, those
/// values are copied into Span token that is wrapped with returned Token.
#[derive(Debug)]
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

   /// This stores current max allowed position in region.
   pos_max: usize,

   /// This is a line number in current region. Copied into Span.
   line: usize,

   /// Tokenization state
   state: TokenizerState,

   // Each item in this Vec is a bytes from template file, if there is an
   // @include or similar directive, it pushes file contents as bytes in next
   // index.
   //
   // region does not work exactly as a stack; it is append only array, where
   // new item is pushed on each @include or similar directive, but pop actually
   // restores current state to region from which @include was called.
   region: Vec<Vec<u8>>,

   /// This buffer stores tokens temporarily. The idea is that while tokenizer is
   /// consuming text, it can happen that it recognizes multiple tokens in one
   /// go, but iterator interface requires us to return only single item. Thus
   /// non returned items can be stored in buffer. This should make it easier to
   /// write a tokenizer.
   tokenbuf: TokenBuf,

   state_snap: Vec<StateSnap>,

   // Region meta is intended to be used when resolving errors, thus we can
   // leave it at the end of the struct so if struct is too big to fit in single
   // CPU cache line, it is in next line.
   region_meta: Vec<SrcRegionMeta>,

   // Previous ParseError. This is set when Tokenizer has some error. If this
   // error is InternalError, then Tokenizer should not add anyting to tokenbuf.
   // If this error is InternalError, it should never be changed to anything 
   // other (less fatal).
   parse_error_prev: ParseError,
}



impl TokenBuf {
   fn new() -> Self {
      Self {
         num_tokens: 0,
         buf: Vec::with_capacity(16)
      }
   }



   fn push(&mut self, tok: Token) -> Result<(), Token> {
      let tb = &mut self.buf;

      // Ensure that there is enough memory in Vec. This is because push will
      // panic if there is not enough capacity, but we do not want to panic on
      // that. There is an experimental API push_within_capacity, but i do not
      // want to use experimental API either.
      let cap = tb.capacity();
      let len = tb.len();
      if cap < len + 1 {
         if let Err(..) = tb.try_reserve(16) {
            return Err(Token::Fatal(ParseError::NoMemory));
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
            return Err(Token::Fatal(ParseError::InternalError));
         }
      }

      self.buf.push(tok);
      self.num_tokens += 1;

      Ok(())
   }



   // Function that takes one token out of tokenbuf.
   // It is allowed to use tokenbuf in all-in/all-out manner only.
   #[inline(always)]
   fn consume(&mut self) -> Result<Option<Token>, Token> {
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

         return Err(Token::Fatal(ParseError::InternalError));
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

         // Unfortunateley we must alter tokenbuf state, because tokenbuf_consume
         // is allowed even if Tokenizer is in Failed state. I don't expect this
         // to happen, so in case it does, will add extra debugging at that time.
         self.num_tokens = 0;
         self.buf.clear();

         return Err(Token::Fatal(ParseError::InternalError));
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

         self.buf.clear();
      }

      Ok(Some(token))
   }
}


// Tokenize signle line into two tokens: WhiteSpace and Newline and push
// results into tokenbuf.
//
// Function assumes that everything in provided region is whitespace till the
// first found 0x0A byte. Function does not analyze if provided region contains
// only whitespace characters till newline byte; it is callers responsibility.
//
// If there is no newline character found, function does not tokenize anything.
//
// This function exists just to make whitespace_into_tokenbuf modular, easier to
// read. It is not intended for reuse.
//
// # Arguments
//
// * `pos` - mutable reference to start position where whitespice tokenization
//        should be started. While iterating through bytes, function will update
//        this value. The goal is so that calling code knows how far bytes have
//        been parsed.
//
// * 'pos_line_base' - mutable reference to variable that describes position in
//        line. Once a newline character is found, this variable is set to 0.
//
// # Return
//
// * None - if there was no error while tokenizing specified region.
// * Some(Token) - returns token, that describes error.
#[inline(always)]
fn tokenizer_line_tokenize(tokenbuf: &mut TokenBuf, index: usize, src: &[u8],
   pos: &mut usize, pos_end: usize, pos_prev: &mut usize, pos_zero_base: usize,
   parsed_wsp: &mut usize, pos_line_base: &mut usize, line: &mut usize,
) -> Option<Token> {
   let mut _pos = *pos;
   let _pos_prev = *pos_prev;

   while _pos < pos_end {
      let byte = src[_pos];
      if let 0x0A = byte {
         let len_wsp = _pos - _pos_prev;

         if len_wsp > 0 {
            if let Err(token) = tokenbuf.push(Token::Real(
               TokenBody::WhiteSpace(Span {
                  index: index,
                  pos_region: _pos_prev,
                  pos_zero: pos_zero_base + *parsed_wsp,
                  pos_line: *pos_line_base,
                  line: *line,
                  length: len_wsp,
               })
            )) {
               return Some(token);
            }
         }

         if let Err(token) = tokenbuf.push(Token::Real(
            TokenBody::Newline(Span {
               index: index,
               pos_region: _pos_prev + len_wsp,
               pos_zero: pos_zero_base + *parsed_wsp + len_wsp,
               pos_line: *pos_line_base + len_wsp,
               line: *line,
               length: 1,
            })
         )) {
            return Some(token);
         }

         // While we could increase self.pos_line, this gives us no
         // benefit since noone cares how things are done on the inside
         // as long as the state from the outside looks correct.
         // This saves us some processing power.

         _pos += 1;
         *parsed_wsp += len_wsp + 1;
         *pos_prev = _pos;
         *line += 1;
         *pos_line_base = 0;

         *pos = _pos;
         return None;
      }

      _pos += 1;
   }

   // This case happens when there was no newline found in provided region. This
   // is not an expected use-case (even if it could be ignored), so we disallow
   // it with error. Sooner or later the bug will manifest, i'd rather have it
   // here.

   // TODO: In future should have better internal error location, so we can find
   // error sooner.

   Some(Token::Fatal(ParseError::InternalError))
}



// Tokenize multi-line white-space region into WhiteSpace and Newline tokens and
// push results into tokenbuf.
//
// Function assumes that every byte in provided region is whitespace. Function
// does not analyze if provided region contains only whitespace characters; it
// is callers responsibility.
//
// If region contains non-white-space characters, they are treated as if
// white-space characters. No panic call is made.
//
// This function exists just to make whitespace_into_tokenbuf modular, easier to
// read. It is not intended for reuse, thus always inline.
//
// # Argument
//
// * `pos_region` - position in src where whitespace region starts.
// * `len_region` - length in bytes for whitespace region. It must include last
//       newline byte as well.
//
// # TODO
//
// * Implement "\r\n" recognition as a newline. Now we have only "\n".
//
#[inline(always)]
fn tokenizer_whitespace_tokenize(tokenbuf: &mut TokenBuf, index: usize,
   src: &[u8], pos_zero: usize, pos_region: usize, len_region: usize,
   line_start: usize, line_end: usize, pos_line: usize, len_token: usize,
) -> Option<Token> {
   let mut num_newlines = line_end - line_start;
   let pos_end = pos_region + len_region;

   let mut pos = pos_region;
   let mut pos_prev = pos_region;
   let mut parsed_wsp = 0;
   let mut line = line_start;

   let pos_zero_base = pos_zero + len_token;
   let mut pos_line_base = pos_line + len_token;

   // Tokenize each expected newline into WhiteSpace + Newline tokens.
   while num_newlines > 0 {
      if let Some(token) = tokenizer_line_tokenize(tokenbuf, index,
         src, &mut pos, pos_end, &mut pos_prev, pos_zero_base,
         &mut parsed_wsp, &mut pos_line_base, &mut line
      ) {
         return Some(token);
      }

      num_newlines -= 1;
   }

   // Build WhiteSpace token from all that is left without any newline. If
   // pos is at pos_end, this means that last character was newline
   if pos_end != pos {
      if let Err(token) = tokenbuf.push(Token::Real(
         TokenBody::WhiteSpace(Span {
            index: index,
            pos_region: pos,
            pos_zero: pos_zero_base + parsed_wsp,
            pos_line: pos_line_base,
            line: line,
            length: pos_end - pos,
         })
      )) {
         return Some(token);
      }
   }

   None
}



impl Tokenizer {
   pub fn new() -> Self {
      Self {
         state: TokenizerState::ExpectInput,
         index: 0,
         pos_zero: 0,
         pos_region: 0,
         pos_line: 0,
         pos_max: 0,
         line: 0,
         tokenbuf: TokenBuf::new(),
         region: Vec::with_capacity(8),
         region_meta: Vec::with_capacity(8),
         state_snap: Vec::with_capacity(8),
         parse_error_prev: ParseError::None,
      }
   }



   // Each time when fatal error is returned, it is necessary to set Tokenizer
   // state to Failed, but i already keep forgetting to do that too often, thus
   // create function to resolve that and forget this forever.
   #[inline(always)]
   fn fail_token(&mut self, return_token: Token) -> Token {
      self.state = TokenizerState::Failed;
      return_token
   }



   // Function that allows us to push token into tokenbuf.
   // It is allowed to use tokenbuf in all-in/all-out manner only.
   #[inline(always)]
   fn tokenbuf_push(&mut self, tok: Token) -> Result<(), Token> {
      if let Err(token) = self.tokenbuf.push(tok) {
         return Err(self.fail_token(token));
      }

      Ok(())
   }



   // Function that takes one token out of tokenbuf.
   // It is allowed to use tokenbuf in all-in/all-out manner only.
   #[inline(always)]
   fn tokenbuf_consume(&mut self) -> Result<Option<Token>, Token> {
      #[cfg(feature = "dbg_tokenbuf_verbose")] {
         println!("Tokenbuf: consume index: {}, pos_region: {}",
            self.index, self.pos_region
         );
      }

      match self.tokenbuf.consume() {
         Ok(some_token) => Ok(some_token),
         Err(token) => {
            Err(self.fail_token(token))
         }
      }
   }



   /// Function that pushes template source into Tokenizers input vector. This
   /// should be called each time @include, @require or similar directive is
   /// handled by outer code.
   ///
   /// This function does not return any meaningful successful result token, just
   /// None, but we define this signature for easier code reuse.
   pub fn src_push(&mut self, filename: Option<&str>, buf: Vec<u8>)
      -> Result<Option<Token>, Token>
   {
      let ss = &mut self.state_snap;

      // We do not want to panic if there is not enough memory.
      let cap = ss.capacity();
      let len = ss.len();
      if cap < len + 1 {
         if let Err(..) = ss.try_reserve(8){
            return Err(self.fail_token(
               Token::Fatal(ParseError::NoMemory)
            ));
         }
      }

      ss.push(StateSnap {
         pos_region: self.pos_region,
         pos_line: self.pos_line,
         line: self.line,
         index: self.index,
      });

      let fname = if let Some(filename) = filename {
         Some(filename.to_owned())
      }
      else {
         None
      };

      let rm = &mut self.region_meta;
      let cap = rm.capacity();
      let len = rm.len();
      if cap < len + 1 {
         if let Err(..) = rm.try_reserve(16) {
            return Err(self.fail_token(
               Token::Fatal(ParseError::NoMemory)
            ));
         }
      }

      rm.push(SrcRegionMeta {
         // We always assume that the reason for src_push is current region
         // soruce/contents. Thus we can use index and other fields.
         // Technically this should be correct in most of cases. Where this
         // can go wrong is when src_push is done manually from tests or so.
         // At the moment i do not think, that it's worth to implement a 
         // special infinity/null value for those cases; but the time will
         // show.
         pos_region: self.pos_region,
         index: self.index,
         pos_zero: self.pos_zero,
         pos_line: self.pos_line,
         line: self.line,

         filename: fname,
      });

      let r = &mut self.region;
      let cap = r.capacity();
      let len = r.len();
      if cap < len + 1 {
         if let Err(..) = r.try_reserve(16) {
            return Err(self.fail_token(
               Token::Fatal(ParseError::NoMemory)
            ));
         }
      }

      self.pos_max = buf.len();

      r.push(buf);

      self.index = self.region.len() - 1;
      self.pos_region = 0;
      self.pos_line = 0;
      self.line = 0;

      // Change mode only if there was no input. Otherwise whoever appended
      // input is responsible to manage tokenizer state. This is by design so,
      // because different situations can require different behavior.
      if let TokenizerState::ExpectInput = self.state {
         self.state = TokenizerState::ExpectDefered;
      }

      Ok(None)
   }



   #[inline(always)]
   fn defered_tokenize(&mut self) -> Option<Token> {
      let src = &self.region[self.index];
      let pos_start = self.pos_region;
      let pos_max = src.len();
      let line_start = self.line;

      // TODO: here we should iterate through each character and change states
      // for Tokenizer. No matter how tokenization goes while iterating, there
      // always will be some chars that are left unmatched, since in loop we
      // can not know what is the expected token type unless all the necessary
      // data is available.

      //
      // Code below deals with correct leftover handling. It either returns
      // last available bytes as Defered or consumes current region and does
      // state_snap.pop().
      //

      if pos_start < pos_max {
         let len_defered = pos_max - pos_start;

         let token = Token::Real(TokenBody::Defered(Span {
            index: self.index,
            pos_region: pos_start,
            pos_zero: self.pos_zero,
            pos_line: self.pos_line,
            line: line_start,
            length: len_defered
         }));

         return self.return_tokenized(token);
      }

      None
   }



   // Tokenize multi-line white-space region into WhiteSpace and Newline tokens and
   // push results into tokenbuf.
   //
   // Intended to be used for template instruction parsing when there is a
   // white-space region after instruction name. Normally Tokenizer will handle
   // WhiteSpace and Newline tokens by other means, but when an instruction is
   // started with @ byte, it is not possible to tell if this will be a valid
   // instruction. Since we expect errors in source, recognize them and warn
   // user, we need to be able to parse whitespace between @instruction and
   // open parenthesis. This function is intended for that.
   //
   // Why don't we return WhiteSpace tokens when they are met? Because when
   // tokenizing template instruction we don't know if it is instruction untill
   // all significant bytes have matched. Maybe it is not an instruction and
   // user has forgotten to escape @ symbol, so in that case we would emit
   // diferent warning/error message to user.
   //
   // This function should never change internal positions for tokenizer,
   // because positions are updated when tokens are really tokenized/returned.
   // At that time integrity is tested as well. So all positions that are
   // necessary to fulfil this function must be kept locally.
   //
   // This function does not check if provided region is really white-space
   // only, it splits everything on newline characters and reports anything else
   // as a WhiteSpace token.
   //
   // If bad arguments are provided, this function will return bad results, no
   // error checking is done. If arguments are out of range, it will panic.
   //
   // # Arguments
   //
   // * `len_region` - This is the length in bytes for region that has to be
   //       tokenized as WhiteSpace and Newline Tokens.
   //
   // * `line_start` - This is the line number for first character that is at
   //       position for whitespace region. Since it is not already stored in
   //       self.line (because it changes only when token is returned), but is
   //       known by calling code, since we can optimize on this, we take it as
   //       argument.
   //
   // * `line_end` - This is the line number that follows last character in
   //    given whitespace region. If it matches the line_start, we use fast-path
   //    for parsing.
   //
   // # Return
   //    Some(Token) - Error-token if any.
   //    None - On success.
   #[inline(always)]
   fn whitespace_into_tokenbuf(&mut self, index: usize,
      pos_region: usize, len_region: usize, line_start: usize, line_end: usize,
   )
      -> Option<Token>
   {
      // When there is no data to be tokenized as WhiteSpace, there is nothing
      // to be pushed, thus it is not an error, just zero Tokens pushed.
      if len_region < 1 {
         return None;
      }

      // This is a length for some token or tokens that have been parsed.
      let len_token = pos_region - self.pos_region;

      // This is a fast-path for cases when whitespace region does not contain
      // any newline char.
      if line_start == line_end {
         if let Err(token) = self.tokenbuf_push(Token::Real(
            TokenBody::WhiteSpace(Span {
               index: index,
               pos_region: pos_region,
               pos_zero: self.pos_zero + len_token,
               pos_line: self.pos_line + len_token,
               line: line_start,
               length: len_region,
            })
         )) {
            return Some(token);
         }

         // TODO: we should buffer warning tokens here as well, since
         // whitespaces after instruction names are discouraged.

         return None;
      }

      //
      // Being here means that line_start != line_end
      //

      let src = &self.region[index];
      let mut tokenbuf = &mut self.tokenbuf;
      tokenizer_whitespace_tokenize(&mut tokenbuf, index, &src, self.pos_zero,
         pos_region, len_region, line_start, line_end, self.pos_line, len_token
      )
   }



   // Since every time when we return token, we must update Tokenizer positions,
   // it is better to have a function that does that for us, so that we do not
   // forget to update some fields.
   //
   // TODO: currently this function is not capable to correctly handle Tokens
   // that span over multiple regions. Maybe that can be implemented later
   // with some feature flags to enable that type of behavior, but currently
   // that seems like an unnecessary code complexity.
   #[inline(always)]
   fn return_tokenized(&mut self, tok: Token) -> Option<Token> {
      // Only tokens that have Span do update Tokenizers current postion
      // variable values.
      // TODO: we could implement tok.span_borrow that works more efficiently
      // and use it here instead of span_clone.
      if let Some(span) = tok.span_clone() {
         let pos_region = span.pos_region;
         let pos_zero = span.pos_zero;
         let pos_line = span.pos_line;
         let line = span.line;

         #[cfg(feature = "dbg_tokenizer_verbose")]{
            println!("INFO(Tokenizer): return_tokenized: {:?}", tok);
         }

         #[cfg(not(feature = "unguarded_tokenizer_integrity"))] {
            if self.index != span.index {
               #[cfg(feature = "dbg_tokenizer_verbose")]{
                  println!("ERROR: Tokenizer index mismatch Token positions.");
               }

               // We can not be sure that tokenbuf_consume or any other function
               // calls return_tokenized, but we want to preserve returned token
               // that caused the error in buffer, but in such a case we could
               // get into infinite loop, which we do not want. This guards
               // against that iif calling function does not change
               // parse_error_prev arbitrarily.
               if let ParseError::InternalError = self.parse_error_prev { }
               else {
                  // We are already failing, if this fails as well, there is
                  // nothing we can do.
                  #[allow(unused_must_use)] {
                     self.tokenbuf_push(tok);
                  }
                  self.parse_error_prev = ParseError::InternalError;
               }

               return Some(self.fail_token(Token::Fatal(
                  ParseError::InternalError
               )));
            }

            // "Next" returned token must always start at the position where
            // Tokenizer is at. It is not allowed to have gaps. If we need gaps,
            // then we should implement token with special body type "Skip" or
            // maybe use Phantom token. While technically there is no problem
            // with skipping some bytes, i think that enforcing this will avoid
            // us from having hard to detect errors.
            if (self.pos_region != pos_region)
            || (self.pos_line != pos_line)
            || (self.pos_zero != pos_zero)
            || (self.line != line)
            {
               #[cfg(feature = "dbg_tokenizer_verbose")]{
                  println!("ERROR: Tokenizer positions mismatch Token positions.");
                  println!("    {:?}", self);
                  println!("    Token::{:?}", tok);
               }

               if let ParseError::InternalError = self.parse_error_prev { }
               else {
                  // We are already failing, if this fails as well, there is
                  // nothing we can do.
                  #[allow(unused_must_use)] {
                     self.tokenbuf_push(tok);
                  }
                  self.parse_error_prev = ParseError::InternalError;
               }

               return Some(self.fail_token(Token::Fatal(
                  ParseError::InternalError
               )));
            }
         }
         let len_token = span.length;

         // While here we could increase each property by len_token, i believe
         // this is more explicit and maybe "faster" since variables might be in
         // CPU registers.
         self.pos_region = pos_region + len_token;
         self.pos_zero = pos_zero + len_token;
         self.pos_line = pos_line + len_token;

         self.line = line;
         match tok {
            Token::Real(body)
            | Token::Phantom(body) => {
               if let TokenBody::Newline(..) = body {
                  self.line = line + 1;
                  self.pos_line = 0;
               }
            }
            _ => { }
         }

         // If token span goes over multiple regions, this case can happen.
         // We do not want to allow it for now, since it would mess up Tokenizer
         // interna state.
         if self.pos_region > self.pos_max {
            self.pos_region = self.pos_max;
            if let ParseError::InternalError = self.parse_error_prev { }
            else {
               // We are already failing, if this fails as well, there is
               // nothing we can do.
               #[allow(unused_must_use)] {
                  self.tokenbuf_push(tok);
               }
               self.parse_error_prev = ParseError::InternalError;
            }

            return Some(self.fail_token(Token::Fatal(
               ParseError::InternalError
            )));
         }

         // If this was last token in current region.
         if self.pos_max == self.pos_region {
            // If this is the "root" region, there is no place to fall back.
            if self.index == 0 {
               return Some(tok);
            }

            if let Some(snap) = self.state_snap.pop(){
               self.index -= 1;
               self.pos_region = snap.pos_region;
               self.pos_line = snap.pos_line;
               self.line = snap.line;

               let src = &self.region[self.index];
               self.pos_max = src.len();

               #[cfg(not(feature = "unguarded_tokenizer_integrity"))] {
                  if self.index != snap.index {
                     if let ParseError::InternalError = self.parse_error_prev { }
                     else {
                        // We are already failing, if this fails as well, there
                        // is nothing we can do.
                        #[allow(unused_must_use)] {
                           self.tokenbuf_push(tok);
                        }
                        self.parse_error_prev = ParseError::InternalError;
                     }

                     return Some(self.fail_token(Token::Fatal(
                        ParseError::InternalError
                     )));
                  }
               }

               return Some(tok);
            }
            else {
               if let ParseError::InternalError = self.parse_error_prev { }
               else {
                  // We are already failing, if this fails as well, there is
                  // nothing we can do.
                  #[allow(unused_must_use)] {
                     self.tokenbuf_push(tok);
                  }
                  self.parse_error_prev = ParseError::InternalError;
               }

               return Some(self.fail_token(Token::Fatal(
                  ParseError::InternalError
               )));
            }
         }
      }

      Some(tok)
   }



   // It is not allowed to print anything in this function because it will be
   // used from SpanFormatter trait impl, that will be called from
   // std::fmt::Debug. This caught me by surprise once. It seems that
   // println! writes in the same buffer as fmt::Debug? Maybe they both use
   // stdout? But the weird part is that output texts overlay.
   pub fn span_slice<'a>(&'a self, span: &'a Span) -> Option<&'a [u8]> {
      // Someone has given us wrong Span. It is impossible to trigger
      // this error unless Span was constructed manually or there is a
      // bug in code.
      if let TokenizerState::ExpectInput = self.state {
         return None;
      }

      // Wrong Span (out of bounds).
      if span.index > self.region.len() - 1 {
         return None;
      }

      let src = &self.region[span.index];
      let inf = src.len() + 1;

      // This implicitly checks for "span.src_pos >= inf" as well.
      if span.pos_region + span.length >= inf {
         return None;
      }

      let start = span.pos_region;
      let end = span.pos_region + span.length;

      Some(&src[start..end])
   }
}



// Tests if tokenizer returns tokens that match given list.
//
// This function is only necessary for testing, thus it is not even visible
// in library unless tests are compiled.
//
// # Arguments
//
// * `list` - What tokens are expected to be returned from current Tokenizer.
//       This is slice, because more complex testing requires us to test only 
//       subset of given list at a time.
//
// * `unbuffered` - For some tests we do not want to allow Tokenizer to tokenize 
//       input string, but only return pre-prepared Tokens from tokenbuf. Thus
//       we set this to true, when Tokenizer is allowed to return as much tokens
//       as it wants. Set this to false, when Tokenizer is allowed to only 
//       return tokens from tokenbuf.
//
// # Return
//
// * Ok(()) - when comparison was successful. All tokens matched.
//
// * Err((expected token, returned token)) - this tuple then can be used to
//       understand what went wrong.
//
#[cfg(all(test, not(feature = "tokenlist_match_or_fail_print_only")))]
fn tokenlist_match_or_fail(t: &mut Tokenizer, list: &[Token], allow_unbuffered: bool)
   -> Result<(), (Option<Token>, Option<Token>)>
{
   let mut idx = 0;

   // This index is out of bounds in relative measure to expected list.
   let idx_oob = list.len();

   // This is a tricky loop, because it must be able to detect if there are
   // enough items in buffer, if Token consumption is limited, then no more
   // items can be consumed than allowed.
   while let Some(token) = t.next() {
      // If tokenizer returns more items than are in expected item buffer,
      // we must error out. This must be done at iteration start.
      if idx >= idx_oob {
         return Err((None, Some(token)));
      }

      // If there are expected items, compare if they match.
      if let Some(expect) = list.get(idx) {
         // println!("expected: {:?}, at idx: {}", expect, idx);
         if *expect != token {
            return Err((Some((*expect).clone()), Some(token)));
         }
      }
      else {
         return Err((None, Some(token)));
      }

      idx += 1;

      // When only tokenbuf must be tested, this is a way to constrain
      // tokenizer. This must be at the end of the iteration, before next token
      // is consumed, otherwize Tokenizer would build token from source.
      if !allow_unbuffered {
         if t.tokenbuf.buf.len() < 1 {
            break;
         }
      }

      // Being here means that Token comparison succeeded.
   }

   // Tokenizer returned less Tokens than expected.
   if idx < idx_oob {
      if let Some(expect) = list.get(idx) {
         return Err((Some((*expect).clone()), None));
      }
   }

   Ok(())
}



// When we do not want to really test by invoking tokenlist_match_or_fail, but
// print returned tokens on screen instead. This is helpful when developing
// tests.
//
// All parameters and meaning is the same as for real tokenlist_match_or_fail.
#[cfg(all(test, feature = "tokenlist_match_or_fail_print_only"))]
fn tokenlist_match_or_fail(t: &mut Tokenizer, _: &[Token], allow_unbuffered: bool)
   -> Result<(), (Option<Token>, Option<Token>)>
{
   while let Some(token) = t.next() {
      println!("{:?}", token);
      if !allow_unbuffered {
         if t.tokenbuf.buf.len() < 1 {
            break;
         }
      }
   }

   Ok(())
}



#[cfg(test)]
mod test;

#[cfg(test)]
mod test_span;

#[cfg(test)]
mod test_iterator;

#[cfg(test)]
mod test_ident;



// ================== EOF: do not write below this ============================
