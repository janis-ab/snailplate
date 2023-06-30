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
         num_tokens: 0,
         tokenbuf: Vec::with_capacity(16),
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
      let tb = &mut self.tokenbuf;

      // Ensure that there is enough memory in Vec. This is because push will
      // panic if there is not enough capacity, but we do not want to panic on
      // that. There is an experimental API push_within_capacity, but i do not
      // want to use experimental API either.
      let cap = tb.capacity();
      let len = tb.len();
      if cap < len + 1 {
         if let Err(..) = tb.try_reserve(16) {
            return Err(self.fail_token(
               Token::Fatal(ParseError::NoMemory)
            ));
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
            return Err(self.fail_token(
               Token::Fatal(ParseError::InternalError)
            ));
         }
      }

      self.tokenbuf.push(tok);
      self.num_tokens += 1;

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

         return Err(self.fail_token(
            Token::Fatal(ParseError::TokenbufBroken(
               Span {
                  index: self.index, pos_region: self.pos_region,
                  pos_line: self.pos_line, pos_zero:self.pos_zero,
                  line: self.line, length: 0
               },
               msg, num_tokens_prev, len_prev
            ))
         ));
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

         return Err(self.fail_token(
            Token::Fatal(ParseError::TokenbufBroken(
               Span {
                  index: self.index, pos_region: self.pos_region,
                  pos_line: self.pos_line, pos_zero:self.pos_zero,
                  line: self.line, length: 0
               },
               msg, num_tokens_prev, len_prev
            ))
         ));
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

      Ok(Some(token))
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



   // Since every time when we return token, we must update Tokenizer positions,
   // it is better to have a function that does that for us, so that we do not
   // forget to update some fields.
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



// Function that checks if tokenizer returns tokens that match given list.
// On error, function returns (expected tken, token got)
fn tokenlist_match_or_fail(t: &mut Tokenizer, list: &Vec<Token>) 
   -> Result<(), (Option<Token>, Option<Token>)>
{
   let mut idx = 0;

   // This index is out of bounds in relative measure to expected list.
   let idx_oob = list.len();

   while let Some(token) = t.next() {
      if idx >= idx_oob {
         return Err((None, Some(token)));
      }

      if let Some(expect) = list.get(idx) {

         if *expect != token {
            return Err((Some((*expect).clone()), Some(token)));
         }
      }
      else {
         return Err((None, Some(token)));
      }

      idx += 1;
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
