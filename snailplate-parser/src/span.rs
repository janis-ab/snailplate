use std::fmt;

/// A structure that describes a slice of a region that spans over specific
/// boundaries.
///
/// Span is used to describe tokens, such as tags, braces, etc., regions such
/// as double quoted strings, tag argument lists, etc.
#[derive(Debug,Clone,Copy)]
pub struct Span {
   /// Index for buffer used by Tokenizer to store Span data. It is possible 
   // that Span goes over multiple regions, in those cases index describes the
   /// location where Span starts, and length overflows/spills into next region.
   pub index: usize,

   /// This is a Span position within region counted by raw bytes, encoding
   /// is not considered in any way.
   pub pos_region: usize,

   /// This is a Span position relative to line start. This is used to detect
   /// if tags are at the same depth from users perspective. This should be
   /// helpful for some error detection situations.
   pub pos_line: usize,

   /// This is a Span position relative to the first region.
   pub pos_zero: usize,

   /// Line number into given file where the span starts.
   pub line: usize,

   /// Bytes used for Span. This is not a count of characters based on input
   /// encoding. This is a count of raw bytes.
   pub length: usize,
}



pub trait SpanFormatter {
   fn fmt_into(&self, fmt: &mut fmt::Formatter, span: &Span) -> fmt::Result;
}


