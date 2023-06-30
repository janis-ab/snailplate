# Design decission

When developing something, it is not possible to have all benefits without
costs. You either sacrifice memory for speed, or speed for memory, or speed for
security, etc.; some design decissions even sacrifice speed for less code. Since
decissions definiteley will be made, this document is a log to keep track about
reasoning and decission made.

<details>
<summary>DD-2023-06-30-01: TokenBuf as a separate struct</summary>
When i started to implement Tokenizer, i used simple Vec as a token buffer. It
turned out not to be a good enough idea. The problem with that approach is that
there is a function Tokenizer.tokenbuf_push which takes &mut Tokenizer, but in
some cases when analyzing input source we take `&self.region[index]` reference,
thus it makes it impossible to push any new token while read reference is held.
This is imposed by Rust's borrow checker.

Thus i have decided to refactor TokenBuf out as a separate struct. For now it
is not intended to be used by anything else but Tokenizer, thus will not move it
to separate module.

Now it is possible to access input source and tokenbuf at the same time using
```rust
let src = &self.region[index];
let tb = &mut self.tokenbuf;
```
</details>



<!-- EOF -->