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

      match self.tokenbuf.popleft() {
         Ok(None) => {},

         Ok(some_token) => {
            return some_token;
         }

         Err(token) => {
            return Some(token);
         }
      }

      match self.state {
         S::Passthrough => {
            self.next_passthrough()
         }

         S::ResolveInclude => {
            self.next_resolve_include()
         }

         S::Failed => {
            // TODO: here we should return some sort of error and remember that
            // we did, and only then return None
            None
         }
      }
   }
}