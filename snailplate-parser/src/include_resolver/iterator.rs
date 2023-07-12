use crate::{
   include_resolver::{
      IncludeResolver,
      IncludeResolverState,
   },
   token::Token,
};



impl Iterator for IncludeResolver {
   type Item = Token;

   fn next(&mut self) -> Option<Self::Item> {
      use IncludeResolverState as S;

      match self.state {
         S::Passthrough => {
            self.next_passthrough()
         }
      }
   }
}