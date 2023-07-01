use crate::span::{
   Span, SpanFormatter
};



use std::fmt;



/// Structure that describes token type and span.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TokenBody {
   /// This token is returned when @include is matched, it's span does not 
   /// envelop opening parenthesis, because we must allow parser to detect
   /// possible errors with forgoten opening parenthesis. This token envelops
   /// all spaces before and after include identifier, thus in worst case it
   /// can be for example like this: "@   include   ". In clean case this should
   /// be span overlaping "@include" exactly.
   Include(Span),

   /// This matches tag starts, for example, "<div" in HTML.
   TagOpenStart(Span),

   /// This matches ">" if it is possible to detect that it is an end for tag
   /// open.
   TagOpenEnd(Span),

   /// This matches the start of closing tag, example "</"
   TagCloseStart(Span),

   /// This matches tag closes, like "</div>" or "</>" or "/>" if it follows
   /// TagOpenStart.
   TagClose(Span),
   
   // This is @@, while we know that it's len is always 2, reuse same structure.
   EscapedAt(Span),

   /// This is a token that envelops span whose parsing is defered for later;
   /// This tag is used at first parsing stages, when there are elements, that
   /// shall be resolved later.
   Defered(Span),

   /// This matches "(" when expected for instructions, like @include. 
   /// Parenthesis are not matched as OpenParen when they are not expected
   /// due to tokenization mode sqithc requested by instructions like @include.
   OpenParen(Span),
   CloseParen(Span),

   /// When < matched without any known context. This could be an erroneous
   /// tag start, or unescaped &lt; within HTML body. Thus warning or error
   /// should be emitted.
   Lt(Span),
   Gt(Span),

   /// This matches characters that are considered to be whitespaces. For
   /// example " \t\r", but does not include newline. See DD-2023-07-01-01.
   WhiteSpace(Span),

   /// This matches newlines, usually "\n" or "\r\n". See DD-2023-07-01-01.
   Newline(Span),

   /// This token describes template file path for @include, @require directive.
   FilePath(Span),
}



impl TokenBody {
   pub fn fmt<'a, F: SpanFormatter>(&'a self, bufowner: &'a F) -> TokenBodyFormatWrapper<F> {
      TokenBodyFormatWrapper(self, bufowner)
   }

   pub fn span_clone(&self) -> Span {
      use TokenBody as Tb;

      match &self {
         Tb::Include(span)
         | Tb::TagOpenStart(span) 
         | Tb::TagOpenEnd(span) 
         | Tb::TagCloseStart(span) 
         | Tb::TagClose(span) 
         | Tb::EscapedAt(span) 
         | Tb::Defered(span) 
         | Tb::OpenParen(span) 
         | Tb::CloseParen(span) 
         | Tb::Lt(span) 
         | Tb::Gt(span) 
         | Tb::WhiteSpace(span) 
         | Tb::FilePath(span)       
         | Tb::Newline(span)
         => {
            let span_clone = *span;
            span_clone
         }
      }
   }
}



/// This is a convenience wrapper structure for situations when it is necessary
/// to debug token content. It should not be used directly.
///
/// Span stores only offsets for text that it overlays, but when debuging we
/// want to print text itself that was matched. To do it in cleaner syntax
/// we use this wrapper and implement fmt::Debug for it. This wrapper 
/// automatically binds buffer owner (that is SpanFormatter), thus when
/// fmt method is called, it knows from which buffer the data has to be taken.
pub struct TokenBodyFormatWrapper<'a, F: SpanFormatter> (&'a TokenBody, &'a F);



// The convenience of having this implemented in this manner is that now it is
// possible to use many different SpanFormatters and user does not have to think
// or pass through Span to tokenizer just to see the text when debugging.
// Another advantage in this is that, non-debugging code would not use this at
// all.
impl<'a, F: SpanFormatter> fmt::Debug for TokenBodyFormatWrapper<'a, F> {
   fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      use TokenBody as Tb;

      // Extract span out of token body.
      let span = self.0.span_clone();

      // Convert enum to text.
      let (start, end) = match self.0 {
         Tb::Include(..)
           => (Some("Include("), Some(")")),
         Tb::TagOpenStart(..) 
            => (Some("TagOpenStart("), Some(")")),
         Tb::TagOpenEnd(..) 
            => (Some("TagOpenEnd("), Some(")")),
         Tb::TagCloseStart(..) 
            => (Some("TagCloseStart("), Some(")")),
         Tb::TagClose(..) 
            => (Some("TagClose("), Some(")")),
         Tb::EscapedAt(..) 
            => (Some("EscapedAt("), Some(")")),
         Tb::Defered(..) 
            => (Some("Defered("), Some(")")),
         Tb::OpenParen(..) 
            => (Some("OpenParen("), Some(")")),
         Tb::CloseParen(..) 
            => (Some("CloseParen("), Some(")")),
         Tb::Lt(..) 
            => (Some("Lt("), Some(")")),
         Tb::Gt(..) 
            => (Some("Gt("), Some(")")),
         Tb::WhiteSpace(..) 
            => (Some("WhiteSpace("), Some(")")),
         Tb::FilePath(..) 
            => (Some("FilePath("), Some(")")),
         Tb::Newline(..) 
            => (Some("Newline("), Some(")")),
      };

      if let Some(start) = start {
         if let Err(e) = f.write_str(start) {
            return Err(e);
         }
      }

      if let Err(e) = self.1.fmt_into(f, &span) {
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
pub(crate) mod test;