use crate::{
   span::{
      Span, SpanFormatter
   },
   tokenizer::Tokenizer,
};



impl SpanFormatter for Tokenizer {
   fn fmt_into(&self, fmt: &mut std::fmt::Formatter, span: &Span) -> std::fmt::Result {
      let text = if let Some(slice) = self.span_slice(span) {
         match std::str::from_utf8(slice) {
               Ok(s) => {
                  let text = s.to_owned();
                  Some(text)
               }
               Err(..) => {
                  // TODO: what to do, return error?
                  // maybe show as binary slice instead since it is not 
                  // readable?
                  None
               }
         }
      }
      else {
         // TODO: IDK what could we write for text? Return error?
         None
      };

      let mut r = fmt.debug_struct("Span");
      r.field("index", &span.index);
      r.field("length", &span.length);
      r.field("pos_region", &span.pos_region);
      r.field("pos_line", &span.pos_line);
      r.field("pos_zero", &span.pos_zero);
      r.field("line", &span.line);

      if let Some(text) = text {
         r.field("text", &text);
      }

      r.finish()
   }
}