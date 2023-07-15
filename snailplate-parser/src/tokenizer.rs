use crate::{
   token::Token,
   tokenbody::TokenBody,
   tokenbuf::TokenBuf,
   span::Span,
   parse_error::{
      ParseError,
      Source,
      Component,
   }
};

mod formatter;
mod iterator;
mod ident;

use ident::{Ident, ident_match};

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

   /// TODO: Tokenizer switches to this state, when instruction start with open
   /// parenthesis has been tokenized, i.e. "@include(", "@if(", etc.
   ExpectInstructionClose,

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

   /// Count for open parenthesis, when instruction is being tokenized.
   cnt_openparen: u32,

   // Count for closing parenthesis, when instruction is being tokenized.
   cnt_closeparen: u32,

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

   // pos_zero for previously handled instruction. At the moment the use-case 
   // for this is to allow generating error tokens regarding instructions.
   pos_zero_prev_instr: usize,
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
            let wsp_token = if *pos_line_base == 0 {
               // If this is the only white space, it uses whole line.
               Token::Real(TokenBody::WhiteSpaceWhole(Span {
                     index: index,
                     pos_region: _pos_prev,
                     pos_zero: pos_zero_base + *parsed_wsp,
                     pos_line: *pos_line_base,
                     line: *line,
                     length: len_wsp,
               }))
            }
            else {
               // If there is some "text" before, this is a trailing whitespace
               // no matter what position. Whitespaces between text would not
               // match 0x0A byte at this point.

               Token::Real(TokenBody::WhiteSpaceTr(Span {
                     index: index,
                     pos_region: _pos_prev,
                     pos_zero: pos_zero_base + *parsed_wsp,
                     pos_line: *pos_line_base,
                     line: *line,
                     length: len_wsp,
               }))
            };

            if let Err(token) = tokenbuf.append(wsp_token) {
               return Some(token);
            }
         }

         if let Err(token) = tokenbuf.append(Token::Real(
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

   Some(Token::Fatal(ParseError::InternalError(Source {
      component: Component::Tokenizer,
      line: line!(),
      code: 0,
      pos_zero: _pos,
   })))
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
      let pos_max = src.len();

      // If there is more "text" following this whitespace, then it is a leading
      // whitespace.
      let wsp_token = if pos_max > pos_end {
         Token::Real(TokenBody::WhiteSpaceLd(Span {
            index: index,
            pos_region: pos,
            pos_zero: pos_zero_base + parsed_wsp,
            pos_line: pos_line_base,
            line: line,
            length: pos_end - pos,
         }))
      }
      // If this is the last whitespace in region, it is just a whitespace.
      else {
         Token::Real(TokenBody::WhiteSpace(Span {
            index: index,
            pos_region: pos,
            pos_zero: pos_zero_base + parsed_wsp,
            pos_line: pos_line_base,
            line: line,
            length: pos_end - pos,
         }))
      }
      ;

      if let Err(token) = tokenbuf.append(wsp_token) {
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
         cnt_openparen: 0,
         cnt_closeparen: 0,
         region_meta: Vec::with_capacity(8),
         state_snap: Vec::with_capacity(8),
         parse_error_prev: ParseError::None,
         pos_zero_prev_instr: 0,
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
      if let Err(token) = self.tokenbuf.append(tok) {
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

      match self.tokenbuf.popleft() {
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
               Token::Fatal(ParseError::NoMemory(Source {
                  pos_zero: self.pos_zero,
                  component: Component::Tokenizer,
                  line: line!(),
                  code: 0,
               }))
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
               Token::Fatal(ParseError::NoMemory(Source {
                  pos_zero: self.pos_zero,
                  component: Component::Tokenizer,
                  line: line!(),
                  code: 0,
               }))
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
               Token::Fatal(ParseError::NoMemory(Source {
                  pos_zero: self.pos_zero,
                  component: Component::Tokenizer,
                  line: line!(),
                  code: 0,
               }))
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

      let pos_line_start = self.pos_region - self.pos_line;
      let pos_token_start = self.pos_region;

      let line = self.line;
      let mut pos = pos_start;
      while pos < pos_max {
         match src[pos] {
            0x0A /* newline */ => {
               // In a way we do not care if there is carriage return or not,
               // since we just need to count lines. Well... if there are some
               // problems iwth file and some lines are ended with "\r\n" some
               // with "\n", we can not detect it. But should we?

               let pos_in_line = pos_token_start - pos_line_start;
               let len_defered = pos - pos_token_start;
               let len_prev_token = pos_token_start - self.pos_region;

               if let Err(token) = self.tokenbuf.append(Token::Real(
                  TokenBody::Newline(Span {
                     index: self.index,
                     pos_region: pos,
                     pos_zero: self.pos_zero + len_prev_token + len_defered,
                     pos_line: pos_in_line + len_defered,
                     line: line,
                     length: 1,
                  })
               )){
                  return Some(token);
               };

               return self.return_tokenized(Token::Real(TokenBody::Defered(
                  Span {
                     index: self.index,
                     pos_region: pos_token_start,
                     pos_zero: self.pos_zero + len_prev_token,
                     pos_line: pos_in_line,
                     line: line,
                     length: len_defered,
                  }
               )));
            }

            0x40 /* @ */ => {
               return self.instruction_tokenize(pos, pos_start, pos_max, line, line);
            }

            _ch => {
               #[cfg(feature = "dbg_tokenizer_verbose")]{
                  println!("non-special char pos: {}, char: 0x{:02X}, do nothing", pos, _ch);
               }
            }
         }

         pos += 1;
      }

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
            line: line,
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
         // This Token always will be WhiteSpace, because by definition this
         // function is called only between instruction name and parenthesis and
         // if there is no newline, it's just a WhiteSpace.

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

         if let Err(token) = self.tokenbuf_push(Token::Warning(
               ParseError::UnwantedWhiteSpace ( Source {
                  pos_zero: self.pos_zero + len_token,
                  component: Component::Tokenizer,
                  line: line!(),
                  code: 2,
               })
            )) {
            return Some(token);
         }

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



   // This function is intended to parse @instructions. Param line_at at this point
   // is a line number for @ symbol, while line_start is a line number where
   // deref token might be starting. It is possible that line_at == line_start.
   #[inline(always)]
   fn instruction_tokenize(&mut self,
      pos_at: usize, pos_start: usize, pos_max: usize, line_at: usize, line_start: usize
   )
      -> Option<Token>
   {
      let src = &self.region[self.index];
      let inf = pos_max + 1; // virtual infinity

      // This is just a guard for possible development bugs to be caught.
      #[cfg(feature = "tokenizer_integrity_guard")] {
         if src[pos_at] != 0x40 {
            match self.fail_result_option_token(
               Token::Fatal(ParseError::InternalError(Source {
                  pos_zero: self.pos_zero,
                  component: Component::Tokenizer,
                  line: line!(),
                  code: 0,
               }))
            ) {
               Ok(tok) => {
                  return tok;
               }
               Err(tok) => {
                  return Some(tok)
               }
            }
         }
      }

      // Since pos was pointing to @ symbol when this function is called. Move
      // position one unit forward.
      let mut pos = pos_at + 1;
      let mut line = line_at;

      let mut pos_first_char = inf;
      let mut pos_last_char = inf;
      let mut pos_close_paren = inf;
      let mut pos_open_paren = inf;
      let mut pos_first_bad_char = inf;
      let mut pos_last_bad_char = inf;
      // whitespace before any meaningful character
      let mut pos_pre_whitespace_start = inf;
      let mut pos_pre_whitespace_end = inf;
      // whitespace after any meaningful character
      let mut pos_post_whitespace_start = inf;
      let mut pos_post_whitespace_end = inf;

      // Normally parenthesis is at the same line where @ is. In case when open
      // parenthesis is not found, pos_open_pare will be infinity.
      let mut line_open_paren = line_at;

      let mut pos_last_linestart = pos_start;

      // At first we try to match all possible characters as instruction name.
      // Yes, this is slower than targeting to matching exact instruction
      // names, but this gives us ability to detect mistyped instruction names.
      // For example, user wrote @niclude(filename) instead of @include(filename),
      // If we match all characters, we can program some logic to decide to
      // print warning with suggestion, that maybe there is an error.
      //
      // What we will not tolerate though is space after @ symbol and error
      // in instruction name. While it could be nice to detect errors and
      // output suggestions in case of phrase "@ niclude (..", it is just
      // too much; being so tolerant most probably would introduce too many
      // false positives, since it is possible to have an @ symbol for other
      // use cases. Though we could warn, that @ symbol has to be escaped as
      // @@.
      while pos < pos_max {
         match src[pos] {
            0x0A /* newline */ | 0x20 /* space */ | 0x09 /* tab */ => {
               if pos_pre_whitespace_start == inf {
                  pos_pre_whitespace_start = pos;
               }

               // While there are no characters matched, consider this char
               // to belong to pre-whitespace token.
               if pos_first_char == inf {
                  pos_pre_whitespace_end = pos;
               }
               else {
                  if pos_post_whitespace_start == inf {
                     pos_post_whitespace_start = pos;
                  }

                  pos_post_whitespace_end = pos;
               }

               // Only newline characters increase line number, but we want to
               // reuse code for whitespace calculations.
               if src[pos] == 0x0A {
                  line += 1;
                  // line starts after this character/byte.
                  pos_last_linestart = pos + 1;
               }
            }

            // This is generally the case we are aiming for. Instruction and
            // open parenthesis.
            0x28 /* ( */ => {
               pos_open_paren = pos;
               line_open_paren = line;

               if pos_open_paren < pos_close_paren {
                  return self.instruction_tokenize_correct_paren(pos_at,
                     pos_start, pos_max, inf, pos_first_char, pos_last_char,
                     pos_close_paren, pos_open_paren, pos_first_bad_char,
                     pos_last_bad_char, line_at, line_start, line_open_paren,
                     pos_last_linestart
                  )
               }
               else {
                  return self.instruction_tokenize_bad_paren(pos_at, pos_start, pos_max, inf,
                     pos_first_char, pos_last_char, pos_close_paren, pos_open_paren,
                     pos_first_bad_char, pos_last_bad_char
                  )
               }
            }

            0x29 /* ) */ => {
               pos_close_paren = pos;
            }

            0x41..=0x5A /* A-Z */ |
            0x61..=0x7A /* a-z */ => {
               if pos_first_char == inf {
                  #[cfg(feature = "dbg_tokenizer_verbose")]{
                     println!("got first char at: {}", pos);
                  }
                  pos_first_char = pos;
               }

               pos_last_char = pos;
            }

            // Template instructions can be composed only of ascii characters,
            // if there are any other characters, this means that it is not a
            // valid instruction. Here we have various paths of possible action:
            // 1) set tokenizer in failed state, and forbid characters
            //    completeley. This is not flexible.
            // 2) buffer some warnings regarding situation and return all contents
            //    as defered token, since it is definiteley not an instruction.
            //    This is flexible, but gives up possibly too early.
            // 3) buffer some warnings and continue trying to match instruction.
            //    This is flexible, but harder to implement, since we must
            //    decide when to give up. This is the most friendly action path
            //    from users perspective.
            //
            // We will try to walk path 3.
            chr => {
               println!("bad char? 0x{:02X}, line_open_paren: {}", chr, line_open_paren);
               if pos_first_bad_char == inf {
                  pos_first_bad_char = pos;
               }

               pos_last_bad_char = pos;
            }
         }

         pos += 1;
      }

      // If instruction was correctly written, Tokenizer should not be here, but
      // in case if it is, give state data to instruction_tokenize_unfinished
      // function to do deeper analysis and output useful tokens for error
      // messages.
      self.instruction_tokenize_unfinished(pos_at, pos_start, pos_max, inf,
         pos_first_char, pos_last_char, pos_close_paren, pos_open_paren,
         pos_first_bad_char, pos_last_bad_char, pos_pre_whitespace_start,
         pos_pre_whitespace_end, pos_post_whitespace_start,
         pos_post_whitespace_end
      )
   }



   // This function is intended to be called only from instruction_tokenize
   // when instruction was not matched and enough characters were not collected
   // to do meaningful analysis. We use separate function so that code length
   // per function is reasonable (not too long).
   // This call should happen if:
   // 1) user has forgotten to write open parenthesis, thus whole template
   //    source is scanned.
   // 2) user has started instruction at the end of a file, but did not finish
   //    it completeley, for example template ends with "@inclu" and nothing
   //    follows.
   #[inline(always)]
   #[allow(unused_variables)]
   fn instruction_tokenize_unfinished(&mut self,
      pos_at: usize, pos_start: usize, pos_max: usize, inf: usize,
      pos_first_char: usize, pos_last_char: usize, pos_close_paren: usize,
      pos_open_paren: usize, pos_first_bad_char: usize,
      pos_last_bad_char: usize, pos_pre_whitespace_start: usize,
      pos_pre_whitespace_end: usize, pos_post_whitespace_start: usize,
      pos_post_whitespace_end: usize
   ) -> Option<Token>{

      // TODO: insert warning tokens with user friendly messages and suggestions
      // into tokenbuf.

      self.return_tokenized(Token::Real(TokenBody::Defered(Span {
         index: self.index, line: self.line, pos_line: self.pos_line,
         pos_region: self.pos_region, pos_zero: self.pos_zero,
         length: pos_max - pos_start
      })))
   }



   // This function is intended to handle instruction parsing phase, where
   // parenthesis are in wrong order. There could be various reasons for this
   // thus analysis are not easy.
   #[inline(always)]
   #[allow(unused_variables)]
   fn instruction_tokenize_bad_paren(&mut self,
      pos_at: usize, pos_start: usize, pos_max: usize, inf: usize,
      pos_first_char: usize, pos_last_char: usize, pos_close_paren: usize,
      pos_open_paren: usize, pos_first_bad_char: usize,
      pos_last_bad_char: usize
   ) -> Option<Token>{
      // TODO: give friendlier error notifications in this case. For now i must
      // move on with higher-priority tasks and leave this for later.

      // In a way related to DD-2023-07-07-01, we return UnespacedAt. We do not
      // fail, due to DD-2023-07-09-01.

      // TODO: if open paren is after close paren, then something
      // is wrong with parenthesis, handle that case better. I.e. analyze if
      // atleast instruction name is correct, parenthesis positioning, maybe can
      // even give an advice how to rearange parenthesis, or where to place
      // some.

      let len_left = pos_start - self.pos_region;
      let pos_zero = self.pos_zero + len_left;

      if let Err(token) = self.tokenbuf_push(Token::Error(
         ParseError::InstructionError(Source {
            pos_zero: pos_zero,
            component: Component::Tokenizer,
            line: line!(),
            code: 0,
         })
      )){
         return Some(token);
      }

      self.return_tokenized(Token::Real(TokenBody::UnescapedAt(Span {
         index: self.index, length: 1, pos_region: self.pos_region,
         pos_line: self.pos_line, pos_zero: self.pos_zero, line: self.line
      })))
   }



   // This case is called, when possible instruction is matched, thus we have
   // something like "@include(" or "@include   (", "@nonexistentinstruction("
   // but atleast is has a pattern "@..(", so we can try to recognize if it is
   // correct instruction, or instruction with error, or forgotten escape.
   #[inline(always)]
   #[allow(unused_variables)]
   fn instruction_tokenize_correct_paren(&mut self,
      pos_at: usize, pos_start: usize, pos_max: usize, inf: usize,
      pos_first_char: usize, pos_last_char: usize, pos_close_paren: usize,
      pos_open_paren: usize, pos_first_bad_char: usize,
      pos_last_bad_char: usize, line_at: usize, line_start: usize,
      line_open_paren: usize, pos_last_linestart: usize
   ) -> Option<Token>{
      use Ident as I;

      let len_left = pos_start - self.pos_region;
      let pos_zero = self.pos_zero + len_left;

      #[cfg(not(feature = "unguarded_tokenizer_integrity"))] {
         if pos_last_char < pos_first_char {
            return Some(self.fail_token(Token::Fatal(ParseError::InstructionError(
               Source {
                  pos_zero: pos_zero,
                  component: Component::Tokenizer,
                  line: line!(),
                  code: 0,
               }
            ))));
         }

         if pos_first_char <= pos_at {
            return Some(self.fail_token(Token::Fatal(ParseError::InstructionError(
               Source {
                  pos_zero: pos_zero,
                  component: Component::Tokenizer,
                  line: line!(),
                  code: 0,
               }
            ))));
         }
      }

      let src = &self.region[self.index];
      #[cfg(feature = "dbg_tokenizer_verbose")]{
         println!("got open ( at pos_at: {}", pos_at);
      }

      if (pos_at + 1) == pos_first_char {
         match ident_match(src, pos_first_char, pos_last_char) {
            I::Include(ident_pos_start, ident_pos_end) => {
               #[cfg(feature = "dbg_tokenizer_verbose")]{
                  println!("got @include {}, {}", ident_pos_start, ident_pos_end);
               }

               // If pos_at is somewhere further than Tokenizers pos_start, this
               // means that there is a defered token that must be returned before
               // instrution token is. This behavior is necessary so that in case
               // if instruction is not matched, everything is returned as defered
               // token. Such behavior us sueful in cases when there is unescaped
               // @ symbol.
               if pos_at > pos_start {
                  // In this case buffer @include and coupled whitespace,
                  // parenthesis and return defered token instead.
                  // Buffer all tokens that were matched regarding this instruction.
                  self.instruction_tokenize_correct_paren_defered(pos_at, pos_start,
                     pos_open_paren, ident_pos_end, line_at, line_start,
                     line_open_paren, pos_last_linestart
                  )
               }
               else {
                  // In this case buffer whitespace after @include and parenthesis,
                  // but return @include token right away.
                  self.instruction_tokenize_correct_paren_now(pos_at, pos_start,
                     pos_open_paren, ident_pos_end, line_at, line_start,
                     line_open_paren, pos_last_linestart
                  )
               }
            }
            I::None => {
               None
            }
         }
      }
      else {
         // Being here means, that there are some spaces between instruciton and
         // at symbol. We do not allow such use cases, but what we can do is,
         // inform uset about such cases when we have detected that following
         // word is correct instruction.

         self.instruction_tokenize_whitespace_before_instruction(pos_at,
            pos_start, pos_max, inf, pos_first_char, pos_last_char,
            pos_close_paren, pos_open_paren, pos_first_bad_char,
            pos_last_bad_char, line_at, line_start, line_open_paren,
            pos_last_linestart
         )
      }
   }



   // Function that does not return @include or any other instruction token
   // coupled with parenthesis right away, because there is Defered token
   // that has to be returned before. This is just to split code in more
   // manageable chunks.
   #[inline(always)]
   fn instruction_tokenize_correct_paren_defered(&mut self,
      pos_at: usize, pos_start: usize, pos_open_paren: usize,
      ident_pos_end: usize, line_at: usize, line_start: usize,
      line_open_paren: usize, pos_last_linestart: usize
   )
      -> Option<Token>
   {
      #[cfg(not(feature = "unguarded_tokenizer_integrity"))] {
         let len_left = pos_start - self.pos_region;
         let pos_zero = self.pos_zero + len_left;

         if line_at != line_start {
            return Some(self.fail_token(Token::Fatal(
               ParseError::InstructionError(Source {
                  pos_zero: pos_zero,
                  component: Component::Tokenizer,
                  line: line!(),
                  code: 0,
               }
            ))));
         }

         // It should be that this function is called with Tokenizer position
         // at location where defered token is.
         if pos_start != self.pos_region {
            return Some(self.fail_token(Token::Fatal(
               ParseError::InstructionError(Source {
                  pos_zero: pos_zero,
                  component: Component::Tokenizer,
                  line: line!(),
                  code: 0,
               }
            ))));
         }
      }

      // Since we need to calculate pos_zero, pos_line we need to know
      // how much bytes shall be returned by defered token.
      let len_defered = pos_at - pos_start;
      let mut pos_next_span = pos_start + len_defered;
      let len_ident = ident_pos_end - pos_at + 1;
      let mut len_to_span = len_defered;

      if let Err(token) = self.tokenbuf_push(Token::Real(TokenBody::Include(
         Span {
            index: self.index,
            pos_region: pos_at,
            pos_zero: self.pos_zero + len_to_span,
            pos_line: self.pos_line + len_to_span,
            line: line_at,
            length: len_ident,
         }
      ))){
         return Some(token);
      }

      pos_next_span += len_ident;
      len_to_span += len_ident;

      let len_whitespace = pos_open_paren - ident_pos_end - 1;
      if len_whitespace > 0 {
         if let Err(token) = self.tokenbuf_push(Token::Real(
            TokenBody::WhiteSpace(Span {
               index: self.index,
               pos_region: pos_next_span,
               pos_zero: self.pos_zero + len_to_span,
               pos_line: self.pos_line + len_to_span,
               line: line_at, // TODO: is this correct?
               length: len_whitespace,
            })
         )){
            return Some(token);
         }

         pos_next_span += len_whitespace;
         len_to_span += len_whitespace;
      }

      let pos_line = pos_open_paren - pos_last_linestart;
      // When instruction is matched, we know that there is open parenthesis available.
      if let Err(token) = self.tokenbuf_push(Token::Real(
         TokenBody::OpenParen(Span {
            index: self.index,
            pos_region: pos_next_span,
            pos_zero: self.pos_zero + len_to_span,
            pos_line: pos_line,
            line: line_open_paren,
            length: 1
         })
      )){
         return Some(token);
      }

      // After tokens shall be consumed, further tokenization has to be
      // in state ExpectDefered.

      // At this point parenthesis have been already been parsed and
      // buffered. By default Tokenizer does not know the context of
      // these parenthesis, thus it swithces to default mode. The
      // calling code can change mode as necessary.
      // For example, @include instruction would require to parse
      // contents as file path, @if would require to parse code as
      // conditional, etc. At some instances maybe it is even
      // forbidden to parse matched instruction in any special way.
      self.state = TokenizerState::ExpectDefered;

      // Return defered token and allow further calls to next to consume
      // token buffer.
      self.return_tokenized(Token::Real(TokenBody::Defered(Span {
         index: self.index,
         pos_region: pos_start,
         pos_zero: self.pos_zero,
         pos_line: self.pos_line,
         line: line_start,
         length: len_defered
      })))
   }



   // Function that returns @include or any other instruction token coupled with
   // parenthesis right away. This is just to split code in more manageable
   // chunks.
   #[inline(always)]
   fn instruction_tokenize_correct_paren_now(&mut self,
      pos_at: usize, pos_start: usize, pos_open_paren: usize,
      ident_pos_end: usize, line_at: usize, line_start: usize,
      line_open_paren: usize, pos_last_linestart: usize
   )
      -> Option<Token>
   {
      #[cfg(feature = "dbg_tokenizer_verbose")]{
         println!("Tokenize paren now:");
      }

      #[cfg(not(feature = "unguarded_tokenizer_integrity"))] {
         let len_left = pos_start - self.pos_region;
         let pos_zero = self.pos_zero + len_left;

         if line_at != line_start {
            return Some(self.fail_token(Token::Fatal(
               ParseError::InstructionError(Source {
                  pos_zero: pos_zero,
                  component: Component::Tokenizer,
                  line: line!(),
                  code: 0,
               }
            ))));
         }
      }

      let len_instruction = ident_pos_end - pos_at + 1;
      let len_whitespace = pos_open_paren - ident_pos_end - 1;

      // Normally there should be no whitespaces, this is a slow code path and
      // it is executed only if user has bad template.
      if let Some(error_token) = self.whitespace_into_tokenbuf(self.index,
          ident_pos_end + 1, len_whitespace, line_at, line_open_paren
      ) {
         return Some(error_token);
      }

      let pos_line = pos_open_paren - pos_last_linestart;
      // When instruction is matched, we know that there is open
      // parenthesis available.
      if let Err(token) = self.tokenbuf_push(Token::Real(TokenBody::OpenParen(
         Span {
            index: self.index,
            pos_region: pos_open_paren,
            pos_zero: self.pos_zero + len_instruction + len_whitespace,
            pos_line: pos_line,
            line: line_open_paren,
            length: 1,
         }
      ))){
         return Some(token);
      }

      self.return_tokenized(Token::Real(TokenBody::Include(Span {
         index: self.index,
         pos_region: pos_start,
         pos_zero: self.pos_zero,
         pos_line: self.pos_line,
         line: line_at,
         length: len_instruction
      })))
   }



   #[inline(always)]
   fn instruction_tokenize_whitespace_before_instruction(&mut self,
      _pos_at: usize, _pos_start: usize, _pos_max: usize, _inf: usize,
      _pos_first_char: usize, _pos_last_char: usize, _pos_close_paren: usize,
      _pos_open_paren: usize, _pos_first_bad_char: usize,
      _pos_last_bad_char: usize, _line_at: usize, _line_start: usize,
      _line_open_paren: usize, _pos_last_linestart: usize
   )
      -> Option<Token>
   {
      // TODO: here we should match on possible identifier and return better
      // error/warning tokens; for now we give no extra information.

      // Based on DD-2023-07-07-01 return UnespacedAt.

      self.return_tokenized(Token::Real(TokenBody::UnescapedAt(Span {
         index: self.index, length: 1, pos_region: self.pos_region,
         pos_line: self.pos_line, pos_zero: self.pos_zero, line: self.line
      })))
   }



   // Tokenize what's inside (), when instruction with args is being tokenized.
   //
   // This state in a way is similar to Defered, just that open parenthesis are
   // being counted and when matching parenthesis are found, then Defered + 
   // CloseParen tokens are returned.
   #[inline(always)]
   fn tokenize_instruction_args(&mut self) -> Option<Token> {
      let src = &self.region[self.index];
      let pos_max = src.len();

      // TODO: check pos_line < pos_region, panic! if not, behind feature flag.

      let mut pos_line_start = self.pos_region - self.pos_line;
      let mut pos_token_start = self.pos_region;

      let mut line = self.line;
      let mut pos = self.pos_region;
      while pos < pos_max {
         match src[pos] {
            0x0A /* newline */ => {
               let pos_in_line = pos_token_start - pos_line_start;
               let len_defered = pos - pos_token_start;
               let len_prev_token = pos_token_start - self.pos_region;

               if let Err(token) = self.tokenbuf.append(Token::Real(
                  TokenBody::Defered(Span {
                     index: self.index,
                     pos_region: pos_token_start,
                     pos_zero: self.pos_zero + len_prev_token,
                     pos_line: pos_in_line,
                     line: line,
                     length: len_defered,
                  })
               )){
                  return Some(token);
               };

               if let Err(token) = self.tokenbuf.append(Token::Real(
                  TokenBody::Newline(Span {
                     index: self.index,
                     pos_region: pos,
                     pos_zero: self.pos_zero + len_prev_token + len_defered,
                     pos_line: pos_in_line + len_defered,
                     line: line,
                     length: 1,
                  })
               )){
                  return Some(token);
               };

               // Same as in defered_tokenize.
               line += 1;
               pos_line_start = pos + 1;
               pos_token_start = pos + 1;
            }

            // TODO: At the moment we ignore @ symbols, since we do not care
            // about them. What we could do is - warn if there are unescaped @
            // symbols? Actually we must handle @symbol escaping, since from
            // user's prespective it would be better to have same behavior
            // everywhere?

            0x28 /* ( */ => {
               self.cnt_openparen += 1;
            }
            0x29 /* ) */ => {
               self.cnt_closeparen += 1;

               if self.cnt_closeparen == self.cnt_openparen {
                  let pos_in_line = pos_token_start - pos_line_start;
                  let len_defered = pos - pos_token_start;
                  let len_prev_token = pos_token_start - self.pos_region;

                  self.state = TokenizerState::ExpectDefered;

                  if len_defered > 0 {
                     if let Err(token) = self.tokenbuf_push(Token::Real(TokenBody::Defered(Span {
                        index: self.index,
                        pos_region: pos_token_start,
                        pos_zero: self.pos_zero + len_prev_token,
                        pos_line: pos_in_line,
                        line: line,
                        length: len_defered,
                     }))) {
                        return Some(token);
                     };

                     if let Err(token) = self.tokenbuf_push(Token::Real(TokenBody::CloseParen(Span {
                        index: self.index,
                        pos_region: pos,
                        pos_zero: self.pos_zero + len_prev_token + len_defered,
                        pos_line: pos_in_line + len_defered,
                        line: line,
                        length: 1,
                     }))){
                        return Some(token);
                     };

                     // There is nothing that we can return, since returnable
                     // tokens are buffered.
                     return Some(Token::StateChange);
                  }
                  else {
                     return self.return_tokenized(Token::Real(TokenBody::CloseParen(Span {
                        index: self.index,
                        pos_region: pos,
                        pos_zero: self.pos_zero,
                        pos_line: pos_in_line,
                        line: line,
                        length: 1,
                     })));
                  }
               }
            }
            _ch => {
               #[cfg(feature = "dbg_tokenizer_verbose")]{
                  println!("non-special char pos: {}, char: 0x{:02X}, do nothing", pos, _ch);
               }
            }
         }

         pos += 1;
      }

      // Being here means that pos == pos_max. Code should not reach this point
      // unless there are no matching closing parenthesis. We do not try to
      // detect if any tokens are buffered. We just buffer more and return
      // state-chaged token.

      // There is Defered token available as well.
      if pos_token_start < pos {
         let pos_in_line = pos_token_start - pos_line_start;
         let len_prev_token = pos_token_start - self.pos_region;

         if let Err(token) = self.tokenbuf_push(Token::Real(TokenBody::Defered(
            Span {
               index: self.index,
               pos_region: pos_token_start,
               pos_zero: self.pos_zero + len_prev_token,
               pos_line: pos_in_line,
               line: line,
               length: pos - pos_token_start,
            }
         ))) {
            return Some(token);
         }
      }

      if let Err(token) = self.tokenbuf_push(Token::Error(
         ParseError::OpenInstruction(Source {
               pos_zero: self.pos_zero_prev_instr,
               component: Component::Tokenizer,
               line: line!(),
               code: 0,
         }))) {
         return Some(token);
      };

      self.state = TokenizerState::ExpectDefered;
      return Some(Token::StateChange);
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
      #[cfg(feature = "dbg_tokenizer_verbose")]{
         println!("INFO(Tokenizer): return_tokenized(1): {:?}", tok);
      }

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
            println!("INFO(Tokenizer): return_tokenized(2): {:?}", tok);
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
               if let ParseError::InternalError(..) = self.parse_error_prev { }
               else {
                  // We are already failing, if this fails as well, there is
                  // nothing we can do.
                  #[allow(unused_must_use)] {
                     self.tokenbuf_push(tok);
                  }
                  self.parse_error_prev = ParseError::InternalError(
                     Source {
                        component: Component::Tokenizer,
                        line: line!(),
                        code: 0,
                        pos_zero: self.pos_zero,
                     }
                  );
               }

               return Some(self.fail_token(Token::Fatal(
                  ParseError::InternalError(Source {
                     component: Component::Tokenizer,
                     line: line!(),
                     code: 0,
                     pos_zero: self.pos_zero,
                  })
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

               if let ParseError::InternalError(..) = self.parse_error_prev { }
               else {
                  // We are already failing, if this fails as well, there is
                  // nothing we can do.
                  #[allow(unused_must_use)] {
                     self.tokenbuf_push(tok);
                  }
                  self.parse_error_prev = ParseError::InternalError(
                     Source {
                        component: Component::Tokenizer,
                        line: line!(),
                        code: 0,
                        pos_zero: self.pos_zero,
                     }
                  );
               }

               return Some(self.fail_token(Token::Fatal(
                  ParseError::InternalError(Source {
                     component: Component::Tokenizer,
                     line: line!(),
                     code: 0,
                     pos_zero: self.pos_zero,
                  })
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
            | Token::Phantom(body)
            => match body {
               TokenBody::Newline(..) => {
                  self.line = line + 1;
                  self.pos_line = 0;
               }
               TokenBody::Include(span) => {
                  // switch into ExpectInstructionClose right away when instruction
                  // with expected partenthesis is returned. This is easier to
                  // implement, rather than switching into this state when
                  // OpenParen is returned.
                  // We can change this in future, if necessary.
                  self.state = TokenizerState::ExpectInstructionClose;
                  self.cnt_openparen = 0;
                  self.cnt_closeparen = 0;
                  self.pos_zero_prev_instr = span.pos_zero;
               }
               TokenBody::OpenParen(..) => {
                  self.cnt_openparen += 1;
               }
               _ => {}
            }
            _ => { }
         }

         // If token span goes over multiple regions, this case can happen.
         // We do not want to allow it for now, since it would mess up Tokenizer
         // interna state.
         if self.pos_region > self.pos_max {
            self.pos_region = self.pos_max;
            if let ParseError::InternalError(..) = self.parse_error_prev { }
            else {
               // We are already failing, if this fails as well, there is
               // nothing we can do.
               #[allow(unused_must_use)] {
                  self.tokenbuf_push(tok);
               }
               self.parse_error_prev = ParseError::InternalError(
                  Source {
                     component: Component::Tokenizer,
                     line: line!(),
                     code: 0,
                     pos_zero: self.pos_zero,
                  }
               );
            }

            return Some(self.fail_token(Token::Fatal(
               ParseError::InternalError(Source {
                  component: Component::Tokenizer,
                  line: line!(),
                  code: 0,
                  pos_zero: self.pos_zero,
               })
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
                     if let ParseError::InternalError(..) = self.parse_error_prev { }
                     else {
                        // We are already failing, if this fails as well, there
                        // is nothing we can do.
                        #[allow(unused_must_use)] {
                           self.tokenbuf_push(tok);
                        }
                        self.parse_error_prev = ParseError::InternalError(
                           Source {
                              component: Component::Tokenizer,
                              line: line!(),
                              code: 0,
                              pos_zero: self.pos_zero,
                           }
                        );
                     }

                     return Some(self.fail_token(Token::Fatal(
                        ParseError::InternalError(Source {
                           component: Component::Tokenizer,
                           line: line!(),
                           code: 0,
                           pos_zero: self.pos_zero,
                        })
                     )));
                  }
               }

               return Some(tok);
            }
            else {
               if let ParseError::InternalError(..) = self.parse_error_prev { }
               else {
                  // We are already failing, if this fails as well, there is
                  // nothing we can do.
                  #[allow(unused_must_use)] {
                     self.tokenbuf_push(tok);
                  }
                  self.parse_error_prev = ParseError::InternalError(
                     Source {
                        component: Component::Tokenizer,
                        line: line!(),
                        code: 0,
                        pos_zero: self.pos_zero,
                     }
                  );
               }

               return Some(self.fail_token(Token::Fatal(
                  ParseError::InternalError(Source {
                     component: Component::Tokenizer,
                     line: line!(),
                     code: 0,
                     pos_zero: self.pos_zero,
                  })
               )));
            }
         }
      }
      else {
         // TODO: Actually if we are returning token that is the last one in
         // region, we should pop the stack one region back...
         // if self.pos_region == self.pos_max => pop-stack
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
// # TODO
//
//   Should rewrite so that this function returns index for returned token as
//   well, because when more than expected tokens are returned it is hard to
//   understand where the error is, since expect None, got Token does not help.
//   Index would allow us to see, how many tokens matched.
#[cfg(all(test, not(feature = "tokenlist_match_or_fail_print_only")))]
fn tokenlist_match_or_fail(t: &mut Tokenizer, list: &[Token], allow_unbuffered: bool)
   -> Result<(), (Option<Token>, Option<Token>)>
{
   use ParseError as Pe;

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

      // We do not care if Tokenizer has changed state. We only care about
      // correct return tokens.
      if let Token::StateChange = token {
         continue;
      }

      // If there are expected items, compare if they match.
      if let Some(expect) = list.get(idx) {
         // println!("expected: {:?}, at idx: {}", expect, idx);

         // Yes i know, this looks like a mess, but what we want to achieve here
         // is that ParseErrors are compared on everything except line number,
         // since it changes too often.
         match (&token, expect) {
            (Token::Error(p1), Token::Error(p2))
            | (Token::Fatal(p1), Token::Fatal(p2))
            | (Token::Warning(p1), Token::Warning(p2))
            => match (p1, p2) {
               (Pe::NoMemory(s1), Pe::NoMemory(s2))
               | (Pe::InternalError(s1), Pe::InternalError(s2))
               | (Pe::OpenInstruction(s1), Pe::OpenInstruction(s2))
               | (Pe::InstructionError(s1), Pe::InstructionError(s2))
               | (Pe::InstructionNotOpen(s1), Pe::InstructionNotOpen(s2))
               | (Pe::InstructionMissingArgs(s1), Pe::InstructionMissingArgs(s2))
               | (Pe::UnwantedWhiteSpace(s1), Pe::UnwantedWhiteSpace(s2))
               => {
                  if s1.pos_zero != s2.pos_zero
                  || s1.component != s2.component
                  || s1.code != s2.code
                  {
                     return Err((Some((*expect).clone()), Some(token)));
                  }
               }
               _ => {
                  if *expect != token {
                     return Err((Some((*expect).clone()), Some(token)));
                  }
               }
            }
            (token, expect) => {
               if *expect != *token {
                  return Err((Some((*expect).clone()), Some((*token).clone())));
               }
            }
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
         if t.tokenbuf.buf_len() < 1 {
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

#[cfg(test)]
mod test_instruction;



// ================== EOF: do not write below this ============================
