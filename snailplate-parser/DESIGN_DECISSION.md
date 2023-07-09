# Design decission

When developing something, it is not possible to have all benefits without
costs. You either sacrifice memory for speed, or speed for memory, or speed for
security, etc.; some design decissions even sacrifice speed for less code. Since
decissions definiteley will be made, this document is a log to keep track about
reasoning and decission made.



<details>
<summary>DD-2023-07-09-02: Code component and location into InternalError</summary>
When Tokenizer fails with InternalError, it is hard to find what the cause is
and from where the error is even emitted. So i've decided that it is necessary
to extend InternalError with extra arguments, that contain Component and 
source file line.

Using this we could see if error happened in Tokenizer, TokenBuf or any other
component. Then line number would allow us to search for relevant code, since
we know which files contain which component code.
</details>



<details>
<summary>DD-2023-07-09-01: Error handling within template sources</summary>
When there are some errors in template source that are related to instruction
parsing, for example "@include)(", wrong parenthesis, Tokenizer should continue
tokenization and return UnescapedAt + Defered tokens instead of failing
completeley.

This style is chosen, because i believe that bad input is not an error for
Tokenizer since a higher layer component could analyze those tokens and make
better suggestions. Since Tokenizer could not understand input, it switches back
to Defered token parsing state (it seems the most logical approach at the
moment).

Tokenizer is allowed to fail only when it has hit error related to it's state,
memory constraints, etc., but not because template source is bad.

In future maybe we could add limit, that if there are too many errors, then
Tokenizer is allowed to fail.
</details>



<details>
<summary>DD-2023-07-07-01: Unmatched instruction due to whitespace after @</summary>
This one seems really hard to decide. What would be correct tokenization result
when template contains string "@ include(path)".

We have decided upfront that we will not accept such an error in template
source. But the question now is, how do we tokenize this.

First returned token should be UnescapedAt, but what should be the following
tokens. At the moment i have two ideas:
1) Switch Tokenizer into ExpectDefered state and let it return whatever it does.
The drawback to this approach is that source text will be reparsed again. The
advantage is that from user's perspective tokenization is consistent.
2) Return whitespace tokens, and then defered token for anything that follows.
Advantage to this is that, since we have already parsed that data, there is no
need to do it again, the information is known and we return it in tokens.
Another advantage is that Phantom warning tokens can be exactly tied to Deref
token that could have been an instruction, if it really matched instruction
name.

OTOH this can be minimized, because Phantom token could be tied to UnescapedAt,
with positive offset in bytes. Thus achieving the same result with a bit more
information.

At the moment i have decided to use 1st approach, but later in future this
behavior could be changed. At the moment the reasonging is this - whitespaces
are not tokenized without reason, thus we keep them in Defered tokens. In
future when we will want to detect whitespaces after tag-start-closes, this
could change.

But what i do not want a the moment is - return whitespace Token for each
space between words, since this seems wasteful to RAM without any particular
gain. But what i would like to have though is - whitespace detection at line
start, and whitespace detection at line end.
</details>



<details>
<summary>DD-2023-07-01-01: Newline tokens</summary>
At first when i started out, i had decided to use only WhiteSpace token to match
all white spaces including newlines. This seemed a nice solution, since newlines
do not provide anything to tokenization and parsing (so i thought). Advantage to
this was that there are less tokens, thus less iterations, thus faster
tokenization. But to have user friendly error warnings, it is necessary to know
line number in specific file, thus counting newlines became a necessity. At first
implementation i did count newlines internally, but there is a problem with
Tokenizer.return_tokenized guards; it is not possible to update line number for
Tokenizer in an easy manner.

If we look at human readable HTML file that is a template, it is expected that
most tokens will not have newlines at all, even many WhiteSpace tags will not
contain newlines. At the moment the only Token that can contain newlines is
WhiteSpace. The inconvenience that WhiteSpace containint newlines creates is
that Tokenizer must know how many lines it must move forward once a Token is
consumed from buffer.

My considered solutions:

### 1) Add num_lines property in Span.
While this seems an easy solution, it seems wasteful towards RAM, since all
other tokens most probably will not have any newline at all. So it is expensive
to increase Span size for no reason at all.

### 2) We can store number of used lines per Token inside tokenbuf.
This is somewhat memory friendlier, since tokenbuf does not grow too big 
anyways. The drawback to this though is that token consumers do not know how
many lines returned Token consumes. It can be calculated in atleast two ways:
- iterate through token bytes and count "\n" characters,
- fetch next token and calculate diff between line properties.

### 3) Change TokenBody::WhiteSpace so that it tuple of Span and usize.
This seems wasteful, since whole TokenBody size will increase, thus there is
no gain if compared to scenario 1. Another drawback is that this creates
necessity to for specific Enum handling, thus more code.

### 4) Deactivate tokenbuf guards for WhiteSpace.
While tempting, this would decrease efficiency for guards to catch bugs. I would
like to avoid this.

### 5) Create a special Phantom token that represents line change.
This seems to be a wasteful approach in a sense that each newline that is
tokenized as WhiteSpace will have a follow-up token just to adjust line
number. Advantage of this approach is that this Phantom token is
returned only when one or more newlines were tokenized, thus non-newline
WhiteSpace tokens does not increase memory requirements. Another advantage is
that new line positioning token could be useful for other puproses, like
we could reason about indentation.

### 6) Create Newline token [CHOSEN]
Since we need to know line number changes, we might as well have a special
tokens for that. I can think of two ways how this could be done:
1) Have a special WhiteSpaceWithNl token, that contains only whitespaces,
and ends at "\n", thus it increases line number by 1.
2) Have a Newline only Token, that is matched always when newlines are matched
[CHOSEN].

If we vote for speed, then 1st way should be the way to go, since this would
produce less tokens. OTOH if we think about HTML as a template, there should not
be many whitespaces before the newline at the end of line. Considering this from
usability perspective, we might want to emit warnings if there are trailing 
whitespaces at the end before newlines; if we use Newline approach, then this
does not require any extra work on Tokens, since they already are, whereas in
WhiteSpaceWithNl case, we would have to analyze if WhiteSpaceWithNl contains
just newline characters or any other whitespaces.

Thus this decission now states that WhiteSpace token contains all white spaces
except newlines. Newline token contains newline symbols, that could be "\n" or
"\r\n" since some file encodings end lines like that.
</details>



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