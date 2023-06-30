// In templating sense, identifiers are words that follow @ symbol. For example:
// include, require, if, end, foreach, etc.
//
// Exactly matched identifier is a tuple in shape: (pos_start, pos_end).
// pos_at is not necessary, because exact Ident requires that there is no
// space after @, thus pos_at is pos_start - 1. line_at is not necessary,
// because it will be the same line number as for Ident symbol. In Ident sense
// they are relative to current region being tokenized.
//
#[derive(Debug)]
pub(super) enum Ident {
   // This Ident is matched for "@include(", "@include  \n  (", but not for
   // "@   include(".
   Include(usize, usize),

   // TODO: create AlmostInclude an Ident that has correct word, but bad 
   // character case. We shall emit warning for those.

   // TODO: create MaybeInclude an Ident that has some bad characters, but is
   // very close to @include. Emit warning and suggestion how to fix it: either
   // correct ident or escape @. This could match "@   include(" and friends.

   // Slice is not matched as identifier.
   None
}



// Function that tries to match identifier. If it returns None, then this means
// that given text could not be matched as identifier. None in a way could be
// interpreted as Illegal.
// start - position (inclusive) in buffer, from where should the ident be matched
// end - position (inclusive) in buffer, till which identifier should be matched.
#[inline(always)]
pub(super) fn ident_match(src: &[u8], start: usize, end: usize) -> Ident {
   // Caller should ensure that ident_match is called with correct parameters.
   // From ident_match perspective, it is not possible to match identifier
   // whose end is after start.
   if end < start {
      return Ident::None;
   }

   let len = end - start + 1;

   if len == 7 {
      return ident_match_7(src, start, end);
   }

   // TODO: implement identifier matching for other lengths and identifiers
   // when available. Implement matching for almost-correct idents as well.
   // For now we must move forward, thus poor-matching is implemented.

   Ident::None
}



// Identifier matching when there are exactly 7 bytes available.
#[inline(always)]
fn ident_match_7(src: &[u8], start: usize, end: usize) -> Ident {
   let ident = &src[start..end + 1];

   match ident[0] {
      0x69 /* i */ => {
         match ident[1] {
            0x6E /* n */ => {
               /* match 'clude' */
               if ident[2] == 0x63 /* c */
               && ident[3] == 0x6C /* l */
               && ident[4] == 0x75 /* u */
               && ident[5] == 0x64 /* d */
               && ident[6] == 0x65 /* e */
               {
                  Ident::Include(start, end)
               }
               else {
                  Ident::None
               }
            }
            _ => {
               Ident::None
            }
         }
      }

      _ => {
         Ident::None
      }
   }
}


